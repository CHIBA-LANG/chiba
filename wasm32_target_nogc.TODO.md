# wasm32 no-GC / Native Layout Spine TODO

This file tracks the work needed to make `wasm32-nogc` possible without turning it
into a second compiler. The same work is also the foundation for a future native
backend.

The thesis is deliberately stronger than "add a wasm32 backend":

> `wasm32-nogc` is the first proof target for Chiba's ownership, storage, layout,
> dyn adapter, continuation, arena, RC/ARC, and FBIP model. It must consume
> target-neutral facts. It must not infer language semantics in the emitter.

If this system is excellent six months from now, `wasm-gc`, `wasm32-nogc`, and a
native backend all consume the same CoreIR ownership/storage/layout facts. They
only differ in physical representation, runtime helper ABI, and final emission.

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

## Non-Negotiable Principles

- [ ] CoreIR / CIR stays target-neutral. It may carry language facts such as usage,
      send, escape, region, ownership decision, continuation kind, replay safety,
      callable storage kind, dyn row access kind, and target-neutral layout ids.
      It must not carry Wasm-GC struct layout, `funcref`, `eqref`, WAT opcodes,
      Binaryen flags, wasm32 linear-memory offsets, native pointer sizes, or
      platform ABI alignment.
- [ ] Backend emitters are dumb. `wasm-gc`, `wasm32-nogc`, and future native
      backends may choose different physical layouts, but none of them may
      re-infer type, escape, send, replay, ownership, callable, method, row, or
      identifier semantics.
- [ ] Dyn row adapters are layout artifacts, not runtime method lookup. Field,
      method, callable-field, closure, `Cont1`, and `ContN` member selection must
      be resolved before layout planning.
- [ ] Arena is a region / reset-boundary fact, not a malloc wrapper optimization.
      It participates in escape, replay safety, continuation, dyn package, and
      ownership decisions.
- [ ] FBIP / inplace reuse, RC, ARC, arena, `Cont1`, `ContN`, and dyn package
      storage are one ownership/storage system. They must not be implemented as
      independent backend tricks.
- [ ] No semantic pass after lexer/tokenization may infer legality from string
      shape. Identifier, callable, namespace, method, row, and type facts must
      come from AST, symbol table, typed facts, CoreIR facts, or shared lexer/XID
      tables.
- [ ] Cache keys and manifests must include backend target, target features,
      target layout policy, ownership runtime mode, and layout plan hash.

## Kill List

- [ ] Kill backend-local semantic dyn ABI inference. Current backend-side
      construction of dyn row field/method ABI must move into target-neutral
      layout planning.
- [ ] Kill direct codegen from CoreIR to target-specific object representation for
      managed values. Codegen must go through layout plans.
- [ ] Kill any fallback where an emitter sees a type/row/method shape and silently
      decides ownership, adapter entries, or callable storage.
- [ ] Kill the idea that `wasm32-nogc` is a special alternate lowering pipeline.
      It is a target family consuming the same language facts as every other
      backend.

## Target Architecture

The backend path should become:

```text
Typed / resolved language facts
  -> CoreIR semantic facts
  -> OwnershipStoragePlan
  -> TargetNeutralLayoutPlan
  -> TargetLayoutPlan
  -> Emit
```

Layer responsibilities:

- [ ] `CoreIR semantic facts`: value/type facts, row access kind, dyn package ops,
      continuation kind, callable storage kind, usage/send/escape/replay colors,
      arena/reset boundary ids, intrinsic ownership namespace.
- [ ] `OwnershipStoragePlan`: target-neutral decision for each managed value,
      aggregate, closure env, continuation frame/package, callable storage, dyn
      package, string, array, slice, vec, and builder.
- [ ] `TargetNeutralLayoutPlan`: abstract object shapes such as record fields,
      tuple fields, ADT variants, closure captures, continuation env slots,
      erased callable variants, dyn package entries, drop requirements, region
      requirements, and layout ids.
- [ ] `TargetLayoutPlan`: physical representation for a target family, such as
      Wasm-GC struct/funcref, wasm32 linear-memory offsets/helper ABI, or native
      pointer/alignment/calling convention.
- [ ] `Emit`: prints WAT/object/native IR from target layout only. Missing layout
      is a diagnostic, not an invitation to infer.

## P0 Outcome

- [ ] Existing scalar programs still compile and run on `wasm-gc`.
- [ ] The same target-neutral CoreIR can produce a `wasm-gc` target layout and a
      `wasm32-nogc` target layout for scalar-only programs.
- [ ] A scalar-only `wasm32-nogc` artifact emits no Wasm-GC constructs.
- [ ] A record / tuple / ADT allocation inside an implicit function reset has an
      ownership decision, neutral layout, target layout, and runnable no-GC
      lowering.
- [ ] A returned aggregate is promoted out of the callee region into a legal
      caller region, outer arena, RC/ARC heap, or is rejected.
- [ ] An escaping closure env is promoted and callable under `wasm32-nogc`.
- [ ] `N && !send` values lower to RC, `N && send` values lower to ARC or are
      rejected when the type cannot be sent.
- [ ] A unique record update or ADT destruct/reconstruct can produce
      `InplaceReuse` / FBIP.
- [ ] Capturing the same value in `ContN` blocks unsafe inplace reuse unless a
      replay-safe immutable or rollback-region proof exists.
- [ ] A dyn package created from a concrete value carries a package layout, adapter
      entries, payload ownership, and target layout. Access goes through adapter
      entries, not backend method lookup.

## 1. Target Families And Configuration

- [ ] Replace single-target assumptions with stable target families:
  - `wasm-gc`
  - `wasm32-nogc`
  - `native`
- [ ] Keep `wasm-gc` as the current production target.
- [ ] Add `wasm32-nogc` as the first non-GC proof target.
- [ ] Reserve `native` as a layout consumer, not an immediate emitter requirement.
- [ ] Define target features:
  - `tailcall`
  - `bulk-memory`
  - `reference-types` only for host interop, not managed Chiba values
  - `threads`
  - `atomics`
  - native pointer width
  - native ABI family
- [ ] Define ownership runtime modes:
  - `none`
  - `arena-only`
  - `rc`
  - `rc-arc`
  - `rc-arc-fbip`
- [ ] Include target family, target features, layout policy, and ownership runtime
      mode in backend cache keys.
- [ ] Include target family, target layout id, ownership decision, and runtime
      helper dependency in manifests and visual dumps.

## 2. OwnershipStoragePlan

- [ ] Define one target-neutral ownership/storage plan rather than scattered
      backend decisions.
- [ ] The plan must cover:
  - binders
  - values
  - aggregate allocations
  - closure env allocations
  - continuation frames / packages
  - erased callable storage
  - dynamic row packages
  - strings / arrays / slices / vecs
  - builders
- [ ] Track these facts per subject:
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
- [ ] Add a validator that rejects:
  - missing ownership decision on managed allocations
  - `send` values lowered to non-atomic RC
  - continuation values crossing world/thread boundaries
  - arena-local values returned or stored past their region
  - escaping dyn payload that remains stack-only or callee-arena-only
  - `ContN` capture of non-replay-safe mutable state without rollback region
  - target-specific facts leaking into CoreIR or the neutral plan

## 3. Region, Arena, And Reset Boundary

- [ ] Represent explicit `reset` / `resetn` as region boundaries.
- [ ] Represent implicit function call and closure call boundaries.
- [ ] Assign every managed allocation site to an initial region.
- [ ] Model nested regions and determine the innermost region that covers each
      value lifetime.
- [ ] Track per region:
  - region id
  - parent region id
  - boundary kind: explicit reset / resetn / function call / closure call
  - answer type
  - replay safety
  - rollback capability
  - world/thread locality
- [ ] Treat arena allocation as a consequence of region facts and ownership plan,
      not as an emitter choice.
- [ ] Define promotion paths:
  - callee region to caller region
  - inner reset to outer arena
  - arena to RC
  - arena to ARC
  - illegal world/thread crossing
- [ ] Add visual dumps for region assignment, promotion, and illegal escape.
- [ ] Add tests for:
  - local aggregate freed with function arena
  - value escaping inner reset but not outer reset
  - value returned from function cannot reference callee arena
  - `ContN` replay with rollback-region-managed local state
  - `ContN` replay rejection without rollback or immutable proof

## 4. TargetNeutralLayoutPlan

- [ ] Introduce a layout planning module after CoreIR / ownership planning and
      before target-specific backend layout.
- [ ] Define neutral layout ids and stable layout hashes.
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
- [ ] Track layout metadata:
  - field/member order
  - payload slots
  - callable variants
  - adapter entries
  - drop requirements
  - ownership decision
  - send requirement
  - replay requirement
  - region requirement
- [ ] Ensure neutral layout does not contain:
  - Wasm-GC `struct`, `array`, `funcref`, `eqref`
  - wasm32 linear-memory offsets
  - native pointer width
  - target ABI alignment
  - WAT opcodes
- [ ] Add a neutral layout dump and golden tests.
- [ ] Add diagnostics for missing or mismatched neutral layouts.

## 5. TargetLayoutPlan

- [ ] Define target-specific physical layout as a separate step from neutral layout.
- [ ] For `wasm-gc`, map neutral layouts to:
  - struct / array shapes
  - reference types
  - funcref or equivalent callable representation
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
- [ ] Add a target layout validator that rejects missing physical layouts and
      target-incompatible layout features.
- [ ] Add a gate proving that `wasm32-nogc` output uses no Wasm-GC constructs.

## 6. Dynamic Rows And Method Adapters

- [ ] Treat `dyn {r | ...}` as a package boundary with
      `OwnershipDecision::DynPackage`.
- [ ] Represent dyn packages target-neutrally as:
  - contract id
  - payload slot
  - adapter layout
  - optional nominal/debug identity
  - payload ownership
  - package ownership
  - send / escape / replay facts
- [ ] Represent adapter entries target-neutrally:
  - field getter
  - receiver method thunk
  - callable field thunk
  - closure callable thunk
  - boxed `Cont1` callable thunk
  - `ContN` callable thunk
- [ ] Preserve diagnostic provenance for whether a member came from a field,
      method, callable field, closure, `Cont1`, or `ContN`.
- [ ] Ensure dyn access in CoreIR distinguishes:
  - `StaticRowAccess`
  - `DynRowAdapterAccess`
  - backend-only shape optimization candidate
- [ ] Ensure dyn adapter access never performs runtime global method search.
- [ ] Expected-type injection from static value to dyn package must build:
  - payload
  - adapter entries
  - payload ownership facts
  - package layout fact
- [ ] `dyn` back to concrete nominal type must require explicit checked
      conversion; never infer nominal type from shape.
- [ ] Target mappings:
  - `wasm-gc`: payload ref + adapter struct/table/funcref representation
  - `wasm32-nogc`: `{payload_ptr, adapter_ptr, contract_id}` or equivalent
  - `native`: `{void* payload, const DynAdapter* adapter}` or equivalent
- [ ] Add tests for:
  - dyn created from record field
  - dyn created from nominal method
  - dyn created from callable field
  - dyn callable member backed by function / closure / boxed `Cont1` / `ContN`
  - escaping dyn payload cannot remain stack-only
  - method extraction does not require backend lookup

## 7. Callable Storage, Closures, And Continuations

- [ ] Lower no-capture closures to direct functions / target-neutral direct callable
      facts.
- [ ] Lower escaping closure envs according to ownership decisions.
- [ ] Lower stored `(A) -> B` to erased callable storage with variants:
  - direct function
  - no-capture closure
  - env closure
  - boxed `Cont1`
  - `ContN` package
- [ ] Direct-resume non-escaping exactly-once `Cont1` without allocation.
- [ ] Box escaping `Cont1` as a one-shot consumed-state machine.
- [ ] Lower `ContN` as a multi-shot package.
- [ ] Ensure `((A) -> B) send` excludes `Cont1`, boxed `Cont1`, `ContN`, and
      all `!send` closures.
- [ ] Ensure `ContN` captured state is replay-safe, immutable/persistent, or
      rollback-region-managed.
- [ ] Reject:
  - cross-world continuation capture / resume
  - `send` continuation storage
  - multi-shot capture of unsafe mutation state
  - FBIP over replayed captured state
- [ ] Add tests for local/global/param capture, `Ref`, `UnsafeRef`, closure
      capture, stored callable invocation, boxed `Cont1` repeated-call trap, and
      repeated `ContN` resume.

## 8. Perceus RC/ARC And Precise Ownership Ops

- [ ] Extend usage analysis beyond `1 | N | Unknown` where needed:
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
- [ ] Insert precise `dup` / `drop` before target-specific layout emission.
- [ ] Prove inserted ops are balanced per control path.
- [ ] Account for branch, match, early return, loop, closure capture,
      continuation capture, callable storage, and dyn package storage.
- [ ] Add visual dump showing exact Perceus operations.
- [ ] Add tests for:
  - last-use drop
  - branch-balanced drop
  - shared closure capture causing dup
  - callable storage causing RC
  - dyn package causing payload ownership handling
  - no extra dup/drop for single-use local value

## 9. Uniqueness And FBIP

- [ ] Define uniqueness as an internal compiler fact, not a mandatory source-level
      type constructor.
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
  - `ContN` replay blocks inplace update
  - dyn package storage blocks unsafe uniqueness assumptions

## 10. Runtime Object Layout For wasm32-nogc

- [ ] Define a wasm32 backend-layer linear-memory block header:
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
  - array / vec
- [ ] Ensure this layout is not visible in CoreIR or neutral layout.

## 11. Runtime Helpers

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
- [ ] Decide whether helpers are imported, linked from a runtime module, or emitted
      per bundle.
- [ ] Add helper manifest entries.
- [ ] Add golden tests for helper symbol names and signatures.
- [ ] Add a minimal standalone runtime for tests.
- [ ] Reserve native helper ABI decisions for the native target layout layer.

## 12. Layout Metadata And Destructors

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

## 13. Strings, Arrays, Slices, Vec

- [ ] Define target-neutral ownership facts for:
  - `str` borrowed view
  - `String` owned managed value
  - `cstr` ABI-compatible view/value
  - `Array[T]` immutable managed value
  - `Slice[T]` borrowed view
  - `Vec[T]` growable owned value
- [ ] Under `wasm32-nogc`, define linear-memory layouts and helper ABIs.
- [ ] Under future native, reuse the same neutral layout and choose native physical
      representation in target layout.
- [ ] Ensure `String.char_at(n)` returns `rune` / `u32` and has correct UTF-8
      traversal semantics.
- [ ] Keep `String[i]` byte-oriented if the spec says byte indexing.
- [ ] Add RC / uniqueness behavior for builders and append operations.
- [ ] Add FBIP candidate tests for unique builder / vec append paths.

## 14. BIR And Emission

- [ ] BIR must consume target layout, not infer layout from source-level type.
- [ ] BIR must distinguish:
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
- [ ] Add negative tests ensuring emitters refuse missing ownership or layout facts.

## 15. Debuggability And Visualization

- [ ] Dump these layers:
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
- [ ] Manifest must map final symbols back to:
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

## 16. Test Plan

- [ ] Scalar no-GC smoke:
  - arithmetic
  - function call
  - branch
  - match
- [ ] Layout spine smoke:
  - same CoreIR produces wasm-gc target layout
  - same CoreIR produces wasm32-nogc target layout
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

## First Proof Point

- [ ] Add `TargetNeutralLayoutPlan` and `TargetLayoutPlan` dumps for an existing
      scalar + record program.
- [ ] Move dyn row field/method ABI discovery out of backend emit and into neutral
      layout planning.
- [ ] Add a `wasm32-nogc` scalar-only target layout and emit path that rejects all
      managed values until their ownership and target layout facts exist.
- [ ] Add a diagnostic test proving emitters do not infer missing dyn adapter,
      ownership, or layout facts.

## Falsifier

This direction is wrong or incomplete if any of these become necessary:

- A backend emitter must inspect source-level row/method/type syntax to decide dyn
  adapter entries.
- `wasm32-nogc` needs different CoreIR semantics from `wasm-gc`.
- Native layout cannot consume the same neutral layout plan because the neutral
  plan already contains Wasm-specific or wasm32-specific representation details.
- `ContN`, dyn package, arena, and FBIP rules cannot be validated before target
  layout.
