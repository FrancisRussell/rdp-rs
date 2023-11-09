use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt as _};

use crate::model::error::{Error, RdpError, RdpErrorKind, RdpResult};

/// All this decompression code is directly inspired from the source code of
/// rdesktop and directly ported to Rust.

fn process_plane(input: &mut Cursor<&[u8]>, width: u32, height: u32, output: &mut [u8]) -> RdpResult<()> {
    let mut last_line: u32 = 0;

    for indexh in 0..height {
        let mut out = (height - indexh - 1) * width * 4;
        let this_line = out;
        let mut indexw = 0;
        if last_line == 0 {
            let mut color = 0u8;
            while indexw < width {
                let code = input.read_u8()?;
                let mut replen = code & 0xf;
                let mut collen = (code >> 4) & 0xf;
                let revcode = (replen << 4) | collen;
                if (16..=47).contains(&revcode) {
                    replen = revcode;
                    collen = 0;
                }
                while collen > 0 {
                    color = input.read_u8()?;
                    output[out as usize] = color;
                    out += 4;
                    indexw += 1;
                    collen -= 1;
                }
                while replen > 0 {
                    output[out as usize] = color;
                    out += 4;
                    indexw += 1;
                    replen -= 1;
                }
            }
        } else {
            let mut color: i8 = 0;
            while indexw < width {
                let code = input.read_u8()?;
                let mut replen = code & 0xf;
                let mut collen = (code >> 4) & 0xf;
                let revcode = (replen << 4) | collen;
                if (16..=47).contains(&revcode) {
                    replen = revcode;
                    collen = 0;
                }
                while collen > 0 {
                    let x = input.read_u8()?;
                    color = if x & 1 != 0 { -i32::from((x >> 1) + 1) as i8 } else { (x >> 1) as i8 };
                    let v = (i32::from(output[(last_line + (indexw * 4)) as usize]) + i32::from(color)) as u8;
                    output[out as usize] = v;
                    out += 4;
                    indexw += 1;
                    collen -= 1;
                }
                while replen > 0 {
                    let v = (i32::from(output[(last_line + (indexw * 4)) as usize]) + i32::from(color)) as u8;
                    output[out as usize] = v;
                    out += 4;
                    indexw += 1;
                    replen -= 1;
                }
            }
        }
        last_line = this_line;
    }
    Ok(())
}

/// Run length encoding decoding function for 32 bpp
pub fn rle_32_decompress(input: &[u8], width: u32, height: u32, output: &mut [u8]) -> RdpResult<()> {
    let mut input_cursor = Cursor::new(input);

    if input_cursor.read_u8()? != 0x10 {
        return Err(Error::RdpError(RdpError::new(RdpErrorKind::RleDecode, "Bad header")));
    }

    process_plane(&mut input_cursor, width, height, &mut output[3..])?;
    process_plane(&mut input_cursor, width, height, &mut output[2..])?;
    process_plane(&mut input_cursor, width, height, &mut output[1..])?;
    process_plane(&mut input_cursor, width, height, &mut output[0..])?;
    Ok(())
}

macro_rules! repeat {
    ($expr:expr, $count:expr, $x:expr, $width:expr) => {
        while (($count & !0x7) != 0) && ($x + 8) < $width {
            $expr;
            $count -= 1;
            $x += 1;
            $expr;
            $count -= 1;
            $x += 1;
            $expr;
            $count -= 1;
            $x += 1;
            $expr;
            $count -= 1;
            $x += 1;
            $expr;
            $count -= 1;
            $x += 1;
            $expr;
            $count -= 1;
            $x += 1;
            $expr;
            $count -= 1;
            $x += 1;
            $expr;
            $count -= 1;
            $x += 1;
        }
        while $count > 0 && $x < $width {
            $expr;
            $count -= 1;
            $x += 1;
        }
    };
}

pub fn rle_16_decompress(input: &[u8], width: usize, mut height: usize, output: &mut [u16]) -> RdpResult<()> {
    let mut input_cursor = Cursor::new(input);

    let mut lastopcode: u8 = 0xFF;
    let mut insertmix = false;
    let mut x: usize = width;
    let mut prevline: Option<usize> = None;
    let mut line: Option<usize> = None;
    let mut colour1 = 0;
    let mut colour2 = 0;
    let mut mix = 0xffff;
    let mut mask: u8 = 0;
    let mut bicolour = false;

    while (input_cursor.position() as usize) < input.len() {
        let mut fom_mask = 0;
        let code = input_cursor.read_u8()?;
        let mut opcode = code >> 4;

        let (mut count, offset): (usize, usize) = match opcode {
            0xC..=0xE => {
                opcode -= 6;
                (usize::from(code & 0xf), 16)
            }
            0xF => {
                opcode = code & 0xf;
                let count = if opcode < 9 {
                    input_cursor.read_u16::<LittleEndian>()?
                } else if opcode < 0xb {
                    8
                } else {
                    1
                };
                (usize::from(count), 0)
            }
            _ => {
                opcode >>= 1;
                (usize::from(code & 0x1f), 32)
            }
        };

        if offset != 0 {
            let isfillormix = (opcode == 2) || (opcode == 7);
            if count == 0 {
                count = usize::from(input_cursor.read_u8()?) + if isfillormix { 1 } else { offset };
            } else if isfillormix {
                count <<= 3;
            }
        }

        match opcode {
            0 => {
                if lastopcode == opcode && !(x == width && prevline.is_none()) {
                    insertmix = true;
                }
            }
            8 => {
                colour1 = input_cursor.read_u16::<LittleEndian>()?;
                colour2 = input_cursor.read_u16::<LittleEndian>()?;
            }
            3 => {
                colour2 = input_cursor.read_u16::<LittleEndian>()?;
            }
            6 | 7 => {
                mix = input_cursor.read_u16::<LittleEndian>()?;
                opcode -= 5;
            }
            9 => {
                mask = 0x03;
                opcode = 0x02;
                fom_mask = 3;
            }
            0xa => {
                mask = 0x05;
                opcode = 0x02;
                fom_mask = 5;
            }
            _ => (),
        }
        lastopcode = opcode;
        let mut mixmask = 0;

        while count > 0 {
            if x >= width {
                if height == 0 {
                    return Err(Error::RdpError(RdpError::new(
                        RdpErrorKind::RleDecode,
                        "count > 0 but all values already written during rle_16 decompress",
                    )));
                }
                x = 0;
                height -= 1;
                prevline = line;
                line = Some(height * width);
            }
            let line = line.ok_or_else(|| {
                Error::RdpError(RdpError::new(RdpErrorKind::RleDecode, "line unset during rle_16 decompress"))
            })?;

            match opcode {
                0 => {
                    if insertmix {
                        output[line + x] = if let Some(e) = prevline { output[e + x] ^ mix } else { mix };
                        insertmix = false;
                        count -= 1;
                        x += 1;
                    }

                    if let Some(e) = prevline {
                        repeat!(output[line + x] = output[e + x], count, x, width);
                    } else {
                        repeat!(output[line + x] = 0, count, x, width);
                    }
                }
                1 => {
                    if let Some(e) = prevline {
                        repeat!(output[line + x] = output[e + x] ^ mix, count, x, width);
                    } else {
                        repeat!(output[line + x] = mix, count, x, width);
                    }
                }
                2 => {
                    if let Some(e) = prevline {
                        repeat!(
                            {
                                mixmask <<= 1;
                                if mixmask == 0 {
                                    mask = if fom_mask != 0 { fom_mask } else { input_cursor.read_u8()? };
                                    mixmask = 1;
                                }
                                output[line + x] =
                                    if (mask & mixmask) != 0 { output[e + x] ^ mix } else { output[e + x] };
                            },
                            count,
                            x,
                            width
                        );
                    } else {
                        repeat!(
                            {
                                mixmask <<= 1;
                                if mixmask == 0 {
                                    mask = if fom_mask != 0 { fom_mask } else { input_cursor.read_u8()? };
                                    mixmask = 1;
                                }
                                output[line + x] = if (mask & mixmask) != 0 { mix } else { 0 };
                            },
                            count,
                            x,
                            width
                        );
                    }
                }
                3 => {
                    repeat!(output[line + x] = colour2, count, x, width);
                }
                4 => {
                    repeat!(output[line + x] = input_cursor.read_u16::<LittleEndian>()?, count, x, width);
                }
                8 => {
                    repeat!(
                        {
                            (output[line + x], bicolour) = if bicolour {
                                (colour2, false)
                            } else {
                                count += 1;
                                (colour1, true)
                            };
                        },
                        count,
                        x,
                        width
                    );
                }
                0xd => {
                    repeat!(output[line + x] = 0xffff, count, x, width);
                }
                0xe => {
                    repeat!(output[line + x] = 0, count, x, width);
                }
                _ => return Err(Error::RdpError(RdpError::new(RdpErrorKind::RleDecode, "invalid opcode"))),
            }
        }
    }

    Ok(())
}

pub fn rgb565torgb32(input: &[u16]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() * 4);
    output.extend(input.iter().copied().flat_map(|v| {
        [
            ((((v & 0x1f) * 527) + 23) >> 6) as u8,
            (((((v >> 5) & 0x3f) * 259) + 33) >> 6) as u8,
            (((((v >> 11) & 0x1f) * 527) + 23) >> 6) as u8,
            0xff,
        ]
    }));
    output
}
