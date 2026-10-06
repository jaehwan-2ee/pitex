//! Byte anchors in rewritten SyncTeX metadata (shared with preview helpers).

/// Recount each `!<bytes>` record after replacing Input paths. An anchor's
/// value is the UTF-8 byte distance from the previous anchor's START; the
/// first counts from the start of the file. Coordinates and tags stay intact.
pub fn recount_anchors(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut previous = 0;
    for line in text.lines() {
        if line.strip_prefix('!').is_some_and(|value| {
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            let start = output.len();
            output.push_str(&format!("!{}", start - previous));
            previous = start;
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewritten_unicode_inputs_and_midstream_records_have_correct_byte_anchors() {
        let header = "SyncTeX Version:1\nInput:1:/논문/a.tex\nContent:\n";
        let content = "{1\nInput:2:/논문/b.tex\nh2,4:10,20:30,40,50\n";
        let source =
            format!("{header}!999999\n{content}!999999\n}}1\n!999999\nPostamble:\nCount:3\n");
        let first = format!("!{}\n", header.len());
        let second = format!("!{}\n", first.len() + content.len());
        let third = format!("!{}\n", second.len() + "}1\n".len());
        let expected = format!("{header}{first}{content}{second}}}1\n{third}Postamble:\nCount:3\n");
        let actual = recount_anchors(&source);
        assert_eq!(actual, expected);
        assert!(header.len() > header.chars().count());
        assert_eq!(recount_anchors(&actual), actual);
    }

    #[test]
    fn line_endings_normalize_before_anchor_byte_counts() {
        assert_eq!(
            recount_anchors("Content:\r\n!100\r\n{1\r\n!100\r\n"),
            "Content:\n!9\n{1\n!6\n"
        );
    }
}
