## 最高优先级强制规则

- **这条规则优先于后面所有约定：禁止 lexer 外字符串语义推断，identifier 合法性只准走 UTF-8/XID / 共享 XID 表**：除 lexer / chibalex / regex tokenization 之外，任何 pass 都不得通过字符串形状推导语法、identifier、callable、namespace、static/global 名称、类型或语义合法性。semantic / pipeline / backend / lowering 必须消费 parser AST、symbol table、typed facts、Core/CIR facts；identifier 合法性统一来自 lexer 的 UTF-8/XID 规则或共享 XID 表。禁止在这些后续阶段临时写 `is_ascii*`、`split`、`contains`、大小写判断、前后缀判断等作为语义依据。

## 强制工程规则

这两个目录是 level-1 的 spec 目录
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/chiba-level1-spec`
- `/Users/yoli/Desktop/lemon/CHIBA/chiba-org-web/src/content/type_system`

阅读前参考: [level-1r](./level-1r.md)

## 已确认设计决定（2026-05-24）

- 总原则：遇到未细化处，**优先看齐严格版 TS / Rust 的语义纪律，但不要为了教条而把语言写死**；优先保证语义正确，其次再收 dump/golden 稳定性。
- post-C12 仓库收口：`level0/` 与 legacy `src/` / `level1c` **直接删除**，不做长期 archive 并存；Git 历史足够承担追溯责任。
- operator overloading：
	- 先按 **方法式** 收口；
	- ambiguous resolution 归 **typed 阶段报错**，不是 parser 阶段；
	- precedence / associativity **固定死**；
	- 当前 grammar/spec 其实**已经有 `interface` surface**（例如 `chiba-level1-grammar-spec/26-test.chiba`），但 overloading 本轮不要求先走 interface-driven resolution。
- generics / template：
	- `def id(x) = x` 这类 **auto-generic surface** 是正式语言特性；
	- 同时保留显式 `[T]` 定义与调用点实例化；
	- `x[T](v)` **不是唯一**显式实例化写法；
	- inference / explicit instantiation / auto-generic 若产生冲突，**报错，不偷偷择一**。
- global/init：
	- 允许顺序依赖 global（如 `VAR2 = VAR`），但 cycle 一律拒绝；
	- 允许副作用 top-level / record init block；
	- init 执行时机按 **module load** 语义处理。
- shadowing / scope：
	- 局部 shadowing 允许；
	- pattern bind 也允许 shadowing；
	- 参数名可以被局部 shadow；
	- **同层 / 同 namespace duplicate 报错**；
	- `use` 导入的名字允许被 local binding shadow。
- pattern / match / `if let`：
	- 走 **语义严格** 路线；
	- `match` 必须做 exhaustiveness；
	- `if let` 失败分支不引入绑定；
	- pattern args for funcs 是**真正语言特性**，不是仅供内部 desugar 的约定；
	- 非穷尽匹配当前按 **warning** 收口，而不是立刻 hard error / runtime trap。
- pipe：
	- placeholder `_` 每出现一次都替换为输入值；
	- `a |> A.b` 是 receiver-first desugar；
	- method-pipe 与 dot-call 要求完全语义等价；
	- pipe 可与 operator overloading 混合参与解析；
	- placeholder 作用域按“只作用于当前 pipe 右侧表达式”处理。
- ADT tuple bridge / ctor lowering：
	- `Ctor(...) <-> (:ctor, ...)` 是语言级 contract；
	- `tuple_to_adt` / `adt_to_tuple` 是 **compiler intrinsic**；
	- typed 层仍要保留“这是 ADT ctor / bridge”的身份，并允许与 tuple 表示做 unify；
	- bridge 需要双向稳定 roundtrip。
- intrinsic / namespace ownership：
	- intrinsic 必须有独立 namespace；
	- `std` / `prelude` 只是 import layer，不拥有 intrinsic 语义；
	- imported item 即使 merged 进同一编译单元，仍保留原始 owner namespace；
	- name resolution 冲突优先按 **local scope / 当前 namespace / use 导入** 的常规优先级处理，冲突不靠合并顺序偷偷决定。
- mangling / debugability：
	- 倾向学习 TypeScript 式“**用户看见的名字尽量贴近 source**”体验，而不是把所有可读性都丢给内部 mangling；
	- 需要补 manifest / debug map，但以语义正确和 source-facing 可理解性为先。
- typed/golden：
	- 首先保证**语义正确**；
	- golden/stability 是第二优先级；
	- 可以接受后续通过 canonicalization 降噪，但不能用 canonicalization 掩盖真实类型错误。
- continuation / callable / closure：
	- `Ref[T]` 是 mut surface；multi-shot continuation 捕获 `Ref[T]` 采用 **shared-reference** 语义，不 snapshot / copy / rollback captured cell。
	- continuation surface type 固定为 `Cont1[A, B]` / `cont1 (A) -> B` 与 `ContN[A, B]` / `contN (A) -> B`；二者默认 `!send`。
	- `shift k { ... }` 捕获 `Cont1`；`resetn {shift k { ... }}` 捕获 `ContN`；带 tag 时写 `shift :tag k { ... }`
	- 非逃逸、静态 exactly-once 的 `Cont1` **必须** direct resume / inline / tail jump，不得分配 continuation package。
	- 逃逸的 `Cont1` 可以进入 `(A) -> B` storage，但必须 boxed 成 one-shot consumed-state machine；第一次调用 consume，重复调用 runtime error / trap。
	- 显式 `cont1 (A) -> B` storage lower 成 boxed one-shot state machine；显式 `contN (A) -> B` storage lower 成 multi-shot continuation package。
	- `ContN` 可以进入普通 `(A) -> B` storage；它可重复恢复，但仍默认 `!send`。
	- 参数位置的 `(A) -> B` 走 checked-template callable obligation，可实例化为 function / closure / `Cont1` / `ContN`。
	- 存储位置的 `(A) -> B` lower 成 erased callable ADT，variant 至少包含 function / closure / boxed `Cont1` / `ContN`；调用时 tag dispatch，静态已知 variant 时必须优化掉 dispatch。
	- `((A) -> B) send` 是 sendable callable storage，必须排除 `Cont1` / boxed `Cont1` / `ContN` 与所有 `!send` closure；传给 `spawn` 这类要求 send callable 的位置时 continuation 必须报错。
	- no-capture closure **必须** 编译优化为 direct function / funref / inline，不得分配 closure env。
- CIR/backend boundary：
	- CIR 层千万不能和 Wasm / Wasm-GC / WAT / Binaryen / target ABI 耦合。
	- CIR 只能携带语言级事实：continuation kind、callable storage kind、closure capture set、usage、send、arena、answer type、是否需要 boxed `Cont1` / `ContN` / erased callable ADT 等 obligation。
	- Wasm-GC struct layout、`funcref` / `eqref`、frame header、WAT opcode、Binaryen feature、target import ABI 必须下沉到 BIR/LIR/backend layout 层。
- nanopass 染色轴：
	- `UsageColor`: `1 | N | Unknown/Obligation`，必须能挂到 type occurrence、binder、value、aggregate shape、callable、closure env、continuation frame/package；最终 typed/lowered AST dump 需要能显示类似 `def xxx(x: N YYY): 1 ZZZ`。
	- `SendColor`: `send | !send | obligation`，属于 builtin capability fact，不进入普通 method/interface 世界。
	- `ContinuationKind`: `None | Cont1 | ContN`，由 `reset` / `resetn` delimiter 决定，不由 callable storage 猜。
	- `EscapeColor`: `local | escapes | storage | promoted | world/thread illegal`，服务 closure、continuation、dyn package 与 arena legality。
	- `ReplaySafety`: `safe | unsafe | rollback-region`，主要服务 `ContN` 与 multi-resume parser/compiler backtracking。
	- `CallableStorageKind`: `direct_fn | no_capture_closure | env_closure | boxed_cont1 | contn_package | erased_callable_adt`。
	- `Arena/ResetBoundary`: implicit/explicit reset id、answer type、region id。
	- `Ownership/NamespaceIdentity`: owner namespace + symbol id + intrinsic owner，防止 source merge 后丢来源。
	- 这些颜色不是同一个字段；CIR 可以携带语言级颜色事实，但 backend layout 只能消费事实，不能重新推语义。
- dynamic row / boxed-unboxed：
	- `dyn {r | ...}` 是带 adapter 的 dynamic row package，不是裸 row，也不是运行时全局 impl search。
	- 纯临时 dynamic row 可以采用 JS-like shape / hidden-class 表示作为 runtime layout 优化，但 type system 只看 `DynRow[Contract, PayloadColor, SendColor, ShapeId?]` 这类语言级事实。
	- 静态 row poly 提取后动态化，形态应是 `T -> dyn {r | fields...}` 的 expected-type injection：编译器打包 value + row adapter + optional nominal/debug identity。
	- `dyn` 反向回 concrete nominal type 不能自动发生，必须 explicit checked conversion；仅按 shape 猜 nominal type 不合法。
	- “一个函数既能吃 box 又能吃 unbox”不应靠隐式到处 box。参数位置优先用 checked-template row obligation：`def f[T: {r | x: A}](v: T)` 接受 unboxed nominal/record，也可接受 `dyn {r | x: A}` 作为一个 concrete dynamic package instance。
	- 若函数确实要求 boxed dynamic storage，则参数写 `dyn {r | ...}`；静态值在 expected type 为 dyn 时自动注入，调用体通过 adapter 访问字段/方法。
	- Core/CIR 里区分 `StaticRowAccess`、`DynRowAdapterAccess`、`DynRowShapeAccessCandidate`；JS-like shape 只能是后端优化，不改变类型规则。
- visualization / mangling / backend ownership：
	- 可视化不是 debug 附属品，而是主线验收：source、resolved、alpha、typed、usage-colored typed、CPS、closure-converted、specialized、CoreIR、layout、final symbol map 都要能 dump。
	- 每层 dump 都应显示：source span、owner namespace、symbol id、type、usage/send/escape/replay colors、continuation kind、callable storage kind、dyn/static access kind、mangled/debug name。
	- mangling 必须分离“稳定唯一 id”和“source-facing debug name”：后端可用 opaque id 保证 collision-free，但 manifest/debug map 必须能从 final symbol 反查 source path / namespace / item / specialization key / pass origin。
	- lowering 后的名字建议形态：`source_path::namespace::item#specialization#lowering_role#stable_id` 作为 debug name；final ABI symbol 可短，但必须进入 manifest。
	- backend target 分两步：当前主线 target 是 **wasm-gc**，之后新增 **wasm** no-GC target；二者必须共享 target-neutral CoreIR / BIR 语义事实。
	- 中端不能被 Wasm-GC 污染：Wasm-GC struct/array/funcref/eqref 是当前 backend layout choice，不是 CoreIR 语义前提。
	- 未来 no-GC Wasm target 下，根据 usage analysis 决定表示：`1` 且非 escaping 走 stack/inplace/direct；`N` 且 `!send` 走 RC；`N` 且 `send` 走 ARC；可唯一更新的数据走 Perceus / functional-but-inplace / Koka-style reuse。
	- `send` 不只影响类型检查，也影响 runtime ownership lowering：跨 world/thread 的共享对象不能走非原子 RC，必须 ARC 或被拒绝。
	- backend emitter 只能消费 CoreIR 中的 ownership decision：`StackValue | InplaceReuse | RC | ARC | StaticData | BorrowedView | DynPackage`，不能重新根据 Wasm target 猜。


## P0 Step 1：非性能剩余落地

这些是当前不考虑编译加速时仍未完全落地的 P0 项。已完成并已有稳定 gate 的历史 checklist 不再保留在这里；新工作优先把下面每项做成实现 + fixture / diagnostic gate + runtime 或 dump 证据。

- [ ] **checked generics / template 完整化**
	- auto-generic surface、显式 `[T]` 定义、调用点 explicit instantiation 三者都必须可组合，并在冲突时报错。
	- generic definition 必须在抽象参数下完成基础检查，产出 `GenericBodyIR` / `ObligationIR`，不能等实例化才发现与任何 concrete type 无关的 body 错误。
	- concrete instantiation 必须兑现 row field、receiver method、operator、shape dispatch、dyn adapter、callable、continuation/capability obligation；错误报 call site 并保留 definition note。
	- specialization key 必须包含 generic id、concrete nominal/type args、canonical shape、capability facts、continuation facts、ABI/layout mode；同 shape 不同 nominal 默认不能合并。

- [ ] **operator / method / row dispatch 闭合**
	- `+ - * /`、`[x]`、`[x..y]`、method-style operator、field-callable-before-method 的 invalid / ambiguous / wrong-operand 矩阵必须进入稳定 gate。
	- `a..b` range value 要成为 slice/index、pattern/range match、iterator range 共享 AST 节点；`.slice(start, len)` 只能作为 parser 未支持前的过渡 fixture。
	- method/operator candidate index 要有稳定 key 和跨 namespace/shape ambiguity diagnostics；不得靠字符串形状或合并顺序选择。

- [ ] **global / module init 完整化**
	- `def x: T = const` 是常量/全局值主 surface；清理 `def x(): T = const` 这种零参函数伪常量依赖。
	- 顺序依赖 global 可用，cycle 拒绝；副作用 top-level / record init block 按 module load 语义执行。
	- module-load ordering、side-effect init、跨 namespace init、重复/循环 diagnostics 必须有 executable 或 dump gate。

- [ ] **deep pattern / exhaustiveness / if-let 闭合**
	- `match`、`if let`、函数参数 pattern、深 tuple/record/ADT payload pattern 的 elaboration、typed env、lowering 必须一致。
	- `if let` 失败分支不引入绑定；`let` 只能接受 irrefutable 子集。
	- exhaustiveness 先按 warning 收口，但必须由 pattern matrix 稳定产出，不允许 lowering 和 typed 结果分叉。

- [ ] **ADT tuple bridge / intrinsic ownership 闭合**
	- `Ctor(...) <-> (:ctor, ...)` 是语言级 contract；`tuple_to_adt` / `adt_to_tuple` 是 compiler intrinsic，不是 std helper。
	- typed/core/lowering 必须保留 bridge intrinsic identity，保证 tuple 表示和 ADT constructor 双向 roundtrip。
	- `unsafe_cast`、tuple type 本体、bridge intrinsic 必须由 compiler-owned namespace 提供；普通库函数或同名 source item 不能伪装 intrinsic。

- [ ] **continuation / callable / capability 安全矩阵**
	- `Cont1` 非逃逸 exactly-once 必须 direct resume / inline / tail jump；逃逸或存储时 boxed one-shot state machine，多次使用由编译器静态拒绝或运行时 trap。
	- `ContN` 必须可重复 resume；捕获 `Ref[T]` 是 shared-reference 语义，捕获 `UnsafeRef[T]`、FFI/world-local/thread-local/Atomic mutation 必须按 replay-safety 或 rollback-region 规则诊断。
	- callable 参数位置接受 function / closure / `Cont1` / `ContN`；存储位置 lower 成 erased callable ADT；`send` callable storage 必须拒绝 continuation 和 `!send` capture。
	- 非法 continuation 不得流入 closure conversion、Core lower 或 backend；validator 要能稳定拒绝残留非法 package。

- [ ] **closure / lambda / usage color 完整化**
	- binder、lambda、closure、continuation、aggregate、env field、code pointer 都要有 `0 | 1 | many` / `UsageColor` 证据。
	- no-capture closure 必须 direct function / funref / inline，不分配 env；escaping closure 才生成 `{code, env}`。
	- closure conversion、lambda lifting、env shrinking、known-callee direct call、dead capture elimination 必须保留 usage/send/escape/replay facts 并有 dump gate。

- [ ] **ownership / layout / runtime lowering 完整化**
	- CoreIR 必须携带 target-neutral ownership decision：`StackValue | InplaceReuse | RC | ARC | StaticData | BorrowedView | DynPackage`。
	- record/data/union/string/slice/Array/Vec/callable/closure/continuation/dyn row package 都要有稳定 layout facts 和 validation。
	- `1` 且非 escaping 走 stack/inplace/direct；`N` 且 `!send` 走 RC；`N` 且 `send` 走 ARC；backend 只能消费这些 facts，不能重新推语义。
	- single-use constructor/value update 应 lower 成 mutation / in-place reuse；需要 fixture 证明不是复制型 fallback。

- [ ] **symbol manifest / debug map / namespace ownership**
	- namespace 是定义所有权边界；`std` / `prelude` 是 import layer，`intrinsic` / `compiler` / `std` / `metalstd` 必须隔离。
	- imported item merged 后仍保留 owner namespace；resolution 明确区分 local、current namespace、explicit use、default prelude、compiler intrinsic。
	- source -> resolved -> alpha -> typed -> CPS -> closure -> specialized -> CoreIR -> final symbol 必须可 dump，并通过 manifest 从 final ABI symbol 反查 source path、namespace、item、specialization key、layout key、ownership decision。
	- mangling 不得把 Wasm-GC / WAT 细节泄漏到 CIR；target-specific final name 只能在 backend symbol layer 出现。

- [ ] **multi-namespace backend / link / ABI**
	- 当前 wasm-gc target 要支持 namespace / specialization bundle 独立产物和最终 link；单 namespace WAT 成功不等于 P0 完成。
	- WASI/env ABI matrix 要稳定：`extern "wasi" "fd_write"`、`extern "C"` / `extern "c"` 归一化、import module/signature hash、错误 ABI diagnostics 都要有 gate。
	- tailcall 或等价 trampoline、thread/world import/export、manifest/link artifact、wat roundtrip 都必须可验证。

- [ ] **no-GC wasm 或共享 target-neutral lowering 证明**
	- 主线可以先继续 wasm-gc，但 CoreIR / BIR 不得依赖 Wasm-GC struct/array/funcref/eqref 作为语义前提。
	- no-GC wasm target 至少要能从同一 CoreIR ownership/layout facts 映射到 linear-memory + RC/ARC runtime helper 方案，或有完整 target-neutral dump gate 证明后端已不需要重新推语义。

- [ ] **legacy lowering shortcut 清理**
	- 清理历史遗留的硬编码 `i64`、`1`、`0`、`if x != 0` 等 lowering shortcut；保留合法测试常量但给 stale fallback 加 scoped lint/audit。
	- 所有 pass 只能消费 AST、symbol table、typed facts、Core/CIR facts；禁止 lexer 外字符串形状语义推断。

## P0 Step 2：编译加速 / 并行 / 增量

Step 2 只在 Step 1 的语义和中后端证据足够稳定后推进；性能优化不能通过削弱 executable WAT 覆盖或跳过语义 gate 获得。

- [ ] **稳定 timing / threshold gate**
	- pass timing、backend emit timing、WAT runtime setup/execution timing 要拆开报告。
	- 给 hot test 和代表性项目定义阈值；超时要指出具体 pass / backend / runtime setup，而不是只给 full test 总耗时。

- [ ] **ProjectSurface / InterfaceSummary 增量缓存**
	- 缓存 `ProjectSurface`、`InterfaceSummary`、`GenericBodyIR`、`ObligationIR`、specialization artifact、CoreIR、wat/object。
	- cache key 包含 source hash、summary hash、compiler version、target backend/features、ownership runtime mode、extern ABI/import module/signature hash。
	- 修改非导出函数体只重编当前 namespace 和受影响 specialization；修改 public summary 触发依赖 namespace 重检。

- [ ] **并发实例化注册表**
	- 实现 `InstantiationRegistry`: `Missing | InProgress | Done | Failed`。
	- 多 worker 请求同一 specialization key 时 join 现有任务；失败结果可缓存并稳定报告。
	- recursive generic 用 SCC / in-progress marker 处理；无限类型增长实例化必须报 `monomorphization does not converge`。

- [ ] **namespace / body 并行区**
	- NameResolve、Alpha、PatternElab、HM+Row、Answer/ContinuationKind、World/Send/Escape/Capability、Usage、CPS、Closure/Lambda、GenericDefinitionCheck、CoreLower、Emit 都按 namespace 或 body 分发。
	- 只允许 ProjectSurface merge、summary conflict reduce、namespace graph/SCC 构建、最终 linker 作为收敛点。
	- 多核输出必须确定，symbol id、diagnostic order、manifest order 不依赖调度。

- [ ] **specialization 并行区**
	- MonomorphizationScheduler 把每个 unique key 作为独立任务，任务只读 summaries/index/cache，只写自己的 artifact。
	- generic-heavy 项目要证明重复实例去重、错误 join、cache hit、并发 artifact 生成都稳定。
