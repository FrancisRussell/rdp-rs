use std::convert::TryFrom;
use std::io::{Cursor, Read};

use num_enum::TryFromPrimitive;

use crate::model::data::{Check, Component, DataType, DynOption, Message, MessageOption, U16, U32};
use crate::model::error::{Error, RdpError, RdpErrorKind, RdpResult};

#[derive(Debug)]
pub enum LicenseMessage {
    NewLicense,
    ErrorAlert(Component),
}

/// License preamble
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/73170ca2-5f82-4a2d-9d1b-b439f3d8dadc
#[repr(u8)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum Preamble {
    Version20 = 0x2,
    Version30 = 0x3,
}

/// All type of message
/// which can follow a license preamble
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/73170ca2-5f82-4a2d-9d1b-b439f3d8dadc
#[repr(u8)]
#[derive(Clone, Copy, Debug, TryFromPrimitive)]
pub enum MessageType {
    LicenseRequest = 0x01,
    PlatformChallenge = 0x02,
    NewLicense = 0x03,
    UpgradeLicense = 0x04,
    LicenseInfo = 0x12,
    NewLicenseRequest = 0x13,
    PlatformChallengeResponse = 0x15,
    ErrorAlert = 0xFF,
}

/// Error code of the license automata
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/f18b6c9f-f3d8-4a0e-8398-f9b153233dca?redirectedfrom=MSDN
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum ErrorCode {
    ErrInvalidServerCertificate = 0x0000_0001,
    ErrNoLicense = 0x0000_0002,
    ErrInvalidScope = 0x0000_0004,
    ErrNoLicenseServer = 0x0000_0006,
    StatusValidClient = 0x0000_0007,
    ErrInvalidClient = 0x0000_0008,
    ErrInvalidProductid = 0x0000_000B,
    ErrInvalidMessageLen = 0x0000_000C,
    ErrInvalidMac = 0x0000_0003,
}

/// All valid state transition available
/// for license automata
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/f18b6c9f-f3d8-4a0e-8398-f9b153233dca
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum StateTransition {
    TotalAbort = 0x0000_0001,
    NoTransition = 0x0000_0002,
    ResetPhaseToStart = 0x0000_0003,
    ResendLastMessage = 0x0000_0004,
}

/// This a license preamble
/// All license messages are built in same way
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/73170ca2-5f82-4a2d-9d1b-b439f3d8dadc
fn preamble() -> Component {
    component![
        "bMsgtype" => 0_u8,
        "flag" => Check::new(Preamble::Version30 as u8),
        "wMsgSize" => DynOption::new(U16::LE(0), |size| MessageOption::Size("message".to_string(), size.inner() as usize - 4)),
        "message" => Vec::<u8>::new()
    ]
}

/// Blob use by licensing protocol
fn license_binary_blob() -> Component {
    component![
        "wBlobType" => U16::LE(0),
        "wBlobLen" => DynOption::new(U16::LE(0), | size | MessageOption::Size("blobData".to_string(), size.inner() as usize)),
        "blobData" => Vec::<u8>::new()
    ]
}

/// Licensing error message
/// use to inform state transition
fn licensing_error_message() -> Component {
    component![
        "dwErrorCode" => U32::LE(0),
        "dwStateTransition" => U32::LE(0),
        "blob" => license_binary_blob()
    ]
}

/// Parse a payload that follow an preamble
/// Actually we only accept payload with type `NewLicense` or `ErrorAlert`
fn parse_payload(payload: &Component) -> RdpResult<LicenseMessage> {
    match MessageType::try_from(cast!(DataType::U8, payload["bMsgtype"])?)? {
        MessageType::NewLicense => Ok(LicenseMessage::NewLicense),
        MessageType::ErrorAlert => {
            let mut message = licensing_error_message();
            let mut stream = Cursor::new(cast!(DataType::Slice, payload["message"])?);
            message.read(&mut stream)?;
            Ok(LicenseMessage::ErrorAlert(message))
        }
        _ => Err(Error::RdpError(RdpError::new(RdpErrorKind::NotImplemented, "Licensing nego not implemented"))),
    }
}

/// A license client side connect message
///
/// Actually we only accept valid client message
/// without any license negotiation
///
/// # Example
/// ```
/// ```
pub fn client_connect(s: &mut dyn Read) -> RdpResult<()> {
    let mut license_message = preamble();
    license_message.read(s)?;

    match parse_payload(&license_message)? {
        LicenseMessage::NewLicense => Ok(()),
        LicenseMessage::ErrorAlert(blob) => {
            if ErrorCode::try_from(cast!(DataType::U32, blob["dwErrorCode"])?)? == ErrorCode::StatusValidClient
                && StateTransition::try_from(cast!(DataType::U32, blob["dwStateTransition"])?)?
                    == StateTransition::NoTransition
            {
                Ok(())
            } else {
                Err(Error::RdpError(RdpError::new(
                    RdpErrorKind::InvalidRespond,
                    "Server reject license, Actually license nego is not implemented",
                )))
            }
        }
    }
}
