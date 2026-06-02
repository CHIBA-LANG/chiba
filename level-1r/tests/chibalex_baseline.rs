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
fn lexer_reports_no_rule_without_scanner_fallback() {
    let lexer = basic_lexer();
    let err = lexer.lex("@").unwrap_err();

    assert!(format!("{err:?}").contains("NoRule"));
}
