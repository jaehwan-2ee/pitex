// Pitex-authored preview diagnostics. Copyright (c) 2026 Pitex contributors.
// SPDX-License-Identifier: AGPL-3.0-or-later
pub fn excerpt(bytes:&[u8])->String {
    let text=String::from_utf8_lossy(bytes);
    if text.len()<=4096{return text.into_owned();}
    const TAIL:&str="\n[truncated; see driver.log]";
    let mut length=4096-TAIL.len();while !text.is_char_boundary(length){length-=1;}
    format!("{}{TAIL}",&text[..length])
}
#[cfg(test)]mod tests {use super::*;
    #[test]fn unicode_warnings_remain_json_strings(){let message="가".repeat(1400);let result=excerpt(message.as_bytes());assert!(result.len()<=4096);assert!(result.ends_with("[truncated; see driver.log]"));assert!(result.starts_with("가가"));assert_eq!(excerpt(b"invalid \xff diagnostic"),"invalid � diagnostic");}
}
