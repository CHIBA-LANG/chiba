# AST To TypedAst Elaboration Contract

This document specifies how the L2 typed pass elaborates parsed/alpha AST into typed facts.

The goal is to make type checking reproducible:

```text
AST + namespace summaries
-> TypedAst + ConstraintSet + ObligationIR + TypeSummary
```

Every generated type variable, constraint, row key, and obligation must have a stable origin for diagnostics and golden dumps.

## General Rules

For each expression:

1. Allocate a result type variable unless the type is syntactically known.
2. Recursively elaborate children.
3. Emit equality/row constraints for local type facts.
4. Emit obligations for deferred structural behavior.
5. Store the final result type fact on the TypedAst node.

For each binder:

1. Resolve explicit type annotation if present.
2. Allocate a fresh synthetic type variable if omitted.
3. Record binder origin and scope.
4. Add binder to the local type environment.

For each item:

1. Check header well-formedness.
2. Build user-visible generic binders from `[T]`.
3. Desugar row-bound shorthand into synthetic generic binders.
4. Elaborate the body under the item environment.
5. Solve local constraints.
6. Generalize remaining eligible variables at the item boundary.
7. Emit exported `TypeSummary`.

## `def`

Input:

```chiba
def id(x) = x
```

Elaboration:

```text
param x: $T0(origin=param x)
body x: $T0
return: $T0
scheme: forall $T0. fn($T0) -> $T0
constraints: []
obligations: []
```

Input:

```chiba
def add(a, b) = a + b
```

Elaboration when operands are abstract:

```text
a: $T0
b: $T1
result: $T2
obligation: OperatorObligation("+", [$T0, $T1], $T2, origin=a+b)
scheme: forall $T0 $T1 $T2. fn($T0, $T1) -> $T2 where obligation
```

If both operands are concrete numeric types, the operator is solved immediately instead of producing an obligation.

Input:

```chiba
def one(): i64 = 1
```

Elaboration:

```text
return annotation: i64
body: i64
constraint: body == i64
scheme: fn() -> i64
```

Input:

```chiba
def bad(): i64 = true
```

Elaboration fails during definition-time checking:

```text
constraint: bool == i64
diagnostic: return type mismatch
```

## Explicit Generic Header

Input:

```chiba
def id[T](x: T): T = x
```

Elaboration:

```text
generic binder T: TyVar(origin=user generic T, visibility=user)
param x: T
body x: T
return: T
scheme: forall T. fn(T) -> T
```

Input:

```chiba
def left[T](x: T, y) = x
```

Elaboration:

```text
generic binder T: user-visible
param x: T
param y: $T0(origin=param y, visibility=synthetic)
body x: T
scheme: forall T $T0. fn(T, $T0) -> T
```

The checker must not reject `y` just because the item has an explicit generic header.

Explicit generic binders are rigid while checking the body. They are not flexible inference variables.

Input:

```chiba
def bad[T, F](x: T): F = x
```

Elaboration fails unless constraints elsewhere prove `T == F`:

```text
T: rigid user generic
F: rigid user generic
body x: T
return annotation: F
constraint T == F cannot be solved by assigning F := T
diagnostic: return type mismatch
```

Input:

```chiba
def map_one[T, F](x: T, f: fn(T): F): F = f(x)
```

Elaboration succeeds:

```text
f: fn(T) -> F
f(x): F
return: F
```

## Row-Bound Shorthand

Input:

```chiba
def get_name(x: {r | name: Str}) = x.name
```

Desugars before body checking to:

```text
synthetic generic $T0 bound {r | name: Str}
param x: $T0
```

Body elaboration:

```text
x.name result: Str
field obligation: x has field name: Str
scheme: forall $T0: {r | name: Str}. fn($T0) -> Str
```

Input:

```chiba
def same(a: {r | name: Str}, b: {r | name: Str}) = a.name
```

Elaboration:

```text
a: $T0 bound {r | name: Str}
b: $T1 bound {r | name: Str}
```

`$T0` and `$T1` are different unless the source explicitly names and reuses a type variable.

## `let`

Input:

```chiba
let x = 1
```

Elaboration:

```text
value: i64
binder x: i64
```

Input:

```chiba
let id = fn(x) = x
```

Elaboration:

```text
lambda param x: $T0
lambda result: $T0
let scheme: forall $T0. fn($T0) -> $T0
```

This generalization is legal only if the lambda passes the value restriction.

Input:

```chiba
let r = ref(0)
```

Elaboration:

```text
r: Ref[i64]
generalization: forbidden by value restriction
```

## Block

Input:

```chiba
{
    let x = 1
    x + 1
}
```

Elaboration:

```text
stmt let x: i64
tail: i64
block result: i64
```

If a block has no tail expression, its result is `Unit`.

Early `return` statements contribute constraints to the enclosing function return type. A block with incompatible tail and return facts fails definition-time checking.

## `if`

Input:

```chiba
if cond { a } else { b }
```

Elaboration:

```text
cond == bool
then_ty == else_ty
result == then_ty
```

If either branch contains answer/control constructs, L2 records the local type facts and leaves detailed continuation legality to the answer/control pass. It must not silently coerce branch answer types.

## `match`

L2 checks:

- scrutinee type
- pattern binder types
- arm result types unify
- constructor/pattern names resolve

Pattern refutability, exhaustiveness, and pattern matrix construction belong to the pattern pass and later diagnostics. L2 consumes those facts when available.

## Field Access

Input:

```chiba
x.name
```

If `x` has known nominal type:

```text
lookup nominal row fields
result: field type
error if field missing
```

If `x` is abstract:

```text
x: $T0
field result: $T1
obligation: FieldObligation($T0, name, $T1)
row fact: $T0 has {r | name: $T1}
```

For a row-bound generic:

```chiba
def get_id[T: {r | id: i64}](x: T) = x.id
```

Elaboration:

```text
T bound includes id: i64
x.id result: i64
obligation: already satisfied by generic bound, still dumpable as checked field use
```

## Method Call

Input:

```chiba
receiver.method(args...)
```

Elaboration tries or records the three paths:

1. field-callable: `receiver.method` is a field whose type unifies with `fn(args...) -> result`
2. nominal receiver method: method index lookup by receiver nominal id
3. qualified callee: if syntax resolved to an explicit function path

If the receiver is abstract, emit:

```text
MethodObligation(receiver_ty, method_name, arg_tys, result_ty, origin)
```

If multiple concrete paths are valid at the same priority, report ambiguity. The chosen path must be stored in TypedAst.

A row field fact does not prove a nominal method. If `receiver` has abstract type `$T0` and L2 knows only `{r | method: ty}`, then `receiver.method` is a field access. For `receiver.method(args...)`, it can become a field-callable call only if `ty` unifies with `fn(args...) -> result`. A receiver method `def X.method(self, ...)` is selected only when the receiver has concrete nominal id `X`, or when a `MethodObligation($T0, "method", ...)` is later discharged at an instantiation whose concrete receiver is `X`.

## Method-Style `def` And `Self`

Input:

```chiba
def X.y(self, arg: A): R = body
```

Elaboration:

```text
owner nominal: X
method binder Self := X
self: Self
arg: A
return: R
method index key: (nominal_id(X), "y")
call target type: fn(Self, A) -> R
```

Input:

```chiba
type Box[T] { value: T }
def Box[T].get(self): T = self.value
```

Elaboration:

```text
owner generic binder T
Self := Box[T]
self: Box[T]
self.value: T
method index key: (nominal_id(Box), "get")
```

`Self` is available only inside the method header/body receiver scope. It is not a global type name and not a structural row alias.

## Operator

Input:

```chiba
a + b
```

Concrete builtin cases:

```text
i64 + i64 -> i64
bool + bool -> error
```

Abstract cases:

```text
OperatorObligation("+", [a_ty, b_ty], result_ty, origin)
```

The checker must not default abstract `+` to `i64` unless a concrete use forces that type.

## Record Literal

Input:

```chiba
{ name: "a", id: 1 }
```

Elaboration:

```text
fields:
  name: str/String alias normalized as needed
  id: i64
row: closed canonical row sorted by field id
result: TyRow(row_id)
```

Duplicate fields fail before row key construction.

## Record Update

Input:

```chiba
{ base | name: "b" }
```

Elaboration:

```text
base must be record-like or row-like
updated fields checked normally
result row = base row with updated field constraints
```

If base is abstract, emit row constraints rather than accepting arbitrary structural subtyping.

## Tuple

Input:

```chiba
(a, b)
```

Elaboration:

```text
row fields:
  _1: type(a)
  _2: type(b)
result: TyTuple([type(a), type(b)]) backed by anonymous positional row
```

One-tuples must stay distinguishable from grouped expressions.

## Data Constructor / Union Payload

L2 checks:

- constructor resolves to a data variant
- payload expression matches variant payload type
- tag identity is stable
- payload row/union shape is canonical

The backend layout is not chosen here. L2 only emits typed constructor facts.

## Assignment

Input:

```chiba
lhs := rhs
```

Elaboration:

```text
lhs == Ref[$T0]
rhs == $T0
result: Unit
```

For field assignment through a ref, L2 records whole-value replacement semantics:

```text
a.b := c
requires a: Ref[row with b: T]
requires c: T
desugaring fact: a := { a.* | b: c }
```

## Index

String and slice indexing use byte semantics:

```text
String == Array[u8]
str == Slice[u8]
s[i] -> u8
```

Character access is `.char_at(n)`, which is a method/helper obligation and not the same as indexing.

## Extern

Input:

```chiba
def f(a: i64): i64 = extern "C" "f"
```

Elaboration:

```text
params and return must be explicit ABI-compatible types
abi: env after canonicalizing C/c/env as configured
symbol: f
typed import ref emitted into TypeSummary
```

Missing parameter or return annotations are errors for extern definitions.

## Unsafe / Capability

`unsafe` creates a lexical capability context for L2 legality checks.

Inside unsafe:

- `Ptr[T]` use is legal
- `UnsafeRef[T]` use is legal

Outside unsafe:

- constructing, casting to, or directly operating on `Ptr[T]` or `UnsafeRef[T]` is illegal
- annotating ordinary non-Metal values, parameters, or returns with `Ptr[T]` or `UnsafeRef[T]` is illegal

Non-Metal source without an explicit unsafe boundary cannot touch `Ptr[T]` or `UnsafeRef[T]`, including by type annotation. Metal modules may define typed pointer boundary APIs, but they still do not get to use raw `i64` pointer APIs as a replacement for typed `Ptr[T]`.

## TypedAst Required Fields

Each typed expression node should carry:

- original node kind
- source span or stable origin id
- result type
- child typed ids
- constraints emitted by this node
- obligations emitted by this node
- resolved symbol/method/operator path when known

Each typed item should carry:

- item symbol id
- generic binders
- synthetic binders
- parameter types
- return type
- generalized scheme
- exported summary facts

## Unsoundness Audit

Potential unsound edges in elaboration:

- **Using "parameter referenced" as inference success**: this accepts underconstrained code without a real type scheme. Required fix: allocate type vars and generalize or diagnose after solving.
- **Rejecting unannotated params in generic functions**: this contradicts automatic generalization and forces accidental annotations. Required fix: explicit and implicit binders must coexist.
- **Defaulting abstract operators to numeric types**: this loses generic operator behavior. Required fix: abstract operands create operator obligations.
- **Treating row shorthand as closed record**: this rejects valid nominal values with extra fields. Required fix: shorthand creates an open-row bound.
- **Sharing synthetic row shorthand variables across parameters**: this wrongly forces two arguments to be the same type. Required fix: each shorthand gets a fresh synthetic binder unless source names one.
- **Let-generalizing unsafe/capability values**: this can break mutation and world-boundary safety. Required fix: value restriction.
- **Treating unsafe types as harmless annotations**: a non-Metal signature containing `Ptr[T]` or `UnsafeRef[T]` can leak unsafe capability without an unsafe context. Required fix: type annotations are checked as unsafe touches too.
- **Choosing method fallback silently**: this changes behavior when a field and nominal method have the same name. Required fix: priority and ambiguity diagnostics are part of TypedAst.
