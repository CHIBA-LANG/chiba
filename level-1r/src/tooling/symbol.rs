pub fn is_chiba_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_chiba_identifier_start(first) && chars.all(is_chiba_identifier_continue)
}

pub fn is_chiba_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic() || is_non_ascii_symbol_identifier(ch)
}

pub fn is_chiba_identifier_continue(ch: char) -> bool {
    is_chiba_identifier_start(ch) || ch.is_alphanumeric() || is_unicode_mark(ch)
}

pub fn encode_debug_symbol(symbol: &str) -> String {
    encode_debug_symbol_with_case(symbol, false)
}

pub fn encode_debug_symbol_lower(symbol: &str) -> String {
    encode_debug_symbol_with_case(symbol, true)
}

fn encode_debug_symbol_with_case(symbol: &str, lower_ascii: bool) -> String {
    let mut out = String::new();
    for ch in symbol.chars() {
        match ch {
            'a'..='z' | '0'..='9' | '_' => out.push(ch),
            'A'..='Z' if lower_ascii => out.push(ch.to_ascii_lowercase()),
            'A'..='Z' => out.push(ch),
            ':' | '.' | '-' | '/' | '\\' => out.push('_'),
            _ => out.push_str(&format!("_u{:X}_", ch as u32)),
        }
    }
    out
}

fn is_non_ascii_symbol_identifier(ch: char) -> bool {
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
    use super::{encode_debug_symbol, is_chiba_identifier};

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
    fn debug_symbol_encoding_keeps_unicode_names_collision_free() {
        assert_eq!(encode_debug_symbol("模块::函数α"), "_u6A21__u5757____u51FD__u6570__u3B1_");
        assert_eq!(encode_debug_symbol("模块::函数β"), "_u6A21__u5757____u51FD__u6570__u3B2_");
        assert_eq!(encode_debug_symbol("结果.🚀Ok"), "_u7ED3__u679C___u1F680_Ok");
    }
}
