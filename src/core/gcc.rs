use std::collections::HashMap;
use std::io::{Cursor, Read};

use crate::core::per;
use crate::model::data::{
    to_vec, Array, Check, Component, DataType, DynOption, Message, MessageOption, Trame, U16, U32,
};
use crate::model::error::{Error, RdpError, RdpErrorKind, RdpResult};
use crate::model::unicode::Unicode;

const T124_02_98_OID: [u8; 6] = [0, 0, 20, 124, 0, 1];
const H221_CS_KEY: [u8; 4] = *b"Duca";
const H221_SC_KEY: [u8; 4] = *b"McDn";
/// RDP protocol version
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/00f1da4a-ee9c-421a-852f-c19f92343d73?redirectedfrom=MSDN
#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Version {
    RdpVersion = 0x0008_0001,
    RdpVersion5plus = 0x0008_0004,
    Unknown,
}

impl From<u32> for Version {
    fn from(e: u32) -> Self {
        match e {
            0x0008_0001 => Version::RdpVersion5plus,
            0x0008_0004 => Version::RdpVersion,
            _ => Version::Unknown,
        }
    }
}

/// Color depth
/// This flag is deprecated
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/00f1da4a-ee9c-421a-852f-c19f92343d73?redirectedfrom=MSDN
#[repr(u16)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum ColorDepth {
    RnsUdColor8BPP = 0xCA01,
    RnsUdColor16BPP555 = 0xCA02,
    RnsUdColor16BPP565 = 0xCA03,
    RnsUdColor24BPP = 0xCA04,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug)]
enum Sequence {
    RnsUdSasDel = 0xAA03,
}

/// Keyboard layout
/// https://docs.microsoft.com/en-us/previous-versions/windows/it-pro/windows-vista/cc766503(v=ws.10)?redirectedfrom=MSDN
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum KeyboardLayout {
    Arabic = 0x0000_0401,
    Bulgarian = 0x0000_0402,
    ChineseUsKeyboard = 0x0000_0404,
    Czech = 0x0000_0405,
    Danish = 0x0000_0406,
    German = 0x0000_0407,
    Greek = 0x0000_0408,
    US = 0x0000_0409,
    Spanish = 0x0000_040a,
    Finnish = 0x0000_040b,
    French = 0x0000_040c,
    Hebrew = 0x0000_040d,
    Hungarian = 0x0000_040e,
    Icelandic = 0x0000_040f,
    Italian = 0x0000_0410,
    Japanese = 0x0000_0411,
    Korean = 0x0000_0412,
    Dutch = 0x0000_0413,
    Norwegian = 0x0000_0414,
}

/// Keyboard type
/// `Ibm101102Keys` is the most common keyboard type
#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum KeyboardType {
    IbmPcXt83Key = 0x0000_0001,
    Olivetti = 0x0000_0002,
    IbmPcAt84Key = 0x0000_0003,
    Ibm101102Keys = 0x0000_0004,
    Nokia1050 = 0x0000_0005,
    Nokia9140 = 0x0000_0006,
    Japanese = 0x0000_0007,
}

#[repr(u16)]
#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
enum HighColor {
    HighColor4BPP = 0x0004,
    HighColor8BPP = 0x0008,
    HighColor15BPP = 0x000f,
    HighColor16BPP = 0x0010,
    HighColor24BPP = 0x0018,
}

/// Supported color depth
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/00f1da4a-ee9c-421a-852f-c19f92343d73?redirectedfrom=MSDN
#[repr(u16)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum Support {
    RnsUd24BPP = 0x0001,
    RnsUd16BPP = 0x0002,
    RnsUd15BPP = 0x0004,
    RnsUd32BPP = 0x0008,
}

/// Negotiation of some capability for pdu layer
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/00f1da4a-ee9c-421a-852f-c19f92343d73?redirectedfrom=MSDN
#[repr(u16)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum CapabilityFlag {
    RnsUdCsSupportErrinfoPDU = 0x0001,
    RnsUdCsWant32BPPSession = 0x0002,
    RnsUdCsSupportStatusInfoPdu = 0x0004,
    RnsUdCsStrongAsymmetricKeys = 0x0008,
    RnsUdCsUnused = 0x0010,
    RnsUdCsValidConnectionType = 0x0020,
    RnsUdCsSupportMonitorLayoutPDU = 0x0040,
    RnsUdCsSupportNetcharAutodetect = 0x0080,
    RnsUdCsSupportDynvcGFXProtocol = 0x0100,
    RnsUdCsSupportDynamicTimezone = 0x0200,
    RnsUdCsSupportHeartbeatPDU = 0x0400,
}

/// Supported encryption method
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/6b58e11e-a32b-4903-b736-339f3cfe46ec?redirectedfrom=MSDN
#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum EncryptionMethod {
    EncryptionFlag40bit = 0x0000_0001,
    EncryptionFlag128bit = 0x0000_0002,
    EncryptionFlag56bit = 0x0000_0008,
    FipsEncryptionFlag = 0x0000_0010,
}

/// Encryption level
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/3e86b68d-3e2e-4433-b486-878875778f4b?redirectedfrom=MSDN
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum EncryptionLevel {
    None = 0x0000_0000,
    Low = 0x0000_0001,
    ClientCompatible = 0x0000_0002,
    High = 0x0000_0003,
    Fips = 0x0000_0004,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MessageType {
    //server -> client
    ScCore = 0x0C01,
    ScSecurity = 0x0C02,
    ScNet = 0x0C03,
    //client -> server
    CsCore = 0xC001,
    CsSecurity = 0xC002,
    CsNet = 0xC003,
    CsCluster = 0xC004,
    CsMonitor = 0xC005,
    Unknown = 0,
}

impl From<u16> for MessageType {
    fn from(e: u16) -> Self {
        match e {
            0x0C01 => MessageType::ScCore,
            0x0C02 => MessageType::ScSecurity,
            0x0C03 => MessageType::ScNet,
            0xC001 => MessageType::CsCore,
            0xC002 => MessageType::CsSecurity,
            0xC003 => MessageType::CsNet,
            0xC004 => MessageType::CsCluster,
            0xC005 => MessageType::CsMonitor,
            _ => MessageType::Unknown,
        }
    }
}

/// In case of client
/// This is all mandatory fields need by client core data
#[derive(Clone, Debug)]
pub struct ClientData {
    pub width: u16,
    pub height: u16,
    pub layout: KeyboardLayout,
    pub server_selected_protocol: u32,
    pub rdp_version: Version,
    pub name: String,
}

/// This is the first client specific data
///
/// This field are obsolete and for modern
/// RDP they are not use
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/00f1da4a-ee9c-421a-852f-c19f92343d73?redirectedfrom=MSDN
pub fn client_core_data(parameter: Option<ClientData>) -> Component {
    let client_parameter = parameter.unwrap_or(ClientData {
        width: 0,
        height: 0,
        layout: KeyboardLayout::French,
        server_selected_protocol: 0,
        rdp_version: Version::RdpVersion5plus,
        name: String::new(),
    });

    let client_name = if client_parameter.name.len() >= 16 {
        client_parameter.name[0..16].to_string()
    } else {
        client_parameter.name.clone() + &"\x00".repeat(16 - client_parameter.name.len())
    };

    component![
        "version" => U32::LE(client_parameter.rdp_version as u32),
        "desktopWidth" => U16::LE(client_parameter.width),
        "desktopHeight" => U16::LE(client_parameter.height),
        "colorDepth" => U16::LE(ColorDepth::RnsUdColor8BPP as u16),
        "sasSequence" => U16::LE(Sequence::RnsUdSasDel as u16),
        "kbdLayout" => U32::LE(client_parameter.layout as u32),
        "clientBuild" => U32::LE(3790),
        "clientName" => client_name.to_string().to_utf16_le(),
        "keyboardType" => U32::LE(KeyboardType::Ibm101102Keys as u32),
        "keyboardSubType" => U32::LE(0),
        "keyboardFnKeys" => U32::LE(12),
        "imeFileName" => vec![0_u8; 64],
        "postBeta2ColorDepth" => U16::LE(ColorDepth::RnsUdColor8BPP as u16),
        "clientProductId" => U16::LE(1),
        "serialNumber" => U32::LE(0),
        "highColorDepth" => U16::LE(HighColor::HighColor24BPP as u16),
        "supportedColorDepths" => U16::LE(
            //Support::RnsUd15BPP as u16 |
            Support::RnsUd16BPP as u16 |
            //Support::RnsUd24BPP as u16 |
            Support::RnsUd32BPP as u16
            ),
        "earlyCapabilityFlags" => U16::LE(CapabilityFlag::RnsUdCsSupportErrinfoPDU as u16),
        "clientDigProductId" => vec![0; 64],
        "connectionType" => 0_u8,
        "pad1octet" => 0_u8,
        "serverSelectedProtocol" => U32::LE(client_parameter.server_selected_protocol)
    ]
}

pub fn server_core_data() -> Component {
    component![
        "rdpVersion" => U32::LE(0),
        "clientRequestedProtocol" => Some(U32::LE(0)),
        "earlyCapabilityFlags" => Some(U32::LE(0))
    ]
}

/// Client security releated to deprecated RDP security layer
/// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/6b58e11e-a32b-4903-b736-339f3cfe46ec?redirectedfrom=MSDN
pub fn client_security_data() -> Component {
    component![
        "encryptionMethods" => U32::LE(
            EncryptionMethod::EncryptionFlag40bit as u32 |
            EncryptionMethod::EncryptionFlag56bit as u32 |
            EncryptionMethod::EncryptionFlag128bit as u32
         ),
        "extEncryptionMethods" => U32::LE(0)
    ]
}

/// In case of non ssl security layer
/// we need to check data in this packet
pub fn server_security_data() -> Component {
    component![
        "encryptionMethod" => U32::LE(0),
        "encryptionLevel" => U32::LE(0)
    ]
}

/// Actually we have no more classic channel
pub fn channel_def(name: &String, options: u32) -> Component {
    component![
        "name"=> name.as_bytes().to_vec(),
        "options" => U32::LE(options)
    ]
}

/// Actually we have no more channel than the classic one
pub fn client_network_data(channel_def_array: Trame) -> Component {
    component![
        "channelCount" => U32::LE(channel_def_array.len() as u32),
        "channelDefArray" => to_vec(&channel_def_array)
    ]
}

pub fn server_network_data() -> Component {
    component![
        "MCSChannelId" => Check::new(U16::LE(1003)),
        "channelCount" => DynOption::new(U16::LE(0), |count| MessageOption::Size("channelIdArray".to_string(), count.inner() as usize * 2)),
        "channelIdArray" => Array::new(|| U16::LE(0))
    ]
}

pub fn block_header(data_type: Option<MessageType>, length: Option<u16>) -> Component {
    component![
        "type" => U16::LE(data_type.unwrap_or(MessageType::CsCore) as u16),
        "length" => U16::LE(length.unwrap_or(0) + 4)
    ]
}

pub fn write_conference_create_request(user_data: &[u8]) -> RdpResult<Vec<u8>> {
    let mut result = Cursor::new(vec![]);
    per::write_choice(0, &mut result)?;
    per::write_object_identifier(&T124_02_98_OID, &mut result)?;
    per::write_length(user_data.len() as u16 + 14)?.write(&mut result)?;
    per::write_choice(0, &mut result)?;
    per::write_selection(0x08, &mut result)?;
    per::write_numeric_string(b"1", 1, &mut result)?;
    per::write_padding(1, &mut result)?;
    per::write_number_of_set(1, &mut result)?;
    per::write_choice(0xc0, &mut result)?;
    per::write_octet_stream(&H221_CS_KEY, 4, &mut result)?;
    per::write_octet_stream(user_data, 0, &mut result)?;
    Ok(result.into_inner())
}

#[derive(Clone, Debug)]
pub struct ServerData {
    pub channel_ids: Vec<u16>,
    pub rdp_version: Version,
}

/// Read conference create response
pub fn read_conference_create_response(cc_response: &mut dyn Read) -> RdpResult<ServerData> {
    per::read_choice(cc_response)?;
    per::read_object_identifier(&T124_02_98_OID, cc_response)?;
    per::read_length(cc_response)?;
    per::read_choice(cc_response)?;
    per::read_integer_16(1001, cc_response)?;
    per::read_integer(cc_response)?;
    per::read_enumerates(cc_response)?;
    per::read_number_of_set(cc_response)?;
    per::read_choice(cc_response)?;
    per::read_octet_stream(&H221_SC_KEY, 4, cc_response)?;

    let length = per::read_length(cc_response)?;
    let mut result = HashMap::new();
    let mut sub = cc_response.take(u64::from(length));
    loop {
        let mut header = block_header(None, None);
        // No more blocks to read
        if header.read(&mut sub).is_err() {
            break;
        }

        let mut buffer = vec![0_u8; (cast!(DataType::U16, header["length"])? - header.length() as u16) as usize];
        sub.read_exact(&mut buffer)?;

        match MessageType::from(cast!(DataType::U16, header["type"])?) {
            MessageType::ScCore => {
                let mut server_core = server_core_data();
                server_core.read(&mut Cursor::new(buffer))?;
                result.insert(MessageType::ScCore, server_core);
            }
            MessageType::ScSecurity => {
                let mut server_security = server_security_data();
                server_security.read(&mut Cursor::new(buffer))?;
                result.insert(MessageType::ScSecurity, server_security);
            }
            MessageType::ScNet => {
                let mut server_net = server_network_data();
                server_net.read(&mut Cursor::new(buffer))?;
                result.insert(MessageType::ScNet, server_net);
            }
            _ => println!("GCC: Unknown server block {:?}", cast!(DataType::U16, header["type"])?),
        }
    }

    // All section are important
    Ok(ServerData {
        channel_ids: cast!(DataType::Trame, result[&MessageType::ScNet]["channelIdArray"])?
            .iter()
            .map(|x| cast!(DataType::U16, x).unwrap())
            .collect(),
        rdp_version: Version::from(cast!(DataType::U32, result[&MessageType::ScCore]["rdpVersion"])?),
    })
}
