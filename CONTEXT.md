# Chiba Level-1 Compiler Bootstrap

This context names the compiler bootstrap concepts used to decide whether level-1b is truly becoming the primary implementation path.

## Language

**Primary compiler behavior**:
The compiler behavior that carries the real success path without relying on the legacy `src/backend/cir` path to supply key semantics. Legacy code may remain as an oracle or diff reference, but it must not be required for level-1b to claim successful compiler behavior.
_Avoid_: primary-ish, mapped, gate-green

**Contract smoke**:
A check that proves an interface, fixture, pass entry point, IR shape, mapping, harness, or expected error path exists and can be invoked. It prevents structural regressions but does not prove user-visible compiler semantics.
_Avoid_: semantic proof, implemented behavior, done

**Semantic validation**:
A check that proves the compiler makes the correct user-visible semantic judgment or generates behavior that preserves the language contract. Shape-only, mapping-only, and comment-only outputs are not semantic validation.
_Avoid_: smoke, mapping, fixture coverage

**Fail-closed stub**:
A known-incomplete compiler component that refuses to act as primary compiler behavior. It may participate in contract smoke, but on a real compiler path it must produce a precise blocker instead of passing through data unchanged.
_Avoid_: silent pass-through, fake success, empty facts

**Blocker taxonomy**:
The fixed classes used when a fail-closed stub blocks real compiler behavior: `missing-facts`, `missing-lowering`, `missing-runtime-contract`, `missing-backend-layout`, `legacy-dependency`, and `oracle-dependency`. Each blocker must name one class and one concrete missing semantic.
_Avoid_: not implemented, TODO, unsupported

**Legacy dependency**:
A level-1b success path that requires the legacy `src/backend/cir` implementation to supply real lowering, backend output, facts, or fallback behavior. Static comparison, documentation references, and non-authoritative oracle diffs do not count as legacy dependency.
_Avoid_: fallback, borrowed success, legacy-backed gate

**Legacy reference**:
Legacy level-0 or `src/backend/cir` code used as a source of implementation patterns, test ideas, or behavioral comparison after spec alignment. A legacy reference is acceptable only when level-1b owns the resulting behavior and does not require legacy execution for success.
_Avoid_: copy-through, fallback path, unreviewed port

**Oracle dependency**:
A compiler success path that requires a builtin or oracle implementation to supply key language capability instead of the self-hosted compiler path. An oracle may be a ruler for comparison or fixture generation, but it must not be a leg the compiler stands on.
_Avoid_: oracle-backed success, builtin-backed success, hidden host behavior

**ChibaCC**:
The Chiba ecosystem parser generator that consumes lexer-produced `TokenItem` streams and produces parser AST results. ChibaCC grammars do not parse characters and do not use string terminals; character handling belongs to ChibaLex.
_Avoid_: character parser, scanner parser, string-terminal grammar

**LabeledAST**:
The top-level ChibaCC parse result wrapper that distinguishes complete AST success from recoverable or skipped error regions. It belongs to the generated start driver output, not to every internal grammar rule.
_Avoid_: rule result wrapper, internal AST type, parser error node

**ChibaCC LL(*) parser**:
The ChibaCC parsing model for ordinary grammar rules: generated recursive-descent parsing with arbitrary lookahead where needed, plus explicit Pratt islands for expression precedence. Alternative retry and recovery are modeled with multi-shot continuation semantics.
_Avoid_: PEG parser, LR parser, GLR parser, character parser

**Parser retry continuation**:
The multi-shot continuation captured while trying ChibaCC alternatives or recovery paths. It is semantically and operationally `shiftn`; generated parsers must not replace it with a separate retry stack fallback.
_Avoid_: one-shot parser retry, shift retry, exception-only recovery, retry stack fallback

**ChibaCC source generation complete**:
The state where ChibaCC can generate parser source that represents the requested grammar and uses real parser retry continuations. This is distinct from ChibaCC AST primary path completion, which also requires the generated parser to execute through the compiler backend.
_Avoid_: AST primary complete, generated parser executed, parser backend complete

**ChibaCC action expression**:
The arbitrary Chiba expression after `=>` in a grammar alternative or Pratt arm. ChibaCC owns binding labels and splicing this expression into generated parser source; the main Chiba compiler owns parsing, typing, CPS, closure, and backend semantics for the expression.
_Avoid_: callback syntax, action DSL, restricted action subset, parser-owned typechecking

**Level-1b bootstrap complete**:
The state where C12 two-round bootstrap comparison succeeds through `level1c -> level1c-next -> level1c-next2` without legacy dependency or oracle dependency. The result must run the required specs and gates outside a Node-only path and record hashes, toolchain versions, and any accepted manifest differences.
_Avoid_: half-bootstrap, gate-green bootstrap, Node-only success

**Comment-only backend output**:
Backend output where comments stand in for executable Core, WAT, continuation frame, closure environment, or layout behavior. Comments may explain debug dumps or contract smoke fixtures, but they must not count as backend success.
_Avoid_: comment emitter, explanatory artifact, fake WAT

**Spec alignment audit**:
A pass over the external Chiba level-1 spec before language implementation work. It identifies hard semantic rules, compares them to level-1b behavior, and classifies gaps before cleanup or implementation changes begin.
_Avoid_: code-first cleanup, stub tidying, implementation-only audit

**Truthfulness audit**:
A fast feedback loop that detects level-1b paths whose names or gates imply semantic progress while the implementation still depends on stubs, empty facts, comment-only output, legacy execution, or oracle-backed success. It reports blockers; it does not implement the missing semantics.
_Avoid_: semantic gate, implementation pass, bootstrap validation

## Example Dialogue

Dev: "The migration gate is green. Is level-1b primary now?"

Domain expert: "Not unless level-1b itself carries the semantics. A green mapping or contract smoke does not establish primary compiler behavior."

Dev: "The backend gate sees the pass and a WAT file exists. Is that semantic validation?"

Domain expert: "No. That is contract smoke unless the emitted artifact carries the required behavior and the gate can reject semantic holes."

Dev: "A pass is not implemented yet. Can it return the module unchanged?"

Domain expert: "Only in contract smoke. On a primary path, an incomplete pass must fail closed with a precise blocker."

Dev: "What should the blocker say?"

Domain expert: "Use a blocker class plus the missing semantic, such as `missing-facts: continuation replay-safety facts absent`."

Dev: "Can level-1b compare itself against legacy output?"

Domain expert: "Yes, if legacy is only an oracle or diff reference. No, if legacy output is required for the level-1b success path."

Dev: "Can we borrow an algorithm from level-0?"

Domain expert: "Yes, as a legacy reference. First check it against the spec, then re-own the behavior in level-1b."

Dev: "Can an oracle stay after level-1b bootstrap completes?"

Domain expert: "Only as a ruler. Any oracle dependency that supplies key compiler capability must be removed."

Dev: "Is level-1b bootstrapped after it emits one next compiler?"

Domain expert: "No. Bootstrap complete means the two-round C12 comparison succeeds without legacy or oracle dependency."

Dev: "The WAT file has comments for every core operation. Is the backend working?"

Domain expert: "No. Comments may explain behavior, but executable backend output must carry it."

Dev: "Can we clean language stubs before reading the spec?"

Domain expert: "No. Run a spec alignment audit first, then decide which stubs become blockers, gates, or real implementations."

Dev: "The truthfulness audit fails. Did we break the build?"

Domain expert: "No. It is a feedback loop for known false-green paths; failure means it found blockers that must not be hidden."
