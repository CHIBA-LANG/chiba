# wasm32 no-GC / Native Layout Spine TODO

This document is the execution plan for adding `wasm32-nogc` without creating a
second compiler. It also defines the spine that a future native backend must
reuse.

## Thesis

`wasm32-nogc` should start now, but only as a layout/ownership proof target.
It must not become an alternate semantic lowering path.

The real work is not "emit another WAT dialect". The real work is to make
ownership, storage, arena/reset regions, dyn adapters, callable storage,
continuations, RC/ARC, and FBIP explicit before emission. Once those facts are
explicit, `wasm-gc`, `wasm32-nogc`, and future native targets are just different
physical layout consumers.

If this system is excellent six months from now, the pipeline looks like this:

```text
Resolved / typed language facts
  -> CoreIR semantic facts
  -> OwnershipStoragePlan
  -> TargetNeutralLayoutPlan
  -> TargetLayoutPlan
  -> Emit
```

The emitters only print target code from layout facts. They do not perform type
lookup, method lookup, dyn row lookup, escape analysis, send analysis,
identifier validation, callable classification, or ownership inference.

## Current State

- [x] `BackendTarget::{WasmGc, Wasm32NoGc, Native}` exists.
- [x] Backend cache config includes target/layout policy/runtime mode.
- [x] Scalar `emit_wasm32_nogc` exists.
- [x] `TargetNeutralLayoutPlan` and `TargetLayoutPlan` exist.
- [x] Visual report exposes neutral and target layout dumps.
- [x] Dyn row neutral layout records adapter entries.
- [x] Dyn row adapter provenance distinguishes field getter vs receiver method
      thunk in the neutral layout.
- [x] Dyn row backend ABI expansion is routed through layout entries.
- [ ] Layout facts are still incomplete for real managed values.
- [ ] Dyn row adapter entries still need exact value lanes from type/layout facts
      instead of conservative placeholders.
- [ ] `wasm32-nogc` currently proves the target spine only for scalar/simple
      cases; it is not yet a full managed runtime backend.
- [ ] Pipeline/CLI target matrix is not complete. Unit tests cover backend
      helpers, but normal compile/run tests still need selectable backend
      execution.

## Non-Negotiable Rules

- [ ] CoreIR / CIR stays target-neutral. It may carry language facts such as
      usage, send, escape, region, ownership decision, continuation kind, replay
      safety, callable storage kind, dyn row access kind, and layout ids. It must
      not carry Wasm-GC structs, `funcref`, `eqref`, WAT opcodes, wasm32 linear
      memory offsets, native pointer sizes, or platform ABI alignment.
- [ ] Backend emitters are dumb. They consume `TargetLayoutPlan`; missing layout
      facts are diagnostics, not an invitation to infer.
- [ ] Dyn row adapters are layout artifacts. Field/method/callable-field/closure
      / boxed `Cont1` / `ContN` member selection must be resolved before target
      layout and emission.
- [ ] Arena is a region/reset-boundary fact, not a malloc wrapper. Arena legality
      participates in escape, replay safety, continuation capture, dyn package
      storage, and ownership decisions.
- [ ] FBIP, RC, ARC, arena, `Cont1`, `ContN`, callable storage, and dyn package
      storage are one ownership/storage system. They must not be implemented as
      independent backend tricks.
- [ ] No semantic pass after lexer/tokenization may infer legality from string
      shape. Identifier, callable, namespace, method, row, and type facts must
      come from AST, symbol tables, typed facts, CoreIR facts, or shared lexer/XID
      tables.
- [ ] Cache keys and manifests include backend target, target features, target
      layout policy, ownership runtime mode, and layout plan hash.

## Kill List

- [ ] Kill backend-local dyn ABI discovery.
- [ ] Kill direct codegen from CoreIR to target object representation for managed
      values.
- [ ] Kill emitter-side method lookup, row lookup, callable classification,
      ownership inference, and identifier/string-shape inference.
- [ ] Kill the idea that `wasm32-nogc` has its own semantic pipeline.
- [ ] Kill any test that only proves the current backend accidentally accepts a
      case while bypassing neutral ownership/layout facts.

## Reuse Strategy

The no-GC backend reuses the existing compiler by adding missing facts between
CoreIR and emission:

- [ ] Reuse parser, resolver, typed, row poly, dynamic row typing, continuation
      typing, and CoreIR lowering.
- [ ] Reuse the same CoreIR for `wasm-gc`, `wasm32-nogc`, and future native.
- [ ] Reuse `TargetNeutralLayoutPlan` as the single abstract object model.
- [ ] Reuse `TargetLayoutPlan` as the target-specific physical representation
      layer.
- [ ] Reuse visual reports and manifests as the proof that each target consumed
      the same semantic facts.
- [ ] Add backend-specific runtime helpers only after ownership/layout facts are
      explicit.

What must not be reused:

- [ ] Do not reuse Wasm-GC reference types as CoreIR assumptions.
- [ ] Do not encode wasm32 offsets into neutral layout.
- [ ] Do not let native pointer/alignment decisions leak into CoreIR.
- [ ] Do not preserve transitional backend-local dyn ABI machinery once layout
      entries can carry the same facts.

## P0 Outcome

P0 for this document is not "the full language runs on no-GC". P0 is the
smallest proof that the architecture is correct and irreversible:

- [ ] Existing scalar programs still compile and run on `wasm-gc`.
- [ ] The same target-neutral CoreIR can produce both `wasm-gc` and
      `wasm32-nogc` target layouts.
- [ ] A scalar-only `wasm32-nogc` artifact emits no Wasm-GC constructs.
- [ ] Managed values cannot be emitted for `wasm32-nogc` unless they have
      ownership decisions, neutral layouts, target layouts, and runtime helper
      requirements.
- [ ] A record/tuple/ADT allocation inside an implicit function reset has a
      region, ownership decision, neutral layout, target layout, and either
      runnable lowering or a precise rejection.
- [ ] Returned aggregates are promoted to a legal caller/outer arena/RC/ARC
      storage location or rejected.
- [ ] Escaping closure environments and callable storage lower through the same
      ownership/layout path as aggregates.
- [ ] `N && !send` values lower to RC; `N && send` values lower to ARC or are
      rejected when the type cannot be sent.
- [ ] Unique update paths can become `InplaceReuse` / FBIP.
- [ ] `ContN` capture blocks unsafe inplace reuse unless replay safety or a
      rollback region proves it safe.
- [ ] Dyn packages created from concrete values carry package layout, adapter
      entries, payload ownership, and target layout. Access goes through adapter
      entries, not backend method lookup.

## Milestone 1: Target Spine And Matrix

- [x] Add target family enum.
- [x] Add target/layout/runtime to backend cache key.
- [x] Add scalar no-GC emitter.
- [x] Add backend target matrix unit tests.
- [ ] Make pipeline/CLI accept a backend target.
- [ ] Make compile/run tests parameterized by backend target.
- [ ] Default to `wasm-gc` until no-GC has managed value support.
- [ ] Add a test harness mode that runs every eligible test under:
  - `wasm-gc`
  - `wasm32-nogc` when the fixture is marked no-GC-ready
  - future `native` layout-only checks when available
- [ ] Ensure full matrix tests are parallelized by test case, not by nested Cargo
      invocations that fight the Cargo lock.

## Milestone 2: OwnershipStoragePlan

- [ ] Define one target-neutral ownership/storage plan.
- [ ] Cover:
  - binders
  - values
  - aggregate allocations
  - closure envs
  - continuation frames/packages
  - boxed `Cont1`
  - `ContN`
  - erased callable storage
  - dyn row packages
  - strings / arrays / slices / vecs
  - builders
- [ ] Track per subject:
  - usage color
  - send color
  - escape color
  - replay safety
  - continuation kind
  - region / arena / reset boundary
  - callable storage kind
  - ownership decision
- [ ] Use target-neutral decisions:
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
- [ ] Validate:
  - missing decision on managed allocations
  - `send` value lowered to non-atomic RC
  - continuation crossing world/thread boundary
  - arena-local value returned or stored past its region
  - escaping dyn payload left stack-only/callee-arena-only
  - `ContN` capture of non-replay-safe mutable state without rollback region
  - target-specific facts leaking into CoreIR or neutral layout

## Milestone 3: Region, Arena, Reset Boundary

Arena should be introduced as the concrete consequence of region facts:

- [ ] Treat explicit `reset` / `resetn` as region boundaries.
- [ ] Treat function and closure calls as implicit regions.
- [ ] Assign every managed allocation site to an initial region.
- [ ] Model nested region containment.
- [ ] Track per region:
  - region id
  - parent region id
  - boundary kind
  - answer type
  - replay safety
  - rollback capability
  - world/thread locality
- [ ] Define promotion paths:
  - callee region -> caller region
  - inner reset -> outer arena
  - arena -> RC
  - arena -> ARC
  - illegal world/thread crossing
- [ ] Reject values that outlive their region without promotion.
- [ ] Reject `ContN` replay over unsafe mutable arena state unless rollback
      region semantics are explicit.
- [ ] Dump region assignment, promotion, and illegal escape.

## Milestone 4: TargetNeutralLayoutPlan

- [x] Add neutral layout plan type.
- [x] Add target layout plan type.
- [x] Generate neutral layouts from current Core layout facts.
- [x] Render neutral and target layout dumps.
- [x] Record dyn row package entries in neutral layout.
- [ ] Move all remaining managed representation facts into neutral layout.
- [ ] Define stable layout ids and layout hashes.
- [ ] Generate neutral layouts for:
  - records
  - tuples
  - ADTs
  - closure envs
  - continuation frames
  - continuation packages
  - boxed `Cont1` state machines
  - erased callable ADTs
  - dyn row packages
  - strings / arrays / slices / vecs
  - arena objects
  - RC/ARC heap objects
- [ ] Track:
  - field/member order
  - payload slots
  - callable variants
  - adapter entries
  - drop requirements
  - ownership decision
  - send requirement
  - replay requirement
  - region requirement
- [ ] Ensure neutral layout contains no Wasm-GC, wasm32 offset, native pointer,
      target ABI, or WAT concepts.
- [ ] Add diagnostics for missing/mismatched neutral layouts.

## Milestone 5: TargetLayoutPlan

- [ ] Define target-specific physical layout as a required step before emission.
- [ ] For `wasm-gc`, map neutral layouts to:
  - struct / array shapes
  - reference types
  - callable representation
  - manifest/debug layout names
- [ ] For `wasm32-nogc`, map neutral layouts to:
  - linear-memory block headers
  - pointer / fat pointer representation
  - layout descriptor ids
  - drop descriptors
  - helper ABI calls
- [ ] For future native, map neutral layouts to:
  - pointer representation
  - alignment and padding
  - platform calling convention
  - function pointer / vtable representation
  - runtime helper ABI
- [ ] Add a target layout validator.
- [ ] Add a no-GC gate proving emitted WAT contains no Wasm-GC constructs.

## Milestone 6: Dynamic Rows And Method Adapters

`dyn {r | ...}` is a package boundary, not a runtime global method search.

- [x] Neutral layout can record dyn adapter entries.
- [x] Field getter vs receiver method thunk provenance is visible in layout dump.
- [x] Backend ABI expansion consumes layout entries.
- [ ] Delete the remaining backend-local dyn member abstraction once layout
      entries carry all required facts.
- [ ] Represent dyn packages target-neutrally:
  - contract id
  - payload slot
  - adapter layout
  - optional nominal/debug identity
  - payload ownership
  - package ownership
  - send / escape / replay facts
- [ ] Represent adapter entries:
  - field getter
  - receiver method thunk
  - callable field thunk
  - closure callable thunk
  - boxed `Cont1` callable thunk
  - `ContN` callable thunk
- [ ] Preserve diagnostic provenance for field/method/callable-field/closure/
      continuation sources.
- [ ] CoreIR must distinguish:
  - `StaticRowAccess`
  - `DynRowAdapterAccess`
  - backend-only shape optimization candidate
- [ ] Expected-type injection from static value to dyn package builds payload,
      adapter entries, ownership facts, and package layout.
- [ ] Dyn-to-concrete nominal conversion requires explicit checked conversion.
- [ ] Target mappings:
  - `wasm-gc`: payload ref + adapter structure/table
  - `wasm32-nogc`: `{payload_ptr, adapter_ptr, contract_id}` or equivalent
  - `native`: `{void* payload, const DynAdapter* adapter}` or equivalent

## Milestone 7: Callable Storage, Closures, Continuations

- [ ] Lower no-capture closures to direct callable facts.
- [ ] Lower escaping closure envs through ownership/layout.
- [ ] Lower stored `(A) -> B` to erased callable storage with variants:
  - direct function
  - no-capture closure
  - env closure
  - boxed `Cont1`
  - `ContN` package
- [ ] Direct-resume non-escaping exactly-once `Cont1` without allocation.
- [ ] Box escaping `Cont1` as a one-shot consumed-state machine.
- [ ] Lower `ContN` as a multi-shot package.
- [ ] Ensure `((A) -> B) send` excludes `Cont1`, boxed `Cont1`, `ContN`, and all
      `!send` closures.
- [ ] Ensure `ContN` captured state is replay-safe, immutable/persistent, or
      rollback-region-managed.
- [ ] Reject:
  - cross-world continuation capture/resume
  - `send` continuation storage
  - multi-shot capture of unsafe mutation state
  - FBIP over replayed captured state

## Milestone 8: RC/ARC And Perceus Ops

- [ ] Extend usage analysis with:
  - last use point
  - owned consume
  - borrowed read
  - duplication point
  - drop point
  - field consume
  - whole-object consume
- [ ] Add target-neutral ownership ops:
  - `Dup`
  - `Drop`
  - `Consume`
  - `Borrow`
  - `ReturnOwned`
  - `StoreOwned`
- [ ] Insert exact `dup` / `drop` before target layout emission.
- [ ] Prove inserted ops are balanced per control path.
- [ ] Cover branch, match, early return, loop, closure capture, continuation
      capture, callable storage, and dyn package storage.
- [ ] Dump exact Perceus operations.

## Milestone 9: Uniqueness And FBIP

Uniqueness is an internal compiler fact first. It does not need to start as a
source-level type constructor.

- [ ] Track uniqueness over:
  - local binders
  - aggregate allocations
  - record updates
  - ADT destruct/reconstruct
  - arrays / vecs / strings where applicable
  - closure envs
  - dyn row packages
- [ ] Detect FBIP candidates:
  - record update
  - ADT match destruct followed by same-layout reconstruct
  - tuple/product update
  - builder freeze / append
- [ ] Reject FBIP when:
  - value has live aliases
  - value is captured by `ContN`
  - value crosses world/thread
  - value is stored in shared callable/dyn package
  - value contains non-replay-safe state
  - layout is incompatible
- [ ] Rewrite CoreIR functional updates to `InplaceReuse` only after proof.

## Milestone 10: Runtime Object Layout For wasm32-nogc

This layer is backend-specific. None of it belongs in CoreIR or neutral layout.

- [ ] Define linear-memory block header:
  - layout id
  - strong count
  - flags
  - size or payload size
  - optional drop/layout descriptor pointer
- [ ] Define RC and ARC count behavior.
- [ ] Define alignment and pointer representation.
- [ ] Define null / unit / immediate representation.
- [ ] Define fat pointer representation for:
  - slice
  - str
  - cstr
  - dyn row package
  - closure package
  - continuation package
  - array / vec
- [ ] Add target layout validation for pointer/fat-pointer shape.

## Milestone 11: Runtime Helpers

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
- [ ] Decide helper delivery:
  - imported host ABI
  - linked runtime module
  - emitted per bundle
- [ ] Add helper manifest entries.
- [ ] Add golden tests for helper symbol names and signatures.
- [ ] Add minimal standalone runtime for tests.
- [ ] Reserve native helper ABI details for native target layout.

## Milestone 12: Layout Metadata And Destructors

- [ ] Generate layout metadata for:
  - record
  - tuple
  - ADT
  - closure env
  - callable storage
  - continuation package
  - boxed `Cont1`
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
- [ ] Reject or promote arena values that would leak RC fields or longer-lived
      references.

## Milestone 13: Strings, Arrays, Slices, Vec

- [ ] Define target-neutral ownership facts for:
  - `str` borrowed view
  - `String` owned managed value
  - `cstr` ABI-compatible view/value
  - `Array[T]` immutable managed value
  - `Slice[T]` borrowed view
  - `Vec[T]` growable owned value
- [ ] Under `wasm32-nogc`, define linear-memory layouts and helper ABIs.
- [ ] Under future native, reuse the same neutral layout and choose native
      physical representation in target layout.
- [ ] Ensure `String.char_at(n)` returns `rune` / `u32` and uses UTF-8 traversal.
- [ ] Keep `String[i]` byte-oriented if the spec says byte indexing.
- [ ] Add RC / uniqueness behavior for builders and append operations.
- [ ] Add FBIP candidate tests for unique builder / vec append paths.

## Milestone 14: BIR And Emission

- [ ] BIR consumes target layout, not source-level type guesses.
- [ ] BIR distinguishes:
  - scalar values
  - owned pointers
  - borrowed pointers
  - arena pointers
  - RC pointers
  - ARC pointers
  - fat pointers
  - function pointers / callable table entries
- [ ] Lower ownership ops into runtime helper calls or direct operations according
      to target layout.
- [ ] Lower `wasm32-nogc` target layout to wasm32 linear-memory WAT.
- [ ] Lower `wasm-gc` target layout to current Wasm-GC WAT without leaking
      Wasm-GC choices back into neutral layout.
- [ ] Reserve native emission behind the same target layout interface.
- [ ] Add WAT validation tests.
- [ ] Add wasmtime execution tests.
- [ ] Add negative tests ensuring emitters refuse missing ownership/layout facts.

## Milestone 15: Visualization And Manifest

- [ ] Dump:
  - typed
  - usage-colored typed
  - region assignment
  - escape graph
  - ownership/storage plan
  - neutral layout plan
  - target layout plan
  - Perceus dup/drop
  - FBIP rewrites
  - CoreIR ownership validation
  - BIR runtime ops
  - final symbols
- [ ] Manifest maps final symbols back to:
  - source path
  - namespace
  - item
  - specialization key
  - neutral layout key
  - target layout key
  - ownership decision
  - runtime helper dependency
- [ ] Add stable golden fixtures for these dumps.
- [ ] Add gates preventing target-specific terms from appearing in CoreIR and
      neutral layout dumps.

## Milestone 16: Test Matrix

Tests should prove the lifecycle contract, not only the current implementation.
Some fixtures may be marked expected-fail for the current compiler if they are
valid Chiba lifecycle spec cases.

- [ ] Scalar no-GC smoke:
  - arithmetic
  - function call
  - branch
  - match
- [ ] Layout spine smoke:
  - same CoreIR produces `wasm-gc` target layout
  - same CoreIR produces `wasm32-nogc` target layout
  - neutral layout dump contains no target-specific terms
  - emit fails when target layout is missing
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
  - `ContN` capture blocks unsafe reuse
- [ ] Continuation smoke:
  - local `Cont1`
  - boxed `Cont1` one-shot trap
  - `ContN` multi-resume
  - replay-unsafe capture rejection
- [ ] Dyn row smoke:
  - static value to dyn package
  - field adapter
  - method adapter
  - callable-field adapter
  - closure / `Cont1` / `ContN` callable member
  - escaping payload ownership
- [ ] Runtime layout smoke:
  - layout metadata present
  - destructor drops nested RC fields
  - arena purity enforced
- [ ] Evil depth smoke:
  - deep `ContN` nesting
  - deeply nested pattern as expression
  - deeply nested pattern as function parameter
  - deeply nested row/dyn adapter contract
  - deep closure/continuation capture chain

## First Proof Point

The next implementation proof should be deliberately narrow:

- [ ] Finish replacing backend-local dyn row ABI machinery with neutral layout
      entries.
- [ ] Give dyn field getter entries exact backend value lanes from typed/layout
      facts.
- [ ] Add a no-GC scalar target layout/emission gate that rejects managed values
      without ownership/layout facts.
- [ ] Add pipeline target selection for `wasm-gc` vs `wasm32-nogc`.
- [ ] Add a small target-matrix test runner that exercises the same source under
      all eligible backends.
- [ ] Add diagnostic tests proving emitters refuse to infer missing dyn adapter,
      ownership, callable, continuation, or layout facts.

## What Would Falsify This Direction

- If a managed no-GC value cannot be emitted without redoing method/type/row
  lookup in the backend, the middle-end facts are incomplete and must be fixed.
- If native requires different semantic lowering from `wasm32-nogc`, the layout
  boundary is wrong.
- If arena legality cannot be expressed before emission, arena has been placed at
  the wrong layer.
- If `ContN` replay safety cannot block FBIP/unsafe mutation before emission,
  continuation and ownership facts are not yet integrated.
- If dyn method extraction needs runtime global lookup, dyn row adapters are not
  being built at the correct phase.

## Spec Anchors

- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/minimal-lifetime-commitments.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/memory-model.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/escape-semantics.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/promotion-rules.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/03-memory-and-lifetimes/uniqueness.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/passes-and-placement.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/cir-cps-ir.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec/10-ir-and-lowering/cir-to-bir-lowering.md`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/blog/chiba2.md`
