# L2 Type System Design

This document fixes the design target for Pre-C03. It is an implementation contract for the level-1b L2 typed pass, not a user-facing tutorial.

Level-1 type checking is:

```text
HM inference
+ row constraints
+ checked structural templates
+ nominal identity
+ method/operator/shape obligations
+ capability and ABI legality
```

It is not a Rust-style trait solver, not a global witness search, and not an old C++ template model where the body is unchecked until instantiation.

## Pipeline Boundary

The L2 pass consumes alpha-resolved AST/CIR and produces:

- `TypedAst`: every expression, pattern, binder, item header, and call site has a type fact.
- `ConstraintSet`: equality, row, field, capability, and ABI constraints collected during checking.
- `ObligationIR`: deferred structural obligations that need concrete shape or method/operator lookup.
- `TypeSummary`: stable exported type headers for namespace/project checking.

The L2 pass must not lower to wasm and must not choose backend layouts. It may create canonical row and nominal ids because later passes need those ids for method lookup, specialization, and wasm-gc layout.

## Type Universe

The implementation should represent type data as explicit ADTs, not opaque `i64` handles.

Minimum type forms:

```text
TyVar(id)
TyConst(name)
TyApp(callee, args)
TyFn(params, result)
TyTuple(elements)
TyRow(row_id)
TyNominal(nominal_id, type_args)
TyData(data_id, type_args)
TyUnion(union_id, type_args)
TyRef(inner)
TyUnsafeRef(inner)
TyPtr(inner)
TyAtomic(inner)
TyContinuation(input, answer, kind)
TyNever
TyUnit
```

`String` is `Array[u8]`; `str` is `Slice[u8]`. The type checker may preserve the surface aliases for diagnostics, but canonical type comparison must use the normalized form.

## Type Variables

`[T]` is a type-variable binder.

It is user-visible, scoped to the item body, and may carry a bound:

```chiba
def f[T: {r | name: Str}](value: T): Str = value.name
```

The checker must record at least:

- `TyVarId`
- `origin`: user generic, implicit parameter, let-generalized variable, row tail, field result, compiler internal
- `level`: for let-generalization
- `kind`: value, row, capability, continuation, ABI scalar
- `visibility`: user-visible or synthetic
- `bounds`: row/named/capability obligations attached to this variable

Implicit variables are synthetic binders. They are still real type variables; they are not parser hacks and not erased diagnostics.

There are two important variable modes:

- **inference variables** are flexible until solved by HM unification.
- **explicit generic binders** such as `[T]` are rigid inside the definition body.

So `[T]` participates in HM checking as a type variable, but it is not a flexible hole that the body may silently solve to a concrete type. This is the core of the checked-template model: definition-time HM checks the body under rigid abstract binders, and instantiation-time later substitutes concrete types and discharges obligations.

Omitted annotations use flexible inference variables first. If they remain free at a generalization boundary and are legal to generalize, they become synthetic generic binders. If they remain free in a position that must be concrete, the checker reports that an annotation is required.

Level-2 may add explicit flexible generic binders:

```chiba
def f[?T](value: ?T) = ...
```

`?T` means the source intentionally asks for a flexible generic variable during definition-time inference. This is not part of the level-1 core contract; level-1 only needs omitted annotations as flexible inference variables and `[T]` as rigid explicit generic binders.

Examples:

```chiba
def id(x) = x
```

Elaborates after checking to something equivalent to:

```text
forall $T0. ($T0) => $T0
```

```chiba
def id[T](x: T): T = x
```

Elaborates to:

```text
forall T. (T) => T
```

Both are polymorphic, but only the second exposes the name `T` to source-level diagnostics and future explicit instantiation syntax.

Example:

```chiba
def f[T, F](value: T): F = value
```

This is ill-typed unless another constraint proves `T == F`, because `F` is a rigid explicit generic binder. The return annotation `F` does not allow the body to manufacture an `F`.

This is valid:

```chiba
def f[T](value: T): T = value
```

This is also valid:

```chiba
def f[T, F](value: T, convert: (T) => F): F = convert(value)
```

`TyNever` may flow into any return type for diverging expressions such as `panic()` or `todo()`, but that is a `Never` rule, not a generic escape hatch.

Underconstrained constructors must not silently become globally polymorphic mutable values. For example, `Ref.new(None)` has shape `Ref[Option[T]]`; without an annotation or contextual use that fixes `T`, the checker must report an annotation-required error rather than generalizing the `Ref`.

## ConstraintSet

Constraints that belong to HM/unification:

- equality: `A == B`
- function application shape: `callee == (args...) => result`
- tuple element equality
- type application arity and kind checks
- concrete numeric/bool/string equality

Constraints that belong to row unification:

- row variable equals row
- open row contains field
- field type consistency
- closed row forbids extra fields

Constraints that are not plain HM equality:

- method lookup
- operator overload lookup on abstract operands
- shape dispatch
- dyn adapter packaging
- continuation resume legality
- ABI import/export legality

Those must become `ObligationIR` or dedicated capability/ABI checks. Collapsing them into equality would accidentally create structural subtyping or a hidden trait solver.

## Row Model

Rows are canonical shape facts.

Canonical row key:

```text
RowKey = (openness, stable_tail_id_or_none, sorted[(field_id, field_type_key)])
```

Rules:

- Source field order must not affect the row key.
- Field ids are stable across deterministic project scans.
- Tuple fields are generated as `_1`, `_2`, ...
- A nominal type may expose a row shape, but the nominal id remains separate.
- A row bound is a shape obligation, not nominal erasure.

Example:

```chiba
type User { name: Str, id: i64 }
type Device { id: i64, name: Str }
```

`User` and `Device` may share a normalized row shape key, but they must not share a nominal id. Method resolution defaults to nominal id, not row shape.

## Automatic Function Generalization

For ordinary non-`extern` functions:

```chiba
def f(a, b, c) = expr
```

Checking order:

1. Create fresh synthetic type variables for unannotated parameters.
2. Create a fresh result variable when return type is omitted.
3. Check the body under those variables.
4. Concrete uses unify immediately.
5. Field/method/operator/shape uses attach obligations.
6. Generalize free variables at the function boundary.

This is definition-time checking. The body is checked once under abstract variables and obligations. Only concrete obligation discharge is deferred.

Explicit generic binders and implicit variables may coexist:

```chiba
def pair_left[T](left: T, right) = left
```

`T` is user-visible. `right` receives a synthetic variable. The checker must not reject this merely because the function is generic.

## Let-Generalization

`let` can generalize only values that pass the value restriction.

Freely generalizable:

- immutable literals
- pure record/tuple values built from generalizable values
- functions/lambdas without unsafe or continuation escape
- pure aliases to already-generalized immutable values

Not freely generalizable:

- `Ref[T]`
- `UnsafeRef[T]`
- `Ptr[T]`
- `Atomic[T]`
- extern/FFI values
- unsafe-block results
- continuation-bearing values
- values that capture world-local state

This restriction prevents polymorphic mutable references and polymorphic control capabilities.

## Checked Template Split

Definition-time checks:

- parse/alpha/name-resolve well-formedness
- explicit and implicit generic binder validity
- ordinary HM unification
- row bound well-formedness
- field obligation creation
- method/operator obligation creation
- return type consistency
- capability syntax legality
- basic answer/control entry facts needed by later continuation pass

Instantiation-time checks:

- concrete row field existence and field type consistency
- concrete method candidate resolution
- concrete operator resolution
- shape-dispatch final candidate selection
- dyn adapter packaging
- builtin capability facts that depend on concrete type
- continuation capability facts that depend on concrete use

Definition-time must reject errors that do not depend on concrete type.

```chiba
def bad[T](x: T) = 1 + true
```

This fails during definition-time checking.

```chiba
def get_name(x) = x.name
```

This succeeds at definition time by producing a field obligation. A later instantiation fails if the concrete argument has no `name` field.

## Method And Operator Boundary

Method resolution is not row unification.

The L2 pass should create a method index:

```text
(namespace_scope_or_behavior_source, nominal_id, method_name) maps to method candidates
```

Method-call resolution uses three paths:

1. Field-callable: `receiver.method` is a field value that can be called.
2. Nominal receiver method: `def Type.method(self, ...)`.
3. Qualified callee: an explicitly named function path.

The chosen path must be recorded in TypedAst. If the receiver is abstract and cannot be decided yet, L2 records a `MethodObligation`.

`MethodObligation` and `OperatorObligation` must reserve a behavior-source field even if level-1 only implements the default visible world:

```text
DefaultVisible
ExplicitVia(namespace_id)
QualifiedPath(symbol_id)
```

Level-2 `via` uses `ExplicitVia`. This field must enter specialization keys whenever it affects implementation choice.

A row fact such as `{r | y: ty}` proves field availability only. It must not be used as proof that nominal method `def X.y(...)` exists. For `x.y`, the row fact is enough to type a field access. For `x.y(...)`, field-callable resolution may use the row fact if `y` has a function type; receiver-method resolution still requires a concrete nominal receiver id. A row-shaped value reaches nominal method resolution only through an explicit cast/checked conversion that produces a concrete nominal type.

Method-style definitions bind `self` through the owner nominal type:

```chiba
def X.y(self, arg: A): R = ...
```

Inside the method body:

```text
Self := X
self : Self
method index key: (nominal_id(X), "y")
callable shape: (Self, A) => R
```

For a generic owner:

```chiba
type Box[T] { value: T }
def Box[T].get(self): T = self.value
```

the method scope binds `Self := Box[T]` and `self : Box[T]`. `Self` is a special receiver-scope alias, not a top-level generic name and not a row type.

Operator checking:

- concrete builtin numeric/bool operators unify immediately
- abstract operands produce structural operator obligations
- invalid concrete combinations fail at definition time

For homogeneous `+` on abstract operands, the default obligation is equivalent to:

```chiba
def add[T: {t | op_add: (Self, Self) => Self}](a: T, b: T): T =
    a.op_add(b)
```

This is a contract shape, not ordinary row-to-method resolution. `op_add` names the operator protocol entry. `Self` means the current `T` in that operator contract. At instantiation, the compiler discharges the operator obligation through the concrete operator implementation selected for the concrete nominal type and behavior source. The row fact alone never selects `def X.op_add`; a concrete nominal receiver, explicit cast, or explicit behavior source must provide the implementation.

## Capability And ABI Boundary

Capability types are ordinary type forms with extra legality rules:

- `Ref[T]` is the safe mutation entry.
- `UnsafeRef[T]` and `Ptr[T]` require explicit `unsafe` context.
- `Atomic[T]` is limited to the supported scalar/pointer-like family.
- top-level `Ref[T]` requires `#[world_local]`.
- non-Metal source without an explicit unsafe boundary cannot mention `UnsafeRef[T]` or `Ptr[T]`, including in ordinary parameter or return annotations.

Assignment:

```text
lhs := rhs
requires lhs : Ref[T], rhs : T
```

Element assignment is legal only when the indexed expression itself has type `Ref[T]`; `Ref[Array[T]]` does not make array elements mutable.

Extern declarations require explicit ABI types:

```chiba
def fd_write(fd: i64, iov: i64, iovcnt: i64, nwritten: i64): i64 =
    extern "wasi" "fd_write"
```

The backend consumes typed import refs. It must not infer ABI signatures during WAT/Core emission.

## Required Dumps

L2 must be able to dump:

- user and synthetic type variables
- substitutions
- final item type schemes
- canonical row keys
- nominal ids
- constraints before/after solve
- obligations
- selected method/operator path when known

The dump order must be deterministic. This is part of the type checker contract, not debug-only behavior.

## Unsoundness Audit

Known unsoundness risks and required guardrails:

- **Polymorphic refs**: unrestricted let-generalization over `Ref[T]` allows writing one type and reading another. Guardrail: value restriction.
- **Nominal erasure by row**: treating `{r | ...}` as the real type erases method identity and layout identity. Guardrail: row shape and nominal id are separate facts.
- **Hidden structural subtyping**: accepting arbitrary row subsumption at any position creates unstable method/dispatch behavior. Guardrail: row unification only at explicit row/generic boundaries.
- **Unchecked templates**: deferring the whole generic body to instantiation hides definition errors. Guardrail: definition-time HM and well-formedness are mandatory.
- **Method as field fallback ambiguity**: silently choosing field-callable or receiver method can change behavior. Guardrail: fixed candidate order and ambiguity diagnostics.
- **Operator inference too eager**: defaulting unconstrained `+` to `i64` can reject valid generic code or hide operator obligations. Guardrail: concrete operands solve immediately; abstract operands produce obligations.
- **Operator protocol treated as ordinary row method**: lowering `a + b` to `a.op_add(b)` too early would let row facts invoke nominal methods. Guardrail: `op_add` is an operator obligation contract; concrete implementation selection happens only after nominal/cast/behavior-source resolution.
- **Synthetic generic leakage**: compiler-generated `$T0` names must not become stable user syntax. Guardrail: expose only in diagnostics/dumps, not in source namespaces.
- **Underconstrained `Option` inside `Ref`**: `Ref.new(None)` creates `Ref[Option[T]]`; generalizing it would create polymorphic mutable state. Guardrail: value restriction plus annotation-required diagnostics for unsolved variables under mutable/capability constructors.
- **ABI inference**: inferring extern signatures from call sites can produce wrong imports. Guardrail: extern requires explicit ABI types.
- **Continuation generalization**: freely generalizing captured continuations can cross answer/world boundaries. Guardrail: continuation-bearing values are not freely generalized; later answer/control pass owns the detailed legality check.
- **Unsafe capability leakage through annotations**: allowing ordinary signatures to mention `Ptr[T]` or `UnsafeRef[T]` without unsafe context makes unsafe values callable from safe code. Guardrail: annotations are checked as capability uses, and Metal still uses typed `Ptr[T]` rather than raw pointer-shaped `i64`.
- **Behavior-source missing from specialization key**: `via ns1` and `via ns2` may select different implementations for the same nominal type and method/operator name. Guardrail: method/operator obligations carry behavior source and include it in specialization keys when used.
