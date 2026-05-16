
## Second Bootstrap: level-1b -> level-1c 自举路线

Second Bootstrap 的目标不是把当前 `src/` 复制一份继续补丁式扩展，而是用 level-1 的语言风格重写一个干净的 `level-1b` 编译器，然后由它生成和运行 `level-1c`。现有 `src/`、level-0、native chibalex/chibacc 只作为 oracle、golden 和过渡工具。

原则：

- `level-1b` 源码必须大量使用 doc comment：public API、namespace、pass entry、核心 ADT、unsafe/Metal 边界、runtime ABI 都用 `///` 写明 contract；不写 block comment。
- 非 `#![Metal]` 源码不新增 opaque pointer `i64` 风格接口，不做隐式 internal mutability；需要低层能力时通过 `Ptr[T]`、`UnsafeRef[T]`、`Atomic[T]`、`Ref[T]` 和显式 `unsafe`。
- Chiba 风味优先：method、pipe、pattern、不可变值组合、FP style；让 optimizer / usage / CPS / closure pass 负责 inline/directify。
- `chibac` CLI 遵循 spec：`chibac <file>... -I <dir>* --target wasm32-unknown-wasi --backend wasm-gc -o <file>`；用户层命令留给 `chiba`。
- target/backend predicate 统一为 `target="wasm32-unknown-wasi"`、`backend="wasm-gc"`。
- `chibac.wasm` 必须是用户可直接用 `wasmtime chibac.wasm -- ...` 执行的 WASI/Wasm-GC compiler；Node runner、Binaryen wrapper、JS harness 只作为开发/CI 便利层和 oracle，不是运行时语义前提。
- level-1b 的 `Array`、`Slice`、`Vec`、String、closure/env、continuation package 和普通 allocation 都采用 Wasm-GC managed object 语义；线性内存只作为 WASI preview1 / host ABI 边界 scratch，不作为普通 Chiba heap。
- `level-1c` 才是 Second Bootstrap 后的主线；C00-C03 先固定 `metalstd/std/prelude`，C04 以后再迁移 regex/chibalex/chibacc/compiler pipeline。

## Second Bootstrap 库边界

- [ ] **C00: level-1b source tree + chibac driver contract**
	- **DONE**: 重新整理 `level-1b/`：`metalstd/`、`std`、`prelude`、`compiler`、`tools`、`tests` 分目录；保留当前 `src/` 作为 oracle，不作为直接复制目标。
	- **DONE**: 新增 `level1b:c00-layout` gate，检查目录、README contract、`TODO.md` / `TODO.longterm.md` 拆分，以及 wasmtime-first / Wasm-GC managed runtime 约束。
	- **DONE**: 新增 `level1b:c00-wasmtime` gate，生成最小 WASI command wasm 并验证 `chibac --help` CLI surface；本机无 `wasmtime` 时明确 SKIP 直跑部分，不把 Node harness 当作替代验收。
	- **DONE**: 新增 level-1b `compiler/cli/contract.chiba`，用 ADT 固定 `wasm32-unknown-wasi` target、`wasm-gc` backend、emit mode 和 CLI config，不用 `i64` 伪装 target/backend。
	- **DONE**: 实现/固定 `chibac` driver 参数 surface：`<file>...`、`-I/--include`、`--target/-t`、`--backend/-B`、`-o/--out`、`--emit`、`-S`、`-c`、`-E`、`-O0/-O1/-O2/-O3/-Os/-Oz`、`-g`、diagnostic flags。
	- **DONE**: 默认 target 为显式传入，不 silent fallback；level-1b 唯一 backend 是 `wasm-gc`。
	- **DONE**: `///` doc comment 进入 lexer/parser source model，并保留 source span；`#[doc(path="...")] namespace` 进入 surface scan；不实现任何 block comment。
	- **TODO**: 生成 `chibac.wasm` 时不得依赖 Node-only import；Node runner 只能包一层 WASI/env convenience，用户直接用 wasmtime 也能执行同一 CLI。
	- **验收**: `timeout 10 chibac ...` 能过 source load/parse/header scan；错误 target/backend/unknown flag 有稳定 diagnostic；`chibac --emit parse` 能输出含 doc/comment span 的 parse surface；`wasmtime chibac.wasm -- --help` 和 `wasmtime chibac.wasm -- <file> -I ... --target wasm32-unknown-wasi --backend wasm-gc --emit parse` 可运行。
	- **并行**: 不并行；先固定目录与 CLI contract。

- [ ] **C01: metalstd contract + rewrite**
	- **TODO**: 明确 `metalstd` 只包含必须贴近 backend/host ABI 的能力，并且每个文件必须 `#![Metal]`。
	- **包含**: Wasm-GC struct/array allocation intrinsic、WASI preview1 boundary、`env` import bridge、process exit/args/env low-level bridge、file descriptor read/write bridge、typed `Ptr[T]` primitive、`UnsafeRef[T]` primitive、`Atomic[T]` primitive、linear-memory scratch helper for ABI boundary only、panic/trap intrinsic。
	- **不包含**: `Option`、`List`、`Map`、`Vec`、String API、parser helpers、regex combinator、普通用户 IO facade、任何非必要 high-level collection。
	- **TODO**: 所有 Metal API 用 typed surface 暴露：`Ptr[T]` / `UnsafeRef[T]` / `Ref[T]` / `Atomic[T]`，不再把 pointer/resource handle 伪装成普通 `i64`。
	- **TODO**: 普通 allocation 只能 lower 到 Wasm-GC `struct.new` / `array.new*` / continuation package；线性内存分配器不能成为 `std`/compiler IR 的默认 heap。
	- **TODO**: Metal 中也优先使用 `.method` 和 doc comment；unsafe 边界必须由 `/// Safety` 段落说明调用前提。
	- **验收**: 非 Metal scanner 不发现 raw pointer `i64` helper；Metal API 有 doc comment；`Ptr`/`UnsafeRef`/`Atomic` invalid smoke 稳定报错；WASI/env import smoke 可运行。
	- **并行**: 不并行；这是 std/prelude 的底座。

- [ ] **C02: std contract + rewrite**
	- **TODO**: 明确 `std` 是普通 Chiba 用户可见库，依赖 `metalstd`，但不向用户暴露 Metal 的存在。
	- **包含**: `Option[T]`、`Result[T,E]`、`List[T]`、不可变 `Array[T]`、`Slice[T]`、`String == Array[u8]`、`str == Slice[u8]`、`Vec[T]` builder、`Map[K,V]` / `StrMap[V]`、`Range[T]`、iterator/sequence 基础、String byte index/slice/`.char_at`、file/process/stdout/stderr facade、diagnostic string builder。
	- **TODO**: API 用 method + pipe-friendly free function 组合；复杂 builder 可以内部用 `Ref`，但 mutable 能力必须在类型上可见并可被 usage/escape 分析。
	- **TODO**: `Array[T]` 是 Wasm-GC array；`Slice[T]` 是 Wasm-GC managed view `{backing: Array[T], offset, len}`；`Vec[T]` 是 builder over Wasm-GC managed backing storage，freeze 后产出 immutable `Array[T]`。
	- **TODO**: `String`/`str` 明确采用 Wasm-GC managed `Array[u8]` / `Slice[u8]`；`s[i]` 是 byte，`s.char_at(i)` 是 UTF-8 codepoint。
	- **TODO**: std 中 public API 全部有 `///` doc comment，跨符号引用使用 `[std.ns.Symbol]`。
	- **验收**: collections/string/io/process smoke 能 parse/check/wat/run；String/Slice WAT layout 与 spec 一致；std 不包含 `#![Metal]` 专属 primitive 的裸调用泄漏。
	- **并行**: 可按模块推进，但 public surface 先冻结。

- [ ] **C03: prelude contract + default import**
	- **TODO**: 明确 `prelude` 是默认导入层，依赖 `std`，不直接依赖 `metalstd`。
	- **包含**: 常用类型别名/导出 `Option`、`Result`、`Array`、`Slice`、`String`、`str`、`Vec`、`Map`、`Range`；常用构造/组合函数 `Some`、`None`、`Ok`、`Err`、`print`、`println`、`panic`、`assert`、基础 `map/filter/fold` facade。
	- **TODO**: 所有非 `#![Metal]` 文件默认插入 `use prelude.*`；`#![no_prelude_import]` 禁用默认导入。
	- **TODO**: `#![Metal]` 文件默认不导入 prelude；Metal 需要的 helper 必须显式 `use`。
	- **验收**: 普通 source 不显式 use 也能使用 prelude symbol；`#![no_prelude_import]` 后同一 symbol 未导入时报错；`#![Metal]` 不隐式导入 prelude。
	- **并行**: 不并行；会影响 name resolution。

## Second Bootstrap C-pass

- [ ] **C04: regex rewrite in level-1b**
	- **TODO**: 用 level-1b + std/prelude 重写 regex AST/parser/compiler/matcher，支持 UTF-8/WTF-8 byte model、Unicode-aware character classes、lookahead、lookbehind、lookaround、longest-match 所需能力。
	- **DESC**: regex 是 chibalex 的底座；必须以 Chiba 风味实现，避免 level-0 旧 matcher 的 opaque pointer 风格。
	- **验收**: regex unit/golden 覆盖 literal、class、repeat、capture、lookahead/lookbehind/lookaround、UTF-8 boundary；`chibac.wasm` 直接在 wasmtime 下可执行同一 regex smoke；node runner 只作为 CI convenience。
	- **并行**: 不并行；先保证语义。

- [ ] **C05: wasm chibalex rewrite**
	- **TODO**: 用 level-1b regex 编写新的 chibalex，包括 `.chibalex` parser、lexer IR、mode/state、longest-match、string/raw-string mode、token codegen。
	- **DESC**: 新版 chibalex 可以直接用 language-level continuation primitive 写 speculative scan、rollback、longest-match 或错误恢复。
	- **验收**: wasm chibalex 读取重写后的 `chiba-level1.chibalex` 并生成 lexer；token stream 与 native oracle 一致；至少一个 lexer backtracking/recovery 用例通过 multi-resume continuation smoke test。
	- **并行**: 不并行。

- [ ] **C06: wasm chibacc rewrite**
	- **TODO**: 用 level-1b 编写新的 chibacc，包括 `.chibacc` meta-parser、grammar IR、Pratt table、recovery IR、parser codegen。
	- **DESC**: chibacc 是语法演化核心。新版 chibacc 应直接用 continuation primitive 表达 parser alternatives、Pratt recovery、局部 retry 和 diagnostic recovery。
	- **验收**: wasm chibacc 能处理 simple/pratt/recovery/list 样例；生成 parser 行为与 native chibacc oracle 一致；multi-resume parser alternative 与 recovery 用例有 golden output；现有 level-1 grammar 也要重写到新 grammar surface。
	- **并行**: 不并行。

- [ ] **C07: source surface + project driver rewrite**
	- **TODO**: 用 level-1b 接管 source loading、`chibac` CLI parse、include path、doc comment capture、namespace/project surface scan、compile_if filtering、diagnostic ordering。
	- **DESC**: 从这里开始日常编译入口不再依赖 level-0 的 project glue。
	- **验收**: 多文件同 namespace + consumer project 通过；`#[doc(path="...")] namespace` 和 `/// namespace` surface 可 dump；compile_if target/backend 使用 `wasm32-unknown-wasi` / `wasm-gc`。
	- **并行**: 暂不并行，接口保留并行边界。

- [ ] **C08: alpha + typed semantic driver rewrite**
	- **TODO**: 在 level-1b wasm 内接管 alpha conversion、pattern elaboration、HM+row inference、checked template definition check、nominal method/operator index、extern ABI typing、Ref/UnsafeRef/Ptr/Atomic capability。
	- **DESC**: 这是 `level-1c` compiler 的 semantic gate；所有 binder 先 alpha-renamed，后续 pass 不再按裸名字判断绑定。
	- **验收**: type-system gate、row/method/operator/namespace/extern/capability gate 全部由 level-1b pass 通过，不靠 JS side script。
	- **并行**: 先单线程，错误排序必须稳定。

- [ ] **C09: continuation answer/control + usage + CPS rewrite**
	- **TODO**: 用 level-1b 接管 answer type check、continuation kind check、multi-shot replay-safety、cross world/thread boundary check、usage analysis、one-pass CPS + beta-reduction。
	- **DESC**: language-level delimited continuation 是 day-0。不要引入 effect 命名；这里的事实叫 answer/control、continuation usage、replay-safety、boundary。
	- **验收**: simple/nested reset-shift、classic Scheme multi-shot、lexer backtracking、parser recovery 通过；answer mismatch、multi-shot 捕获不可 replay state、cross world/thread continuation 稳定报错。
	- **并行**: 函数体级接口预留，先单线程。

- [ ] **C10: closure/lambda/continuation package rewrite**
	- **TODO**: 用 level-1b 接管 CPS usage analysis、continuation simplification、closure conversion、lambda lifting、closure/env shrinking、multi-shot continuation package lowering input。
	- **DESC**: no-capture/single-use lambda 和 continuation 必须 directify/inline；只有 many-use continuation 才实体化为 replayable package。
	- **验收**: no-capture lambda 不分配 closure；single-use closure direct call；multi-shot package env capture 可 dump；非法 Ptr/UnsafeRef/world/thread capture 不进入 Core。
	- **并行**: 函数体级接口预留。

- [ ] **C11: wasm-gc Core/backend rewrite**
	- **TODO**: 用 level-1b 接管 Wasm-GC Core lowering、layout table、Core validation、runtime glue、WAT emitter、Wasm binary emit/validate；Binaryen compile/opt runner 只作为开发/CI 辅助。
	- **DESC**: Core 明确表示 struct/array/funcref、closure env、continuation frame/package、tailcall、thread/world boundary facts；backend emitter 保持 dumb，只序列化已验证 Core。
	- **验收**: level-1b backend 能生成 `level1c-next.wat/wasm`；Core validator 能拒绝 dangling symbol/layout、非法 tailcall、非法 continuation package；tailcall 与 multi-shot continuation smoke 通过；生成的 `chibac.wasm` 不需要 Node-only host import。
	- **并行**: 不并行；layout 表稳定排序。

- [ ] **C12: Second Bootstrap validation**
	- **TODO**: 完成 `level1c.wasm -> level1c-next.wasm -> level1c-next2.wasm` 两轮自举对拍。
	- **DESC**: 第二次 bootstrap 完成的含义是 level-1 wasm 已接管 metalstd/std/prelude、regex、chibalex、chibacc、semantic driver、usage/CPS、closure/lambda lifting、wasm-gc Core/backend 和单线程 driver。
	- **验收**: `level1c-next.wasm` 和 `level1c-next2.wasm` 能用 wasmtime 直接重跑 lexer/parser specs、semantic/type gates、continuation smoke、core/backend smoke；node runner 路径也可跑但不能是唯一成功路径；关键 IR 输出一致或差异有 manifest 解释；记录 seed hash、level1c hash、level1c-next hash、toolchain versions。
	- **并行**: 不并行；这是收敛验证点。
