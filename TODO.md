# Level-1 Bootstrap + Semantic/Wasm TODO

这个文件同时跟踪两条线：

1. bootstrap 路线：先用 level-0 以最简陋、单线程、单 module 的方式产出可运行的 level-1 wasm compiler；再用第一次 bootstrap 出来的 level-1 wasm 重写 chibalex、chibacc 和后端，完成第二次 bootstrap。
2. 目标路线：level-1 自驱稳定后，再建设可验证、可并行、可缓存的 nanopass pipeline，输出 `wasm-gc + tailcall + wasi + thread` 的 `.wat` / wasm object。

原则：在完成 level-0 自举到 level-1 之前，level-0 不承担并行编译、namespace object、`wasm-ld` linking、完整 monomorphization registry、增量 cache 等长期复杂度。level-0 只负责第一跳；长期架构由 level-1 wasm 自己长出来。

约束：generics 走 monomorphize，但不是 C++ 老模板式“定义期不检查”。level-1 generics 是 checked structural templates：定义期在抽象参数下检查一次，实例化期只兑现 concrete shape / method / operator / dispatch obligation，并生成 specialization。

约束：language-level delimited continuation 是 day-0 能力，不是后置优化项。level-1 compiler、chibalex、chibacc 可以直接用 `reset` / `shift` 或等价 primitive 写 lowering、回溯和错误恢复。continuation 可 multi-resume，但跨 world/thread capture 或 resume 永远非法；multi-resume 必须经过 answer type、replay-safety 和 usage analysis 检查。

## Bootstrap 非目标

- [ ] **level-0 不做并行构建**
	- **TODO**: bootstrap 阶段保持单线程、单任务队列、确定性顺序。
	- **DESC**: 不在 level-0 引入 worker pool、namespace 并行、specialization 并行、并发 cache 或 scheduler。
	- **验收**: 第一跳 bootstrap 全程单线程可跑；没有为了并行引入的全局锁、并发 registry 或诊断排序问题。

- [ ] **level-0 不做 namespace object / wasm-ld**
	- **TODO**: 第一跳只输出一个 merged module 的 `.wat` / `.wasm`。
	- **DESC**: 跨 namespace linking 先由 project merge 解决；`wasm-ld` 等到 level-1 自驱后再接入。
	- **验收**: bootstrap compiler 不要求生成 per-namespace object；单 wasm module 能运行 smoke tests。

- [ ] **level-0 不做完整 checked monomorphize 基建**
	- **TODO**: 第一跳限制 bootstrap 子集，避免依赖 generic-heavy compiler code。
	- **DESC**: specialization key、InstantiationRegistry、generic SCC、跨 namespace 去重都留给 level-1 pipeline。
	- **验收**: level-0 wasm target 能编译 level-1 compiler skeleton，而不是完整长期 compiler。

## Bootstrap Layout Contract

- [ ] **固定 wasm-gc bootstrap object model**
	- **TODO**: 写下第一跳 wasm-gc 的对象布局，并在 level-0 wasm emitter 中按这个布局输出。
	- **DESC**: bootstrap 阶段布局宁可简单，不追求优化。规则先固定为：普通 level-1 object 100% heap object；标量 `i64/i32/u*/bool/atom/tag` 可以是 wasm immediate；managed record/tuple/data/string/slice/closure 都是 gc heap refs。
	- **验收**: 文档和 emitter 使用同一套 layout id；没有隐式栈上 managed object；Core/IR dump 能显示每个 object 的 layout kind。
	- **并行**: 不并行；layout table 按稳定顺序构建。

- [ ] **`type == row`**
	- **TODO**: 把 `type T { ... }` 降为带 nominal identity 的 row object layout。
	- **DESC**: `type` 的本体是 row/record shape，但保留 nominal identity。字段按 canonical field id 排序，源码顺序不影响 layout hash。首版全部 heap struct，不做 unboxed record。
	- **验收**: 同字段不同顺序的 row canonical key 相同；不同 nominal type 即使 shape 相同也保留不同 nominal id。
	- **并行**: 不并行；canonical field id 和 nominal id 由全项目稳定表分配。

- [ ] **`tuple == type`**
	- **TODO**: 把 tuple 当作匿名 `type`/row layout，字段为稳定序号字段。
	- **DESC**: `(A, B)` 等价于匿名 positional row `{_1: A, _2: B}` 的 heap object。tuple 字段名由位置稳定生成，首版不做 tuple scalar replacement，不做多返回值优化。
	- **验收**: tuple literal、tuple destruct、tuple pattern 使用同一匿名 layout；1-tuple 和 grouped expr 语义可区分。
	- **并行**: 不并行；匿名 tuple layout key 由元素类型序列确定。

- [ ] **`data == (:tag, union)`**
	- **TODO**: 把 ADT/data variant 表示为 tag + payload union。
	- **DESC**: 这只是 bootstrap wasm lowering contract，不是语言层“`data` 本体等于二元组”的语义定义。首版每个 `data` value 降为 heap object：第 0 字段为 variant tag（可用 atom/tag immediate），第 1 字段为 payload union。无 payload variant 的 payload 可为 unit/null sentinel。payload union 由该 data 的所有 variant payload layout 合成。
	- **验收**: constructor、match、tag test、payload field extract 都只依赖 `(:tag, union)` 布局；variant tag 稳定且可 dump。
	- **并行**: 不并行；variant tag 分配按 source/canonical order 稳定。

- [ ] **`union` 作为 payload/row carrier**
	- **TODO**: 固定 level-1 `union` 在 bootstrap wasm 中的 heap 表示。
	- **DESC**: 首版 union 是一组字段的 heap row object，可作为 data payload carrier；不做 C union 式重叠内存优化。
	- **验收**: union field access/update 与 record/type field access/update 共享 row-field lowering；data payload union 可复用同一表示。
	- **并行**: 不并行。

- [ ] **string / slice / cstr bootstrap layout**
	- **TODO**: 固定 `Str`、slice、`cstr` 的 wasm runtime layout。
	- **DESC**: `Str`/slice 首版用 heap object `{ptr, len}` 或 `{arrayref, len}`；`cstr` 只作为 WASI/FFI boundary view，不作为普通 managed string 本体。
	- **验收**: file read、stdout write、lexer input、parser source span 都走同一 string/slice contract。
	- **并行**: 不并行。

- [ ] **closure / function / continuation bootstrap layout**
	- **TODO**: 首版 closure 是 heap object `{funref, env}`；top-level function 可为 direct function symbol；continuation 有 linear 与 multi-resume 两种 lowering 路径。
	- **DESC**: `reset` / `shift` 是 bootstrap subset 必需能力。ordinary administrative continuation 由 one-pass CPS 在编译期 beta-reduce；用户级 captured continuation 只有在 usage analysis 判定需要实体化时才落到 wasm-gc continuation/frame package。跨 world/thread 的 continuation capture 或 resume 直接非法。
	- **验收**: chibalex/chibacc bootstrap 子集能用 continuation primitive 写回溯和错误恢复；simple reset/shift、multi-resume backtracking、answer type mismatch、cross-thread/world continuation error 都有 smoke test。
	- **并行**: 不并行。

## First Bootstrap: level-0 -> level-1 wasm

- [x] **Bootstrap Pass B00: Freeze Bootstrap Subset**
	- **TODO**: 定义第一版 level-1 compiler skeleton 可使用的语言子集。
	- **DESC**: 子集应足够写 CLI、文件 IO、字符串处理、lexer/parser runner、简单 pass driver、one-pass CPS lowering、continuation-based backtracking/recovery。可以暂时规避或限制 complex generics、dyn packaging、复杂 escape、operator overload、Atomic/world-boundary 细节，但不能规避 language-level continuation primitive。
	- **验收**: 有 `bootstrap-subset` 清单；level-1 compiler skeleton、chibalex/chibacc skeleton 源码可以使用 `reset` / `shift`、multi-resume continuation、普通 closure 和 tailcall。
	- **并行**: 不并行。

- [x] **Bootstrap Pass B01: Freeze level-0 Seed**
	- **TODO**: 选定稳定 `level0/target/debug/main.o`，记录构建命令、hash、测试结果。
	- **DESC**: 第一跳所有问题都必须能区分是 seed 变化、wasm emitter 变化，还是 level-1 skeleton 变化。
	- **验收**: clean checkout 可复现 seed；seed 通过 level-0 tests。
	- **并行**: 不并行。

- [x] **Bootstrap Pass B02: level-1 Nanopass CIR Entry**
	- **TODO**: 决定 level-0 wasm emitter 消费 CIR、BIR，还是 BIR-adjacent 简化 IR。
	- **DESC**: 修正：level-0 只作为 seed/reference，不继续扩展长期后端。level-1 自举源码在 `src/`，先建立 nanopass CIR：同一个 CIR 大 ADT 中保留 `L0*` surface core、`L1*` alpha core、后续 typed/answer-control/CPS/closure/core 节点族。第一跳 wasm 后端消费 level-1 自己验证后的 Core/CIR 子集，而不是 level-0 BIR。
	- **验收**: `src/backend/cir` 有 nanopass ADT 和 pass 边界；至少有 alpha-conv pass 骨架；不支持 AST/CIR 节点能进入显式 unsupported/diagnostic，而不是落回 opaque `i64`。
	- **并行**: 不并行。

- [x] **Bootstrap Pass B03: Minimal Wat Emit**
	- **TODO**: 支持函数、局部变量、整数运算、比较、block/loop/branch、call、tailcall/return、WASI imports、`env` imports、bootstrap heap object emit、最小 continuation frame/package emit。
	- **DESC**: emitter 保持 dumb，只序列化已知 IR 到可读 wat。
	- **验收**: 算术、if、loop、函数调用、tail-recursive call、stdout、file read、simple reset/shift、multi-resume backtracking smoke tests 都能生成合法 wat 并运行。
	- **并行**: 不并行。

- [x] **Bootstrap Pass B04: Runtime/Extern Glue**
	- **TODO**: 提供 allocator、argv/env、file read、write stdout/stderr、exit/status、string/slice helpers，并支持 typed extern import declaration：`def name(args): ret = extern "wasi" "function_name"` 进入 WASI import，`def name(args): ret = extern "C" "function_name"` 进入 wasm embedder / JS 侧 `env` import。
	- **DESC**: 先走 WASI + `env` 两类最小外部边界；不要提前实现完整 managed runtime。extern 函数通过 typed declaration 进入 symbol table，backend 只按已检查签名生成 import/call，不在 emit 阶段猜 ABI。为兼容 level-0 既有写法，`"c"` 和 `"C"` 应 canonicalize 到同一个 C/env ABI。
	- **验收**: wasm 程序能读输入文件、写输出文件、错误返回非零；`extern "wasi" "fd_write"` 能生成稳定 WASI import；`extern "C" "js_log"` 能生成稳定 `(import "env" "js_log" ...)` 并被普通 call 调用。
	- **并行**: 不并行。

- [x] **Bootstrap Pass B05: level-1 Compiler Skeleton**
	- **TODO**: 建立最小 level-1 compiler wasm 项目：CLI、文件扫描、parse-only、check-stub、diagnostic 输出、continuation smoke runner。
	- **DESC**: 首版可以直接携带当前 generated level-1 lexer/parser，不要求先自举 chibalex/chibacc；但 skeleton 必须能编译并运行使用 continuation primitive 的小型 lowering/backtracking/recovery 示例。
	- **验收**: `level1c.wasm --help`、`level1c.wasm parse file.chiba`、`level1c.wasm check file.chiba` 可运行；continuation smoke runner 能覆盖 single resume、multi resume、nested reset、非法跨 world/thread continuation。
	- **并行**: 不并行。

- [x] **Bootstrap Pass B06: First Bootstrap Validation**
	- **TODO**: 用 level-0 seed 编译 `level1c.wasm`，并对拍 native parser runner 与 wasm parser runner。
	- **DESC**: 第一成功点不是完整自举，而是 level-1 compiler wasm 能稳定运行。
	- **验收**: 25 个 grammar spec 正向 OK；错误 spec Err；关键 AST 节点一致；continuation smoke tests 通过；记录 seed hash、wasm hash、toolchain version。
	- **并行**: 不并行。

### 最终验收标准
- [x] 有没有正确的使用 nanopass 思想：每个pass只干一件事
- [x] 有没有实现 nodejs runner 并能够运行示例程序（需要准备一系列程序涉及到你实现的所有的功能并运行）
- [x] 有没有实现 `.method(call)` 有没有正确的按类型实现，符合Spec的三种标准不？
- [x] 有没有正确实现 row poly `def row_name[T: {r| name: Str, id: i64}](value: T): Str = value.name` 或着用户采用简便写法 `def row_name(value: {r| name: Str, id: i64}): Str = value.name` 哦对, 还有比如返回 r
- [x] 有没有正确实现 namespace, 测试两个文件同一个namespace，第三个文件调用这一个namespace里面的两个文件里面的函数
- [x] 有没有实现 `Ref[T]` `UnsafeRef[T]` `Ptr[T]` `Atomic[T]`, `:=` 实现和spec对的上不, Ref[T] 和 UnsafeRef[T] 跟 wasm-gc 的行为对齐没有
- [x] 有没有实现 delimited continuation, 可不可以multi entry, 测试经典scheme `(+ 1 (reset (* 2 (shift k (k (k 4))))))
- [x] string string interpolation slice 这些的 test 然后 wasmgc 后端的 string 和 slice 应该都是托管对象，所以你得想办法打洞
- [x] 所有测试都生成对应的 `.wat` 文件

## Second Bootstrap 启动前置 TODO

这一段是 C00 之前的硬门槛：只有当 `level-1b` 能用 level-0 seed 编译、用 node 运行，并完整表达 level-0 当前承担的 generator/runtime/compiler 子集时，才算真正开始 Second Bootstrap。这里仍允许 `level-1b` 复用 level-0 的标准库实现作为过渡，但新写的 `level-1b` 源码不能继续扩散 opaque `i64`、隐式 mutable、非 `#![Metal]` 的低层写法。

- [x] **Pre-C00: level-1b source tree + build contract**
	- **TODO**: 建立 `level-1b/` 或等价的新源码树，明确哪些源码来自当前 `src/` 的复制编辑，哪些仍临时复用 level-0/seed stdlib。
	- **DESC**: `level-1b` 是准备接管 generators 和 compiler pipeline 的干净实现层；不能直接把 level-0 当长期源码继续补丁式扩展。
	- **验收**: 有固定入口、固定 project layout、固定构建命令；`timeout 10` 能通过 phase1/语法检查，放宽后能编译出 node runner 可加载的 wasm/object。
	- **并行**: 不并行；先固定目录和构建协议。

- [x] **Pre-C01: level-1b std surface freeze**
	- **TODO**: 定义 level-1b 可用标准库 surface：`Option`、`List`、`Array[T]`、`Slice[T]`、`String == Array[u8]`、`str == Slice[u8]`、`Vec`/builder、`Map`、file/process/print、WASI/env boundary。
	- **DESC**: 允许实现暂时复用 level-0 stdlib，但 level-1b 代码必须按 level-1 语义写调用面；低层 raw pointer/opaque `i64` 只能出现在 `#![Metal]` 标注模块。
	- **DONE**: `level-1b/supports/std-surface.md` 固定 surface；`level-1b/supports/pre-c01-smokes/` 覆盖 string/slice、collections、IO/process、Ref/Atomic valid/invalid fixture；`vp run level1b:std-surface` 解析 fixture、检查 string/slice WAT layout，并扫描非 Metal source 的 raw pointer 用法。
	- **验收**: string/index/slice/`.char_at`、file read/write、stdout/stderr、argv/env、Vec/Map 基础操作都有 level-1b smoke；非 Metal 源码中没有新增裸 pointer 风格 helper。
	- **并行**: 不并行；先稳定 surface，再迁移实现。

- [ ] **Pre-C02: real typed nanopass spine past L1**
	- **TODO**: 把当前只到 `L1Alpha` 的 nanopass 继续拆到 `L2Typed`、`L3AnswerControl`、`L4Usage`、`L5Cps`、`L6Closure`、`L7Core`、`L8ValidatedCore`。
	- **DESC**: 每个 pass 只做一件事，并且产物进入新的 ADT/节点族，而不是 side script 检查后继续让 WAT emitter 直接吃 L1。
	- **PROGRESS**: 已建立 L2-L8 ADT 节点族、独立 pass 文件和 `level1c.o nanopass` dump smoke；L2 typed pass 已维护函数体局部类型环境，`let` 绑定后的 local ref 会 dump 为对应 `L2OpTyped` 类型事实，并由 bootstrap smoke 顺序断言；L3 answer/control pass 已递归标注表达式子树，continuation smoke 会断言 `reset` 子树的 `delimited` fact 和 `shift` 子树的 `shift` fact；L7/L8 现在递归标注表达式节点，并在 bootstrap smoke 中断言 string/slice 与 continuation package Core facts。当前仍是保守骨架，尚未满足真实 type/CPS/Core 语义。
	- **验收**: CIR/Core 中能 dump `L2*` typed refs、`L3*` answer/control-boundary facts、`L5*` CPS continuation、`L6*` closure/env、`L7Core*` wasm-gc 节点；每层至少有一个 golden smoke。
	- **并行**: 函数体级并行暂不实现；设计上保留 arena/symbol id 边界。

- [ ] **Pre-C03: L2 type/method/row semantic implementation**
	- **TODO**: 实现真实 type checker：HM 基础、row poly、nominal row/data/union、method resolution 三路径、extern ABI typing、`Ref[T]`/`UnsafeRef[T]`/`Ptr[T]`/`Atomic[T]` capability。
	- **DESC**: 现在很多 semantic gate 还是脚本级检查；C00 前要进入 compiler pass，后端只消费已检查事实。
	- **PROGRESS**: `level1c.o check` 已接入 compiler-side source semantic gate，能接受/拒绝 Ref/Atomic、method resolution、row poly valid/invalid fixtures；namespace 仍需从 JS gate 下沉到 project-aware compiler check，整体仍需迁入 L2 typed pass。
	- **验收**: `.method(call)`、row identity、namespace 多文件、Ref/Atomic invalid cases 不再只靠 JS gate；`level1c.o check` 能稳定接受/拒绝同一组 semantic fixtures。
	- **并行**: 暂不并行；错误排序必须确定。

- [ ] **Pre-C04: continuation answer/control + one-pass CPS**
	- **TODO**: 实现 answer type check、continuation kind check、replay-safety check、one-pass CPS transformation 和 administrative continuation beta-reduction。
	- **DESC**: `reset`/`shift` 不能停留在 check gate；chibalex/chibacc 的 backtracking/recovery 要能落到同一 CPS core。
	- **PROGRESS**: `level1c.o cps` 已能把 continuation fixture dump 到 L5 CPS/continuation package，并由 bootstrap smoke 与 wasm bridge 覆盖；当前仍是骨架 CPS，不含完整 answer/control/replay-safety 与 beta-reduction。
	- **验收**: simple reset/shift、nested reset、multi-resume Scheme smoke、lexer backtracking、parser alternative/recovery 都能 dump CPS；answer mismatch、multi-resume 捕获不可 replay state、跨 world/thread continuation 稳定报错。
	- **并行**: 不并行；先保证语义正确和 dump 稳定。

- [ ] **Pre-C05: closure/lambda/continuation package lowering**
	- **TODO**: 实现 usage facts 驱动的 dead continuation 删除、single-use continuation inline、many-use continuation package、closure conversion、lambda lifting、env shrinking。
	- **DESC**: level-1b 需要能写 generator 和 compiler helper，而不是所有 lambda/continuation 都分配成 opaque runtime 包。
	- **验收**: no-capture lambda direct call；single-use closure directified；escaping closure 有可 dump env layout；multi-resume continuation package 可重复 resume；非法 package 不进入 Core。
	- **并行**: 不并行。

- [ ] **Pre-C06: wasm-gc CoreIR + validator**
	- **TODO**: 建立独立 Wasm-GC CoreIR，覆盖 struct/array/funcref/import/tailcall/layout id、String/Array/Slice、closure env、continuation package、world/thread facts。
	- **DESC**: WAT emitter 必须从 validated Core 序列化，不能继续在 emitter 中做 unresolved hole、semantic fallback 或类型猜测。
	- **PROGRESS**: L7 Core pass 已递归给表达式节点打 `L7CoreOp`，L8 validator 保留 L7 payload 并递归输出 `L8ValidatedCoreOp`；`vp run smoke:bootstrap` 覆盖 `core-op string-slice`、`core-op continuation-package`、validation ok，以及 `core-invalid-smoke` 的 validator error dump。仍缺真实 symbol/layout/world-thread validator 和 emitter 改道 validated Core。
	- **验收**: Core validator 能拒绝 dangling symbol、错误 layout ref、非法 tailcall、非法 continuation package、错误 String/str/Array/Slice layout；valid Core 全量能 emit `.wat` 并由 Binaryen validate。
	- **并行**: 不并行；layout table 稳定排序。

- [ ] **Pre-C07: real String/Array/Slice runtime**
	- **TODO**: 实现 `String == Array[u8]`、`str == Slice[u8]` 的真实 payload lowering 和 runtime helpers：literal bytes、interpolation concat、byte index、range slice、bounds check、`.char_at`、WASI encode/decode。
	- **DESC**: 当前 string/slice 是 wasm-gc managed layout hole；C00 前至少要能支撑 lexer/parser 输入、source span、diagnostic 输出和 generated code 拼接。
	- **BLOCKER**: 当前 parser AST 把 string 降成 payload-less `Expr_String`，string chunk/interpolation 内容没有进入 `lower_ast`；真实 byte payload 需要先在 grammar/parser AST 中保留 string pieces，再进入 CIR/Core。
	- **验收**: string literal WAT 含真实 byte payload；`s[i]` 返回 byte/slice 语义；`s[a..b]` 不复制并保活 backing array；`.char_at(n)` 做显式 codepoint 访问；file read/stdout/lexer input 共用同一 contract。
	- **并行**: 不并行。

- [ ] **Pre-C08: namespace/project driver**
	- **TODO**: 实现 level-1b project scan、namespace summary、multi-file merge、entry selection、dependency ordering、diagnostic ordering。
	- **DESC**: chibalex/chibacc/metalstd/compiler 不会是单文件；C00 前必须能稳定处理多个文件、同 namespace 多 fragment 和第三方 consumer。
	- **PROGRESS**: 已新增 `level-1b/supports/namespace-project` 多文件 smoke 和 `vp run level1b:namespace`，覆盖两个文件同 namespace + consumer 编译与 WAT/node 运行；runner 现在写出稳定 project summary 并断言 summary hash。诊断排序和真正 project-aware level1c driver 仍未完成。
	- **验收**: 两个文件同 namespace + 第三个 consumer 的 wasm/Core path 通过；summary hash 稳定；错误输出不依赖文件系统遍历顺序。
	- **并行**: 暂不并行；接口为后续 namespace 并行预留。

- [ ] **Pre-C09: level-1b regex + chibalex bootstrap slice**
	- **TODO**: 在 level-1b 写最小 regex IR、scanner runtime、chibalex parser/codegen，先覆盖 chibalex 自身需要的 token 子集。
	- **DESC**: C00 的第一刀必须有可运行闭环，而不是一次性重写完整 chibalex。
	- **PROGRESS**: 已固定 `level-1b/supports/chibalex-mini` 三个 lexer spec fixtures；`vp run level1b:chibalex-mini` 会调用 native chibalex、检查生成结构，并用 fixture-local regex/Vec stub 编译运行 generated lexer token golden。level-1b 自身 regex/parser/codegen 尚未实现，当前 runner stub 只覆盖 mini fixtures。
	- **验收**: wasm chibalex-mini 能读 3-5 个 lexer spec，生成 lexer 或 token stream；与 native chibalex/lexer runner golden 对拍；至少一个 longest-match/backtracking/recovery 用例使用 multi-resume continuation。
	- **并行**: 不并行。

- [ ] **Pre-C10: level-1b chibacc/bootstrap parser slice**
	- **TODO**: 在 level-1b 写最小 `.chibacc` meta-parser、grammar IR、Pratt/recovery skeleton 和 parser codegen。
	- **DESC**: 在完整 C01 前先证明 parser generator 的核心数据结构、diagnostic recovery 和 generated parser runner 能在 wasm/node 下工作。
	- **PROGRESS**: 已固定 `level-1b/supports/chibacc-mini` simple/pratt/list grammar fixtures；`vp run level1b:chibacc-mini` 会调用 native chibacc、检查生成结构、合成最小 Token/TokenSpan runner，并用 level-0 编译运行 generated parser AST golden。Pratt 缺失 inner expr 的 recovery path 已覆盖 `Err(Some(ast), ...)` golden；level-1b 自身 meta-parser/IR/codegen 尚未实现。
	- **验收**: simple grammar、Pratt expression、错误恢复三类样例可生成 parser 并运行；输出与 native chibacc golden 对拍。
	- **并行**: 不并行。

- [ ] **Pre-C11: node/browser/WASI execution harness**
	- **TODO**: 固定 node runner、WASI imports、env imports、wasi-thread 预留、Binaryen opt/validate、all-wat run、artifact/hash 记录。
	- **DESC**: level-1b 产物必须能由 node 执行 generators，不能只生成静态 `.wat`。
	- **PROGRESS**: `level1c.wasm` host bridge 已覆盖 help/parse/check/typed/nanopass/cont-usage；first-bootstrap validation 同时跑 typed 和 nanopass wasm bridge，并继续记录 Binaryen/hash。
	- **验收**: `vp run level1b:*` 能编译、运行、对拍、记录 seed/object/wasm/toolchain hash；所有生成 `.wat` 都 run 或 instantiate；opt 与 non-opt 均通过核心 smoke。
	- **并行**: runner 可串行；输出必须确定。

- [ ] **Pre-C12: Metal/unsafe/capability discipline**
	- **TODO**: 把 `#![Metal]`、`unsafe` 块、`Ref[T]`/`UnsafeRef[T]`/`Ptr[T]`/`Atomic[T]` capability 规则从最终验收拆成可执行 gate：源码扫描、parser/semantic check、valid/invalid fixtures、level-1b std surface 审计。
	- **DESC**: 非 Metal 的 level-1b 源码 **不能包含任何** opaque pointer `i64` 风格接口；Metal 内部也必须优先使用 typed `Ptr[T]`/capability wrapper，而不是裸 `i64` 指针。`UnsafeRef`/`Ptr` 只能在显式 unsafe 区域使用，非 Metal 源码没有 unsafe 块时不能触碰 unsafe capability。
	- **PROGRESS**: 新增 `level-1b/supports/pre-c12-smokes` valid/invalid fixtures 与 `vp run level1b:capability`；gate 会扫描非 Metal raw pointer、非 unsafe 块 `Ptr`/`UnsafeRef`、Metal raw `i64` pointer API，并通过 `level1c.o check` 对同一组 fixture 做 compiler-side 对拍。当前仍是 source-level gate，尚未迁入真实 L2 capability/type pass。
	- **验收**: `vp run level1b:std-surface` 或独立 gate 能拒绝非 Metal 裸 pointer API、Metal 裸 `i64` pointer 扩散、非 unsafe 块使用 `UnsafeRef`/`Ptr`、错误 `Atomic[T]`/`Ref[T]` assignment；valid fixtures 覆盖 Metal typed pointer helper、safe `Ref[T]`、unsafe block 中的 `UnsafeRef`/`Ptr`、Atomic 操作。
	- **并行**: 不并行；先保证诊断稳定，再把 gate 并入全量 validation。

### Second Bootstrap 启动前最终验收标准

- [ ] `level-1b` 可以由当前 level-0 seed 编译，并在 node/WASI runner 下运行。
- [ ] `level-1b` 源码能完整表达 level-0 当前承担的核心工具链：regex、chibalex、chibacc、metalstd surface、compiler semantic driver、wasm-gc backend skeleton。
- [ ] 非 `#![Metal]` 的 level-1b 源码不新增 opaque pointer `i64` 风格接口，Metal的也全体采用 Ptr， 然后还得有unsafe 检查，非metal没有开unsafe块不能碰 unsaferef 和 ptr；`Ref`/`UnsafeRef`/`Ptr`/`Atomic` 的使用符合 spec。
- [ ] nanopass pipeline 不止停在 L1：至少 `L2Typed`、`L3AnswerControl`、`L5Cps`、`L6Closure`、`L7Core`、`L8ValidatedCore` 有真实 ADT、dump 和 smoke。
- [ ] WAT emitter 只吃 validated Core；semantic fallback/hole 不再作为普通成功路径。
- [ ] `String == Array[u8]`、`str == Slice[u8]` 的真实 runtime contract 支撑 lexer/parser/diagnostic/file IO。
- [ ] wasm chibalex-mini 和 chibacc-mini 能在 node 下运行并与 native generator/runner golden 对拍。
- [ ] continuation day-0 能力进入 CPS/Core lowering：multi-resume lexer/parser recovery 可运行，非法跨 world/thread capture/resume 稳定报错。
- [ ] 全量 bootstrap/semantic/all-wat validation 通过，并记录 seed hash、level-1b wasm hash、generated lexer/parser hash、toolchain version。


## Second Bootstrap: level-1 wasm 接管 generators + Optimized CPS Core

level-1 在重构的时候请不要再像level-0一样动不动就引入 i64 这种 opaque 和 internal mutability, 请参考spec我对mutable的控制，这个时候你可以把创建文件夹 level-1b 然后把 src 挪进去，并重新开一个 src，从头使用level-1的崭新的整洁的语法写出符合level-1 spec的代码!

所以 `metalstd` 应该是拿 `.method` 写 `#![Metal]` 而不是现在这种不符合语言语义的东西

- [ ] **Bootstrap Pass C00: wasm chibalex rewrite**
	- **TODO**: 用第一次 bootstrap 出来的 `level1c.wasm` 按照旧的编写新的 regex + chibalex。
	- **DESC**: chibalex 输入输出清楚，是第一批自驱目标。新版 chibalex 可以用 language-level continuation primitive 写 speculative scan、rollback、longest-match 或错误恢复，但 continuation 不能跨 world/thread。
	- **验收**: wasm chibalex 读取 `src/frontend/chiba-level1.chibalex` （也需要重写） 并生成 lexer；token stream 与 native chibalex 一致；至少一个 lexer backtracking/recovery 用例通过 multi-resume continuation smoke test。
	- **并行**: 不并行。

- [ ] **Bootstrap Pass C01: wasm chibacc rewrite**
	- **TODO**: 用 `level1c.wasm` 编写新的 chibacc，包括 `.chibacc` meta-parser、grammar IR、parser codegen。
	- **DESC**: chibacc 是语法演化核心，放在 chibalex 之后迁移。新版 chibacc 应直接用 continuation primitive 表达 parser alternatives、Pratt recovery、局部 retry 和 diagnostic recovery。
	- **验收**: wasm chibacc 能处理 simple/pratt/recovery 样例；生成 parser 行为与 native chibacc 一致；multi-resume parser alternative 与 recovery 用例有 golden output。（现有的level1.chibacc也要重写）
	- **并行**: 不并行。

- [ ] **Bootstrap Pass C02: wasm alpha + continuation semantic driver**
	- **TODO**: 在 level-1 wasm 内接管 alpha conversion、answer type check、continuation kind check、replay-safety check 的单线程版本。
	- **DESC**: 这是 language-level continuation day-0 的语义 gate。所有 binder 先 alpha-renamed；`reset` / `shift` 必须通过 answer type；continuation 分为 linear / multi-resume；跨 world/thread capture 或 resume 直接报错；multi-resume 只能捕获 replay-safe state 或 reset-local rollback region。
	- **验收**: alpha dump 稳定；answer type mismatch、multi-resume 捕获不可 replay 资源、跨 world/thread continuation 都能稳定报错；合法 parser backtracking 用例通过。
	- **并行**: 不并行。

- [ ] **Bootstrap Pass C03: wasm usage + one-pass CPS rewrite**
	- **TODO**: 用 level-1 wasm 接管 high-level usage analysis 与 one-pass CPS transformation，并在 CPS 变换中直接做 beta-reduction。
	- **DESC**: 这一步继承 level-0 CIR 的核心纪律，但不继承 BIR 作为目标。lowering API 使用 meta-continuation；administrative continuation 默认不实体化；用户级 `reset` / `shift` 和 multi-resume continuation 作为语义控制点保留。usage analysis 记录 binder/lambda/continuation/closure 的 `0 | 1 | many`。
	- **验收**: one-pass CPS dump 不含可避免的 administrative cont；single-use continuation 被 beta/inline；multi-resume continuation 被标记为实体化候选；compiler lowering、chibalex backtracking、chibacc recovery 都能用同一 CPS core 表示。
	- **并行**: 不并行。

- [ ] **Bootstrap Pass C04: wasm continuation + closure simplification rewrite**
	- **TODO**: 用 level-1 wasm 接管 CPS usage analysis、continuation simplification、closure conversion、lambda lifting、closure/env simplification。
	- **DESC**: 这是性能不输 Go 的 day-0 中端卫生线：dead continuation 删除，single-use continuation inline，many-use continuation 降成 multi-resume package；escaping lambda 做 closure conversion，nested function 做 lambda lifting；no-capture / single-use closure 尽量 direct call；env 做 shrinking。
	- **验收**: closure env layout 可 dump；no-capture lambda 不分配 closure；single-use closure directified；multi-resume parser continuation 可重复 resume；非法跨 world/thread continuation 不可能进入 lower。
	- **并行**: 不并行。

- [ ] **Bootstrap Pass C05: wasm-gc core/backend rewrite**
	- **TODO**: 用 level-1 wasm 接管 wasm-gc Core lowering、Core validation、runtime glue、wat emitter。
	- **DESC**: level-1 不需要继承 level-0 BIR，但必须有已验证的 Wasm-GC Core。Core 明确表示 struct/array/funcref、closure env、continuation frame/package、tailcall、thread/world boundary facts；backend emitter 保持 dumb，只序列化已验证 Core。
	- **验收**: level-1 wasm backend 能重新生成 `level1c-next.wasm`；Core validator 能检查 symbol/type/layout/continuation frame 引用完整；tailcall 与 multi-resume continuation smoke tests 通过。
	- **并行**: 不并行。

- [ ] **Bootstrap Pass C06: wasm semantic/pass driver rewrite + Second Bootstrap Validation**
	- **TODO**: 在 level-1 wasm 内实现正式 pass driver 的单线程版本，并完成第二次 bootstrap 对拍。
	- **DESC**: 第二次 bootstrap 的含义不是“能跑生成器”而已，而是 level-1 wasm 已经接管 chibalex、chibacc、alpha/continuation semantics、usage、one-pass CPS、closure/lambda lifting、wasm-gc Core/backend 和单线程语义 driver，不再依赖 level-0 的临时 generator/backend/pass glue 进行日常演化。
	- **验收**: `level1c.wasm -> level1c-next.wasm` 成功；`level1c-next.wasm` 能重跑 parser/lexer specs、continuation smoke tests 与核心 check/lower smoke tests；两代关键 IR 输出一致或差异可解释。
	- **并行**: 不并行。

## Long-term Pipeline 总览

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
	- **DESC**: 这是 value type 的主推断层，但不做 method/operator 最终选择，不做 monomorphization，不做 escape。row 必须 canonical：字段稳定 id、稳定排序、hash 不依赖编译顺序。这里同时固定 tuple 的 positional row 字段 `_1`, `_2`, ...，以及不可变 `Array[T]` 的基础类型行为。
	- **验收**: 能输出 `TypedAst` + `ConstraintSet` + canonical row/shape ids；错误包括普通类型不匹配、缺字段、record update 不合法、非法 let-generalization。
	- **并行**: 函数体级并行；type variable id 分配使用 per-task arena，合并时重编号或使用 scoped id。

- [ ] **Pass 07: Answer / Continuation Kind Check**
	- **TODO**: 检查 `reset` / `shift` answer type、implicit reset、continuation kind、resume count contract、multi-resume replay-safety。
	- **DESC**: language-level continuation 是 day-0。continuation kind 至少区分 linear 与 multi-resume；multi-resume 用于 lexer/parser/compiler backtracking 与 recovery。跨 world/thread 的 capture 或 resume 永远非法。multi-resume 只能捕获 replay-safe state，或捕获由 reset-local rollback region 管理的局部可回滚状态。
	- **验收**: simple reset/shift、nested reset、multi-resume parser alternative 通过；answer type mismatch、multi-resume 捕获 FFI/UnsafeRef/world-local/Atomic mutation、跨 world/thread continuation 都能报错。
	- **并行**: 函数体级并行；跨函数 continuation escape 摘要按 call graph SCC 调度。

- [ ] **Pass 08: World / Send / Escape / Capability Check**
	- **TODO**: 检查 `return`、`break`、`continue`、loop tag、tail position、escape/promotion、`send`/`!send`、world boundary、thread boundary、world-local、`UnsafeRef`、Atomic。
	- **DESC**: 这里把 continuation 与并发/世界边界彻底隔开：continuation 默认不能 send，不能跨 thread/world；ordinary closure 的 send 分类与 captured values 相关；escape 规则先保守，允许后续放宽。
	- **验收**: 非法 escape、非法 send/capture、错误 loop tag、world-local 泄漏、Atomic capability 错误都能报错；TypedAst 标注 tail-call sites 和 arena/promotion facts。
	- **并行**: 函数体级并行；跨函数 escape 摘要需要 fixpoint，按 call graph SCC 并行调度。

- [ ] **Pass 09: Usage Analysis 0: High-Level Core**
	- **TODO**: 在 typed/alpha core 上统计 binder、lambda、closure、continuation 的 `0 | 1 | many` 使用情况，并标记 escaping、immediate-call、multi-resume 需求。
	- **DESC**: 这是 CPS 和 closure 前的必需优化事实，不是后端优化。没有 usage facts，就无法安全地区分 single-use continuation、真正 multi-resume continuation、可 direct call lambda、必须实体化 closure。
	- **验收**: 未使用 binder/lambda 可诊断；immediately-called lambda 标记为 directification candidate；parser alternative 中多次 resume 的 continuation 标记为 many。
	- **并行**: 函数体级并行；只读 typed/alpha core。

- [ ] **Pass 10: Generic Definition Check**
	- **TODO**: 在抽象 generic 参数下检查 generic body，生成 `GenericBodyIR` 和 `ObligationIR`，包括 field/method/operator/shape-dispatch/answer-type/generic-continuation 限制。
	- **DESC**: 这是 checked template 的核心。generic 定义期不是黑盒：普通 HM、row 约束、基本 well-formedness、answer type 入口都必须先过。不能等实例化时才发现 body 本身不可类型化。
	- **验收**: 能拒绝与任何实例无关的 generic body 错误；能保存未兑现 structural obligations；错误消息指向定义点；obligation 可序列化、可 hash、可缓存。
	- **并行**: generic definition 级并行；只读 summaries 和 resolved/canonical type info。

- [ ] **Pass 11: Method / Operator / Dispatch Index**
	- **TODO**: 建立 nominal method index、operator index、shape-dispatch candidate index，并提供按 `(name/operator, nominal id, normalized shape)` 查询的缓存接口。
	- **DESC**: level-1 默认 method resolution 基于 nominal identity，并保留分层候选顺序：field-callable、receiver method、qualified callee。shape dispatch 是独立 structural obligation。`via namespace` 若存在，应放到未来 level-2 的显式行为来源选择，不作为这里的主线机制。这里不做全局 witness search。
	- **验收**: 候选筛选稳定；二义性可诊断；同 shape 不同 nominal type 不被错误合并；查询缓存 key 可复现。
	- **并行**: index 构建可按 namespace 并行后 merge；查询缓存可并发读，写入用 content-addressed key 去重。

- [ ] **Pass 12: One-Pass CPS Transformation + Beta Reduction**
	- **TODO**: 用 meta-continuation 做 CPS lowering，并在变换过程中 beta-reduce administrative redex。
	- **DESC**: 这是 level-1 中端核心。普通表达式求值顺序产生的 administrative continuation 默认不实体化；只有 call/switch/prompt/control/multi-resume 等语义控制点进入 CPS Core。这个 pass 支撑 compiler lowering、chibalex backtracking、chibacc recovery 的共同实现模型。
	- **验收**: atom lowering 不产生额外 continuation；single-use administrative continuation 被直接 beta-reduce；`reset` / `shift` 和 multi-resume continuation 在 CPS dump 中保留清晰语义节点。
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
	- **DESC**: closure conversion 解决 lexical binding 到 runtime env 的边界。no-capture lambda 可以直接化；escaping closure 才需要 wasm-gc env package；continuation package 与 closure env 要共享 capture legality facts。
	- **验收**: top-level/nested/direct/unknown callee 路径可 dump；free vars 顺序稳定；no-capture lambda 不分配 env。
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
	- **DESC**: monomorphize key 建议为 `(generic_symbol, concrete_type_tuple, normalized_shape_tuple, builtin_capability_facts, abi_mode)`。若未来 level-2 恢复显式 `via namespace` 行为来源，再把它作为扩展 key 维度加入。定义期 checked 过的 body 不重做全量 HM，只兑现 concrete obligations：字段、method、operator、shape dispatch、dyn adapter packaging、continuation capability facts。
	- **验收**: 同一 key 只实例化一次；不同 namespace 同时请求同一实例不会重复产物；实例化错误报在 call site，同时保留定义点 note；递归 generic 通过 in-progress marker / SCC worklist 处理。
	- **并行**: 高并行。每个 specialization 是独立任务；全局只允许原子注册 `key -> artifact/status`，禁止实例化任务修改共享语义表。

- [ ] **Pass 20: Wasm-GC Core Lower + Layout**
	- **TODO**: 把 optimized CPS/closure core / specialization 降到 wasm-gc-friendly `CoreIR`，完成 method/operator direct target、extern direct target、closure env、continuation frame/package、record/data/union/string/slice/Array layout。
	- **DESC**: 这里决定 wasm-gc 表示：struct/array、tag、field offset、closure env、multi-resume continuation package、dyn package adapter、不可变 `Array[T]` 布局、WASI/thread boundary ABI、JS embedder `env` ABI。`extern "wasi" "symbol"` 在这里降为 WASI typed import ref + direct call target；`extern "C" "symbol"` / `extern "c" "symbol"` 降为 `env` typed import ref + direct call target。后端 emitter 不再做语义选择。
	- **验收**: layout hash 稳定；同一 summary + same specialization key 生成同一 CoreIR；tail position 和 continuation package 可 dump。
	- **并行**: namespace + specialization 级并行；layout table 由 canonical type/layout key 去重。

- [ ] **Pass 21: Wasm-GC Core Validation**
	- **TODO**: 验证 CoreIR 的 symbol/type/layout/block/continuation frame/package 引用完整，以及 replay-safety facts 已兑现。
	- **DESC**: validator 是 backend dumbness 的护栏。它必须确认 multi-resume continuation package 不包含非法 world/thread/unsafe capture，tailcall sites 合法，layout refs 完整。
	- **验收**: CoreIR validator 能拒绝 dangling symbol、错误 layout ref、非法 continuation package、非法 tailcall；错误信息稳定。
	- **并行**: namespace + specialization 级并行。

- [ ] **Pass 22: Wat Emit + wasm-ld Link**
	- **TODO**: 从 CoreIR 直接打印 `.wat` 或 wasm object，使用 wasm-gc、tailcall、wasi、thread features；序列化 `extern "wasi" "symbol"` 对应的 WASI import，以及 `extern "C" "symbol"` / `extern "c" "symbol"` 对应的 `(import "env" "symbol" ...)`；每个 namespace / specialization bundle 可独立产物，最后交给 `wasm-ld` 或等价 linker 链接。
	- **DESC**: emitter 保持 dumb：只序列化已验证 CoreIR，不做优化、不做重型验证。跨 namespace linking 交给 linker，但符号、ABI、layout 在前面 pass 已固定。
	- **验收**: 单 namespace 可独立 emit；多 namespace 能链接；tail position 生成 `return_call` / 等价 tailcall；WASI/thread import/export 名称稳定；`extern "wasi" "fd_write"` 生成的 WASI import 名称和签名可 golden test；`extern "C" "js_log"` 生成的 `env` import 名称和签名可 golden test；wat roundtrip 工具能解析。
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
	- **DESC**: cache key 必须包含 source hash、summary hash、compiler version、target features (`wasm-gc`, `tailcall`, `wasi`, `thread`) 和 canonical extern ABI/import module/signature hash，例如 `wasi::fd_write` 与 `env::js_log` 必须区分，`"C"` / `"c"` 必须归一到同一 key。
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

## 第一阶段 MVP 切片

- [ ] **MVP-A: continuation-aware 非 generic 子集出 wat**
	- **TODO**: 实现 Pass 00-09、Pass 12-18、Pass 20-22 的最小子集，跳过 generic specialization，但不跳过 continuation / closure / lambda lifting。
	- **验收**: 简单函数、record、data、match、`def fd_write(...): ... = extern "wasi" "fd_write"`、`def js_log(...): ... = extern "C" "js_log"`、WASI print/exit、JS/env import smoke、simple reset/shift、multi-resume parser backtracking、no-capture closure direct call 都能生成 wat 并运行。

- [ ] **MVP-B: checked generic + monomorphize**
	- **TODO**: 实现 Pass 10、Pass 19 的最小子集：函数 generic、row field obligation、method obligation、continuation capability facts、specialization cache、最小 dyn packaging hook。
	- **验收**: 两个 namespace 同时实例化同一 generic，只生成一个 specialization；缺字段/缺方法在 call site 报错。

- [ ] **MVP-C: control / escape / tailcall**
	- **TODO**: 加强 Pass 07、Pass 08、Pass 12-14 的 answer type、continuation kind、multi-resume replay-safety、tail position、保守 escape，并让 Pass 22 输出 tailcall。
	- **验收**: tail-recursive 函数生成 tailcall；非法 continuation escape / cross-thread resume 被拒绝；multi-resume continuation 只在 replay-safe 区域通过。

- [ ] **MVP-D: namespace object + wasm-ld**
	- **TODO**: 每个 namespace 生成独立 wat/object，最终用 wasm-ld 链接。
	- **验收**: 修改一个 namespace 只重编该 namespace 和受影响 specializations；最终 linked wasm 通过 smoke test。
