这两个目录是 level-1 的 spec 目录
- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec`
- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/type_system`


## Second Bootstrap: remaining work only

这份文件现在只保留**未完成项**。

已完成的 C00-C10、库边界冻结、语义/CPS/closure 主干落地、以及对应 gate 建设，统一视为历史完成项，不再继续堆在这里干扰判断。详细过程保留在 git 历史、`TODO.checkpoint.md` 和各 gate / runner 中。

## 当前判断

- 当前必须修正前一版乐观判断：**level-1b 当前处于不可用状态**。它的 control / continuation / closure / backend 多数仍是 contract stub 或注释型 emitter，不具备作为 primary compiler behavior 的资格。
- `checkpoint:gates` / `level1b:c11-backend` / `level1b:cir-migration` 已绿只能说明映射、接口和部分 smoke 覆盖成立；不能说明 level-1b 已经能承载真实 continuation / closure / Wasm-GC backend 语义。
- 当前主线应改为“**先清理 level-1b 过时代码与假成功路径 + 固定 continuation/closure/callable 语义 + 再恢复 frontend migration 与 C12 自举推进**”。
- 当前离 checkpoint 完成大约还差：
	- **1 轮 spec alignment audit**：语言类工作先读取 spec repo 中 continuation / callable / closure / send / CIR-BIR placement / frontend migration 规则，列出 spec 硬规则与 level-1b 实现差距；之后才能开始改 pass/gate，避免把 stub 清理成“更整齐的错实现”。
	- **1 轮 level-1b truthfulness cleanup**：把 `Ok(module)` / 空 facts / 注释 WAT emitter / contract-only pass 从“看似已实现”改成显式 stub、真实 gate 或删除，避免继续误导路线判断。
	- **1 轮 continuation/closure 语义落地**：`Cont1` / `ContN`、boxed one-shot state machine、erased callable ADT、`Ref[T]` shared-reference capture、one-shot continuation 与 no-capture closure 的必需优化。
	- **1 轮未门禁语义收尾**：globals / `Self` with generics / ADT tuple bridge / compiler intrinsic surface / deep pattern lowering runtime / pipe matrix / scope shadowing / debugability / namespace ownership。
- 当前 **`validate:first-bootstrap` 已全绿**：parser compare / parser error smoke / semantic gates / checkpoint gates / all-wat / level1c.wasm smoke 均已通过，说明 first-bootstrap 主链路现阶段已经打通。
- 当前自举后的主要剩余压力转为：
	- **level-1b 不可用状态修复**：control / continuation / closure / backend 的 contract-only 代码必须先清理并替换成真实 lowering 或明确 blocker。
	- **frontend migration builtin/oracle 债务**：`std.regex` / `std.chibalex` / `std.chibacc` 还需要从“gate 已成立”继续推进到“self-host truly primary”，但不能继续假设当前 level-1b backend 已可承载真实 continuation。
	- **C11 primary-path / C12 second bootstrap**：first-bootstrap 过关不等于 second bootstrap 完成，后续重点是 `level1c-next` / `level1c-next2`。
- 当前离 Second Bootstrap 完成大约还差：
	- **C11 真正迁移清零**：虽然 `level1b:c11-backend` 与 `level1b:cir-migration` 已绿，但 level-1b backend 当前仍不可用；必须先清掉 contract-only / stub-only / comment-only path，再让旧 `src/backend/cir` 不再承担核心语义路径。
	- **C12 两轮 bootstrap 对拍**：`level1c.wasm -> level1c-next -> level1c-next2`。
	- **post-C12 仓库形态收口**：Second Bootstrap 验证完成后，仓库目标不是长期并存多套 compiler 主实现，而是让 `level-1b/` 成为唯一 primary compiler tree；`level0/` 与 legacy `src/` / `level1c` 路径直接删除（Git 历史保留追溯），不再与主实现并列演化。

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
	- `Ctor(...) <-> tuple-tagged-form` 是语言级 contract；
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
	- `shift k { ... }` 捕获 `Cont1`；`shiftn k { ... }` 捕获 `ContN`；带 tag 时分别写 `shift :tag k { ... }` / `shiftn :tag k { ... }`。
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

## 工具链原则更新

- `chibac.wasm` 仍必须是用户可直接用 `wasmtime chibac.wasm -- ...` 执行的 WASI/Wasm-GC compiler。
- Node runner / JS harness 仍然只是开发和 CI 便利层，不是运行时语义前提。
- **Binaryen 不是当前要去除的依赖。**
	- Binaryen CLI / `binaryen.js` / 仓库内携带的 `binaryen-linux-x86-64-version_129/` 都可以作为可接受的组装、验证、优化和分发工具链组成部分。
	- 当前目标不是“去掉 Binaryen”，而是“不要把 Binaryen 混进 Chiba backend 语义本体”。
	- 换句话说：**可以依赖 Binaryen 做 `.wat -> .wasm`、validate、opt、roundtrip；不能依赖 Binaryen 替 Chiba 偷做 unresolved semantic hole。**

## 剩余 checkpoint 收口

### checkpoint checklist：剩余语言/语义面收口

这些项里有些是“补实现”，有些是“补 fixture / 补 gate / 补 compiler-side lowering 验收”。目标不是把所有条目都重新发明一遍，而是把它们全部收进稳定 gate。

- [ ] operator overloading
	- **目标**:
		- `+ - * /` 等常规 operator；
		- `[x]` / `[x..y]` 对应 `op_index` / `op_index_slice`；
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
	- **当前已知前沿**:
		- 当前 `semantic gates` 已通过；
		- 最近补的是 gate 对内部 binder/ref 表示的去脆弱化，而不是放宽 pipe 语义本身；
		- 后续若 pipe 再出问题，应优先按“语义错了”与“harness 绑死内部细节”分开诊断。

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
	- **已确认**:
		- 这是正式语言特性，不只是 desugar 约定。


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
	- **备注**:
		- 这项不是要立刻取消 mangling，而是要让 mangling 变成“可逆、可观察、可定位”；
		- gdb/backtrace、nanopass dump、WAT emit、validation err 至少要能共享同一套 symbol 追踪信息。
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


## Second Bootstrap 剩余项

### C11: wasm-gc Core/backend rewrite 收口

- [ ] level-1b 不可用状态修复 / truthfulness cleanup
	- **现状**: `level-1b/compiler/control/*`、`level-1b/compiler/closure/*`、`level-1b/compiler/backend/*` 中存在大量 contract-only / stub-only 实现：空 facts、直接 `Ok(module)`、只生成 layout 壳、WAT emitter 只输出注释等。
	- **前置**: 语言类 cleanup 必须先对齐 spec repo，尤其 continuation / callable / closure / send / CIR-BIR placement。truthfulness cleanup 不是单纯搜索替换 stub，而是把实现缺口按 spec 硬规则分类。
	- **目标**: level-1b 不再让“接口已存在”伪装成“语义已实现”。所有暂未实现 pass 必须显式 blocker/stub；所有 gate 必须区分 contract smoke 与真实语义验收。
	- **第一轮 audit 清单**:
		- 新增轻量反馈环：`vp run level1b:truthfulness-audit`。该 gate 当前预期失败，用 blocker taxonomy 暴露假成功路径；不要把它并入 expensive bootstrap。
		- 当前基线计数：`legacy-compiler-execution` / `oracle-success-path` 已清零；剩余 `primary-path-blocked` 26；`ok-module-pass-through` / `empty-facts` / `comment-only-backend-output` 已清零为显式 blocker 或 source-contract 拒绝。
		- `level1b:c09-control-cps` 已改成 fail-closed：缺 answer/control scan、usage subject collection、boundary scan、one-pass CPS beta lowering 时返回 blocker diagnostic；该 gate 当前只通过 source contract，真实 valid/invalid continuation gates 标记为 primary-path-blocked。
		- `level1b:c10-closure-package` 已保留 continuation/lambda/closure subject kind，并区分 escaped boxed `Cont1` 与 repeatable `ContN` package；缺 continuation capture extraction 时 fail-closed；该 gate 当前只通过 source contract，closure/directification/nanopass 验收标记为 primary-path-blocked。
		- `level1b:c11-backend` 现已在 source contract 阶段明确拒绝：comment-only `emit_core_op` / fake WAT backend。
		- spec 要求 answer type checking 在 level-1 / CIR 层完成；当前 `check_answer_control` 已改为 fail-closed blocker，尚未生成真实 answer/control facts。
		- spec 要求 continuation boundary / replay / usage 保留 control boundary、answer type、arena/world legality 与 usage；当前 boundary / usage 已改为 fail-closed blocker，replay 仍只有轻量事实壳。
		- one-pass CPS 当前只是 `CpsModule(module)` wrapper，没有实际 CPS transform 或 administrative beta-reduction。
		- 旧 `src/backend/cir/*` 与 level-0 可以作为 legacy reference 借鉴算法、fixture、失败模式；但每次借鉴都必须先过 spec alignment，且 level-1b 重新拥有行为，不能形成 legacy dependency。
		- 旧 `src/backend/cir/cps.chiba` 也只是 L5 wrapper / synthetic continuation package 方向，不是可直接搬运的 spec 级 one-pass CPS + beta 实现。
		- CPS usage 当前把 usage facts 全部转成 `UseSubjectBinder`，会丢 continuation / lambda / closure subject kind。
		- spec 要求 `shift` 捕获 `Cont1`、`shiftn` 捕获 `ContN`，逃逸 `Cont1` boxed 且不升级；当前 control/closure 只按 UseZero/UseOne/UseMany 做壳级 decision，未承载 `Cont1` / `ContN` storage 语义。
		- 已新增 target-independent IR contract：`ContinuationCont1` / `ContinuationContN`、`ContinuationBoxedOneShot`、`ContinuationRepeatableFrameChain`、`ContinuationFact`、sendable callable exclusion hook。
		- continuation package 当前由 `UseMany` 直接驱动，未区分 `Cont1` escaped boxed 与 `ContN` repeatable package。
		- closure conversion 当前对 packaged continuation 生成 empty capture fields，未抽取真实 capture set。
		- 已新增 closure/backend contract：`StacklessResumeFunction`、`ContinuationFrame`、`ContinuationLowerBoxedCont1`、`ContinuationLowerRepeatableContN`、`CoreOpStacklessFunction`、`CoreOpContinuationFrameChain`、`CoreOpContNPackage`。
		- C10/C11 source path 已把 `ContinuationSimplification` 转成 `ContinuationLoweringFact` 并贯穿到 backend Core op lowering；boxed `Cont1` 保留 consumed-state，`ContN` lower 为 frame-chain op + repeatable package op，但真实 capture extraction / frame body / WAT emission 仍是 blocker。
		- C10 capture model 已区分 `CaptureSharedRefCell`，并把 materialized continuation blocker 拆成 boxed `Cont1` capture extraction 与 `ContN` frame-chain capture extraction；这只提高 fail-closed 诊断精度，尚未实现真实 capture set / frame body 抽取。
		- C11 已修正 erased callable continuation package layout：`ContinuationLowerErasedCallableVariant` 不再复用 closure env layout，而是进入独立 `LayoutContinuationPackage`（tag + payload）；真实 callable ADT dispatch / WAT emission 仍是 blocker。
		- C11 validator 已补 `ContN` ordering invariant：`CoreOpContNPackage` 必须在已见 `CoreOpContinuationFrameChain` 后出现；这防止其他 lowering path 绕过 repeatable frame-chain 语义。
		- `ContN` lowering 验收必须证明：repeatable frame chain 由 stackless resume functions 驱动；frame chain 可重复恢复；捕获 `Ref[T]` 是 shared-reference，不 snapshot / rollback。
		- closure lowering 验收必须证明：no-capture closure 走 direct function / funref / inline；capturing closure 只有逃逸或确需 env 时才 materialize env；env 内 continuation / `Ref[T]` 不被能力洗白。
		- spec 要求 `(A) -> B` 参数位置是 checked-template callable obligation，存储位置 lower 成 erased callable ADT；显式 `cont1` / `contN` storage 不走 erased callable ADT；当前未见真实 callable storage lowering。
		- frontend grammar source 已补 `cont1 (A) -> B`、`contN (A) -> B`、`shiftn` contract 与 chibalex/chibacc mini fixtures；生成版 lexer/parser 已刷新，`shift :tag` / `shiftn :tag` label parse 已对齐 lexer 的 `Colon Ident` tokenization，且 AST 保留 tag 名；native chibalex oracle 尚未覆盖 continuation surface keywords。
		- spec 要求 `((A) -> B) send` 排除 continuation 与 `!send` closure；当前 send/capability 与 callable storage / continuation 没有真实集成。
		- spec 要求 no-capture closure 不分配 env，capturing closure 只有需要时 materialize env；当前 closure env layout 由 continuation packaging decision 壳生成，capture extraction 仍缺。
		- spec 要求 CIR 承载 type / usage / send / arena / answer-type 语义，BIR materialize frame/prompt/control；当前 backend WAT emitter 已不再 comment-only 成功，但真实 Wasm-GC / continuation frame / closure env 发射仍是 missing-backend-layout blocker。
	- **完成标准**:
		- `check_answer_control` / usage / boundary / replay-safety / CPS / closure conversion / backend emit 不再以空 facts 或 pass-through 伪装完成；
		- continuation lowering 能区分 `Cont1` direct、boxed `Cont1`、`ContN` package；
		- closure lowering 能保证 no-capture closure 不分配 env，capturing closure 才 materialize env；
		- parser / grammar 支持 `shift` / `shiftn` 与 `cont1 (A) -> B` / `contN (A) -> B` 类型糖；
		- 已新增 grammar 契约源文件：`chiba-level1-grammar-spec/32-test.chiba` 与 `chiba-level1-grammar-error-spec/111-test.chiba`、`112-test.chiba`、`113-test.chiba`；当前 `level1c.o parse` 已支持 continuation keyword/type grammar，后续仍需 level-1b 自举主路径接管这些 fixtures；
		- `(A) -> B` storage lower 成 erased callable ADT，并支持 function / closure / boxed `Cont1` / `ContN` dispatch；
		- 显式 `cont1 (A) -> B` / `contN (A) -> B` storage 分别走 continuation-specific storage，不走 erased callable ADT；
		- sendable callable storage 排除 continuation 和 `!send` closure；
		- CIR 不含 Wasm-GC / WAT / Binaryen / target ABI 细节；这些只允许出现在 BIR/LIR/backend layout 层；
		- WAT/Core emitter 不再只输出注释型 core-op，而是能承载真实 Wasm-GC / continuation frame / closure env 语义。

- [ ] 完成 `src/backend/cir` 迁移清零
	- **现状**: `level1b:c11-backend` 与 `level1b:cir-migration` 已通过，说明 gate 和映射覆盖已打通；但 level-1b 当前仍不可用，剩余问题不是“身份收尾”，而是“真实语义与真实 backend 尚未落地”。
	- **目标**: `compiler/MIGRATION.md` 中旧 pass 不再停留在 `missing rewrite` / `contract only`；C08-C11 gate 不再把旧 `src/backend/cir` 当 primary behavior。
	- **完成标准**:
		- level-1b backend 成为主路径；
		- 旧 `src/backend/cir` 最多保留 oracle / diff 参考，不再主导成功路径；
		- `level1b:cir-migration` 变成真正的“已清零”而不是“映射已建立”。

- [ ] 收掉 frontend migration builtin/oracle 债务
	- **目标**:
		- `std.regex` 不再把 parser/compiler/matcher 的关键能力留在 builtin/oracle；
		- `std.chibalex` 不再只停在 parser/lowering/engine/codegen contract + mini oracle；
		- `std.chibacc` 不再只停在 grammar IR / recovery / codegen contract + mini oracle。
		- level-1b 完成自举后，所有 key compiler capability 的 oracle dependency 必须移除；oracle 只能保留为 fixture / diff / expected-output 对拍工具。
	- **备注**: 这不是要推翻已完成的 C04-C06，而是把它们从“gate 已建立、主干已在”推进到“self-host 真正 primary”。

- [ ] 让 level-1b backend 稳定生成 `level1c-next.wat`
	- **完成标准**:
		- Core validator 能稳定拒绝 dangling symbol / layout hole / illegal tailcall / illegal continuation package；
		- `level1c-next.wat` 可由 Binaryen v129 组装、validate、opt、run；
		- 生成产物不要求 Node-only host import。

### C12: Second Bootstrap validation

- [ ] 完成两轮 bootstrap 对拍
	- **流程**:
		- `level1c.wasm -> level1c-next.wat -> level1c-next.wasm`
		- `level1c-next.wasm -> level1c-next2.wat -> level1c-next2.wasm`
	- **前置条件**:
		- `level1b:cir-migration` 已清零；
		- C11 backend 主路径稳定；
		- `std.regex` / `std.chibalex` / `std.chibacc` 不再依赖关键 builtin/oracle 才能成功。
	- **验收**:
		- `level1c-next` / `level1c-next2` 能在 Binaryen v129 + wasmtime 路径上稳定重跑：
			- lexer/parser specs
			- semantic/type gates
			- continuation smoke
			- core/backend smoke
		- Node runner 路径可以保留，但不能是唯一成功路径；
		- 记录 seed / `level1c` / `level1c-next` / `level1c-next2` hash 与 toolchain versions；
		- 关键 IR / WAT diff 若不完全一致，必须有 manifest 解释。

### post-C12: repository cleanup / single-primary-tree 收口

- [ ] 仓库最终形态收口：只保留 `level-1b` 为 primary implementation
	- **目标**:
		- `level-1b/` 成为仓库内唯一继续演化的 primary compiler tree；
		- `level0/` 不再作为并行主实现长期维护；
		- legacy `src/` / `level1c` 路径在完成 second bootstrap 验证后直接删除；
		- 文档、脚本、CI、README、bootstrap manifest 与目录结构一致，不再让新读者误以为仓库长期支持多套同级 compiler 主线。
	- **备注**:
		- 这项发生在 C12 验证完成之后，不与当前“先让 level-1b 成为语义主路径”相混淆；
		- `level0/` 也按同样原则直接删除，追溯依赖 Git 历史而不是仓库内并存目录。

## 当前优先级顺序

1. 先修复 level-1b 不可用状态：清理 contract-only / stub-only / comment-only pass，避免假成功路径继续误导判断。
   - 剩余清理顺序：先断 `legacy-dependency`，再断 `oracle-dependency`；comment-only backend、`Ok(module)` pass-through、empty facts 已完成第一轮 truthfulness 清理。
2. 落地 continuation / closure / callable 语义：`Cont1` / `ContN`、boxed `Cont1` state machine、erased callable ADT、sendable callable storage、one-shot/no-capture 必需优化。
3. 清 frontend migration 的 builtin/oracle 债务，尤其 `std.chibalex` / `std.chibacc`，但必须基于真实 continuation/closure lowering。
4. 收掉 checkpoint checklist 里仍未门禁或未 runtime 化的语义面：ADT tuple bridge、compiler intrinsic surface、globals、`Self` generics、deep pattern / pipe 全矩阵、scope shadowing。
5. 继续完成 `src/backend/cir` 的 primary-path 清零，让 level-1b backend 成为 unquestioned primary path。
6. 让 level-1b backend 稳定生成并跑通 `level1c-next` / `level1c-next2`。
7. 做 C12 两轮 bootstrap 对拍。
8. 做 post-C12 repository cleanup，让 `level-1b/` 成为唯一 primary compiler tree。
