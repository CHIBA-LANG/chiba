pub fn is_chiba_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_chiba_identifier_start(first) && chars.all(is_chiba_identifier_continue)
}

pub fn is_chiba_identifier_start(ch: char) -> bool {
    ch == '_' || is_xid_start(ch) || is_chiba_symbol_identifier_start(ch)
}

pub fn is_chiba_identifier_continue(ch: char) -> bool {
    is_chiba_identifier_start(ch) || is_xid_continue(ch)
}

pub fn is_xid_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

pub fn is_xid_continue(ch: char) -> bool {
    is_xid_start(ch) || ch.is_alphanumeric() || is_unicode_mark(ch)
}

fn is_chiba_symbol_identifier_start(ch: char) -> bool {
    !ch.is_ascii()
        && !ch.is_whitespace()
        && !ch.is_control()
        && !is_unicode_mark(ch)
        && !is_chiba_delimiter_or_operator(ch)
}

fn is_chiba_delimiter_or_operator(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')'
            | '{'
            | '}'
            | '['
            | ']'
            | ','
            | ';'
            | ':'
            | '.'
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '='
            | '<'
            | '>'
            | '!'
            | '?'
            | '|'
            | '&'
            | '^'
            | '~'
            | '@'
            | '#'
            | '$'
            | '`'
            | '"'
            | '\''
            | '，'
            | '。'
            | '、'
            | '；'
            | '：'
            | '（'
            | '）'
            | '【'
            | '】'
            | '「'
            | '」'
            | '『'
            | '』'
    )
}

fn is_unicode_mark(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0300..=0x036F
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xFE20..=0xFE2F
    )
}

#[cfg(test)]
mod tests {
    use super::{is_chiba_identifier, is_xid_continue, is_xid_start};

    #[test]
    fn chiba_identifier_accepts_chinese_greek_emoji_and_combining_marks() {
        for ident in ["函数α", "标量名", "🚀Ctor", "Ωmega", "e\u{301}"] {
            assert!(is_chiba_identifier(ident), "{ident} should be a Chiba identifier");
        }
    }

    #[test]
    fn chiba_identifier_rejects_delimiters_whitespace_and_leading_marks() {
        for ident in ["a+b", "a.b", "a b", "，name", "\u{301}e"] {
            assert!(
                !is_chiba_identifier(ident),
                "{ident:?} should not be a single Chiba identifier"
            );
        }
    }

    #[test]
    fn xid_policy_accepts_utf8_start_and_continue_cases() {
        assert!(is_xid_start('λ'));
        assert!(is_xid_start('中'));
        assert!(is_xid_continue('\u{301}'));
        assert!(is_xid_continue('2'));
        assert!(!is_xid_start('\u{301}'));
        assert!(!is_xid_start('，'));
    }
}
