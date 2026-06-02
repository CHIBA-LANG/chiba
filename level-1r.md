# Chiba Level-1R Reference Compiler Spec

## 0. 范围

Level-1R 是 level-1 的 Rust reference compiler 路线。

它不是新的 Chiba 语言层级，也不是要求用户在 Chiba source 中写 Rust 风格所有权类型。它的目标是先用 Rust 写出可测试、可观察、可迁移的 reference compiler，然后把通过测试的结构迁回 Chiba。

Level-1R 必须覆盖：

- nanopass pipeline
- chibalex
- chibacc
- regex subset / VM
- CIR
- single-shot continuation
- multi-shot continuation
- one-pass CPS
- baseline optimization
- usage color lowering visualization

## 0. Scope

Level-1R is the Rust reference compiler path for level-1.

It is not a new Chiba language level, and it does not require users to write Rust ownership types in Chiba source. Its purpose is to build a testable, observable, and portable reference compiler in Rust first, then port the tested structure back to Chiba.

Level-1R must cover the nanopass pipeline, chibalex, chibacc, regex subset / VM, CIR, single-shot continuations, multi-shot continuations, one-pass CPS, baseline optimization, and usage-color lowering visualization.

## 1. 为什么需要 Level-1R

level-1 自举需要同时稳定语言语义、编译器 pass、continuation lowering、checked template、method/operator resolution、regex/chibalex/chibacc 与 CIR。

直接在未自举的 Chiba compiler 中追这些语义，容易把临时 fallback、gate、scanner patch 与真实语义混在一起。

Level-1R 的职责是提供一个外部可验证的 reference：

- Rust 单元测试固定语义。
- Rust 类型系统暴露 move-only / shared ownership 压力。
- Rust lowering AST 显示 Chiba typed/lowered AST 应该携带的 usage color。
- Rust 实际使用的 `std` 能力反推 Chiba `std` 的首批需求。

## 1. Why Level-1R Exists

Level-1 bootstrap has to stabilize language semantics, compiler passes, continuation lowering, checked templates, method/operator resolution, regex/chibalex/chibacc, and CIR at the same time.

Doing all of that directly inside a not-yet-bootstrapped Chiba compiler can mix temporary fallbacks, gates, scanner patches, and real semantics.

Level-1R provides an external reference: Rust unit tests pin semantics, Rust ownership exposes move-only and shared-ownership pressure, Rust lowering AST shows the usage color Chiba typed/lowered AST must carry, and the Rust `std` surface actually used by the compiler becomes evidence for the first Chiba `std` requirements.

## 2. Rust `Rc` 与 Chiba Usage Color

Rust `Rc[T]` 不直接等价于 Chiba source-level `T`。

`Rc[T]` 是 Level-1R lowering AST 的可视化审计证据。Rust 中为了表达多路径持有、重复 resume、共享 callable storage 而需要 `Rc` 的地方，Chiba source 仍然可以写普通类型；但是 Chiba type checking / lowering 后必须显式显示 `N` usage color。

例如 Rust reference lowering 可以显示：

```text
fn xxx(x: Rc<YYY>) -> ZZZ
```

对应 Chiba source 可以是：

```chiba
def xxx(x: YYY): ZZZ
```

但 Chiba typed/lowered AST 必须能显示为：

```text
def xxx(x: N YYY): 1 ZZZ
```

这里的 `N YYY` 必须和 Chiba compiler 内部 usage-color 实现对上。若返回值不需要 sharing、replay 或 aliasing，则返回可显示为 `1 ZZZ`。

因此：

- Rust `Rc` 是审计信号，不是 Chiba source 语法。
- `Rc<T>` 对应 Chiba lowering 中的 `N T`。
- Rust move-only / affine value 对应 Chiba lowering 中的 `1 T`。
- 若 Rust 必须使用 `Rc<T>`，但 Chiba typed/lowered AST 没有对应 `N T`，则 usage color 信息丢失。

## 2. Rust `Rc` and Chiba Usage Color

Rust `Rc[T]` is not directly equivalent to source-level Chiba `T`.

`Rc[T]` is visible audit evidence in the Level-1R lowering AST. Where Rust needs `Rc` to express multi-path ownership, repeated resume, or shared callable storage, Chiba source may still use ordinary types; but after Chiba type checking and lowering, the typed/lowered AST must explicitly show `N` usage color.

For example, Rust reference lowering may show:

```text
fn xxx(x: Rc<YYY>) -> ZZZ
```

The corresponding Chiba source may be:

```chiba
def xxx(x: YYY): ZZZ
```

But Chiba typed/lowered AST must be able to show:

```text
def xxx(x: N YYY): 1 ZZZ
```

The `N YYY` annotation must match Chiba compiler's internal usage-color implementation. If the return value does not require sharing, replay, or aliasing, the return may be shown as `1 ZZZ`.

Therefore Rust `Rc` is an audit signal, not Chiba source syntax. `Rc<T>` corresponds to `N T` in Chiba lowering. Rust move-only / affine values correspond to `1 T` in Chiba lowering. If Rust must use `Rc<T>` but Chiba typed/lowered AST has no corresponding `N T`, usage-color information has been lost.

## 3. 可视化 Lowering AST

Level-1R 必须输出可检查的 lowering AST，而不只是最终可执行结果。

至少需要能观察：

- source item signature
- typed signature
- usage-colored typed signature
- closure capture set
- continuation kind: `Cont1` / `ContN`
- continuation frame ownership
- callable storage lowering
- CIR item boundary

usage color 显示应足够直接，避免隐藏在 debug-only 内部结构里。

推荐形态：

```text
source:
  def xxx(x) = ...

typed:
  def xxx(x: YYY): ZZZ = ...

usage:
  def xxx(x: N YYY): 1 ZZZ = ...

rust-reference:
  fn xxx(x: Rc<YYY>) -> ZZZ = ...
```

这四层必须能放在同一个报告里比较。

## 3. Visual Lowering AST

Level-1R must output inspectable lowering AST, not just executable results.

At minimum it must expose source item signatures, typed signatures, usage-colored typed signatures, closure capture sets, continuation kind, continuation frame ownership, callable storage lowering, and CIR item boundaries.

Usage color display should be direct enough to audit; it must not be hidden only inside debug-only internal structures.

These four layers must be comparable in the same report.

## 4. 与 Checked Template 的关系

Chiba 的 `[T]`、省略类型触发的自动泛化、row shorthand 与用户写出的鸭子形状，语义上都是 checked template。

Level-1R 不把它们实现成 Rust generics 的 trait solver。Rust reference compiler 应把它们实现为：

- definition-time body check
- obligation collection
- instantiation-time obligation discharge
- monomorphization
- visible specialization key

若 Rust 中为了复用实现使用泛型函数或 trait bound，必须在 lowering report 里说明它对应 Chiba 的 checked-template obligation，而不是引入新的 Chiba trait/interface 语义。

## 4. Relation to Checked Templates

Chiba `[T]`, auto-generalization from omitted types, row shorthand, and user-facing duck-shaped syntax are checked templates semantically.

Level-1R must not implement them as a Rust-generics trait solver. The Rust reference compiler should model them as definition-time body checking, obligation collection, instantiation-time obligation discharge, monomorphization, and visible specialization keys.

If Rust uses generic functions or trait bounds to reuse implementation code, the lowering report must state which Chiba checked-template obligation they represent. They must not introduce new Chiba trait/interface semantics.

## 5. Continuation 规则

Level-1R 必须把 delimiter 决定 continuation kind 的规则作为硬语义：

- `reset + shift` 捕获 `Cont1`，usage color 为 `1`。
- `resetn + shift` 捕获 `ContN`，usage color 为 `N`。
- `Cont1` 最多恢复一次。
- `ContN` 可重复恢复，但必须通过 replay-safety 检查。
- escaping `Cont1` 若进入 callable storage，lower 成 boxed one-shot consumed-state machine。
- `ContN` storage lower 成 multi-shot continuation package。

Rust reference lowering 中，`ContN` 或 escaping shared continuation 需要 `Rc` 时，Chiba lowering 必须显示对应 `N`。

## 5. Continuation Rules

Level-1R must treat delimiter-driven continuation kind as hard semantics:

- `reset + shift` captures `Cont1` with usage color `1`.
- `resetn + shift` captures `ContN` with usage color `N`.
- `Cont1` may be resumed at most once.
- `ContN` may be resumed repeatedly, but only after replay-safety checking.
- Escaping `Cont1` entering callable storage lowers to a boxed one-shot consumed-state machine.
- `ContN` storage lowers to a multi-shot continuation package.

When Rust reference lowering needs `Rc` for `ContN` or an escaping shared continuation, Chiba lowering must show the corresponding `N`.

## 5.1 One-Pass CPS / Beta Reduction

Level-1R 的 CPS pass 必须使用 one-pass CBV CPS，不允许先生成 naive CPS 再靠后续 pass 清理 administrative redex。

核心签名形态是：

```text
T(expr, k_meta)
k_meta: CpsAtom -> CpsTerm
```

这里 `k_meta` 是编译器内部 continuation。在 Rust reference 中它对应 `MetaKont = FnOnce(CpsAtom, &mut CpsCtx) -> CpsTerm`；在 Chiba level0 参考实现中对应 `level0/src/backend/cir/lower.chiba` 的 `lower_expr(expr, k: (Val) => CpsExpr, ...)`。它不是 object-level continuation，也不应该作为 runtime lambda/app 出现在 CPS Core 中。

必须保持以下规则：

- atom / variable 直接执行 `k_meta(atom)`，不产生 `LetCont`、`AppCont`、object lambda 或 runtime call。
- lambda 生成 object-level function value，函数体内部才通过 object-level continuation parameter 返回。
- call 先通过 meta-continuation 计算 callee，再计算 argument，最后只 materialize 真实 object-level call 和必要 object-level continuation。
- tuple / record / ADT ctor / operator / 多参数 call 必须按 CBV 左到右用嵌套 meta-continuation 串联。
- `if` / `match` 只在真实 branch join 需要时 materialize continuation；不能给 atom 分支制造 administrative join。
- `reset` / `resetn` / `shift` 是真实控制边界，必须 materialize CPS semantic node，并保留 answer type、continuation kind、usage/send/escape/replay facts。

反例：

```text
(\a. (\b. a b k) x) f
```

这种 object-level administrative beta-redex 不允许作为中间 CPS 结果出现。`f(x)` 应直接接近：

```text
f(x, cont w => k_meta(w))
```

其中 `cont w => ...` 只有在必须把 `k_meta` reify 成 runtime continuation 时才生成。

## 5.1 One-Pass CPS / Beta Reduction

Level-1R's CPS pass must use one-pass CBV CPS. It must not generate naive CPS first and rely on later passes to remove administrative redexes.

The core shape is `T(expr, k_meta)`, where `k_meta: CpsAtom -> CpsTerm` is a compiler-level continuation. In the Rust reference this is `MetaKont = FnOnce(CpsAtom, &mut CpsCtx) -> CpsTerm`; in the level0 reference it is `level0/src/backend/cir/lower.chiba`'s `lower_expr(expr, k: (Val) => CpsExpr, ...)`. It is not an object-level continuation and must not leak into CPS Core as runtime lambda/app.

Atoms execute `k_meta(atom)` directly. Calls evaluate callee and arguments left-to-right through nested meta-continuations, then materialize only the real object-level call and the necessary runtime continuation. Branches materialize joins only for real control joins. `reset` / `resetn` / `shift` are real control boundaries and must preserve answer type, continuation kind, usage, send, escape, and replay facts.

## 6. `std` 反推规则

Level-1R 的 Rust compiler 不应随意使用 Rust `std` 后再忽略。

每个实际使用的 Rust `std` 能力都应分类：

- Chiba `std` 首发必须提供。
- 只属于 Rust implementation convenience，不迁移到 Chiba。
- 需要 Chiba unsafe / metal boundary。
- 可由 compiler intrinsic 提供。

这个分类应跟随 lowering / test report 输出，作为迁回 Chiba 时的 `std` backlog。

## 6. Deriving Chiba `std`

The Rust compiler for Level-1R must not freely use Rust `std` and then ignore that dependency.

Each Rust `std` capability actually used by the compiler should be classified as one of: required for the first Chiba `std`, Rust implementation convenience only, requiring a Chiba unsafe / metal boundary, or implementable as a compiler intrinsic.

This classification should be emitted with lowering and test reports, becoming the `std` backlog for the Chiba port.

## 7. 验收标准

Level-1R 的验收不只是“Rust 程序能跑”。

必须同时满足：

- Rust unit tests 覆盖 nanopass、regex、chibalex、chibacc、CIR、CPS、Cont1、ContN。
- 每个关键 fixture 都输出 source / typed / usage / rust-reference lowering report。
- `Rc<T>` 与 `N T` 对齐可自动检查。
- move-only / affine Rust path 与 `1 T` 对齐可自动检查。
- checked-template obligation 不被 Rust trait solver 偷换。
- continuation kind 不由 callable storage 猜测，而由 delimiter 决定。
- 迁回 Chiba 时，Rust `std` 依赖已有分类。

## 7. Acceptance Criteria

Level-1R acceptance is not merely "the Rust program runs".

It must satisfy all of the following: Rust unit tests cover nanopass, regex, chibalex, chibacc, CIR, CPS, Cont1, and ContN; each key fixture emits source / typed / usage / rust-reference lowering reports; `Rc<T>` to `N T` alignment is automatically checkable; move-only / affine Rust paths to `1 T` alignment is automatically checkable; checked-template obligations are not replaced by Rust trait solver semantics; continuation kind is delimiter-driven rather than inferred from callable storage; and Rust `std` dependencies are already classified before porting back to Chiba.
