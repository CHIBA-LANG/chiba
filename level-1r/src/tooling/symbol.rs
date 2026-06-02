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

#[cfg(test)]
mod tests {
    use super::encode_debug_symbol;

    #[test]
    fn debug_symbol_encoding_keeps_unicode_names_collision_free() {
        assert_eq!(
            encode_debug_symbol("模块::函数α"),
            "_u6A21__u5757____u51FD__u6570__u3B1_"
        );
        assert_eq!(
            encode_debug_symbol("模块::函数β"),
            "_u6A21__u5757____u51FD__u6570__u3B2_"
        );
        assert_eq!(
            encode_debug_symbol("结果.🚀Ok"),
            "_u7ED3__u679C___u1F680_Ok"
        );
    }
}
