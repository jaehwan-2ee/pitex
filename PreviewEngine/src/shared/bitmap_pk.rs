// Original Pitex reader for the published PK font format (PKtype, sections 14–29).
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Glyph {
    pub width: usize,
    pub height: usize,
    pub x_offset: i32,
    pub y_offset: i32,
    pub width_fix: i32,
    /// Top-to-bottom rows, most significant bit first, padded separately to bytes.
    pub mask: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Font {
    pub design: u32,
    pub checksum: u32,
    pub horizontal_ppp: u32,
    pub vertical_ppp: u32,
    pub glyphs: BTreeMap<u32, Glyph>,
}
impl Font {
    pub fn dpi(&self) -> (f64, f64) {
        (
            self.horizontal_ppp as f64 * 72.27 / 65536.,
            self.vertical_ppp as f64 * 72.27 / 65536.,
        )
    }
}
struct Reader<'a> {
    data: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], &'static str> {
        let end = self.cursor.checked_add(count).ok_or("PK offset overflow")?;
        let value = self.data.get(self.cursor..end).ok_or("truncated PK file")?;
        self.cursor = end;
        Ok(value)
    }
    fn unsigned(&mut self, bytes: usize) -> Result<u32, &'static str> {
        Ok(self
            .take(bytes)?
            .iter()
            .fold(0, |n, b| (n << 8) | *b as u32))
    }
    fn signed(&mut self, bytes: usize) -> Result<i32, &'static str> {
        let n = self.unsigned(bytes)?;
        Ok(((n << (32 - bytes * 8)) as i32) >> (32 - bytes * 8))
    }
}
struct Nibbles<'a> {
    data: &'a [u8],
    cursor: usize,
    dynamic: u32,
}
impl Nibbles<'_> {
    fn next(&mut self) -> Result<u32, &'static str> {
        let byte = *self
            .data
            .get(self.cursor / 2)
            .ok_or("truncated PK run encoding")?;
        let value = if self.cursor % 2 == 0 {
            byte >> 4
        } else {
            byte & 15
        };
        self.cursor += 1;
        Ok(value as u32)
    }
    fn number(&mut self, first: u32) -> Result<usize, &'static str> {
        let value = if first == 0 {
            let mut digits = 1usize;
            let mut value = self.next()?;
            while value == 0 {
                digits += 1;
                if digits > 7 {
                    return Err("PK run integer overflow");
                }
                value = self.next()?;
            }
            for _ in 0..digits {
                let digit = self.next()?;
                value = value
                    .checked_mul(16)
                    .and_then(|v| v.checked_add(digit))
                    .ok_or("PK run integer overflow")?;
            }
            value
                .checked_sub(15)
                .and_then(|v| v.checked_add((13 - self.dynamic) * 16 + self.dynamic))
                .ok_or("PK run integer overflow")?
        } else if first <= self.dynamic {
            first
        } else if first < 14 {
            (first - self.dynamic - 1) * 16 + self.next()? + self.dynamic + 1
        } else {
            return Err("nested PK row repeat");
        };
        if value == 0 {
            return Err("zero PK run count");
        }
        Ok(value as usize)
    }
    fn run(&mut self, repeat: &mut usize) -> Result<usize, &'static str> {
        let mut first = self.next()?;
        if first >= 14 {
            if *repeat != 0 {
                return Err("multiple PK repeats on one row");
            }
            *repeat = if first == 15 {
                1
            } else {
                let n = self.next()?;
                self.number(n)?
            };
            first = self.next()?;
        }
        self.number(first)
    }
}
fn raster(
    data: &[u8],
    width: usize,
    height: usize,
    dynamic: u32,
    starts_black: bool,
) -> Result<Vec<u8>, &'static str> {
    let pixels = width.checked_mul(height).ok_or("PK raster size overflow")?;
    if width > 16384 || height > 16384 || pixels > 16 * 1024 * 1024 {
        return Err("PK raster exceeds limit");
    }
    let stride = (width + 7) / 8;
    let mut result = vec![0u8; stride * height];
    if pixels == 0 {
        if !data.is_empty() {
            return Err("nonempty PK empty raster");
        }
        return Ok(result);
    }
    if dynamic == 14 {
        if data.len() != (pixels + 7) / 8 {
            return Err("invalid PK bitmap length");
        }
        for index in 0..pixels {
            if data[index / 8] & (128 >> (index % 8)) != 0 {
                result[index / width * stride + index % width / 8] |= 128 >> (index % width % 8);
            }
        }
        return Ok(result);
    }
    let mut packed = Nibbles {
        data,
        cursor: 0,
        dynamic,
    };
    let (mut row, mut column, mut repeat, mut black) = (0usize, 0usize, 0usize, starts_black);
    while row < height {
        let mut count = packed.run(&mut repeat)?;
        while count > 0 {
            if row >= height {
                return Err("PK run exceeds raster");
            }
            let part = count.min(width - column);
            if black {
                for x in column..column + part {
                    result[row * stride + x / 8] |= 128 >> (x % 8);
                }
            }
            column += part;
            count -= part;
            if column == width {
                if repeat >= height - row {
                    return Err("PK repeat exceeds raster");
                }
                let first = row * stride;
                for next in row + 1..=row + repeat {
                    result.copy_within(first..first + stride, next * stride);
                }
                row += repeat + 1;
                column = 0;
                repeat = 0;
            }
        }
        black = !black;
    }
    if packed.cursor.div_ceil(2) != data.len() {
        return Err("unused PK raster bytes");
    }
    if packed.cursor % 2 == 1 && data.last().unwrap() & 15 != 0 {
        return Err("nonzero PK raster padding");
    }
    Ok(result)
}
pub fn parse(data: &[u8]) -> Result<Font, &'static str> {
    let mut input = Reader { data, cursor: 0 };
    if input.unsigned(1)? != 247 || input.unsigned(1)? != 89 {
        return Err("invalid PK preamble");
    }
    let comment = input.unsigned(1)? as usize;
    input.take(comment)?;
    let mut font = Font {
        design: input.unsigned(4)?,
        checksum: input.unsigned(4)?,
        horizontal_ppp: input.unsigned(4)?,
        vertical_ppp: input.unsigned(4)?,
        glyphs: BTreeMap::new(),
    };
    if [font.design, font.horizontal_ppp, font.vertical_ppp]
        .iter()
        .any(|n| *n == 0 || *n > i32::MAX as u32)
    {
        return Err("invalid PK design size or resolution");
    }
    let mut total = 0usize;
    loop {
        let flag = input.unsigned(1)?;
        match flag {
            245 => {
                if input.data[input.cursor..].iter().any(|b| *b != 246) {
                    return Err("invalid PK postamble");
                }
                return Ok(font);
            }
            246 => continue,
            240..=243 => {
                let count = input.unsigned((flag - 239) as usize)?;
                input.take(count as usize)?;
                continue;
            }
            244 => {
                input.take(4)?;
                continue;
            }
            247..=255 => return Err("undefined PK command"),
            _ => {}
        }
        let form = flag & 7;
        let length = if form == 7 {
            input.unsigned(4)?
        } else if form >= 4 {
            (flag & 3) * 65536 + input.unsigned(2)?
        } else {
            (flag & 3) * 256 + input.unsigned(1)?
        };
        let code = input.unsigned(if form == 7 { 4 } else { 1 })?;
        let mut packet = Reader {
            data: input.take(length as usize)?,
            cursor: 0,
        };
        let width_fix = if form == 7 {
            packet.signed(4)?
        } else {
            packet.unsigned(3)? as i32
        };
        let dimensions = if form == 7 {
            packet.take(8)?;
            4
        } else if form >= 4 {
            packet.take(2)?;
            2
        } else {
            packet.take(1)?;
            1
        };
        let width = packet.unsigned(dimensions)? as usize;
        let height = packet.unsigned(dimensions)? as usize;
        let x_offset = packet.signed(dimensions)?;
        let y_offset = packet.signed(dimensions)?;
        let mask = raster(
            &packet.data[packet.cursor..],
            width,
            height,
            flag >> 4,
            flag & 8 != 0,
        )?;
        total = total
            .checked_add(mask.len())
            .ok_or("PK font size overflow")?;
        if total > 64 * 1024 * 1024 {
            return Err("PK font exceeds limit");
        }
        font.glyphs.insert(
            code,
            Glyph {
                width,
                height,
                x_offset,
                y_offset,
                width_fix,
                mask,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header() -> Vec<u8> {
        let mut d = vec![247, 89, 0];
        for v in [10 << 20, 0, 65536, 65536] {
            d.extend_from_slice(&u32::to_be_bytes(v));
        }
        d
    }
    #[test]
    fn all_packet_forms_and_non_byte_aligned_rows() {
        for form in [0, 4, 7] {
            let mut d = header();
            d.push(224 | form);
            let length = if form == 7 {
                29u32
            } else if form == 4 {
                14
            } else {
                9
            };
            d.extend_from_slice(
                &length.to_be_bytes()[if form == 7 {
                    0
                } else if form == 4 {
                    2
                } else {
                    3
                }..],
            );
            d.extend_from_slice(&65u32.to_be_bytes()[if form == 7 { 0 } else { 3 }..]);
            d.extend_from_slice(&524288u32.to_be_bytes()[if form == 7 { 0 } else { 1 }..]);
            if form == 7 {
                d.extend_from_slice(&[0; 8]);
            } else {
                d.extend_from_slice(&[0; 2][if form == 4 { 0 } else { 1 }..]);
            }
            let n = if form == 7 {
                4
            } else if form == 4 {
                2
            } else {
                1
            };
            for value in [3i32, 2, -1, 1] {
                d.extend_from_slice(&value.to_be_bytes()[4 - n..]);
            }
            d.extend_from_slice(&[0b10101000, 245, 246]);
            let glyph = &parse(&d).unwrap().glyphs[&65];
            assert_eq!(glyph.mask, [0b10100000, 0b01000000]);
            assert_eq!((glyph.x_offset, glyph.y_offset), (-1, 1));
        }
    }
    #[test]
    fn repeats_and_runs_cross_rows() {
        assert_eq!(
            raster(&[0x1f, 0x31, 0x30], 4, 3, 13, true).unwrap(),
            [128, 128, 128]
        );
        assert_eq!(
            raster(&[0x1e, 0x23], 4, 3, 13, true).unwrap(),
            [128, 128, 128]
        );
        assert_eq!(raster(&[0x80], 4, 2, 13, true).unwrap(), [240, 240]);
        assert_eq!(
            raster(&[0x00, 0x10, 0x20], 128, 2, 13, true).unwrap(),
            vec![255; 32]
        );
    }
    #[test]
    fn malformed_input_is_bounded() {
        assert!(parse(&header()).is_err());
        assert!(raster(&[0xf4], 4, 1, 13, true).is_err());
        assert!(raster(&[0xee], 4, 3, 13, true).is_err());
        assert!(raster(&[0x10], 1, usize::MAX, 13, true).is_err());
        assert!(raster(&[0x80, 0], 1, 1, 14, false).is_err());
    }
    #[test]
    fn installed_pk_fixture_when_requested() {
        if let Some(path) = std::env::var_os("PITEX_PK_FIXTURE") {
            let font = parse(&std::fs::read(path).unwrap()).unwrap();
            assert!(font.glyphs.len() > 90);
            let a = &font.glyphs[&65];
            assert!(a.width > 0 && a.height > 0 && a.mask.iter().any(|b| *b != 0));
            assert!((font.dpi().0 - 600.).abs() < 1.);
        }
    }
}
