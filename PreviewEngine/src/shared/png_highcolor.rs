// Pitex-authored 16-bit PNG scanline decoder, from the PNG specification.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub channels: usize,
    pub color: Vec<u8>,
    pub alpha: Vec<u8>,
}
pub unsafe fn decode(data: &[u8]) -> Option<Decoded> {
    if data.len() < 33 || data[24] != 16 {
        return None;
    }
    let u32be = |at: usize| Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?));
    let width = u32be(16)?;
    let height = u32be(20)?;
    let kind = data[25];
    let input_channels = match kind {
        0 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return None,
    };
    if width == 0 || height == 0 || width > 30000 || height > 30000 {
        return None;
    }
    let output_channels = if kind == 0 || kind == 4 { 1 } else { 3 };
    let mut compressed = Vec::new();
    let mut transparent = None;
    let mut at = 8usize;
    while at + 12 <= data.len() {
        let length = u32be(at)? as usize;
        let body = data.get(at + 8..at + 8 + length)?;
        match data.get(at + 4..at + 8)? {
            b"IDAT" => compressed.extend_from_slice(body),
            b"tRNS" => transparent = Some(body.to_vec()),
            b"IEND" => break,
            _ => {}
        }
        at = at.checked_add(length + 12)?;
    }
    let mut raw = pbuf {
        data: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    };
    if pbuf_inflate(&mut raw, compressed.as_ptr(), compressed.len()) != 0 {
        pbuf_free(&mut raw);
        return None;
    }
    let bytes = if raw.len == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(raw.data, raw.len)
    };
    let result = (|| {
        let pixels = (width as usize).checked_mul(height as usize)?;
        let mut color = vec![0u8; pixels.checked_mul(output_channels * 2)?];
        let mut alpha = if kind == 4 || kind == 6 || transparent.is_some() {
            vec![255u8; pixels.checked_mul(2)?]
        } else {
            vec![]
        };
        let passes: &[(usize, usize, usize, usize)] = if data[28] == 0 {
            &[(0, 0, 1, 1)]
        } else if data[28] == 1 {
            &[
                (0, 0, 8, 8),
                (4, 0, 8, 8),
                (0, 4, 4, 8),
                (2, 0, 4, 4),
                (0, 2, 2, 4),
                (1, 0, 2, 2),
                (0, 1, 1, 2),
            ]
        } else {
            return None;
        };
        let bpp = input_channels * 2;
        let mut cursor = 0;
        for &(x0, y0, dx, dy) in passes {
            if x0 >= width as usize || y0 >= height as usize {
                continue;
            }
            let pw = (width as usize - x0 + dx - 1) / dx;
            let ph = (height as usize - y0 + dy - 1) / dy;
            let rowlen = pw.checked_mul(bpp)?;
            let mut previous = vec![0u8; rowlen];
            for y in 0..ph {
                let filter = *bytes.get(cursor)?;
                cursor += 1;
                let mut row = bytes.get(cursor..cursor + rowlen)?.to_vec();
                cursor += rowlen;
                for i in 0..rowlen {
                    let left = if i >= bpp { row[i - bpp] } else { 0 };
                    let above = previous[i];
                    let corner = if i >= bpp { previous[i - bpp] } else { 0 };
                    let predictor = match filter {
                        0 => 0,
                        1 => left,
                        2 => above,
                        3 => ((left as u16 + above as u16) / 2) as u8,
                        4 => {
                            let p = left as i32 + above as i32 - corner as i32;
                            let a = (p - left as i32).abs();
                            let b = (p - above as i32).abs();
                            let c = (p - corner as i32).abs();
                            if a <= b && a <= c {
                                left
                            } else if b <= c {
                                above
                            } else {
                                corner
                            }
                        }
                        _ => return None,
                    };
                    row[i] = row[i].wrapping_add(predictor);
                }
                for x in 0..pw {
                    let pixel = (y0 + y * dy) * width as usize + x0 + x * dx;
                    let sample = &row[x * bpp..(x + 1) * bpp];
                    color[pixel * output_channels * 2..(pixel + 1) * output_channels * 2]
                        .copy_from_slice(&sample[..output_channels * 2]);
                    if !alpha.is_empty() {
                        if kind == 4 || kind == 6 {
                            alpha[pixel * 2..pixel * 2 + 2].copy_from_slice(
                                &sample[output_channels * 2..output_channels * 2 + 2],
                            );
                        } else if transparent
                            .as_ref()
                            .is_some_and(|key| key.as_slice() == &sample[..output_channels * 2])
                        {
                            alpha[pixel * 2..pixel * 2 + 2].fill(0);
                        }
                    }
                }
                previous = row;
            }
        }
        Some(Decoded {
            width,
            height,
            channels: output_channels,
            color,
            alpha,
        })
    })();
    pbuf_free(&mut raw);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_low_bytes_of_sixteen_bit_samples() {
        let data = [
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 5, 0, 0, 0, 1,
            16, 0, 0, 0, 0, 99, 5, 231, 108, 0, 0, 0, 19, 73, 68, 65, 84, 120, 156, 99, 96, 96, 80,
            23, 104, 96, 56, 28, 240, 255, 63, 0, 11, 172, 3, 201, 203, 91, 196, 34, 0, 0, 0, 0,
            73, 69, 78, 68, 174, 66, 96, 130,
        ];
        let image = unsafe { decode(&data) }.unwrap();
        assert_eq!((image.width, image.height, image.channels), (5, 1, 1));
        assert_eq!(image.color, vec![0, 0, 39, 16, 128, 0, 195, 80, 255, 255]);
        assert!(image.alpha.is_empty());
    }
}
