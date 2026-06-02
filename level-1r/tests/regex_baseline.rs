use chiba_level1r::regex::{compile, is_match, parse, RegexError, RegexInst};

#[test]
fn regex_literal_search_and_miss() {
    assert_eq!(is_match("bc", "abc").unwrap(), true);
    assert_eq!(is_match("ac", "abc").unwrap(), false);
}

#[test]
fn regex_alt_and_repeat_emit_split_jump_vm_shape() {
    let ast = parse("a|b*").unwrap();
    let program = compile(&ast);

    assert!(program
        .code
        .iter()
        .any(|inst| matches!(inst, RegexInst::Split(_, _))));
    assert!(program
        .code
        .iter()
        .any(|inst| matches!(inst, RegexInst::Jump(_))));
    assert_eq!(is_match("a|b*", "zzbbb").unwrap(), true);
    assert_eq!(is_match("a|b*", "zzc").unwrap(), true);
}

#[test]
fn regex_class_range_dot_plus_and_optional() {
    assert_eq!(is_match("[a-z]+", "ABCxyz").unwrap(), true);
    assert_eq!(is_match("a.b", "a中b").unwrap(), true);
    assert_eq!(is_match("ab?c", "ac").unwrap(), true);
    assert_eq!(is_match("ab?c", "abc").unwrap(), true);
}

#[test]
fn regex_rejects_pcre_only_features_early() {
    assert_eq!(
        parse("(?=a)").unwrap_err(),
        RegexError::UnsupportedPcreFeature("lookaround")
    );
    assert_eq!(
        parse("\\1").unwrap_err(),
        RegexError::UnsupportedPcreFeature("backref")
    );
}
