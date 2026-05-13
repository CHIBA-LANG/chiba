# Level-1b Type System / Pre-C03 TODO

本文把 `TODO.md` 里的 **Pre-C03: L2 type/method/row semantic implementation** 拆成可审计、可测试、可逐步打勾的类型系统工作看板。

目标不是再堆 source-level gate，而是写出 level-1b 自己会长期使用的 L2 typed semantic pass：

- HM 基础推断
- row polymorphism
- checked structural template
- nominal row/data/union identity
- method / operator / shape obligation
- `Ref[T]` / `UnsafeRef[T]` / `Ptr[T]` / `Atomic[T]` capability
- extern ABI typing
- 可 dump、可 hash、可缓存的 `TypedAst + ConstraintSet + ObligationIR`

## 0. 设计边界

- [x] **Type system design note**
	- **TODO**: 写 `supports/type-system/l2-type-system.md`，固定 L2 类型系统的核心术语与边界。
	- **DESC**: 明确 level-1 是 `HM + row + checked structural template + nominal identity + method/operator obligation`，不是 Rust trait solver，也不是旧 C++ 模板。
	- **DONE**: `supports/type-system/l2-type-system.md` 已固定 L2 pipeline boundary、type universe、`[T]` binder 语义、ConstraintSet/ObligationIR 分层、row canonical model、automatic generalization、let value restriction、checked template split、method/operator/capability/ABI 边界，并附 unsoundness audit。
	- **验收**: 文档回答：什么进入 HM unify，什么进入 row obligation，什么留到 instantiation，什么属于 method/operator dispatch。

- [x] **AST -> TypedAst elaboration contract**
	- **TODO**: 写 `supports/type-system/ast-to-typed.md`，规定每类 AST 节点如何产生 type variable、constraint、obligation、typed node。
	- **DESC**: 明确 `def`、`let`、lambda、block、if、match、field access、method call、operator、record/tuple/data constructor、extern 的 elaboration。
	- **DONE**: `supports/type-system/ast-to-typed.md` 已规定 def/generic/row shorthand/let/block/if/match/field/method/operator/record/update/tuple/data/assignment/index/extern/unsafe 的 elaboration 输出，并附 elaboration unsoundness audit。
	- **验收**: 每个 AST family 至少有一个输入例子和期望的 TypedAst/ConstraintSet 摘要。

- [x] **Definition-time vs instantiation-time split**
	- **TODO**: 固定 checked template 的分界线。
	- **DESC**: 定义期必须完成普通 HM、row constraint generation、基本 well-formedness、answer/control 入口检查；实例化期只兑现 concrete shape / method / operator / dispatch obligation。
	- **DONE**: `supports/type-system/l2-type-system.md` 的 `Checked Template Split` 明确了定义期与实例化期职责，并用 `def bad[T](x: T) = 1 + true` 与 `def get_name(x)=x.name` 区分定义期错误和实例化期 obligation 兑现。
	- **验收**: 能解释为什么 `def bad[T](x: T) = 1 + true` 在定义期报错，而 `def f(x)=x.name` 的 concrete field 存在性可以在实例化点兑现。

## 1. L2 类型 IR 与数据结构

- [x] **Type ADT**
	- **TODO**: 在 `src/backend/cir/` 增加或重写 L2 type ADT：`TyVar`、`TyConst`、`TyApp`、`TyFn`、`TyTuple`、`TyRow`、`TyNominal`、`TyRef`、`TyUnsafeRef`、`TyPtr`、`TyAtomic`、`TyContinuation`。
	- **DESC**: 不用 `i64` 当 opaque type handle；内部 id 必须有明确 newtype/ADT 表示。
	- **DONE**: `src/backend/cir/ir.chiba` 已增加 `CirTyVarId`、`CirRowId`、namespace-qualified `CirNominalId`、完整 `CirType` ADT、capability type、continuation type；`src/backend/cir/show.chiba` 已增加稳定 type dump。
	- **验收**: dump 能显示稳定类型结构；测试覆盖函数、tuple、row、nominal、capability、continuation 类型。

- [x] **Type variable model**
	- **TODO**: 设计 `TyVarId`、kind、level、scope、origin、synthetic/user-visible 标记。
	- **DESC**: 支撑 let-generalization、隐式 generic 参数、显式 `[T]`、row tail variable、diagnostic。
	- **DONE**: `CirTyVarMeta` 固定 `id/kind/level/scope/origin/visibility/rigidity`，其中 `[T]` 可表示为 user-visible rigid，省略参数可表示为 synthetic flexible。
	- **验收**: dump 能区分用户写的 `T` 和 compiler synthetic 的 `$T0`；错误消息能回指参数、let、field access 等 origin。

- [x] **ConstraintSet**
	- **TODO**: 定义普通 equality constraint、row constraint、field presence constraint、field type constraint、capability constraint、ABI constraint。
	- **DESC**: 普通类型不匹配进入 unify；shape/method/operator 不应被强行塞进普通 equality。
	- **DONE**: `CirConstraint` / `CirConstraintList` 已覆盖 eq、row-eq、field-present、field-type、capability、ABI，并有稳定 dump。
	- **验收**: `ConstraintSet` 可 dump、可排序、可 hash；同一源码两次运行输出稳定。

- [x] **ObligationIR**
	- **TODO**: 定义 `FieldObligation`、`MethodObligation`、`OperatorObligation`、`ShapeDispatchObligation`、`DynAdapterObligation`、`ContinuationCapabilityObligation`。
	- **DESC**: 这是 checked structural template 的核心记录，后续 specialization 和 method index 使用它，不再回读 AST 猜语义。
	- **DONE**: `CirObligation` / `CirObligationList` 已覆盖 field、method、operator、shape dispatch、dyn adapter、continuation capability；method/operator obligation 均携带 behavior source。
	- **验收**: `def get_name(x)=x.name` 产生 field obligation；`def len(x)=x.len()` 产生 method obligation；`def add(a,b)=a+b` 产生 operator obligation 或 concrete numeric unify。

## 2. `[T]` 与自动泛化

- [x] **Explicit generic parameter semantics**
	- **TODO**: 明确 `[T]` 在 L2 中是 user-visible type variable binder，不是 namespace constraint witness，也不是 monomorphized body 的立即复制。
	- **DESC**: `[T: Bound]` 绑定一个 rigid 抽象类型变量，Bound 可以包含 named constraint 与最多一个 row constraint。省略类型标注走 flexible inference variable；level-2 可加入显式 `[?T]` flexible generic binder。
	- **DONE**: `CirTyVarMeta.rigidity` 已区分 explicit `[T]` 的 user-visible rigid binder 与省略类型产生的 synthetic flexible var；semantic gate 不再把 explicit generic 当成“所有签名必须标注”的开关。
	- **验收**: `def id[T](x: T): T = x` 的 TypedAst 保留 `T`；重复 `T` 报错；bound 良构性报错稳定。

- [x] **Implicit function parameter generalization**
	- **TODO**: 实现 `def f(a,b,c)=expr` 的 fresh type var 分配、使用点约束收集、函数边界自动泛化。
	- **DESC**: 参数没标注不是错误；如果使用点需要 concrete type 就 unify，如果需要 row/method/operator shape 就挂 obligation。
	- **DONE**: source semantic gate 已移除“未引用参数/泛型参数必须标注”的旧错误路径；`supports/semantic-gates/type_inference.chiba` 覆盖未标注参数、未使用参数、identity 与 binary 使用。
	- **验收**: `def id(x)=x` 推出 `[T](x:T):T`；`def add(a,b)=a+b` 推出 numeric/operator 约束；未使用参数产生稳定 generic 或按语言规则诊断。

- [x] **Explicit + implicit generic merge**
	- **TODO**: 支持显式 `[T]` 与省略参数类型共存。
	- **DESC**: `def f[T](x, y: T)=x` 中 `T` 是用户变量，`x` 可生成 synthetic var；不能继续要求 generic 函数所有参数/返回都标注。
	- **DONE**: `supports/semantic-gates/type_inference.chiba` 增加 `explicit_and_implicit[T](left, right: T) = left` 与 `generic_return_from_param[T](value) = value`，`vp run semantic:gates` 通过。
	- **验收**: 删除当前 `generic parameter type requires annotation` / `generic return type requires annotation` 这类错误路径，改为真实 inference 失败才报错。

- [x] **Return type inference**
	- **TODO**: 省略返回类型时，从 tail expression / return statements / block 合并结果推断。
	- **DESC**: block、if、match、early return 需要统一规则；不确定时给出要求标注的诊断。
	- **DONE**: 省略返回类型不再因 explicit generic 被拒绝；保留 concrete return mismatch gate；`Ref.new(None)` 无上下文会要求显式 `Option[T]` 注解。
	- **验收**: `def one()=1`、`def pick(b)=if b {1}else{2}` 推断成功；分支类型不一致报错。

- [x] **Let-generalization**
	- **TODO**: 实现 HM let-generalization，带 value restriction。
	- **DESC**: immutable pure value 可泛化；`Ref`、`UnsafeRef`、`Ptr`、`Atomic`、extern/unsafe/capability-bearing value、continuation-bearing value 保守不自由泛化。
	- **DONE**: `CirTypeScheme` / `CirTyVarMetaList` 已进入 IR；`src/backend/cir/type_generalize.chiba` 已实现 candidate var generalization 与 capability/continuation value restriction；`level1c type-generalize-smoke` 验证 pure identity scheme 会泛化，`Ref[$T]` 不泛化。
	- **验收**: polymorphic identity lambda 可多处实例化；`let r = ref(...)` 不被泛化；错误指向 let binder。

## 3. Unification 设计

- [x] **Unifier core**
	- **TODO**: 实现 substitution、occurs check、union/find 或等价结构、type level 管理。
	- **DESC**: 需要支持一阶 HM，不做 higher-kinded type，不做全局 trait solver。
	- **DONE**: `src/backend/cir/type_unify.chiba` 已实现 substitution apply/lookup、occurs check、var binding、一阶递归 unify；覆盖 function、tuple、type app、nominal namespace identity、Ref/Ptr/Atomic/continuation 等结构。`level1c type-unify-smoke` 已接入 bootstrap smoke。
	- **验收**: 单测覆盖 var-var、var-concrete、fn-fn、tuple、type app、occurs check、错误路径。

- [x] **Kind / type family checks**
	- **TODO**: 区分 value type、row type、capability type、continuation type、ABI scalar type。
	- **DESC**: 防止把 row tail 当普通 value、把 `Ptr[T]` 当安全 value、把 ABI-only type 泄漏到 managed value。
	- **DONE**: `src/backend/cir/type_kind.chiba` 已实现独立 kind pass，区分 value、row、capability、continuation、ABI scalar；`level1c type-kind-smoke` 覆盖正确分类、row-as-value mismatch、String-as-ABI error。
	- **验收**: 错误 fixture 覆盖 kind mismatch、错误 type app arity、非法 row tail 使用。

- [x] **Row unification**
	- **TODO**: 实现 open row / closed row unify、field presence、field type consistency、extra field rejection。
	- **DESC**: row canonical key 必须与源码字段顺序无关；tuple 使用 `_1`, `_2`, ... positional row 字段。
	- **DONE**: `src/backend/cir/type_row.chiba` 已实现 `CirRowMeta` open/closed row field lookup、order-independent field set check、field type unify、closed row extra field rejection、open row extra field acceptance；`level1c type-row-smoke` 已接入 bootstrap smoke。
	- **验收**: `{x,y}` 与 `{y,x}` key 相同；closed row extra field 报错；open row field access 生成 tail obligation。

- [x] **Nominal identity + row shape**
	- **TODO**: `type T { ... }`、record shape、tuple shape、data/union payload shape 同时进入 nominal id 与 row shape 系统。
	- **DESC**: 同 shape 不同 nominal type 不能被错误合并；row 约束只描述 shape obligation。
	- **DONE**: `CirNominalId` 已携带 namespace/module name 与 local name；`src/backend/cir/type_nominal.chiba` smoke 证明同 row shape 的不同 namespace nominal type 不会 unify。
	- **验收**: 两个同字段 nominal type 的 shape key 可相同，但 nominal id 不同；method resolution 仍按 nominal receiver 默认查找。

## 4. Row / Checked Template

- [x] **Row-bound shorthand**
	- **TODO**: 支持 `def f(a: {r | name: Str}) = ...` elaborates to `def f[T: {r | name: Str}](a: T) = ...`。
	- **DESC**: 如果 parser 不能把 row type 放在参数位置，先修 grammar/generator，不能手改 generated parser。
	- **DONE**: `src/backend/cir/type_template.chiba` 固定 row-bound shorthand 的 L2 形态：synthetic generic var + open `CirRowMeta` bound + obligation dump。现有 `supports/semantic-gates/row_poly.chiba` 覆盖显式 row bound source gate。
	- **验收**: shorthand 与显式 `[T: row]` 产生同构 TypedAst/ObligationIR；多个 shorthand 参数默认不同 synthetic `T`。

- [x] **Field access obligation**
	- **TODO**: `x.name` 对 `x` 生成 `{r | name: a}` obligation，并返回字段类型 `a`。
	- **DESC**: 如果 `x` 是已知 nominal type，可直接检查字段；如果是 abstract/generic，则保留 obligation。
	- **DONE**: `CirObligationField(receiver, field, result)` 已进入 ObligationIR；`type-template-smoke` 输出 `obligation field $T0.name: $T1`，字段结果类型有独立 origin `CirTyOriginFieldResult("name")`。
	- **验收**: `def get_name(x)=x.name`、`def row_identity[T: {r| name: Str}](x:T):T=x` 通过；缺字段 fixture 报错。

- [ ] **Record literal/update typing**
	- **TODO**: record literal 产生 closed row；record update 通过 base row + updated fields 合成结果 row。
	- **DESC**: duplicate field、field type conflict、更新非 record-like value 必须报错。
	- **验收**: duplicate 字段、非法 update、字段顺序稳定 key 都有 fixture。

- [ ] **Generic body check**
	- **TODO**: 在抽象 generic 参数下检查 body，产出 `GenericBodyIR + ObligationIR`。
	- **DESC**: 不依赖 concrete type 的错误必须定义期报错；依赖 concrete shape 的行为留 obligation。
	- **验收**: `1 + true` 定义期报错；`x.name` 定义期生成 obligation；instantiation fixture 能兑现或失败。

## 5. Method / Operator / Type 关系

- [x] **Method index**
	- **TODO**: 建立 nominal receiver method index：`def Type.method(self, ...)` 按 `(nominal_id, method_name)` 注册。
	- **DESC**: 默认 method resolution 基于 nominal identity，不把 receiver 降成 structural receiver shape。
	- **DONE**: `src/backend/cir/type_method.chiba` 已实现 nominal receiver method index、lookup、duplicate detection；key 使用 namespace-qualified `CirNominalId + method_name`。
	- **验收**: 同 shape 不同 nominal type 调用各自 method；重复 method 定义报错；receiver 非 nominal 报错。

- [x] **Method call 三路径**
	- **TODO**: 固定并实现三路径：field-callable、nominal receiver method、qualified callee。
	- **DESC**: `.method(call)` 要按 spec 的三种标准稳定选择，二义性必须可诊断。
	- **DONE**: `CirMethodRoute` 固定 field-callable、nominal-receiver、qualified-callee 三路径；`type-method-smoke` 和 `supports/semantic-gates/method_resolution.chiba` 覆盖三路径。
	- **验收**: 每条路径有 valid fixture；冲突/缺失/receiver 错误有 invalid fixture；dump 显示最终 candidate kind。

- [x] **Operator obligation**
	- **TODO**: numeric builtin operator 直接 unify；abstract/generic receiver 生成 structural operator obligation。
	- **DESC**: `i64 + i64` 定义期解决；`T + T` 在有抽象参数时保留 `op_add` obligation，等价 contract 为 `T: {t | op_add: (Self, Self) => Self}`，但不能把 row fact 当普通 nominal method 证据。
	- **DONE**: `CirObligationOperator` 已记录 `op_add`、receiver、arg list、result 和 behavior source；`type-template-smoke` 覆盖 `op_add self=$T0 args=($T0) => $T0`。
	- **验收**: `1 + true` 报错；`def add(a,b)=a+b` 泛化为 `def add[T:{t|op_add: (Self, Self) => Self}](a:T,b:T):T`；generic operator 实例化失败能定位 call site。

- [x] **Shape dispatch boundary**
	- **TODO**: shape dispatch 只使用 row/shape facts 和 explicit dispatch syntax，不引入全局 witness search。
	- **DESC**: 为未来 level-2 `via namespace` 保留字段，但 Pre-C03 不默认引入 via 行为来源。
	- **DONE**: `CirObligationShapeDispatch` 已与 method/operator 分离，携带 explicit behavior source；`type-template-smoke` 覆盖 `source=via local`，不引入全局 witness search。
	- **验收**: shape dispatch obligation 可 dump、可 hash；没有全局 trait/interface 搜索。

## 6. Capability / ABI Typing

- [x] **Ref / assignment typing**
	- **TODO**: `lhs := rhs` 要求 `lhs: Ref[T]` 且 `rhs: T`；field assignment through ref 降为 whole-value replacement 语义事实。
	- **DESC**: `Ref[Array[T]]` 不允许直接 element assignment；`Array[Ref[T]]` 可通过元素 ref 赋值。
	- **DONE**: `src/backend/cir/type_capability.chiba` 已实现 `Ref[T]` assignment type check；`type-capability-smoke` 覆盖 ok 与 mismatch。
	- **验收**: safe ref valid/invalid fixtures 全进 compiler-side L2 check。

- [x] **UnsafeRef / Ptr unsafe boundary**
	- **TODO**: `UnsafeRef[T]`、`Ptr[T]` 只能在显式 `unsafe` 区域使用；非 Metal 源码不能裸 pointer API。
	- **DESC**: Metal 内部也优先 typed `Ptr[T]`，不能扩散 opaque `i64` pointer 接口。
	- **DONE**: `type_capability` 已实现 unsafe-depth gate；`type-capability-smoke` 覆盖 `Ptr` safe error 与 unsafe ok。
	- **验收**: Pre-C12 source gate 迁入 L2 pass；valid/invalid fixture 不再依赖 JS 扫描为唯一来源。

- [x] **Atomic typing**
	- **TODO**: 限定 `Atomic[T]` 的 T 集合与操作 ordering；load/store/cas/fetch op 类型检查。
	- **DESC**: ordering 是 atomic API 参数，不属于 ordinary value typing 或 send 推断。
	- **DONE**: `type_capability` 已限定 `Atomic[T]` 支持 `i32/i64/usize/bool/Ptr[T]`；`type-capability-smoke` 覆盖 `Atomic[Ptr[i32]]` 与 `Atomic[String]`。
	- **验收**: unsupported `Atomic[String]`、错误 ordering、错误 value type 都有 L2 diagnostic。

- [x] **Extern ABI typing**
	- **TODO**: `extern "wasi"`、`extern "C"`/`"c"`/`"wasi"` 必须显式参数/返回 ABI type；backend 只消费 typed import ref。
	- **DESC**: emit 阶段不猜 ABI。先固定 WASI fd_write 等最小签名，C/wasi 保留 embedder typed import。
	- **DONE**: `type_capability` 已复用 `cir_type_abi_scalar_kind` 检查 ABI scalar；`type-capability-smoke` 覆盖 `i32` ABI ok 与 `String` ABI error。
	- **验收**: ABI unsupported、缺标注、fd_write 签名错误都由 L2 check 报错。

## 7. 单测与验证矩阵

- [ ] **Unifier unit tests**
	- **TODO**: 建 `supports/type-system/unify-*` fixtures 或专用 runner。
	- **验收**: var、fn、tuple、type app、row、occurs check、kind mismatch、substitution dump 全覆盖。

- [ ] **Inference fixtures**
	- **TODO**: 扩展 `supports/semantic-gates/type_inference.chiba`。
	- **验收**: unannotated params、return inference、explicit+implicit generic、let-generalization、value restriction、bad branch type 全覆盖。

- [ ] **Row fixtures**
	- **TODO**: 扩展 row poly fixtures。
	- **验收**: row shorthand、field obligation、record literal/update、canonical field order、nominal identity、tuple positional row。

- [ ] **Checked template fixtures**
	- **TODO**: 增加定义期错误、实例化期错误、成功 specialization 三组 fixture。
	- **验收**: 定义期错误不等到实例化；实例化错误指向 concrete call site；obligation dump 稳定。

- [ ] **Method/operator fixtures**
	- **TODO**: 增加 `.method(call)` 三路径和 operator obligation fixtures。
	- **验收**: valid/invalid/golden dump 全覆盖；二义性诊断稳定。

- [ ] **Capability/ABI fixtures**
	- **TODO**: 把 Pre-C12 与 extern gate 全部接入 L2 semantic runner。
	- **验收**: `vp run semantic:gates`、`vp run level1b:capability`、`level1c.o check` 对同一组 fixture 结论一致。

- [ ] **Golden dump**
	- **TODO**: 为 TypedAst、ConstraintSet、ObligationIR、canonical row key 建 golden。
	- **验收**: 字段顺序、synthetic generic id、diagnostic order、constraint order 全部确定。

## 8. Pre-C03 完成标准

- [ ] `level1c.o check` 不再以 source-level pattern gate 作为主要类型系统实现。
- [ ] L2 pass 能输出稳定 `TypedAst + ConstraintSet + ObligationIR`。
- [ ] `def f(a,b,c)=expr` 自动泛化符合 spec。
- [ ] `def f(a: {r | ...})` row shorthand 与显式 `[T: row]` 等价。
- [ ] `[T]` 的语义、作用域、diagnostic、specialization identity 稳定。
- [ ] unification 有独立测试，不只通过 parser fixture 间接覆盖。
- [ ] row canonicalization 可 dump、可 hash、字段顺序无关。
- [ ] method resolution 三路径可 dump、可诊断、可测试。
- [ ] capability/ABI typing 进入 L2 pass。
- [ ] TODO.md 的 Pre-C03 可以打勾，并附上测试命令、seed hash、关键 dump hash。
