# Chiba Level-1R P0 Audit

Date: 2026-06-10

This audit maps the `AGENTS.md` checkpoint checklist to current evidence in `level-1r`.
It is intentionally stricter than "tests are green": a row is `Complete` only when the
implementation, executable or diagnostic gate, and architectural requirement are all
covered by durable evidence.

Status values:

- `Complete`: implemented with durable executable/diagnostic tests or equivalent gate.
- `Partial`: meaningful implementation and tests exist, but the `AGENTS.md` requirement is broader.
- `Missing`: no sufficient implementation/gate exists yet.

## Summary

Current state: the core language/runtime slice is strong, but P0 is not complete. The
largest open gaps are performance, self-bootstrap, production pass scheduling/cache,
symbol/debug manifest, and no-GC wasm/ownership lowering.

Recent evidence:

- Batch WAT test runner keeps executable WAT coverage while avoiding per-assertion Node startup.
- `f15c75c level-1r: expose program pass timings`
- `381518c level-1r: preserve runtime local kinds in captured continuations`
- `cargo test --manifest-path level-1r/Cargo.toml` was observed passing on 2026-06-10 before the timing-audit slice.

## Language And Semantic Checklist

| AGENTS item | Status | Evidence | Remaining P0 gap |
| --- | --- | --- | --- |
| operator overloading | Partial | `source_arithmetic_operators_lower_to_executable_intrinsic_wat`, `program_nominal_operator_resolves_to_receiver_method_and_lowers_to_wat`, index/slice operator tests, operator obligation/core tests. | Full ambiguous/wrong-operand/operator dispatch matrix and spec-aligned range value surface are not fully closed. |
| template / generics 剩余收口 | Partial | Row callable/dyn row generic tests, specialization/monomorphize unit tests, explicit interface summary tests. | Auto-generic and explicit instantiation are not proven across the whole source-to-WAT path with production specialization/cache semantics. |
| global variable / init block 规则 | Partial | `global_init_*` executable tests cover dependencies, strings, aggregates, cycles, duplicate statics, and module start lowering. | Side-effect init and full module-load semantics are not production-complete; performance is poor in this subset. |
| `Self` type with generics | Complete | `source_method_self_with_generics_reaches_typed_and_core_record_update`, `generic_receiver_method_call_specializes_self_and_lowers_to_executable_wat`, Self summary tests. | Keep covered by regression tests. |
| deep pattern matching | Partial | Pattern unit tests plus executable tuple/ADT/payload/nested parameter pattern dispatch tests. | Exhaustiveness is still warning-oriented and not every deep `if let`/record/ADT shape has executable lowering evidence. |
| pipe behaviour | Complete | `source_pipe_*` frontend and executable WAT tests cover default call, placeholder reuse, chains, method path, method chain, and operator mixing. | Keep covered by regression tests. |
| ADT tuple bridge / ctor lowering | Partial | `adt_to_tuple_intrinsic_surface_extracts_payload_through_tuple_field`, tuple/ADT constructor/pattern executable tests. | Full bidirectional `tuple_to_adt`/`adt_to_tuple` compiler intrinsic identity and nontrivial payload roundtrip are not fully production-proven. |
| compiler-internal intrinsic surface | Partial | Core validation rejects wrong intrinsic owner; ADT tuple and unsafe-cast surfaces have targeted tests. | Need full namespace resolution/lowering gate proving intrinsics are compiler-owned, not library-shaped helpers. |
| pattern args for funcs | Complete | `source_pattern_clause_defs_*` executable tests cover wildcard/binding/tuple/zero-arg ADT/payload/nested payload dispatch. | Keep covered by regression tests. |
| 作用域和let shadowing作用域的检查 | Partial | Alpha and program tests cover local shadowing, if-let branch scope, lambda/static dependency scope, extern target shadowing. | Need larger adversarial same-scope/namespace/use/prelude duplicate matrix. |
| 不再出现 `i64` 和 `1` `0` 还有 `if x != 0` 这种历史遗留代码 | Missing | No repo-wide invariant or lint gate currently proves this. | Add a scoped lint/audit that distinguishes legitimate tests/constants from stale lowering shortcuts. |
| mangling / symbol debugability 收口 | Partial | Owner namespace preservation tests and visual summaries exist. | Final symbol manifest/debug map with source -> resolved -> lowered -> final lineage is not complete. |
| namespace ownership / isolation 收口 | Partial | Resolve and program tests preserve owner namespace, visibility, private filtering, ambiguity, and structured identity. | Full namespace graph/import/prelude/intrinsic isolation and debug manifest integration remain incomplete. |
| constructor like `once` used value lower to mutation | Missing | No dedicated usage-to-mutation lowering evidence found. | Requires ownership/usage lowering decision gate. |

## Pass Checklist

| AGENTS item | Status | Evidence | Remaining P0 gap |
| --- | --- | --- | --- |
| **Pass 00: Project Surface Scan** | Partial | Project surface, namespace headers, duplicate namespace, deterministic merge, and invalid header tests. | File-level parallel scan and full public/private surface audit are not production-complete. |
| **Pass 01: Interface Summary Build** | Partial | Interface summary hash, owner namespace, method/static/type/constructor signature tests. | `.chiba.meta` caching and namespace-SCC summary scheduling are not complete. |
| **Pass 02: TopDef / Kind Check** | Partial | Duplicate extern/def/static tests, method receiver/Self summary tests, extern summary tests. | Full kind matrix, private leakage matrix, and ABI normalization diagnostics need durable gates. |
| **Pass 03: Name Resolve** | Partial | Resolve tests cover owner symbols, arity mismatch, private filtering, ambiguity, structured identity. | Full source body symbol-id stability and prelude/use conflict matrix remain incomplete. |
| **Pass 04: Alpha Conversion** | Complete | `alpha_baseline` covers distinct stable binder ids, capture by binder id, if-let scope, visual layer. | Keep covered by regression tests. |
| **Pass 05: Pattern Elaboration** | Complete | Pattern unit tests cover tuple/record/at/function parameter patterns, duplicate bindings, if-let env. | Keep covered by regression tests. |
| **Pass 06: HM + Row Inference** | Partial | Typed row/member/dyn row/nominal row field tests and template row obligation tests. | Full HM generalization, canonical row ids, and every error class are not yet exhaustively gated. |
| **Pass 07: Answer / Continuation Kind Check** | Partial | Control nanopass and program continuation tests cover reset/shift/resetn, answer type, replay safety basics. | World/thread boundary and full answer-type diagnostics are not complete. |
| **Pass 08: World / Send / Escape / Capability Check** | Partial | Send callable rejects continuation argument; Core validator rejects sendable continuation callable storage. | Full world/thread/Atomic/UnsafeRef escape matrix remains incomplete. |
| **Pass 09: Usage Analysis 0: High-Level Core** | Partial | Usage audit and continuation usage tests classify Cont1/ContN and directification candidates. | General binder/lambda/aggregate usage coloring is not fully proven. |
| **Pass 10: Generic Definition Check** | Partial | Template audit tests prove obligations are recorded without Rust trait solver. | Definition-time generic body checking and serializable cacheable `GenericBodyIR` are not complete. |
| **Pass 11: Method / Operator / Dispatch Index** | Partial | Resolve/template/core dispatch tests cover method index, operator obligations, field-before-method row behavior. | Full stable candidate cache and ambiguity diagnostics across namespaces/shapes remain incomplete. |
| **Pass 12: One-Pass CPS Transformation + Beta Reduction** | Partial | CPS tests prove atom/simple call avoid administrative `LetCont+AppCont`; continuation CPS facts exist. | Full source expression one-pass CPS with all constructs and dump-level redex audit is not complete. |
| **Pass 13: Usage Analysis 1: CPS Core** | Partial | `cps_usage_baseline` covers dead/single/many continuation usage. | Broader lambda/function value usage after CPS is not fully gated. |
| **Pass 14: Continuation Simplification** | Partial | Continuation simplification and executable Cont1/ContN tests cover single-use vs repeatable package cases. | Need complete proof that illegal continuations cannot reach closure/lower stages. |
| **Pass 15: Closure Conversion** | Partial | Closure core tests cover direct/no-capture/env closure and continuation package facts. | Full lexical free-var conversion with stable env colors is not complete. |
| **Pass 16: Lambda Lifting** | Partial | Lambda lift baseline covers basic lifted env params. | Mutually recursive nested functions and stable lifted symbol merge need gates. |
| **Pass 17: Usage Analysis 2: Closure Core** | Partial | Closure core usage tests cover continuation packages and closure facts. | Env field and code pointer usage across full closure core remain incomplete. |
| **Pass 18: Closure / Env Simplification** | Partial | Closure simplify tests cover no-capture/dead env/contn package cases. | Full env shrinking and known-callee direct call coverage remain incomplete. |
| **Pass 19: Monomorphization Scheduler** | Partial | Monomorphize/specialize tests cover stable artifact names and key dimensions. | Concurrent registry, SCC, failure caching, and duplicate work join are not complete. |
| **Pass 20: Core Lower + Ownership/Layout Decision** | Partial | Core ownership tests cover layout hash, dyn package, Cont1/ContN layout facts. | Full target-neutral ownership decisions for all aggregates/callables and no-GC wasm mapping are incomplete. |
| **Pass 21: CoreIR Validation** | Partial | Core validation tests reject missing continuation/dyn/static row layouts and wrong intrinsic ownership. | Full dangling symbol/layout/tailcall/RC/ARC validation matrix is incomplete. |
| **Pass 22: Backend Emit + Link** | Partial | Program/backend executable WAT tests cover many language slices and backend link artifacts. | Multi-namespace object/link model, manifest, WASI/env ABI matrix, and no-GC wasm backend are incomplete. |

## Monomorphize And Checked Generics

| AGENTS item | Status | Evidence | Remaining P0 gap |
| --- | --- | --- | --- |
| **定义期检查一次** | Partial | Template audit and row/member obligation tests prove some definition-time obligations. | Full `GenericBodyIR` definition-time body check is not complete. |
| **实例化期兑现 concrete obligation** | Partial | Row/member/dyn row instantiation tests cover fields, methods, adapters, negative cases. | Full call-site obligation discharge across operators/capabilities/continuations remains incomplete. |
| **specialization key 稳定化** | Partial | Specialize/monomorphize tests cover dyn contract and shape dimensions. | Complete key with capability/continuation/ABI/layout mode and namespace constraints is not fully gated. |
| **并发实例化注册表** | Missing | No concurrent registry stress gate found. | Implement `InstantiationRegistry` with join/failure-cache behavior. |
| **递归 generic / SCC 策略** | Missing | No recursive generic SCC/convergence gate found. | Add recursive generic specialization strategy and diagnostics. |
| **增量缓存** | Missing | Interface summary hash tests exist, but no artifact cache pipeline. | Add cache keys and invalidation tests for summary/body/specialization/CoreIR/WAT artifacts. |

## Parallel Compilation Boundary

| AGENTS item | Status | Evidence | Remaining P0 gap |
| --- | --- | --- | --- |
| **全局轻量串行/归约区** | Missing | No timing or scheduler gate proves only reduce/link phases are serial. | Add pass-level scheduling/timing evidence and deterministic reduce tests. |
| **namespace 并行区** | Missing | No multi-core namespace scheduling gate found. | Add namespace/body parallel execution model and deterministic output test. |
| **specialization 并行区** | Missing | No generic-heavy parallel specialization stress gate found. | Add specialization work queue and duplicate-key de-dup under concurrency. |

## Cross-Cutting P0 Gates

| Gate | Status | Evidence | Remaining P0 gap |
| --- | --- | --- | --- |
| Performance | Partial | `ProgramCompileOutput::render_summary()` now exposes pass timings. Batch WAT runner reduced hot `program_pipeline_baseline` from about 33s to about 1.07s while keeping executable WAT assertions. | Split compile/backend/WAT runtime setup timings into stable reports and define a threshold gate. |
| Self-bootstrap | Missing | `level-1r` is a Rust reference compiler. | Chiba self-hosting path is not complete. |
| Wasm target completeness | Partial | Executable WAT coverage is broad. | Full wasm-gc production target, future no-GC wasm target, manifest, and ownership runtime decisions are not complete. |
| Spec alignment | Partial | Specs have been updated for recent rune/string and continuation decisions in separate docs. | Audit must keep code/spec/test rows synchronized as items close. |
