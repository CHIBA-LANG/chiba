use chiba_level1r::chibalex::{compile_lexer, LexerRule, LexerSpec};

fn basic_lexer() -> chiba_level1r::chibalex::Lexer {
    compile_lexer(LexerSpec {
        rules: vec![
            LexerRule {
                name: "Whitespace".to_string(),
                pattern: "[ \n]+".to_string(),
                skip: true,
            },
            LexerRule {
                name: "Let".to_string(),
                pattern: "let".to_string(),
                skip: false,
            },
            LexerRule {
                name: "Ident".to_string(),
                pattern: "[a-zA-Z_][a-zA-Z0-9_]*".to_string(),
                skip: false,
            },
            LexerRule {
                name: "Number".to_string(),
                pattern: "[0-9]+".to_string(),
                skip: false,
            },
        ],
    })
    .unwrap()
}

#[test]
fn lexer_uses_longest_match_then_rule_order() {
    let lexer = basic_lexer();
    let tokens = lexer.lex("let letter 42").unwrap();

    assert_eq!(
        tokens
            .iter()
            .map(|token| (token.name.as_str(), token.lexeme.as_str()))
            .collect::<Vec<_>>(),
        vec![("Let", "let"), ("Ident", "letter"), ("Number", "42")]
    );
}

#[test]
fn lexer_tracks_codepoint_offsets_for_utf8() {
    let lexer = compile_lexer(LexerSpec {
        rules: vec![
            LexerRule {
                name: "Cn".to_string(),
                pattern: "中+".to_string(),
                skip: false,
            },
            LexerRule {
                name: "Ident".to_string(),
                pattern: "[a-z]+".to_string(),
                skip: false,
            },
        ],
    })
    .unwrap();

    let tokens = lexer.lex("a中中").unwrap();
    assert_eq!(tokens[0].start, 0);
    assert_eq!(tokens[0].end, 1);
    assert_eq!(tokens[1].start, 1);
    assert_eq!(tokens[1].end, 3);
}

#[test]
fn lexer_supports_xid_property_classes_for_utf8_identifiers() {
    let lexer = compile_lexer(LexerSpec {
        rules: vec![
            LexerRule {
                name: "Whitespace".to_string(),
                pattern: "[ ]+".to_string(),
                skip: true,
            },
            LexerRule {
                name: "Ident".to_string(),
                pattern: "\\p{XID_START}\\p{XID_CONTINUE}*".to_string(),
                skip: false,
            },
        ],
    })
    .unwrap();

    let tokens = lexer.lex("λ é 中_2").unwrap();
    assert_eq!(
        tokens
            .iter()
            .map(|token| (
                token.name.as_str(),
                token.lexeme.as_str(),
                token.start,
                token.end
            ))
            .collect::<Vec<_>>(),
        vec![
            ("Ident", "λ", 0, 1),
            ("Ident", "é", 2, 4),
            ("Ident", "中_2", 5, 8),
        ]
    );
}

#[test]
fn lexer_accepts_chiba_symbol_identifiers_beyond_strict_xid() {
    let lexer = compile_lexer(LexerSpec {
        rules: vec![
            LexerRule {
                name: "Whitespace".to_string(),
                pattern: "[ ]+".to_string(),
                skip: true,
            },
            LexerRule {
                name: "Ident".to_string(),
                pattern: "\\p{XID_START}\\p{XID_CONTINUE}*".to_string(),
                skip: false,
            },
        ],
    })
    .unwrap();

    let tokens = lexer.lex("函数α 标量名 🚀Ctor").unwrap();
    assert_eq!(
        tokens
            .iter()
            .map(|token| (token.name.as_str(), token.lexeme.as_str()))
            .collect::<Vec<_>>(),
        vec![("Ident", "函数α"), ("Ident", "标量名"), ("Ident", "🚀Ctor"),]
    );
}

#[test]
fn lexer_reports_no_rule_without_scanner_fallback() {
    let lexer = basic_lexer();
    let err = lexer.lex("@").unwrap_err();

    assert!(format!("{err:?}").contains("NoRule"));
}
