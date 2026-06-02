这两个目录是 level-1 的 spec 目录
- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec`
- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/type_system`

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


## checkpoint checklist：剩余语言/语义面收口

这些项里有些是“补实现”，有些是“补 fixture / 补 gate / 补 compiler-side lowering 验收”。目标不是把所有条目都重新发明一遍，而是把它们全部收进稳定 gate。

- [ ] operator overloading
	- **目标**:
		- `+ - * /` 等常规 operator；
		- `[x]` / `[x..y]` 对应 `op_index` / `op_index_slice`；
		- long-term：表达式语法支持 `a..b` range value，作为 slice/index、pattern/range match、iterator range 的共享 AST 节点；当前 mini lexer 里使用 `.slice(start, len)` 只是 parser 未支持 `a..b` 前的过渡写法。
		- invalid / ambiguous / wrong-operand case 进入统一 gate。
	- **已确认**:
		- 当前按方法式收口；
		- ambiguous 归 typed 报错；
		- precedence / associativity 固定；
		- 不要求这轮先做 interface-driven resolution。

- [ ] template / generics 剩余收口
	- **目标**:
		- `def id(x) = x` 与 auto-generic surface 对齐；
		- `x[T](v)` 明确走 explicit instantiation；
		- checkpoint 已覆盖 row shorthand / type inference / checked instantiation；剩余是 explicit instantiation、auto-generic surface 与更深 runtime 验收。
	- **已确认**:
		- `def id(x) = x` 要成为正式语言特性；
		- auto-generic / explicit `[T]` / 调用点实例化三者并存；
		- 冲突时报错，不隐式择一。

- [ ] global variable / init block 规则
	- **目标**:
		- `def ONE:i64 = 1` 这类全局值稳定可用；
		- 常量/全局值 surface 收敛到 `def x: T = const`，后续清理 `def x(): T = const` 这种零参函数伪常量；
		- record/global init block 有确定执行时机；
		- `def VAR2 = VAR` 这类依赖可行；
		- cycle / co-dependent global 稳定拒绝。
	- **已确认**:
		- 允许顺序依赖；
		- 允许副作用 init；
		- cycle 一律拒绝；
		- 执行时机按 module load 处理。

- [ ] `Self` type with generics
	- **目标**: `type X[T] {x:T}` + `def X[T].update_x(self: Self, new_x: T): Self = {self|x:new_x}` 一类模式通过 typed + runtime 验收。

- [ ] deep pattern matching
	- **目标**:
		- `match` expr 深模式；
		- `if let` expr；
		- exhaustiveness / lowering / typed pattern env 三者一致。
	- **已确认**:
		- 走语义严格路线；
		- exhaustiveness 要做；
		- `if let` 失败分支不引入绑定；
		- 非穷尽匹配先按 warning 收口。

- [ ] pipe behaviour
	- **目标**:
		- `a.b() == a |> A.b`
		- `a.b().c() == a |> A.b |> A.c`
		- `a |> f(b,_,_) == f(b,a,a)`
		- `a |> f == f(a)`
		- `a |> f |> g == g(f(a))`
	- **备注**: 这里既要看 parse，也要看 method/operator lowering 与 placeholder expansion 是否一致。
	- **已确认**:
		- `_` 每次都替换输入值；
		- `a |> A.b` 是 receiver-first desugar；
		- dot-call 与 pipe 要求完全语义等价；
		- 可与 operator overloading 混合解析。

- [ ] ADT tuple bridge / ctor lowering
	- **目标**:
		- `HttpError(400, "...") <-> (:http_error, 400, "...")`
		- `tuple_to_adt[T]` / `adt_to_tuple[T]` method-first builtin surface 固定；
		- constructor 经 type / exhaustive check 后 lower 到 canonical tuple / record contract。
	- **备注**:
		- 这项和当前 nominal/data constructor backend blocker 强耦合，应优先与 `codegen_contract` 一起收。
		- `tuple_to_adt[Tuple, T]` / `adt_to_tuple[T, Tuple]` 不应被当作普通 std helper，而应与 compiler intrinsic 同级：由编译器打洞、类型检查和 lowering 共同承认。
	- **已确认**:
		- 这是语言级 contract；
		- bridge intrinsic 需要保留 typed 身份，并与 tuple 表示可 unify；
		- 要保证双向稳定 roundtrip。

- [ ] compiler-internal intrinsic surface
	- **目标**:
		- `unsafe_cast[T, F](self: T): F` 明确是 internal / compiler-only intrinsic，不暴露成普通安全 std API；
		- `tuple_to_adt[Tuple, T]` / `adt_to_tuple[T, Tuple]` 明确归为同级 internal bridge，而不是普通库函数语义；
		- `Tuple[T1, T2, T3, T4, ...]` 的类型本体由编译器打洞和 lowering 承认，不要求用户层 std 自己伪装出真正的 tuple type system。
	- **备注**:
		- 这些能力可以有用户可见 surface，但语义来源必须是 compiler intrinsic / builtin contract；
		- 不能靠普通库层“模拟”替代真正的 type/lowering 支持。

- [ ] pattern args for funcs
	- **目标**:
		- `def name(Some(x): Option[X]): Y = ...`
		- `def name(None: Option[X]): Y = ...`
		- compiler-side desugar / typed env / runtime dispatch 一致，不停留在手工约定。

- [ ] 作用域和let shadowing作用域的检查
	- **目标**: 函数式编程语言经常复用名称，所以千万千万要构建逆天的测试集
	- **已确认**:
		- local / pattern / param shadowing 允许；
		- 同层（同 namespace）duplicate 报错；
		- local 可以 shadow `use` 导入名。

- [ ] 不再出现 `i64` 和 `1` `0` 还有 `if x != 0` 这种历史遗留代码

- [ ] mangling / symbol debugability 收口
	- **目标**:
		- mangled symbol 不再只剩 `_bir_fun_4307_entry` 这种几乎不可追踪的信息；
		- compiler 能稳定输出 `source symbol -> lowered symbol -> final mangled name` 的 manifest / debug dump；
		- 把“唯一性”与“可读 debug 名”拆开：允许 backend 用内部 id 保证 collision-free，但调试、报错、IR dump、WAT 注释里保留可读 source path / item path；
		- 对 imported module / method / ctor / intrinsic 统一给出可 grep 的命名约定，不再靠猜当前 pass 的临时编号。
		- 每个 pass 的 visual dump 都能显示 symbol lineage：`source -> resolved -> alpha -> typed -> cps -> closure -> specialization -> core -> final`。
		- final ABI symbol 即使被压短，也必须能通过 manifest 反查 owner namespace、source span、specialization key、layout key、ownership lowering decision。
	- **备注**:
		- 这项不是要立刻取消 mangling，而是要让 mangling 变成“可逆、可观察、可定位”；
		- gdb/backtrace、nanopass dump、WAT emit、validation err 至少要能共享同一套 symbol 追踪信息。
		- mangling 不允许泄漏 Wasm-GC / WAT 细节到 CIR；target-specific final name 只能在 backend symbol layer 出现。
	- **已确认**:
		- 方向上参考 TypeScript 的 source-facing 体验；
		- 优先让用户可理解的名字、报错、调试体验正确，再补 manifest/debug map。

- [ ] namespace ownership / isolation 收口
	- **目标**:
		- namespace 是定义所有权边界，不是简单的 source merge 标签；
		- imported item 即使被 driver 合并进同一个编译单元，语义上仍保留原始 owner namespace；
		- name resolution 明确区分 local scope、当前 namespace、自显式 `use` 导入、default prelude、compiler intrinsic，冲突时稳定报 ambiguous / duplicate；
		- nominal type、method、ctor、global value、intrinsic surface 都有稳定的 `owner namespace + item path` 身份，不再因合并顺序丢失来源。
	- **备注**:
		- 后端 mangling/debug 名必须建立在 namespace ownership 之上，否则 debug map 仍然不可靠；
		- `prelude` 是 import layer，不应偷变成 owner namespace；`intrinsic` / `compiler` / `std` / `metalstd` 也应各自隔离。
	- **已确认**:
		- intrinsic 必须独立 namespace；
		- imported item merged 后仍保留 owner namespace；
		- resolution 不得依赖合并顺序偷偷决定结果。

- [ ] constructor like `once` used value lower to mutation
	- **目标**: 
		- `def x(x:X):X = ...` 且 x 只用了一次，这个函数应该变成传入x的mutation

- [ ] **Pass 00: Project Surface Scan**
	- **TODO**: 扫描项目文件，解析 source file header、`namespace`、`use`、item header、attrs、public/private surface，不进入函数体语义。
	- **DESC**: 这是全项目最轻的串行/低并行度 pass，用来构建 namespace graph 和后续并行任务队列。函数体、表达式、局部类型推断都不在这里做。
	- **验收**: 给定 N 个 source file，能产出稳定排序的 `ProjectSurface`；重复 namespace、非法 source-file header、显式 entry 冲突能报错；输出不依赖文件系统遍历顺序。
	- **并行**: 文件级 parse 可并行；最终合并 namespace graph 需要确定性 reduce。

- [ ] **Pass 01: Interface Summary Build**
	- **TODO**: 为每个 namespace 生成接口摘要：导出函数签名、type/data/union layout header、method header、static header、generic parameter headers、约束头、可见性。
	- **DESC**: 这是并行编译的边界。跨 namespace 的 body 编译只读 summary，不读对方函数体。summary 必须足够支撑 name resolve 和 definition-time typecheck。
	- **验收**: 任意 namespace 可独立加载依赖 namespace 的 `.chiba.meta` / in-memory summary；修改非导出函数体不改变 summary hash；修改导出签名会改变 summary hash。
	- **并行**: namespace 级并行；依赖环只允许在 signature 层形成 SCC，SCC 内做确定性合并。



- [ ] **Pass 02: TopDef / Kind Check**
	- **TODO**: 检查顶层定义 kind：函数、extern、static、type、data、union、method-style `def Type.method`、generic header、row bound header、`via` 可见路径 shape。
	- **DESC**: 只检查 header 良构性和重复定义，不解析 body。extern declaration 的 day-0 surface 是 `def f(args): ret = extern "abi" "symbol"`；wasm backend 至少支持 `abi == "wasi"` 和 canonical C/env ABI（接受 `"C"` 与 `"c"`，内部归一化）；其他 future ABI 必须在这里稳定报错。长期 level-1 主线先以 row / shape / structural obligation 为 generic header 基础；若未来恢复 namespace-scoped named constraints，应作为 level-2 扩展单列，不默认进入本 pass 的 level-1 contract。`send`/`!send`、world boundary、Atomic 先作为 builtin capability family 处理，不进入普通 method/interface 世界。
	- **验收**: 能拒绝重复 top-level symbol、重复 constructor、非法 generic bound、多个 row bound、method receiver 非 nominal、`private` 跨 namespace 泄漏。
	- **并行**: namespace 级并行；全局 symbol table 合并需要确定性冲突报告。

- [ ] **Pass 03: Name Resolve**
	- **TODO**: 把 body 内所有 value/type/path 引用解析为稳定 symbol id；处理 `use`、inline namespace、constructor、field、method 名称，并为未来 level-2 的 `via namespace` 显式来源保留扩展钩子。
	- **DESC**: 解析结果不携带具体类型，只携带绑定目标和候选索引。generic body 中 shape-dependent method/operator 不在这里最终决议，只生成可延迟的候选引用。`via namespace` 若保留，作为未来 level-2 显式行为来源钩子，不应成为 level-1 默认解析前提。
	- **验收**: 未定义名称、二义性 import、不可见 private、错误 namespace path、错误 constructor arity 能报错；同一输入多次编译 symbol id 稳定。
	- **并行**: namespace body 可并行；只读 ProjectSurface 和 InterfaceSummary。

- [ ] **Pass 04: Alpha Conversion**
	- **TODO**: 给函数体、pattern binding、lambda、continuation binder、generic local binder 分配唯一 id，并消除 shadowing/capture 歧义。
	- **DESC**: alpha conversion 是 usage analysis、one-pass CPS、closure conversion、lambda lifting 的地基。后续 pass 不再按裸名字判断绑定关系。
	- **验收**: 同名 shadowing 的 body 产生不同 binder id；capture 不会因改名改变语义；alpha dump 稳定且可 golden test。
	- **并行**: 函数体级并行；body-local id 可用 scoped arena，summary 边界只暴露稳定 symbol id。

- [ ] **Pass 05: Pattern Elaboration**
	- **TODO**: 把 `let` / `if let` / `match` / function parameter pattern 规范化为 `PatternCore`，并标记 refutable / irrefutable、binding set、constructor/literal/record/tuple destruct。
	- **DESC**: 先把 pattern 语义从表达式 typecheck 中拿出来。不同位置的 pattern 支持矩阵在这里落实：`let` 只接受 DFT irrefutable 子集，`if let` / `match` 接受 refutable pattern。
	- **验收**: 能拒绝 `let` 中可能失败的 constructor/literal pattern；能发现重复 binding；能产出 match exhaustiveness 输入所需的 pattern matrix。
	- **并行**: 函数体级并行；每个 body 只依赖 resolved/alpha AST。

- [ ] **Pass 06: HM + Row Inference**
	- **TODO**: 做基础 HM 推断、unify、let-generalization、row/open-row 约束生成、field access / record update / tuple / ADT / function type 检查。
	- **DESC**: 这是 value type 的主推断层，但不做 method/operator 最终选择，不做 monomorphization，不做 escape。row 必须 canonical：字段稳定 id、稳定排序、hash 不依赖编译顺序。这里同时固定 tuple 的 positional row 字段 `_1`, `_2`, ...，以及不可变 `Array[T]` 的基础类型行为。`TypeRef` / `RowShape` / `DynRowContract` 在这里预留 `UsageColor`、`SendColor`、`ShapeId` 槽位，但允许先是 unknown/obligation。
	- **验收**: 能输出 `TypedAst` + `ConstraintSet` + canonical row/shape ids + usage/send/color slots；错误包括普通类型不匹配、缺字段、record update 不合法、非法 let-generalization。
	- **并行**: 函数体级并行；type variable id 分配使用 per-task arena，合并时重编号或使用 scoped id。

- [ ] **Pass 07: Answer / Continuation Kind Check**
	- **TODO**: 检查 `reset` / `shift` answer type、implicit reset、continuation kind、resume count contract、multi-resume replay-safety。
	- **DESC**: language-level continuation 是 day-0。continuation kind 至少区分 linear 与 multi-resume；multi-resume 用于 lexer/parser/compiler backtracking 与 recovery。跨 world/thread 的 capture 或 resume 永远非法。multi-resume 只能捕获 replay-safe state，或捕获由 reset-local rollback region 管理的局部可回滚状态。
	- **验收**: simple reset/shift、nested reset、multi-resume parser alternative 通过；answer type mismatch、multi-resume 捕获 FFI/UnsafeRef/world-local/Atomic mutation、跨 world/thread continuation 都能报错。
	- **并行**: 函数体级并行；跨函数 continuation escape 摘要按 call graph SCC 调度。

- [ ] **Pass 08: World / Send / Escape / Capability Check**
	- **TODO**: 检查 `return`、`break`、`continue`、loop tag、tail position、escape/promotion、`send`/`!send`、world boundary、thread boundary、world-local、`UnsafeRef`、Atomic。
	- **DESC**: 这里把 continuation 与并发/世界边界彻底隔开：continuation 默认不能 send，不能跨 thread/world；ordinary closure 的 send 分类与 captured values 相关；escape 规则先保守，允许后续放宽。这里产出 `SendColor`、`EscapeColor`、`Arena/ResetBoundary`、world/thread legality、dyn package escape facts。
	- **验收**: 非法 escape、非法 send/capture、错误 loop tag、world-local 泄漏、Atomic capability 错误都能报错；TypedAst 标注 tail-call sites、arena/promotion facts、send/escape colors。
	- **并行**: 函数体级并行；跨函数 escape 摘要需要 fixpoint，按 call graph SCC 并行调度。

- [ ] **Pass 09: Usage Analysis 0: High-Level Core**
	- **TODO**: 在 typed/alpha core 上统计 binder、lambda、closure、continuation 的 `0 | 1 | many` 使用情况，并标记 escaping、immediate-call、multi-resume 需求。
	- **DESC**: 这是 CPS 和 closure 前的必需优化事实，不是后端优化。没有 usage facts，就无法安全地区分 single-use continuation、真正 multi-resume continuation、可 direct call lambda、必须实体化 closure。这里把 `0 | 1 | many` 归约为初步 `UsageColor`：single-use 可为 `1`，multi-resume / shared storage / dynamic package aliasing 倾向 `N`。
	- **验收**: 未使用 binder/lambda 可诊断；immediately-called lambda 标记为 directification candidate；parser alternative 中多次 resume 的 continuation 标记为 many；usage-colored typed dump 可显示关键 binder/type occurrence。
	- **并行**: 函数体级并行；只读 typed/alpha core。

- [ ] **Pass 10: Generic Definition Check**
	- **TODO**: 在抽象 generic 参数下检查 generic body，生成 `GenericBodyIR` 和 `ObligationIR`，包括 field/method/operator/shape-dispatch/answer-type/generic-continuation 限制。
	- **DESC**: 这是 checked template 的核心。generic 定义期不是黑盒：普通 HM、row 约束、基本 well-formedness、answer type 入口都必须先过。不能等实例化时才发现 body 本身不可类型化。row/dynamic 参数统一进入 obligation：静态 `T: {r | ...}`、参数位置 `dyn {r | ...}`、以及 boxed/unboxed 可接受性都必须在 `ObligationIR` 中可表示。
	- **验收**: 能拒绝与任何实例无关的 generic body 错误；能保存未兑现 structural obligations；错误消息指向定义点；obligation 可序列化、可 hash、可缓存；`def f[T:{r|x:A}](v:T)` 可同时记录 static row 与 dyn package instantiation 路径。
	- **并行**: generic definition 级并行；只读 summaries 和 resolved/canonical type info。

- [ ] **Pass 11: Method / Operator / Dispatch Index**
	- **TODO**: 建立 nominal method index、operator index、shape-dispatch candidate index，并提供按 `(name/operator, nominal id, normalized shape)` 查询的缓存接口。
	- **DESC**: level-1 默认 method resolution 基于 nominal identity，并保留分层候选顺序：field-callable、receiver method、qualified callee。shape dispatch 是独立 structural obligation。`via namespace` 若存在，应放到未来 level-2 的显式行为来源选择，不作为这里的主线机制。这里不做全局 witness search。
	- **验收**: 候选筛选稳定；二义性可诊断；同 shape 不同 nominal type 不被错误合并；查询缓存 key 可复现。
	- **并行**: index 构建可按 namespace 并行后 merge；查询缓存可并发读，写入用 content-addressed key 去重。

- [ ] **Pass 12: One-Pass CPS Transformation + Beta Reduction**
	- **TODO**: 用 meta-continuation 做 one-pass CBV CPS lowering，并在变换过程中 beta-reduce administrative redex；不得先生成朴素 CPS 再靠后续 pass 清理。
	- **DESC**: 这是 level-1 中端核心。算法必须区分 object-level 与 meta-level：object-level lambda/app 是生成后运行时仍存在的 CPS 代码；meta-level continuation 是编译器内部函数，例如 `T(expr, k_meta)` 中的 `k_meta: Value -> Term`，变换结束后必须通过宿主语言求值消失。level0 已有参考实现：`level0/src/backend/cir/lower.chiba` 的 `lower_expr(expr, k: (Val) => CpsExpr, ...)`，文件头明确标注 “one-pass CPS + beta-reduction”；level-1R/Rust reference 与 level-1b 迁移都应以这个纪律为基线，而不是迁回目标语言 lambda redex。
	- **DESC**: 普通表达式求值顺序产生的 administrative continuation 默认不实体化；只有 call/switch/prompt/control/multi-resume 等语义控制点进入 CPS Core。这个 pass 支撑 compiler lowering、chibalex backtracking、chibacc recovery 的共同实现模型。CPS 节点必须保留 continuation kind、answer type、usage/send/escape facts，不得把颜色丢到后端重推。
	- **规则**:
		- 变量 / atom：`T[x](k_meta) = k_meta(x)`，不产生 `LetCont` / object lambda / runtime call。
		- lambda：生成 object-level 函数值，函数体内部用新的 meta-continuation 把结果传给 object-level continuation 参数。
		- call：先用 meta-continuation 求 callee，再求参数，最后只 materialize 真实 object-level call 与必要 object-level continuation。
		- 多参数 call / tuple / record / ADT ctor / operator args 必须按 CBV 左到右，用嵌套 meta-continuation 串起来。
		- `if` / `match` / branch join 只在真实 join 需要时 materialize continuation；不能为 atom/变量分支制造行政 redex。
		- `reset` / `shift` / `resetn` 是真实控制边界，必须 materialize 清晰 CPS 节点并携带 answer type、ContinuationKind、UsageColor。
	- **反例**: `f(x)` 不允许先生成 `(\a. (\b. a b k) x) f` 这种 object-level administrative beta-redex；one-pass 输出应直接接近 `f x (\w. k w)`，其中 `(\w. k w)` 只有在必须把 meta-continuation reify 成 runtime continuation 时才出现。
	- **验收**: atom lowering 不产生额外 continuation；`f(x)` / `f(g(x), y)` / operator call / constructor call 的 CPS dump 不含 administrative `LetCont+AppCont` 链；single-use administrative continuation 被 meta-level beta 直接消掉；`reset` / `shift` 和 multi-resume continuation 在 CPS dump 中保留清晰语义节点和颜色事实。
	- **并行**: 函数体级并行；只读 typed/answer-control/usage facts。

- [ ] **Pass 13: Usage Analysis 1: CPS Core**
	- **TODO**: 在 CPS Core 上重新统计 continuation、lambda、closure、function value 的 `0 | 1 | many` 使用情况。
	- **DESC**: CPS 后会产生新的可消除结构，也会暴露真正需要实体化的 multi-resume continuation。这个 pass 给 continuation simplification 和 closure conversion 提供最终事实。
	- **验收**: dead continuation、single-use continuation、many-resume continuation 分类稳定；分类结果可 dump、可 golden test。
	- **并行**: 函数体级并行。

- [ ] **Pass 14: Continuation Simplification**
	- **TODO**: 删除 unused continuation，inline single-use continuation，把 many-use continuation 标记为 multi-resume package lowering 输入。
	- **DESC**: 这是 day-0 性能卫生 pass。multi-resume 是语言能力，但只有真实 many-resume continuation 才付实体化成本；cross-world/thread continuation 在这里应已不存在，若残留则 internal error。
	- **验收**: single-use continuation 不分配 runtime object；multi-resume parser continuation 可重复 resume；非法 continuation 不进入 closure/lower 阶段。
	- **并行**: 函数体级并行。

- [ ] **Pass 15: Closure Conversion**
	- **TODO**: 做 free-var analysis，把 escaping lambda/closure 转为 `{code, env}`，并区分 direct function、known callee、unknown closure callee。
	- **DESC**: closure conversion 解决 lexical binding 到 runtime env 的边界。no-capture lambda 可以直接化；escaping closure 才需要 wasm-gc env package；continuation package 与 closure env 要共享 capture legality facts。env field 必须携带 `UsageColor`、`SendColor`、`EscapeColor`、`ReplaySafety`，以支持 env shrinking、`Rc <-> N` 审计、dyn package capture legality。
	- **验收**: top-level/nested/direct/unknown callee 路径可 dump；free vars 顺序稳定；no-capture lambda 不分配 env；closure env dump 能显示每个 field 的 usage/send/escape/replay colors。
	- **并行**: 函数体级并行；跨函数 lifted symbol 由确定性 allocator 分配。

- [ ] **Pass 16: Lambda Lifting**
	- **TODO**: 把 nested function 提升为稳定 function symbol，把捕获变量变为 env 或显式参数。
	- **DESC**: 后端不处理词法嵌套函数语义。lifting 后的函数边界必须适合 wasm `funcref`、tailcall 和 namespace/specialization bundle emit。
	- **验收**: 互递归 nested function、捕获参数、直接调用/间接调用样例 lowering 稳定；lifted symbol name/id 可复现。
	- **并行**: 函数体级并行；namespace 内 lifted symbol merge 需要确定性排序。

- [ ] **Pass 17: Usage Analysis 2: Closure Core**
	- **TODO**: 在 closure conversion/lambda lifting 后统计 closure package、env field、code pointer、continuation package 的使用情况。
	- **DESC**: conversion 后还会出现可消除的 package 和 dead capture。这个 pass 支撑 env shrinking、single-use closure directification、known callee direct call。
	- **验收**: dead capture field、single-use closure、known code pointer、unused continuation package 都能被标记。
	- **并行**: 函数体级并行。

- [ ] **Pass 18: Closure / Env Simplification**
	- **TODO**: 做 env shrinking、no-capture closure erasure、single-use closure directification、dead capture field elimination、known callee direct call。
	- **DESC**: 这是避免 wasm-gc 后端背负无谓 allocation 的最后一道中端卫生线。它不是高级全局 inline，只消除由语言 lowering 必然产生、且 usage facts 已证明可消除的结构。
	- **验收**: no-capture closure 不生成 env；dead capture 不进入 layout；single-use closure 变 direct call；tail position 不被破坏。
	- **并行**: 函数体级并行。

- [ ] **Pass 19: Monomorphization Scheduler**
	- **TODO**: 收集 concrete instantiation sites，生成 specialization work items，按 key 去重并并行调度实例化检查和代码生成。
	- **DESC**: monomorphize key 建议为 `(generic_symbol, concrete_type_tuple, normalized_shape_tuple, builtin_capability_facts, usage_facts, send_facts, continuation_facts, dyn_contract_facts, abi_mode)`。若未来 level-2 恢复显式 `via namespace` 行为来源，再把它作为扩展 key 维度加入。定义期 checked 过的 body 不重做全量 HM，只兑现 concrete obligations：字段、method、operator、shape dispatch、dyn adapter packaging、continuation capability facts。
	- **验收**: 同一 key 只实例化一次；不同 namespace 同时请求同一实例不会重复产物；实例化错误报在 call site，同时保留定义点 note；递归 generic 通过 in-progress marker / SCC worklist 处理；同一 static row 和 dyn row package 的实例化 key 能区分但共享 definition-time body facts。
	- **并行**: 高并行。每个 specialization 是独立任务；全局只允许原子注册 `key -> artifact/status`，禁止实例化任务修改共享语义表。

- [ ] **Pass 20: Core Lower + Ownership/Layout Decision**
	- **TODO**: 把 optimized CPS/closure core / specialization 降到 target-neutral `CoreIR`，完成 method/operator direct target、extern direct target、closure env、continuation frame/package、record/data/union/string/slice/Array layout、dyn row package、ownership lowering decision。
	- **DESC**: 这里决定语言级 layout 与 ownership：`StackValue | InplaceReuse | RC | ARC | StaticData | BorrowedView | DynPackage`。当前 wasm-gc backend 可以把这些 decision materialize 为 wasm-gc struct/array/funcref/eqref；未来 no-GC wasm backend 可以 materialize 为 linear-memory block + RC/ARC runtime helpers。二者只能消费同一套 CoreIR 事实，不能改变 type rules。`1` 且非 escaping 走 stack/inplace/direct；`N` 且 `!send` 走 RC；`N` 且 `send` 走 ARC；唯一更新路径允许 Perceus / functional-but-inplace / Koka-style reuse。`extern "wasi" "symbol"` 降为 target-neutral import fact；`extern "C"` / `extern "c"` 降为 target-neutral env import fact。后端 emitter 不再做语义选择。
	- **验收**: layout hash 稳定；同一 summary + same specialization key 生成同一 CoreIR；tail position、continuation package、dyn row package、shape adapter、ownership decision 可 dump；static row access 与 dyn row adapter access 在 CoreIR 中可区分；CoreIR 不含 Wasm-GC-only 类型名，但可在 backend-specific layout manifest 中映射到 wasm-gc 表示。
	- **并行**: namespace + specialization 级并行；layout table 由 canonical type/layout key 去重。

- [ ] **Pass 21: CoreIR Validation**
	- **TODO**: 验证 CoreIR 的 symbol/type/layout/block/continuation frame/package/dyn package/ownership decision 引用完整，以及 replay-safety facts 已兑现。
	- **DESC**: validator 是 backend dumbness 的护栏。它必须确认 multi-resume continuation package 不包含非法 world/thread/unsafe capture，tailcall sites 合法，layout refs 完整，`send` 对象不走非原子 RC，wasm-gc / future no-GC wasm backend 都不需要重新推语义。
	- **验收**: CoreIR validator 能拒绝 dangling symbol、错误 layout ref、非法 continuation package、非法 tailcall、错误 RC/ARC decision、Wasm-GC-only 语义泄漏；错误信息稳定。
	- **并行**: namespace + specialization 级并行。

- [ ] **Pass 22: Backend Emit + Link**
	- **TODO**: 当前主线从已验证 CoreIR 打印 wasm-gc `.wat` / wasm object；未来新增 no-GC wasm backend，从同一 CoreIR 打印 linear-memory + RC/ARC runtime helper 版本。序列化 WASI/env imports、tailcall 或等价 trampoline；每个 namespace / specialization bundle 可独立产物，最后交给 `wasm-ld` 或等价 linker 链接。
	- **DESC**: emitter 保持 dumb：只序列化已验证 CoreIR，不做优化、不做重型验证。跨 namespace linking 交给 linker，但符号、ABI、layout、ownership decision 在前面 pass 已固定。当前 wasm-gc backend 根据 CoreIR ownership/layout facts 生成 wasm-gc 表示；未来 no-GC wasm backend 根据同一 facts 生成 stack/inplace/RC/ARC code。`send` 的 ARC 选择不能在 emitter 临时决定。
	- **验收**: 当前 wasm-gc target 单 namespace 可独立 emit，多 namespace 能链接；tail position 生成 `return_call` / 等价 tailcall/trampoline；WASI/thread import/export 名称稳定；未来 no-GC wasm target 的 RC/ARC runtime helper 名称稳定；`extern "wasi" "fd_write"` 生成的 WASI import 名称和签名可 golden test；`extern "C" "js_log"` 生成的 `env` import 名称和签名可 golden test；wat roundtrip 工具能解析；manifest 可从 final symbol 反查 source/debug/lowering lineage。
	- **并行**: namespace / object 级并行 emit；最终 linker 是收敛点。

## Monomorphize + Checked Generics 处理规则

- [ ] **定义期检查一次**
	- **TODO**: generic body 在抽象参数下完整通过基础类型检查，生成 `GenericBodyIR` 和 `ObligationIR`。
	- **DESC**: 类似 C++ template 的实例化生成代码，但不是 C++ 老模板的“定义期几乎不检查”。定义期必须拒绝不依赖 concrete type 的错误。
	- **验收**: `def f[T](x: T) = y` 在没有 `y` 的情况下定义期报错；`def f[T](x: T) = x.m()` 定义期通过并记录 method obligation。
	- **并行**: generic definition 级并行。

- [ ] **实例化期兑现 concrete obligation**
	- **TODO**: concrete call site 触发 specialization，检查 row field、method/operator、shape-dispatch、dyn adapter、builtin capability、continuation capability facts。
	- **DESC**: obligation 尽量局部兑现，不引入 Rust trait solver 或全局 witness search。level-1 主线先只兑现 row/shape/method/operator/dyn adapter/builtin capability obligations；named constraint 和 `via` 行为来源若恢复，应明确作为 level-2 扩展加入。
	- **验收**: `f[User](u)` 若 `User` 没有所需字段/方法，在 call site 报错；若满足则生成或复用 specialization。
	- **并行**: specialization work item 级并行。

- [ ] **specialization key 稳定化**
	- **TODO**: 定义 key 编码和 hash：`generic id + concrete nominal ids + canonical type args + normalized shape ids + capability facts + continuation facts + ABI/layout mode`。
	- **DESC**: normalized shape 必须 canonical，不能依赖源码字段顺序或编译顺序。名义类型默认进入 key，避免同 shape 不同 nominal 的 method 世界被合并。当前 level-1 key 不默认包含 `explicit via`；那一维只在未来 level-2 引入显式行为来源时追加。
	- **验收**: `{x, y}` 与 `{y, x}` shape key 相同；同 shape 不同 nominal type 默认 key 不同；continuation capability facts 改变时 key 可区分；未来若引入显式 `via ns`，该来源必须改变 key。
	- **并行**: key 计算纯函数，可任意并行。

- [ ] **并发实例化注册表**
	- **TODO**: 实现 `InstantiationRegistry`：`Missing | InProgress | Done | Failed`，支持多个 worker 请求同一 key 时 join 已有任务。
	- **DESC**: 这是 monomorphize 多核编译的关键。实例化任务只追加产物，不修改全局语义环境。
	- **验收**: 压测 100 个 namespace 同时请求同一 generic instance，只生成一个 artifact；失败结果可缓存并稳定报告。
	- **并行**: 高并行，注册表需要线程安全或进程安全锁。

- [ ] **递归 generic / SCC 策略**
	- **TODO**: 对实例化依赖图做 SCC；递归实例先注册 stub，再完成 body lower，禁止无限展开。
	- **DESC**: monomorphize 不能无界实例化。需要限制递归实例深度或要求 key 收敛。
	- **验收**: 直接递归 generic 生成一个 specialization；无限类型增长实例化能报“monomorphization does not converge”。
	- **并行**: SCC 间并行，SCC 内确定性顺序或协作调度。

- [ ] **增量缓存**
	- **TODO**: 缓存 `InterfaceSummary`、`GenericBodyIR`、`ObligationIR`、specialization artifact、CoreIR、wat/object。
	- **DESC**: cache key 必须包含 source hash、summary hash、compiler version、target backend (`wasm-nogc` / optional `wasm-gc` / future native)、target features (`tailcall`, `wasi`, `thread`)、ownership runtime mode (`stack/inplace/rc/arc helpers`) 和 canonical extern ABI/import module/signature hash，例如 `wasi::fd_write` 与 `env::js_log` 必须区分，`"C"` / `"c"` 必须归一到同一 key。
	- **验收**: 修改非导出函数体只重编当前 namespace 和受影响 specialization；修改 public summary 触发依赖 namespace 重检。
	- **并行**: cache lookup 全并行，cache write content-addressed。

## 并行编译分界

- [ ] **全局轻量串行/归约区**
	- **TODO**: 只允许 ProjectSurface merge、summary conflict reduce、namespace graph/SCC 构建、最终 wasm-ld 处于收敛点。
	- **验收**: 这些阶段耗时应远低于 body typecheck + specialization + emit。

- [ ] **namespace 并行区**
	- **TODO**: NameResolve、Alpha、PatternElab、HM+Row、Answer/ContinuationKind、World/Send/Escape/Capability、Usage、OnePassCPS、ContinuationSimplify、Closure/Lambda、GenericDefinitionCheck、CoreLower、WatEmit 都按 namespace 或 body 分发。
	- **验收**: 在 8 核机器上，多 namespace 项目 CPU 利用率明显高于单核；输出顺序仍确定。

- [ ] **specialization 并行区**
	- **TODO**: MonomorphizationScheduler 把每个 unique key 作为独立任务执行，任务只读 summaries/index/cache，只写自己的 artifact。
	- **验收**: generic-heavy 项目能在多核下并发实例化；重复实例去重。
