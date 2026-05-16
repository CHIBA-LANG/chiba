# level-1b frontend migration

This file tracks the replacement of regex, chibalex, and chibacc oracle
builtins. The std libraries are part of the Second Bootstrap input; CLI wrappers
live outside `std`.

## Regex

| area | owner | status |
| --- | --- | --- |
| UTF-8 byte boundary helpers | `std/regex/utf8.chiba` | rewritten |
| Regex cursor advance | `std/regex/parser.chiba` | rewritten |
| Unicode XID property tables | `std/regex/utf8.chiba` plus generated data | contract only |
| Regex parser | `std/regex/parser.chiba` | contract only |
| Regex compiler | `std/regex/program.chiba` | contract only |
| Regex matcher and longest match | `std/regex/matcher.chiba` | contract only |

## Chibalex

| area | owner | status |
| --- | --- | --- |
| `.chibalex` AST | `std/chibalex/ast.chiba` | rewritten |
| `.chibalex` parser | `std/chibalex/parser.chiba` | contract only |
| Lexer IR lowering | `std/chibalex/ir.chiba` | contract only |
| Longest-match engine | `std/chibalex/engine.chiba` | contract only |
| Lexer source codegen | `std/chibalex/codegen.chiba` | contract only |

## Chibacc

| area | owner | status |
| --- | --- | --- |
| `.chibacc` AST | `std/chibacc/ast.chiba` | rewritten |
| `.chibacc` parser | `std/chibacc/parser.chiba` | contract only |
| Grammar IR lowering | `std/chibacc/ir.chiba` | contract only |
| Pratt/recovery engine | `std/chibacc/engine.chiba` | contract only |
| Parser source codegen | `std/chibacc/codegen.chiba` | contract only |

## Exit Criteria

- `level1b:c04-regex` rejects UTF-8 boundary builtins and must eventually reject
  every regex parser/compiler/matcher builtin.
- `level1b:c05-chibalex` must eventually reject every chibalex builtin and run
  the generated lexer against native oracle token streams.
- `level1b:c06-chibacc` must eventually reject every chibacc builtin and run
  generated parsers against native oracle parse trees.
