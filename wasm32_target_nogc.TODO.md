# wasm32 no-GC Target TODO

This file tracks the design and implementation work needed to add a `wasm32` no-GC backend that uses arena / escape / Perceus RC / uniqueness / FBIP facts instead of relying on Wasm-GC objects.

The goal is not to replace the current `wasm-gc` target. The goal is to make both targets consume the same target-neutral CoreIR / BIR semantic facts.

Relevant spec anchors:

- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/minimal-lifetime-commitments.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/memory-model.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/escape-semantics.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/promotion-rules.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/uniqueness.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/passes-and-placement.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/cir-cps-ir.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/cir-to-bir-lowering.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/blog/chiba2.md`

## Hard Constraints

- [ ] CIR / CoreIR must stay target-neutral. It may carry language-level facts such as usage, send, escape, region, ownership decision, continuation kind, replay safety, callable storage kind, and dyn row access kind. It must not carry Wasm-GC struct layout, `funcref`, `eqref`, WAT opcodes, Binaryen flags, or wasm32 linear-memory offsets.
- [ ] Backend emitters must be dumb. `wasm-gc` and `wasm32-nogc` may choose different physical layouts, but neither backend may re-infer type, escape, send, replay, ownership, callable, method, row, or identifier semantics.
- [ ] No semantic pass after lexer/tokenization may infer legality from string shape. Identifier, callable, namespace, method, row, and type facts must come from AST, symbol table, typed facts, CoreIR facts, or shared lexer/XID tables.
- [ ] `wasm32-nogc` must be introduced as a target, not as an alternate lowering pipeline that changes language behavior.
- [ ] Cache keys and manifests must include target backend, target features, and ownership runtime mode.

## P0 Outcome

- [ ] A small source program using scalar values still compiles to current `wasm-gc`.
- [ ] The same target-neutral CoreIR can be emitted as `wasm32-nogc` for scalar-only programs.
- [ ] A small program allocating a record / tuple / ADT inside an implicit function `reset` can run on `wasm32-nogc` without Wasm-GC.
- [ ] A returned aggregate is promoted out of the callee arena into a legal caller / outer arena / RC target.
- [ ] A closure capture that escapes the current call boundary is promoted and callable under `wasm32-nogc`.
- [ ] `N && !send` values lower to RC, `N && send` values lower to ARC or are rejected when the type cannot be sent.
- [ ] A simple unique record update or ADT destruct/reconstruct pattern can trigger `InplaceReuse` / FBIP under `wasm32-nogc`.
- [ ] A multi-shot continuation capture blocks unsafe inplace reuse, unsafe mutation capture, or non-replay-safe RC/arena state according to replay facts.

## 1. Target Model

- [ ] Add stable backend target names:
  - `wasm-gc`
  - `wasm32-nogc`
- [ ] Add target feature flags:
  - `tailcall`
  - `bulk-memory`
  - `reference-types` only if used for host interop, not for managed Chiba values
  - `threads`
  - `atomics`
- [ ] Add ownership runtime modes:
  - `none`
  - `arena-only`
  - `rc`
  - `rc-arc`
  - `rc-arc-fbip`
- [ ] Include target + feature + ownership runtime mode in backend cache keys.
- [ ] Include target + ownership decision in debug manifest and visual dumps.

## 2. Ownership Decision Contract

- [ ] Define a target-neutral `OwnershipDecision` enum in CoreIR / ownership side data:
  - `StackValue`
  - `InplaceReuse`
  - `ArenaLocal`
  - `PromoteToCallerRegion`
  - `PromoteToOuterArena`
  - `RC`
  - `ARC`
  - `StaticData`
  - `BorrowedView`
  - `DynPackage`
- [ ] Attach ownership decision to:
  - binders
  - values
  - aggregate allocations
  - closure env allocations
  - continuation frames / packages
  - callable storage
  - dynamic row packages
  - strings / arrays / slices / vecs
- [ ] Add a CoreIR validator that rejects:
  - missing ownership decision on managed allocations
  - `send` values lowered to non-atomic RC
  - continuation values crossing world/thread boundaries
  - arena-local values returned or stored past their region
  - Wasm-GC-only facts leaking into CoreIR

## 3. Region And Arena Model

- [ ] Represent explicit `reset` as a region / arena boundary in CIR.
- [ ] Represent implicit function call and closure call `reset` boundaries.
- [ ] Assign every managed allocation site to an initial region.
- [ ] Model nested `reset` regions and determine the innermost region that covers the value lifetime.
- [ ] Define region facts:
  - region id
  - parent region id
  - boundary kind: explicit reset / function call / closure call
  - answer type
  - replay safety
  - world/thread id or locality
- [ ] Add visual dumps for region assignment and promotion decisions.
- [ ] Add tests for:
  - local aggregate freed with function arena
  - value escaping inner reset but not outer reset
  - value returned from function cannot reference callee arena

## 4. Escape Analysis

- [ ] Build an escape graph over typed / CIR values.
- [ ] Treat these as escape points:
  - return
  - closure capture
  - continuation capture
  - `send`
  - store into longer-lived object
  - static/global storage
  - dyn row package storage
  - callable storage
- [ ] Implement contagious escape propagation: if an arena value contains or points to an escaping value, the containing value must also escape or be rejected if illegal.
- [ ] Distinguish escape targets:
  - caller region
  - outer arena
  - RC heap
  - ARC heap
  - illegal world/thread crossing
- [ ] Ensure arena objects marked pure do not contain RC / ARC pointers unless the arena destructor strategy explicitly supports dropping them.
- [ ] Add negative tests for dangling callee arena references.

## 5. Perceus Precise RC

- [ ] Extend usage analysis beyond `1 | N | Unknown` where needed:
  - last use point
  - owned consume
  - borrowed read
  - duplication point
  - drop point
  - field consume
  - whole-object consume
- [ ] Add target-neutral CoreIR ownership ops:
  - `Dup`
  - `Drop`
  - `Consume`
  - `Borrow`
  - `ReturnOwned`
  - `StoreOwned`
- [ ] Insert precise `dup` / `drop` at CIR/CoreIR level, before BIR/backend.
- [ ] Prove inserted ops are balanced per control path.
- [ ] Account for branch, match, early return, loop, closure capture, and continuation capture paths.
- [ ] Add visual dump showing exact Perceus operations.
- [ ] Add tests for:
  - last-use drop
  - branch-balanced drop
  - shared closure capture causing dup
  - callable storage causing RC
  - no extra dup/drop for single-use local value

## 6. Uniqueness And FBIP

- [ ] Define uniqueness as an internal compiler fact, not a mandatory source-level type constructor.
- [ ] Track uniqueness over:
  - local binders
  - aggregate allocations
  - record updates
  - ADT destruct/reconstruct
  - arrays / vecs / strings where applicable
  - closure envs
  - dyn row packages
- [ ] Detect FBIP candidates:
  - record update `{old | field: new}`
  - ADT match destruct followed by reconstruct of same layout
  - tuple/product update when layout-compatible
  - builder freeze / append patterns where ownership is unique
- [ ] Reject FBIP when:
  - value has live aliases
  - value is captured by `ContN`
  - value crosses world/thread
  - value is stored in shared callable / dyn package
  - value contains non-replay-safe state
  - layout is not compatible
- [ ] Add CoreIR rewrite from functional update to `InplaceReuse`.
- [ ] Add tests showing:
  - pure functional code emits inplace update under unique ownership
  - shared value falls back to RC allocation
  - ContN replay blocks inplace update

## 7. Runtime Object Layout For wasm32-nogc

- [ ] Define a target-specific linear-memory block header in the wasm32 backend layer:
  - layout id
  - strong count
  - flags
  - size or payload size
  - optional drop descriptor / layout descriptor pointer
- [ ] Define separate RC and ARC count behavior.
- [ ] Define alignment and pointer representation.
- [ ] Define null / unit / immediate representation.
- [ ] Define fat pointer representation for:
  - slice
  - str
  - cstr
  - dyn row package
  - closure package
  - continuation package
- [ ] Ensure this layout is not visible in CIR.

## 8. Runtime Helpers

- [ ] Specify stable helper names and ABIs:
  - `chiba_alloc`
  - `chiba_free`
  - `chiba_rc_dup`
  - `chiba_rc_drop`
  - `chiba_arc_dup`
  - `chiba_arc_drop`
  - `chiba_unique`
  - `chiba_reuse_or_alloc`
  - `chiba_arena_new`
  - `chiba_arena_alloc`
  - `chiba_arena_reset`
  - `chiba_panic`
- [ ] Decide whether helpers are imported, linked from a runtime module, or emitted per bundle.
- [ ] Add helper manifest entries.
- [ ] Add golden tests for helper symbol names and signatures.
- [ ] Add a minimal standalone runtime for tests.

## 9. Layout Metadata And Destructors

- [ ] Generate layout metadata for:
  - record
  - tuple
  - ADT
  - closure env
  - continuation package
  - dyn row package
  - string / array / vec
- [ ] Track whether layout contains:
  - RC fields
  - ARC fields
  - arena pointers
  - `Ref`
  - `UnsafeRef`
  - continuation values
  - callable storage
  - dyn row package
- [ ] Generate drop descriptors for recursive field drops.
- [ ] Ensure arena-local pure objects do not need per-object destructor traversal.
- [ ] Reject or promote arena values that would leak RC fields.

## 10. Closures And Callable Storage

- [ ] Lower no-capture closures to direct function / funcref-equivalent target-neutral callable facts.
- [ ] Lower escaping closure envs according to ownership decision.
- [ ] Lower stored `(A) -> B` to erased callable ADT with variants:
  - direct function
  - no-capture closure
  - env closure
  - boxed `Cont1`
  - `ContN` package
- [ ] Under wasm32-nogc, materialize callable storage as linear-memory package or static dispatch table entry.
- [ ] Ensure `((A) -> B) send` excludes `Cont1`, boxed `Cont1`, `ContN`, and `!send` closures.
- [ ] Add tests for storing and calling functions, closures, `Cont1`, and `ContN` under no-GC.

## 11. Continuations

- [ ] Attach ownership decisions to continuation frames and packages.
- [ ] Direct-resume non-escaping exactly-once `Cont1` without allocation.
- [ ] Box escaping `Cont1` as one-shot consumed-state machine.
- [ ] Lower `ContN` as multi-shot package.
- [ ] Ensure `ContN` captured state is replay-safe or rollback-region managed.
- [ ] Reject:
  - cross-world continuation capture / resume
  - `send` continuation storage
  - multi-shot capture of unsafe mutation state
  - FBIP over replayed captured state
- [ ] Add tests for local/global/param capture, `Ref`, `UnsafeRef`, closure capture, and repeated `ContN` resume.

## 12. Dynamic Rows And Method Adapters

- [ ] Treat `dyn {r | ...}` as `DynPackage` ownership decision.
- [ ] Represent dyn row package target-neutrally as:
  - payload
  - adapter table / member table
  - optional nominal/debug identity
  - send / ownership facts
- [ ] Under wasm32-nogc, lower dyn package to linear-memory package plus adapter table.
- [ ] Preserve distinction between:
  - `StaticRowAccess`
  - `DynRowAdapterAccess`
  - backend-only shape optimization
- [ ] Ensure row method extraction does not require runtime global impl search.
- [ ] Add tests for dyn row creation from value, method extraction, field extraction, and RC/ARC ownership.

## 13. Strings, Arrays, Slices, Vec

- [ ] Define target-neutral ownership facts for:
  - `str` borrowed view
  - `String` owned managed value
  - `cstr` ABI-compatible view/value
  - `Array[T]` immutable managed value
  - `Slice[T]` borrowed view
  - `Vec[T]` growable owned value
- [ ] Under wasm32-nogc, define linear-memory layouts and helper ABIs.
- [ ] Ensure `String.char_at(n)` returns `rune` / `u32` and has correct UTF-8 traversal semantics.
- [ ] Keep `String[i]` byte-oriented if the spec says byte indexing.
- [ ] Add RC / uniqueness behavior for builders and append operations.
- [ ] Add FBIP candidate tests for unique builder / vec append paths.

## 14. BIR And wasm32-nogc Emission

- [ ] BIR must distinguish:
  - scalar values
  - owned pointers
  - borrowed pointers
  - arena pointers
  - RC pointers
  - ARC pointers
  - fat pointers
- [ ] Lower CoreIR ownership ops into BIR runtime helper calls or direct operations.
- [ ] Lower BIR to wasm32 linear-memory WAT.
- [ ] Do not use Wasm-GC types in wasm32-nogc output.
- [ ] Add WAT validation tests.
- [ ] Add wasmtime execution tests.
- [ ] Add negative tests ensuring wasm32-nogc emitter refuses missing ownership facts.

## 15. Debuggability And Visualization

- [ ] Dump these layers:
  - typed
  - usage-colored typed
  - region assignment
  - escape graph
  - ownership decisions
  - Perceus dup/drop
  - FBIP rewrites
  - CoreIR ownership validation
  - BIR runtime ops
  - final wasm32 symbols
- [ ] Manifest must map final symbols back to:
  - source path
  - namespace
  - item
  - specialization key
  - layout key
  - ownership decision
  - runtime helper dependency
- [ ] Add stable golden fixtures for these dumps.

## 16. Test Plan

- [ ] Scalar no-GC smoke:
  - arithmetic
  - function call
  - branch
  - match
- [ ] Arena smoke:
  - local record allocation
  - local ADT allocation
  - nested reset region
  - arena reset after function return
- [ ] Escape smoke:
  - return aggregate
  - closure capture
  - store into longer-lived object
  - dyn row package
- [ ] RC smoke:
  - shared value
  - branch-balanced dup/drop
  - closure env sharing
  - callable storage
- [ ] ARC / send smoke:
  - sendable owned value
  - non-send closure rejection
  - continuation send rejection
  - `Ref` send rejection
- [ ] FBIP smoke:
  - record update
  - ADT destruct/reconstruct
  - unique vec append
  - shared fallback no-reuse
- [ ] Continuation smoke:
  - local `Cont1`
  - boxed `Cont1` one-shot trap
  - `ContN` multi-resume
  - replay-unsafe capture rejection
- [ ] Runtime layout smoke:
  - layout metadata present
  - destructor drops nested RC fields
  - arena purity enforced
