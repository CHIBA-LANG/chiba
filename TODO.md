
## Second Bootstrap: remaining work only

这份文件现在只保留**未完成项**。

已完成的 C00-C10、库边界冻结、语义/CPS/closure 主干落地、以及对应 gate 建设，统一视为历史完成项，不再继续堆在这里干扰判断。详细过程保留在 git 历史、`TODO.checkpoint.md` 和各 gate / runner 中。

## 当前判断

- 当前主线已经不是“架构还没搭起来”，而是“**C11 后端闭环 + C12 自举验证 + 一轮 checkpoint checklist 收口**”。
- 当前离 checkpoint 完成大约还差：
	- **1 个核心后端 blocker**：`std.chibacc` / `codegen_contract` 的 nominal / record / data constructor/value lowering 与 Core symbol synthesis 闭环。
	- **1 轮 checklist 验收与补实现**：methods / operators / generics / globals / ADT tuple bridge / pattern lowering / deep pattern / pipe consistency。
- 当前离 Second Bootstrap 完成大约还差：
	- **C11 真正迁移清零**：level-1b backend 成为 primary behavior，旧 `src/backend/cir` 不再承担核心语义路径。
	- **C12 两轮 bootstrap 对拍**：`level1c.wasm -> level1c-next -> level1c-next2`。

## 工具链原则更新

- `chibac.wasm` 仍必须是用户可直接用 `wasmtime chibac.wasm -- ...` 执行的 WASI/Wasm-GC compiler。
- Node runner / JS harness 仍然只是开发和 CI 便利层，不是运行时语义前提。
- **Binaryen 不是当前要去除的依赖。**
	- Binaryen CLI / `binaryen.js` / 仓库内携带的 `binaryen-linux-x86-64-version_129/` 都可以作为可接受的组装、验证、优化和分发工具链组成部分。
	- 当前目标不是“去掉 Binaryen”，而是“不要把 Binaryen 混进 Chiba backend 语义本体”。
	- 换句话说：**可以依赖 Binaryen 做 `.wat -> .wasm`、validate、opt、roundtrip；不能依赖 Binaryen 替 Chiba 偷做 unresolved semantic hole。**

## 剩余 checkpoint 收口

### 1. `std.chibacc` contract 的后端闭环

- [ ] 打通 `level-1b/supports/chibacc-mini/codegen_contract.chiba` 的 `wat/run`
	- **现状**: `parse` / `check` 已经通过；真正卡点不在 parser，而在 backend/Core lowering。
	- **当前 blocker**:
		- imported nominal / record / data constructor 与 runtime value 的 lowering 仍不完整；
		- `LoweredParser`、`LoweredRule`、`GeneratedParser`、`RecoveryInsert`、`RecoveryNone` 等值层符号在 L8 validated Core 里仍可能落成 `dangling symbol`；
		- `wat` 路径仍可能因 `validated Core required for wat emit` 被拦住。
	- **完成标准**:
		- `level1c.o wat level-1b/supports/chibacc-mini/codegen_contract.chiba` 通过；
		- 生成的 `.wat` 可经 Binaryen v129 构建并运行；
		- `std.chibacc` runtime values 的 constructor / field access / show path 不再依赖 parser 特判或 source 改写兜底。

### 2. checkpoint checklist：剩余语言/语义面收口

这些项里有些是“补实现”，有些是“补 fixture / 补 gate / 补 compiler-side lowering 验收”。目标不是把所有条目都重新发明一遍，而是把它们全部收进稳定 gate。

- [ ] string interpolation
	- **目标**: `"a {y} b" == "a " + Y.to_string(y) + " b"`
	- **备注**: 除了 surface parse，还要确认 lowering / return ABI / WAT payload 与跨函数 String 返回路径一致。

- [ ] methods
	- **目标**:
		- 未导入 namespace 上的方法不可见；
		- method 定义 / 调用路径完全走 type-based resolution；
		- duplicate method definition 稳定报错。

- [ ] operator overloading
	- **目标**:
		- `+ - * /` 等常规 operator；
		- `[x]` / `[x..y]` 对应 `op_index` / `op_index_slice`；
		- invalid / ambiguous / wrong-operand case 进入统一 gate。

- [ ] template / generics 剩余收口
	- **目标**:
		- `def id(x) = x` 与 auto-generic surface 对齐；
		- `x[T](v)` 明确走 explicit instantiation；
		- row-bound generic 返回 `r` 的 case 有稳定 type + runtime 验收。

- [ ] global variable / init block 规则
	- **目标**:
		- `def ONE:i64 = 1` 这类全局值稳定可用；
		- record/global init block 有确定执行时机；
		- `def VAR2 = VAR` 这类依赖可行；
		- cycle / co-dependent global 稳定拒绝。

- [ ] `Self` type with generics
	- **目标**: `type X[T] {x:T}` + `def X[T].update_x(self: Self, new_x: T): Self = {self|x:new_x}` 一类模式通过 typed + runtime 验收。

- [ ] deep pattern matching
	- **目标**:
		- `match` expr 深模式；
		- `if let` expr；
		- exhaustiveness / lowering / typed pattern env 三者一致。

- [ ] pipe behaviour
	- **目标**:
		- `a.b() == a |> A.b`
		- `a.b().c() == a |> A.b |> A.c`
		- `a |> f(b,_,_) == f(b,a,a)`
		- `a |> f == f(a)`
		- `a |> f |> g == g(f(a))`
	- **备注**: 这里既要看 parse，也要看 method/operator lowering 与 placeholder expansion 是否一致。

- [ ] ADT tuple bridge / ctor lowering
	- **目标**:
		- `HttpError(400, "...") <-> (:http_error, 400, "...")`
		- `tuple_to_adt[T]` / `adt_to_tuple[T]` method-first builtin surface 固定；
		- constructor 经 type / exhaustive check 后 lower 到 canonical tuple / record contract。
	- **备注**:
		- 这项和当前 nominal/data constructor backend blocker 强耦合，应优先与 `codegen_contract` 一起收。
		- `tuple_to_adt[Tuple, T]` / `adt_to_tuple[T, Tuple]` 不应被当作普通 std helper，而应与 compiler intrinsic 同级：由编译器打洞、类型检查和 lowering 共同承认。

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

- [ ] namespace ownership / isolation 收口
	- **目标**:
		- namespace 是定义所有权边界，不是简单的 source merge 标签；
		- imported item 即使被 driver 合并进同一个编译单元，语义上仍保留原始 owner namespace；
		- name resolution 明确区分 local scope、当前 namespace、自显式 `use` 导入、default prelude、compiler intrinsic，冲突时稳定报 ambiguous / duplicate；
		- nominal type、method、ctor、global value、intrinsic surface 都有稳定的 `owner namespace + item path` 身份，不再因合并顺序丢失来源。
	- **备注**:
		- 后端 mangling/debug 名必须建立在 namespace ownership 之上，否则 debug map 仍然不可靠；
		- `prelude` 是 import layer，不应偷变成 owner namespace；`intrinsic` / `compiler` / `std` / `metalstd` 也应各自隔离。

- [ ] constructor like `once` used value lower to mutation
	- **目标**: 
		- `def x(x:X):X = ...` 且 x 只用了一次，这个函数应该变成传入x的mutation


## Second Bootstrap 剩余项

### C11: wasm-gc Core/backend rewrite 收口

- [ ] 完成 `src/backend/cir` 迁移清零
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

## 当前优先级顺序

1. 先打通 `codegen_contract` 的 nominal/data/runtime value backend blocker。
2. 把 checkpoint checklist 中和 backend 强耦合的项一起收掉：ADT tuple bridge、constructor lowering、pattern/method/operator 相关语义面。
3. 完成 `src/backend/cir` 迁移清零，让 level-1b backend 成为 primary path。
4. 清 frontend migration 的 builtin/oracle 债务，尤其 `std.chibalex` / `std.chibacc`。
5. 做 C12 两轮 bootstrap 对拍。
