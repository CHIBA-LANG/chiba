# level-1b frontend migration

This file tracks the replacement of regex, chibalex, and chibacc oracle
builtins. The std libraries are part of the Second Bootstrap input; CLI wrappers
live outside `std`.

## Regex

Second Bootstrap target: `std.regex` should first become a Rust-like
Thompson NFA/DFA subset suitable for chibalex and compiler frontend bootstrap.
Perl/PCRE2-compatible VM behavior remains a later explicit project, not the
first self-hosting target. Unsupported PCRE2-only constructs must fail at parse
time instead of producing partial or misleading matches.

| area | owner | status |
| --- | --- | --- |
| UTF-8 byte boundary helpers | `std/regex/utf8.chiba` | rewritten |
| Regex cursor advance | `std/regex/parser.chiba` | rewritten |
| Unicode XID property tables | `std/regex/utf8.chiba` plus generated data | rewritten |
| Regex parser | `std/regex/parser.chiba` | contract only |
| Regex compiler | `std/regex/program.chiba` | contract only |
| Regex matcher and longest match | `std/regex/matcher.chiba` | partial rewrite: find/longest_at traversal rewritten; match_at VM still builtin |
| PCRE2/Perl VM compatibility | future `std.regex` VM design | TODO after bootstrap subset; backrefs, variable lookbehind, recursive patterns, conditionals, atomic/possessive combos must be explicit unsupported errors until then |

## Chibalex

| area | owner | status |
| --- | --- | --- |
| `.chibalex` AST | `std/chibalex/ast.chiba` | rewritten |
| `.chibalex` parser | `std/chibalex/parser.chiba` | contract only |
| Continuation keywords (`cont1`, `contN`, `shiftn`) | `std/chibalex/ast.chiba` + frontend lexer spec | contract only |
| Attribute tokens | frontend lexer spec | specified: lexer emits ordinary `#`, `#!`, brackets, delimiters, literals, and identifiers; it must not collapse `#[ident]` into a single legacy token |
| Lexer IR lowering | `std/chibalex/ir.chiba` | contract only |
| Longest-match engine | `std/chibalex/engine.chiba` | partial rewrite: state advance and continuation choice rewritten; rule matching still builtin |
| Lexer source codegen | `std/chibalex/codegen.chiba` | partial rewrite: GeneratedLexer text wrapper owned; full lexer source serialization pending |

## Chibacc

| area | owner | status |
| --- | --- | --- |
| `.chibacc` AST | `std/chibacc/ast.chiba` | rewritten |
| `.chibacc` parser | `std/chibacc/parser.chiba` | contract only |
| Continuation type grammar (`cont1 (A) -> B`, `contN (A) -> B`) | `std/chibacc/ast.chiba` + frontend parser spec | contract only |
| Attribute argument grammar | `std/chibacc/ast.chiba` + frontend parser spec | `AttrArg` AST covers bare, string/int/bool, named, call, list, and object args; mini fixture records structured parser grammar; parser execution pending |
| Grammar IR lowering | `std/chibacc/ir.chiba` | contract only |
| Pratt/recovery engine | `std/chibacc/engine.chiba` | partial rewrite: recovery and continuation retry rewritten; Pratt parse still builtin |
| Parser source codegen | `std/chibacc/codegen.chiba` | partial rewrite: GeneratedParser text wrapper owned; full parser source serialization pending |

## Exit Criteria

- `level1b:c04-regex` rejects UTF-8 boundary builtins and must eventually reject
  every regex parser/compiler/matcher builtin.
- `level1b:c05-chibalex` must eventually reject every chibalex builtin and run
  the generated lexer against native oracle token streams.
- `level1b:c06-chibacc` must eventually reject every chibacc builtin and run
  generated parsers against native oracle parse trees.
