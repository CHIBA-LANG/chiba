这两个目录是 level-1 的 spec 目录
- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/chiba-level1-spec`
- `/home/lemonhx/Desktop/LJVM/chiba-org-web/src/content/type_system`


## Second Bootstrap: remaining work only

这份文件现在只保留**未完成项**。

已完成的 C00-C10、库边界冻结、语义/CPS/closure 主干落地、以及对应 gate 建设，统一视为历史完成项，不再继续堆在这里干扰判断。详细过程保留在 git 历史、`TODO.checkpoint.md` 和各 gate / runner 中。

## P0
- [ ] P0 快速补实现策略：先补 primary lowering 行为，再后补完整 gate；每个补实现 slice 必须在本文件留下 gate TODO，不能用 contract smoke 冒充真实验收。
	- [x] gate TODO：为 C11 backend 增加 synthetic Core/WAT fixture，验证 `return_call` target 不再使用固定 `$chiba.tail_target`；这不是 level-1b source generated-path。
	- [x] source tailcall fixture：`level-1b/supports/pre-c11-smokes/source_primary_backend.chiba` 已覆盖 `callee()` -> `main_noarg_tail()`，C11 runner 生成 `.scratch/level-1b/c11-backend/source-primary-backend.wat` 并用 wasmtime 验证 `return_call $chiba.callee` 与返回 42。
- [ ] chibalex/chibacc frontend：chibalex token primary path 已开始替换 scanner fallback；chibacc mini AST evidence 已进入 C07 fact shape，完整 chiba-level1 source AST 仍未接管 primary path。
	- [x] source parser facts blocker：`SourceProjectFacts.parser` 已拆分 `token_primary_path_ready` 与 `ast_primary_path_blocked`，不再让 scanner fallback 冒充 parsed AST。
	- [x] chibalex/chibacc lowering/codegen status：C05/C06 已移除 `ContractOnly`/`missing-lowering` codegen 状态，lowering 从 AST 生成 typed IR，codegen 产出可检查入口文本；mini generated lexer/parser 已经经 `level1c.o wat` 生成可查看 WAT 并实际执行，C07 已新增 `SourceFrontendEvidence` 合并入口并校验 chibacc-mini AST primary evidence；完整 chiba-level1 source facts AST primary path 仍未接管。
	- [x] chibalex parser builtin removal slice：`std.chibalex.parser` 的 top-level namespace / `@regex =` / token rule action+transition 解析已从 `std.chibalex_parse*` compiler builtin 迁到 source-level parser helper；完整 `.chibalex` rule list assembly 与 regex AST parse 仍 blocked。
	- [x] chibacc parser builtin removal slice：`std.chibacc.parser` 的 namespace / `start` / `rule` name / `pratt` body detection 已从 `std.chibacc_parse*` compiler builtin 迁到 source-level parser helper；mini generated parser execution 已覆盖 simple/pratt/list/continuation/attribute/calculator AST slices，完整 chiba-level1 grammar AST primary 仍 blocked。
	- [x] chibacc Pratt engine builtin removal slice：`parse_pratt_at` 不再调用 `std.chibacc_parse_pratt_at` compiler builtin；mini Pratt parser generated WAT 已执行通过，C07 runner 已消费 `.scratch/level-1b/chibacc-mini/ast-primary-evidence.json`，完整 parser recovery/source-driver AST primary 接入仍未完成。
	- [x] calculator grammar AST evidence：`level1b:chibacc-mini` 已生成并运行 `calculator.exec.exec.wat`，C07 source driver 现在强制要求 `calculator.chibacc` 出现在 AST primary evidence 中，避免 calculator parser slice 回退时假绿；完整 chiba-level1 source facts 仍未由 generated AST 替换 scanner facts。
	- [x] calculator grammar executable AST slice：`calculator.exec.exec.wat` 现在不只做 AST shape check，而是把 `var x := 2 + 3 * 4; x` 的 generated parser AST 驱动到 harness 执行结果 14；C07 要求 evidence 中 `expectedMainResult/mainResult == "14"`，避免 Pratt precedence/parser AST 只靠结构存在假通过。
	- [x] full chiba-level1 generated parser executable slice：`level1b:chibacc-mini` 现在为 `src/frontend/chiba-level1.chibacc` 生成 standalone parser WAT，并额外生成 `.scratch/level-1b/chibacc-full/chiba-level1-parser.exec.wat`，用 token stream `namespace demo def main() = 2 + 3 * 4` 实际调用 full `parse_tokens`，WAT `main -> 14`；C07 消费该 evidence，`run:all-wat` 也运行该 artifact。完整 source-driver AST item facts 接管 scanner 仍未完成。
	- [x] full chiba-level1 namespace action slice：`namespace_decl` action 已从 `Namespace(path as Str)` 改成 `Namespace(path)`，AST data 从 `Namespace(Str)` 改成 `Namespace(AST)`，避免 `Path_Cons` 被 runtime cast 成 `Str`；full parser executable WAT 已覆盖 namespace path + def item，不再 illegal cast。
	- [x] AST primary evidence 非空项数：chibacc-mini evidence 现在记录 `astItemCount`，C07 要求所有 AST-checked cases 都有非零 item count，`source_parsed_modules_all_ast_primary` 也拒绝 `has_ast=true` 但 `ast_item_count=0` 的假 primary；完整 AST item facts 接管 scanner item facts 仍未完成。
	- [x] full parser AST item evidence threading：`SourceFrontendEvidence` / `SourceParsedModuleFacts` 已新增 `ast_namespace_count` 与 `ast_def_item_count`，`source_parsed_modules_all_ast_primary` 要求 item/namespace/def 三类 primary facts 都非空；full grammar JSON evidence 记录 `astItemCount=1`、`astNamespaceCount=1`、`astDefItemCount=1`，C07 强制校验这些字段。真实 `SourceItemScanResult`/owner symbol 仍未由 AST 构造，scanner 仍未删除。
	- [x] full parser AST-owned symbol evidence：full grammar executable evidence 现在记录 parser-owned `demo::main`，`SourceFrontendEvidence` 线程 `ast_owner_namespace` / `ast_def_item_name` 到 `SourceAstOwnerSymbolFact`，C07 强制校验该 symbol 来自可运行 full parser WAT；真实 `SourceItemScanResult` replacement 与 scanner 删除仍未完成。
	- [x] AST evidence source-gate presence slice：C07 source semantic gate 现在允许 parser-owned `SourceAstOwnerSymbolFact` 证明 namespace/item presence，不再必须由 byte scanner 提供 namespace/item 才能越过 presence gate；scanner 仍负责 use/attributes/surface/body coarse facts，完整 `SourceItemScanResult` replacement 仍未完成。
	- [x] AST-owned symbol reaches alpha/typed：`AlphaModule` 现在保留 `AlphaAstOwnerOrigin`，C08 typed module 线程 `TypedAstOwnerSymbolFact`，把 chibacc parser-owned `demo::main` 从 C07 source facts 推到 C08 typed fact surface；完整 typed item skeleton/body traversal 仍未由 AST 替换。
	- [x] AST-owned add/mul function typed fact：full parser executable evidence 现在把 `demo::main = 2 + 3 * 4` 的 body fact 线程到 `SourceAstOwnerSymbolFact` / `AlphaAstOwnerOrigin` / `TypedAstOwnerSymbolFact`，C08 生成 parser-owned `TypedFunctionFact`，不再只能从 scanner body slice 得到 `main=42`。
	- [x] AST-owned typed main C11 artifact：C09/C11 contract 已锁住 parser-owned typed function facts 进入 CPS/Core 的路径；C11 读取 full parser evidence 生成 `.scratch/level-1b/c11-backend/ast-primary-typed-main.wat`，并用 wasmtime 验证 parser-owned `demo::main -> 14`。完整任意 AST expression/body lowering 仍未完成。
	- [x] full grammar expression execution blocker cleared：full generated parser 现在为 `2 + 3 * 4` expr-rule harness 生成/compile/run WAT，C07 记录 `expressionExecutableWat` 且要求 `expressionMainResult == 14`；`run:all-wat` 跳过旧 split debug artifacts，只运行 combined executable WAT。完整 full AST expression primary replacement 仍未完成，因为 C07/C08 source facts 还没有从 parser AST item/body 全量构造。
	- [x] full grammar expression entry truthfulness slice：`level1b:chibacc-mini` 不再把 source-file `main` 结果复用成 expression evidence；full parser executable WAT 现在额外 export `expr_main`，直接调用 generated pratt entry `parse_rule_176` 并验证表达式 AST `2 + 3 * 4 -> 14`。C07 继续消费该独立 `expressionMainResult`。完整 source-driver AST item facts 接管 scanner 仍未完成。
- [ ] UTF8, `level-1b/compiler/semantic/adt_tuple_lowering.chiba`, ADT ctor tag canonicalization 当前按 ASCII byte 做 BigCamel -> snake_case，UTF-8 ctor 名会逐 byte 变成 `_` 或错误分词；fix：ctor tag 由 chibacc AST 的 UTF-8/XID identifier token 派生，使用 Unicode-aware case fold / word-boundary 规则，或在该规则落地前对非-ASCII ctor fail-closed。
	- [x] C08-C11 已线程 `requires_utf8_identifier_lowering`，UTF-8 ADT ctor identifier 不能被当作普通 ASCII tag lowering 静默通过；真实 Unicode-aware case fold / word-boundary 仍未完成。
- [ ] UTF8, `level-1b/compiler/source/scan.chiba`, C07 transition scanner 仍用 ASCII `source_is_ident_start/continue`、uppercase ctor scan、byte-level item/name scan；day0 UTF-8 identifier 会被漏扫或误分类；fix：让 chibalex/chibacc 成为 source facts primary path，scanner fallback 遇到非-ASCII identifier/source semantic region 必须 fail-closed，不得产完整 facts。
	- [x] C07 已新增 UTF-8 identifier fixture（namespace / data ctor / function name），当前明确 blocked 于 chibalex/chibacc primary parser execution，避免 scanner fallback 被误当 day0 UTF-8 支持。
- [ ] UTF8, `level-1b/compiler/semantic/type_infer.chiba`, C08 minimal body/callee/data-ctor scanner 复用 ASCII source scanner 和 `byte_at` identifier 判断，tail-call target、namespace-qualified symbol、data ctor arity/name 在 UTF-8 下不可靠；fix：改为消费 parser AST/typed identifier facts，临时路径遇到非-ASCII owner/name/callee/ctor 时 fail-closed。
	- [x] C08 UTF-8 scanner fallback fail-closed：`type_infer` 现在遇到 source UTF-8 warning 且 chibacc AST primary 仍 blocked 时返回 `missing-facts: UTF-8 typed identifier facts require chibacc AST primary path`，避免 ASCII scanner-derived typed facts 静默进入 C08。
- [ ] UTF8, `tools/node/run-level1b-chibalex-mini.mjs`, mini lexer runner 只用 UTF-8 lead-byte 宽度推进，XID 表与 invalid sequence validation 仍是近似；fix：接入真实 `std.regex.utf8` / Unicode XID table 生成物，并补 invalid UTF-8 / combining mark / non-ASCII keyword-boundary fixtures。
	- [x] chibalex mini XID table-backed slice：mini runner 已停止使用 `cp == 233/955/769` 手写 codepoint 特例，改为读取 `level-1b/std/regex/xiddata.chiba` 并要求相关 Unicode 15.1 XID ranges 存在后生成 standalone predicate；fixtures 覆盖 `café`、`λ`、combining mark、invalid continuation，生成 lexer WAT 已执行通过。完整 primary lexer 仍需直接调用共享 `std.regex.utf8`/`xiddata`，不能长期靠 runner 展开 subset。
- [ ] UTF8, `level-1b/std/regex/utf8.chiba`, regex UTF-8 helper 已有 codepoint boundary，但 Unicode property / XID / invalid-sequence policy 仍未成为 parser+lexer 共享真源；fix：把 UTF-8 decode、XID_Start/XID_Continue、boundary、invalid sequence diagnostics 提升为 std/frontend 共享模块。
	- [x] shared UTF-8 validation/decode/XID wrapper slice：`std.regex.utf8` 现在提供 `str.utf8_valid_at`、`str.utf8_codepoint_at`、`str.is_xid_start_at`、`str.is_xid_continue_at`，统一处理 overlong/surrogate/out-of-range/invalid continuation 后再查 Unicode 15.1 XID 表；XID-at-offset 不再依赖 `str.char_at` builtin。C04 regex 与 chibalex UTF-8 mini WAT 已通过。完整 chibalex/chibacc primary 仍需直接调用该共享 helper，不能长期保留 runner 内联 decode。
- [ ] attribute grammar：#[attr(...)] nested/named/list args 没 spec/golden/AST/parser。
	- [x] attribute spec/golden：spec 已定义 AttrArg AST 与 nested/named/list/object grammar，并新增复杂 attribute fixture/gate。
	- [x] chibalex attribute token fixture：lexer golden 覆盖 `#`/bracket/delimiter/literal/ident token 流，不折叠 legacy attribute token；generated lexer WAT execution 已覆盖 complex attribute token stream。
	- [x] chibacc attribute AST/fixture：`std.chibacc.ast.AttrArg` 与 mini grammar 已覆盖 bare/named/call/list/object；generated parser WAT execution 已覆盖 attribute args AST slice。
	- [x] chibacc attribute native oracle：`attribute-args.chibacc` 已纳入 chibacc-mini native oracle，generated parser WAT execution 已通过；完整 chiba-level1 source parser AST primary path 仍未接管。
- [ ] namespace resolution：只有扫描和 owner slice；真实 import/name resolution、owner namespace symbol、visibility/private、冲突规则没做。
	- [x] namespace ownership facts：alpha/typed skeleton 已把 item owner namespace 线程化进 `TypedModule.namespace_ownership`；import/name resolution 仍未完成。
	- [x] namespace import facts：source `use` 已经穿过 alpha 进入 `TypedModule.namespace_imports`；真实 visibility/private/conflict/name lookup 仍 fail-closed。
	- [x] namespace resolution obligations：source import scopes 已生成 visibility/conflict/prelude obligations；真实 symbol lookup/private check 仍未完成。
	- [x] function symbol uniqueness：C08 已拒绝重复 owner-qualified function symbol；完整 import/name lookup conflict matrix 仍未完成。
	- [x] namespace resolution fact slice：C08 已生成 `NamespaceResolutionFact`，把 item owner、file import count、current-namespace-priority、private visibility check、ambiguous import reject policy 串进 typed module；真实 symbol table lookup、private 跨 namespace 拒绝、use/prelude conflict matrix 仍未完成。
- [ ] method resolution：没做。method index、receiver binding、qualified callee、Self with generics 都缺。
	- [x] method surface facts：source scan 已抽取 `def Receiver.member` 的 receiver/member slices，后续 method index 不再只靠 boolean。
	- [x] typed method/operator facts：typed module 已聚合 `MethodSurfaceFact`，保留 owner、receiver、member、operator 标记。
	- [x] method/operator index surface entries：index builder 已把 `MethodSurfaceFact` 转成稳定 `MethodOperatorSurfaceEntry`；nominal receiver binding/candidate lookup 仍未完成。
	- [x] method/operator nominal keys：index builder 已从 receiver/member source slice 生成 `MethodKey` / `OperatorKey`，并按 qualified receiver 拆 namespace/name；真实 `Self` binding、generic receiver、candidate lookup、visibility 仍 fail-closed。
	- [x] no-method index baseline：无 method/operator declarations 的 module 现在得到空 MethodOperatorIndex；有 method surface 仍 fail-closed。
	- [x] method `Self` binding fact slice：method/operator index 已生成 `MethodSelfBindingFact`，把 receiver nominal、member、`Self := receiver`、generic receiver surface 串到 index；完整 generic receiver substitution、visibility、namespace-aware candidate lookup 仍未完成。
	- [x] method candidate visibility slice：`MethodKey` / `OperatorKey` 现在保留 declaration file / owner namespace / private，method candidate lookup 会过滤 private 跨 namespace candidate，并对不可见 private method 产出 hard diagnostic；完整 import/prelude resolution matrix、generic receiver substitution、method body instantiation 仍未完成。
- [ ] operator overloading：已进入 method-protocol candidate path；完整 overload resolution 仍未完成。
	- [x] operator method surface facts：`.op_*` 现在和普通 method 共用 receiver/member slices；真实 overload resolution 仍未完成。
	- [x] operator use surface facts：typed module 已记录 infix/operator/index/slice use surface；真实 candidate collection、operand matching、ambiguity 仍 fail-closed。
	- [x] operator resolution obligations：`OperatorUseSurfaceFact` 已转成稳定 overload-resolution obligation；candidate collection / operand matching / ambiguity 仍未完成。
	- [x] operator candidate-set facts：method/operator index 已为每个 operator use 绑定当前 operator declaration candidate set，并显式要求 operand matching / ambiguity check；真实按 receiver/operand 过滤、missing/ambiguous/wrong-operand 报错仍 fail-closed。
	- [x] operator kind threading：source/typed/operator obligation 已线程 `+ - * / [] [..]` 到 `op_add/op_sub/op_mul/op_div/op_index/op_index_slice` method-style member key。
	- [x] operator candidate diagnostics：candidate set 已产出 missing / ambiguity / operand arity / operand type / unknown-operator 诊断事实。
	- [x] operator single-candidate resolution：当前 operator use 若按固定 operator name 只找到一个 declaration candidate，不再报 operand-matching pending；多候选 ambiguity 与缺失候选仍 fail-closed。
	- [x] operator receiver/operand filtering slice：infix operator 会从 typed param slice 抽 receiver/right operand 类型并要求同型；`[]`/`[..]` 会校验 index/start/len 是整数索引族；完整多候选 trait/interface 约束、generic operand unify、namespace visibility 仍未完成。
	- [x] builtin intrinsic receiver whitelist：C08 method/operator index 明确只有 `u8/u16/u32/u64/i8/i16/i32/i64/f32/f64/Vec/Array/Slice/String/str` 的 operator key 可以走 compiler-provided intrinsic body；其他用户 nominal type 必须通过普通 method/operator declaration candidate，不得被 builtin key 吃掉。
- [ ] typed AST elaboration：现在很多是 skeleton/facts；真实 expression/type/item traversal 不完整。
	- [x] typed item skeleton threading：`TypedModule.items` 保留 alpha 后的 item skeleton，后续 method/operator/branch/control pass 不再只能重扫 source。
	- [x] typed elaboration obligations：`TypedElaboration` 现在携带 item/type/body traversal obligations；真实 typed AST expression lowering 仍未完成。
	- [x] minimal codegen typed function fact：C08 现在为无复杂 surface 的简单 `def` 生成 `TypedFunctionBodyFact`，能保留 `main` 与 `i32.const 42` 竖切；完整 type expression / expression AST traversal 仍未完成。
		- [x] minimal tail-call body fact：C08 现在能把极窄 `callee()` body 标成 `TypedFunctionTailCall(target)`，用于验证 CPS tail-call lowering；完整 typed call graph / namespace-aware callee resolution 仍未完成。
		- [x] minimal i32 param/tail-call fact：C08 现在能保留 `id(x: i32)=x` 与 `main=id(42)` 这类一参 i32 tail-call 竖切；完整 param AST / call argument lowering 仍未完成。
		- [x] minimal param0 tail-call fact：C08/C11 现在能保留 `forward(x: i32)=id(x)` 并 emit `local.get 0 return_call $target`；source-primary WAT 已覆盖多参 tailcall 与混合 param/const 参数，复杂表达式参数仍未完成。
		- [x] minimal callee symbol check：极窄尾调用 target 必须能在当前 typed function set 中解析到同名函数；同 namespace 裸调用会解析成 owner-qualified symbol，完整 import/mangle-aware call graph 仍未完成。
		- [x] minimal i32 const call exactness：`f(42)` 窄 tail-call slice 现在要求 const arg 精确闭合且处于尾位置，避免把 `f(42 + x)` / `f(42, y)` / `f(42) + 1` 误编译成 `f(42)`。
		- [x] minimal small i32 const call exactness：`f(7)` 这类非 0/42 small const tail-call arg 现在保留为统一 `TypedI32ConstLiteral` 并可穿到 runnable WAT，避免 source-slice backend 把 small const 漏掉或降成 0。
		- [x] minimal no-arg call exactness：`f()` 窄 tail-call slice 现在要求空参数精确闭合且处于尾位置，避免把 `f() + 1` 误编译成 `f()`。
		- [x] minimal param return exactness：`id(x)=x` 窄 return-param slice 现在要求参数名处于尾位置，避免把 `x + 1` 误编译成 `x`。
		- [x] minimal const return exactness：`42` / `0` 常量返回与 branch const arm 现在要求 atom 占满 tail region，避免把 `42 + x` 误编译成 `42`。
			- [x] typed expression extensibility slice：TypedExpr/CPS/Core 已支持 generic small i32 const atom、tail-call param0 arg、nested branch tail-call target validation；source-primary WAT 已覆盖 nested branch / match fallback；仍未替代 chibacc AST 级 expression traversal。
			- [x] typed expression traversal API：C08 现在有 `TypedExprVisit` / `typed_expr_traverse` / `typed_function_expr_visits_all`，C09 `CpsModule.expr_visits` 消费该 DFS traversal；真实输入仍是 source-slice typed expr，等待 chibacc AST primary 替换。
			- [x] AST binary op recursive lowering：`SourceAstExprNodeFact` 的 binary node 现在携带 primitive op ordinal，C08 `typed_expr_from_ast_node` 直接按节点 op 生成 `TypedExprPrimitiveBinary(op, left, right)`；`2 + 3 * 4` 不再依赖 add/mul hardcoded fallback 才保持 `*`。
			- [x] chibacc AST owner no-shape-fallback：C08 对 `from_chibacc` owner 缺少 AST node root 时不再回退到旧 `SourceAstExprShape` hardcode，避免 full parser evidence 丢 node graph 仍假装 typed AST primary 成功。
			- [x] AST primary skeleton priority：C08 在 parser `ast_primary_path_blocked == false` 且有 chibacc AST owner 时优先构造 parser-owned skeleton，scanner item skeleton 只作 fallback，避免 full parser evidence 进入 typed pass 后仍被 byte scanner 主导。
- [ ] generics/template：auto-generic、explicit instantiation、generic body full check 还没完整 primary 实现。
	- [x] generic surface facts：typed module 已记录 explicit params、auto-generic、explicit instantiation、generic Self method surface；真实 template body checking 与 instantiation discharge 仍 fail-closed。
	- [x] generic template obligations：typed pass 已把 generic surface 转成 `GenericTemplateObligation`，区分显式参数、auto-generic、实例化 discharge、generic Self receiver binding。
- [ ] globals/init：module-load init order、dependency/cycle、side-effect init lowering 没完整。
	- [x] global init surface facts：typed module 已记录静态值 initializer/dependency/world_local/ref surface；真实 init order、cycle、module-load lowering 仍 fail-closed。
	- [x] global init obligations：typed pass 已把 init surface 转成 `GlobalInitObligation`；依赖排序、cycle 检测、module-load lowering 仍未完成。
	- [x] global init runtime slice：C08 已生成稳定 `GlobalInitPlanStep`（source-order、prior-global dependency、cycle-free prefix），C11 source-primary WAT 已 emit mutable global + start init，并用 wasmtime 验证 `def ONE: i32 = 7`、`def TWO: i32 = ONE + 35`、`TWO + ONE == 49`；完整任意表达式 init、forward-reference 诊断、side-effect init 仍未完成。
- [ ] pattern/match：deep pattern lowering、exhaustiveness、if let env 规则没完整 primary 实现。
	- [x] pattern surface facts：typed module 已记录 match/if-let/deep-pattern/function-pattern/wildcard-let surface；真实 DFT lowering、exhaustiveness、if-let 失败分支 env 仍 fail-closed。
	- [x] pattern lowering obligations：typed pass 已把 pattern surface 转成 `PatternLoweringObligation`，显式区分 match exhaustiveness、if-let 双分支 env、deep DFT、函数参数 desugar。
	- [x] pattern DFT/exhaustiveness fact stream：pattern pass 现在从 alpha origins 生成 `PatternDftStep` / `PatternExhaustivenessFact` / `PatternEnvFact`，明确 match 缺失静态 case 是 error、`if let` 失败分支不引入 binding；真实 decision-tree/body lowering 仍未完成。
	- [x] typed pattern fact threading：C08 `TypedModule` 现在保留 DFT steps、exhaustiveness facts 与 pattern env facts，后续 CPS/Core 不再只能重扫 source surface。
	- [x] source-primary i32 match runtime slice：C11 source-primary WAT 已把 `match value { 0 => ... 1 => ... _ => ... }` lower 成 `i32.eq` test-chain，并用 wasmtime 验证 0/1/fallback 三路；深 ADT/record/tuple pattern field extract 仍未完成。
- [ ] pipe lowering：placeholder、receiver-first method pipe、operator/method interaction 还没完整 primary lowering。
	- [x] pipe surface facts：typed module 已记录 pipe/placeholder/chain surface；真实 placeholder expansion、receiver-first desugar、operator/method interaction 仍 fail-closed。
	- [x] pipe repeated-placeholder gate：spec 与 checkpoint fixture 已锁定 `a |> f(b,_,_) == f(b,a,a)`；真实 lowering 仍未完成。
	- [x] pipe lowering obligations：typed pass 已把 pipe surface 转成 `PipeLoweringObligation`，并显式记录 repeated `_` 共享一次 lhs eval；真实 expansion/desugar 仍未完成。
- [ ] ADT tuple bridge：Ctor <-> (:ctor, ...)、tuple_to_adt/adt_to_tuple intrinsic lowering 没完整。
	- [x] C08 data skeleton ctor scan：从 `data` item 粗扫 constructor 名与 arity，生成 canonical `LoweredAdtCtor`；payload type 暂用稳定 type-var 占位，完整 type expr lowering 仍未完成。
	- [x] C09/C11 ctor obligation threading：`LoweredAdtCtor` 已进入 CPS/Core obligation；C11 validator 对未 executable 的 ADT ctor tuple helper fail-closed，不再静默丢弃。
	- [x] C11 executable ctor helper slice：ADT ctor lowering obligation 现在会生成 `CoreOpAdtTupleCtor` helper op，WAT emitter 产出 typed tuple struct + `struct.new` helper；`level1b:c11-backend` 已生成 `.scratch/level-1b/c11-backend/adt-tuple-ctor.wat` 并用 wasmtime 验证 tag/payload field read。
- [ ] compiler intrinsic namespace：有部分 contract；真实 intrinsic resolution/lowering 没完整。
	- [x] compiler intrinsic surface facts：typed module 已记录 compiler intrinsic call 与 tuple type surface；真实 intrinsic namespace resolution/lowering 仍 fail-closed。
	- [x] intrinsic lowering obligations：typed pass 已把 compiler intrinsic/tuple bridge surface 转成 `IntrinsicLoweringObligation`，固定 `compiler.intrinsic` namespace；真实 resolution/lowering 仍未完成。
	- [x] intrinsic identity split：C08 已区分 `tuple_to_adt` / `adt_to_tuple` / `unsafe_cast`，tuple bridge 绑定 `AdtTupleIntrinsicIdentity`，`unsafe_cast` 标记 compiler-only；真实 namespace lookup 与 executable lowering 仍未完成。
	- [x] intrinsic bridge C09/C11 threading：`IntrinsicLoweringObligation` 现在保存在 `TypedModule`，并进入 `CpsIntrinsicBridgeFact` / `CoreIntrinsicBridgeLoweringObligation`；C11 validator 校验 `compiler.intrinsic` namespace，避免 tuple bridge identity 停在 C08 surface 后丢失。真实 typed identity roundtrip / executable conversion lowering 仍未完成。
	- [x] intrinsic bridge typed roundtrip fact：C08 现在对同一 owner 内同时出现 `tuple_to_adt` 与 `adt_to_tuple` 的 bridge 生成 `IntrinsicBridgeRoundtripFact`，记录双向 `AdtTupleConversion` 与 `preserves_identity`；C09/C11 已线程到 `CoreIntrinsicBridgeLoweringObligation.has_typed_roundtrip`，C11 validator 会拒绝双向 bridge 丢失 typed roundtrip evidence。
	- [x] intrinsic bridge executable identity helper slice：C11 会把有 typed roundtrip evidence 的 tuple bridge 降成 `compiler.intrinsic.tuple_to_adt` / `adt_to_tuple` eqref identity conversion helpers，并要求 `emits_identity_conversion_helpers`；`adt-tuple-intrinsic-roundtrip.wat` 已验证 tuple -> ADT -> tuple roundtrip 后 payload 保持 41。真实 namespace lookup 与非 identity representation conversion 仍未完成。
- [ ] answer/control scan：reset/shift/shiftn answer facts 没做。
	- [x] control surface detection：source/typed surface 已标记 reset/shift/shiftn；无 delimited-control syntax 的 module 可越过 answer_control，出现 control syntax 仍 fail-closed。
	- [x] control surface facts：`TypedModule.control_surface` 聚合 reset/shift/shiftn owner facts，answer_control 以后消费该事实流。
	- [x] answer/control obligations：control surface 已转成 reset/shift/shiftn answer obligations；真实表达式级 answer scan 仍未完成。
	- [x] reset/shift/shiftn CPS body facts：C09 已从 `ControlBodyFact` 生成 `CpsControlTermFact`，区分 reset inline、shift Cont1 direct、shiftn ContN package；真实 expression answer type inference 与 continuation body lowering 仍未完成。
- [ ] usage analysis：只有粗 fact threading；真实 binder/lambda/closure/continuation usage count 没做。
	- [x] closure/lambda surface facts：source/typed facts 已记录 lambda 与 trailing closure surface，usage/capture analysis 不再只能靠 callable storage 猜。
	- [x] CPS closure usage subjects：CPS usage pass 已把 lambda/trailing closure surface 转成 `UseSubjectLambda` / `UseSubjectClosure` facts；真实 capture/use-count traversal 仍未完成。
	- [x] no-continuation usage path：无 callable/continuation obligations 的 module 现在可通过 usage/boundary/replay no-op path；有 continuation facts 仍 fail-closed 等真实分析。
- [ ] continuation boundary：world/thread/send boundary 检查没做。
	- [x] continuation boundary obligations：usage facts 已转成 world/thread/send boundary scan obligations；真实表达式级 boundary scan 仍未完成。
- [ ] ContN replay safety：capture classification、Ref[T] shared-reference、non-replay reject 没做。
	- [x] ContN replay obligations：UseMany continuation usage 已转成 `ContNReplaySafetyObligation`，明确 Ref[T] capture 是 shared-reference semantics；真实 capture classification/non-replay reject 仍未完成。
- [ ] one-pass CPS + beta：没做。现在还是 fail-closed，不是 level0 那条真实 CPS。
	- [x] no-continuation CPS baseline：无 control obligations 的 module 现在生成空 `CpsModule`；真实 expression CPS + beta 仍未完成。
	- [x] CPS tail-form invariant facts：无 multi-shot continuation 的普通函数现在生成 `CpsTailFormFact`，明确 `all_non_multishot_calls_are_tail`；真实 call graph CPS / administrative beta 仍未完成。
	- [x] CPS tail-call fact：`TypedFunctionTailCall(target)` 会在 C09 标成 `CpsTailCall(target)`，普通非 multi-shot 调用必须保持尾位置。
	- [x] CPS multi-shot exception fact：ContN continuation facts 会追加 `CpsMaterializedMultiShotFrame`，把 multi-shot frame/package 作为尾递归规则的显式例外。
	- [x] CPS pattern decision facts：typed DFT pattern facts 现在进入 `CpsPatternDecisionFact`，显式要求 deep pattern lower 成 test-chain + if/else + field extract，并保留 if-let 失败分支无 binding 与 match 缺失静态 case error。
	- [x] CPS atom/arg threading：one-pass CPS fact stream 已能把 typed i32 const、param0、tail-call arg、branch term 递归线程到 CpsTerm；真实 reset/shift 物化 continuation body 与 administrative beta 仍未完成。
	- [x] CPS typed traversal consumption：C09 `CpsModule.expr_visits` 保留 typed DFS visit stream，后续 primary AST expression lowering 不再需要重扫 source slice；administrative beta 仍未完整。
- [ ] CIR nanopass IR：清晰分层还不够；需要语言级 CPS/CIR facts，且保证不耦合 Wasm。
	- [x] CIR/backend boundary gate：compiler/ir/control/closure 不允许出现 Wasm/WAT/Binaryen/funcref/eqref/backend opcode；backend 细节只允许在 backend 层。
- [ ] closure conversion：no-capture direct、capturing env、env field extraction、call rewrite 没完整。
	- [x] no-continuation closure usage path：无 runtime closure/continuation subjects 的 CPS module 可通过 C10 usage/simplification 空事实路径；真实 closure extraction 仍未完成。
	- [x] closure capture blocker：lambda/closure CPS usage subjects 不再空 layout 假通过；无可用 capture facts 时必须经 conservative capture/env extraction path 或后续 primary free-var extraction。
	- [x] syntactic no-capture direct path：无明显 capture surface 的 lambda/closure 会生成 empty env layout，并在 env simplify 降成 `ClosureNoCaptureDirect`；capturing env field extraction 与 call-site rewrite 仍未完成。
	- [x] conservative capture/env extraction path：C10 现在为非 no-capture lambda/closure 生成非空 `ClosureEnvLayout`，ContN frame 生成非空 replay-safe capture EnvField；真实 free-var set、field projection、call-site rewrite 仍未完成。
	- [x] closure call_ref executable slice：C11 `CoreOpClosureCall` 不再 emit 空 `(param funcref) (param eqref)` stub，WAT emitter 生成 typed `call_ref` dispatch shape；`.scratch/level-1b/c11-backend/closure-call-ref.wat` 已用 wasmtime 验证 funcref 调用返回 42。真实 env field projection / call-site direct rewrite 仍未完成。
- [ ] Cont1 lowering：direct resume / boxed one-shot consumed state machine 没完整。
	- [x] Cont1 one-shot obligations：Cont1 simplification 已生成 direct resume vs boxed state-machine obligation，并显式要求 consumed state。
	- [x] Cont1/ContN capture plans：C10 现在为 materialized continuation 生成 `ContinuationCapturePlan`，区分 boxed Cont1 consumed-state、ContN frame body extraction，并固定 Ref capture shared-reference semantics；真实 capture scan/body extraction 仍未完成。
	- [x] boxed Cont1 WAT consumed-state slice：C11 `CoreOpBoxedCont1` 不再 emit `i32.const 0` fake resume，WAT emitter 会读取 consumed field、重复 resume 时 `unreachable` trap、首次 resume 后写 consumed；`.scratch/level-1b/c11-backend/boxed-cont1-state-machine.wat` 已验证首次 resume 返回 41、第二次 resume trap。真实 frame body resume/capture projection 仍未完成。
- [ ] ContN lowering：stackless resume frame、frame chain、repeatable package 没完整 primary。
	- [x] ContN frame-chain obligations：packaged ContN simplification 已生成 stackless resume function / frame chain / repeatable package obligation；真实 frame extraction 仍未完成。
	- [x] ContN non-zero frame fact：`ContinuationPackaged` lowering fact 现在会携带至少一个 stackless resume frame shell，避免 C11 收到 zero-frame repeatable package；真实 capture/frame body extraction 仍 fail-closed。
	- [x] ContN frame-chain primary shell：C10 不再因 materialized continuation capture blocker 提前停止，packaged ContN 会生成空 capture frame shell、frame chain、repeatable package 进入 C11；真实 capture set / frame body extraction 仍未完成。
	- [x] ContN stackless resume body threading slice：C10 现在从 `CpsControlTermFact` 为 frame resume 标记 `StacklessResumeBodyShiftN` 等 body kind，C11 `CoreOpStacklessFunction` 不再 emit 空 `(func)` shell；真实 resume body expression lowering / capture projection 仍未完成。
	- [x] ContN replay capture classification threading：C10 continuation frame capture 不再硬编码为 replay-safe continuation，已消费 C09 replay-safety facts，把 shared `Ref` capture 线程成 `CaptureSharedRefCell` / EnvField；真实 free-var capture scan、field projection、non-replay state reject 仍未完成。
	- [x] ContN package allocation slice：C11 `CoreOpContinuationFrameChain` / `CoreOpContNPackage` 不再 emit 空 param stub，WAT emitter materialize frame-chain/package layouts；现有 `contn-package.wat` 与 `continuation-frame-body.wat` 已验证 repeatable resume smoke。真实 captured frame body / projection / replay legality 仍未完成。
- [ ] Core/block lowering：CPS/CIR 到 backend-neutral block/core 还没真实 executable path。
	- [x] Core block obligations：Core lowering 现在产出 block terminator / return / tailcall obligations；真实 executable block emission 仍未完成。
	- [x] Core pattern decision obligations：C11 现在从 `CpsPatternDecisionFact` 生成 backend-neutral `CorePatternDecisionLoweringObligation`，保留 test-chain / if-else / field-extract / exhaustiveness-error 需求；真实 executable decision tree emission 仍未完成。
	- [x] Core pattern decision fail-closed：C11 validator 会拒绝尚未 executable lowering 的 pattern decision obligation，避免 match/if-let 义务被静默丢弃后继续 emit WAT。
	- [x] typed function Core op：C11 现在会从 C08 typed function body facts 生成 `CoreOpFunction`，并保持 owner / export-main / body fact；真实 block body lowering 仍只覆盖常量返回竖切。
	- [x] ordinary CPS tail invariant：普通 `CoreOpFunction` 现在必须 lower 成 `CoreBlockReturn` / tail terminator，validator 拒绝 non-tail fallthrough；`ContN` frame/package 仍是 materialized multi-shot 例外。
	- [x] function-body terminator selection：C11 block obligation 现在按 `CoreFunctionBody` 区分 return / tail-call / pending fallthrough，不再用 `CoreOpFunction.tail = CoreTailCall` 假装全函数 tail-safe。
	- [x] param0 body validation：C11 validator 会拒绝读取 `local.get 0` 但没有 i32 param 的 function body，包括 return-param 和变量条件 branch/tailcall。
- [ ] Wasm-GC backend layout：layout 有一部分；真实 lowering 到 Wasm-GC object/funcref/eqref 不完整。
	- [x] heap allocation WAT slice：C11 `CoreOpStructNew` / `CoreOpArrayNew` 不再返回 `ref.null eq`，改为 `struct.new` / `array.new` materialization；`.scratch/level-1b/c11-backend/heap-allocation.wat` 已验证 struct non-null 与 array length。真实 typed struct/array field layout integration 仍未完成。
	- [x] erased continuation package WAT slice：C11 `CoreOpContinuationPackage` 不再 emit 空 param stub，改为 tag + payload `LayoutContinuationPackage` allocation；`.scratch/level-1b/c11-backend/erased-continuation-package.wat` 已验证 tag/payload field read。真实 erased callable ADT dispatch 与 storage/call-site lowering 仍未完成。
- [ ] WAT emit：没有 executable WAT；现在 fail-closed。
	- [x] empty Core WAT baseline：空 validated Core module 现在 emits 最小 `(module)`；真实 Core op WAT lowering 仍 fail-closed。
	- [x] WAT emission obligations：非空 Core ops 现在生成 executable instruction/layout serialization obligations，不再只靠笼统非空判断。
	- [x] non-empty Core WAT skeleton：已验证 Core op 现在会序列化 canonical Wasm-GC layout type 与 per-op runtime stub，不再整体 fail-closed 于 `executable WAT emission absent`；真实 typed Core body / continuation frame body / chibac-next smoke 仍 blocked。
	- [x] exported main constant slice：`CoreOpFunction` 可 emit `(export "main")` 且返回 `i32.const 42`；真实 source parser execution / wasmtime end-to-end smoke 仍 blocked。
	- [x] minimal function WAT parse smoke：C11 gate 现在用 Binaryen parse/validate canonical layout + exported `main` 常量返回 WAT，避免 WAT skeleton 语法回归。
	- [x] tailcall WAT parse smoke：C11 gate 现在用 Binaryen parse/validate `return_call` WAT；真实 typed call graph / tail-position lowering 仍 blocked。
	- [x] minimal tail-call WAT emission：`CoreFunctionTailCall(target)` 现在 emit `return_call $chiba.<target>`，用于锁住尾调用代码生成形状；完整 namespace/mangle resolution 仍未完成。
	- [x] minimal param tail-call WAT emission：C11 可 emit 一参/多参 i32 function、`local.get n` 与 `i32.const` mixed args 的 `return_call $target`；复杂表达式参数与普通 non-tail call 仍未完成。
	- [x] param tail-call Core/WAT threading：CPS/Core/WAT 已区分 `return_call f(x)` 与 `return_call f(x, y, const)`，不再把 param arg 误当 `i32.const 0`；复杂 arg lowering 仍未完成。
	- [x] Core tail-call symbol validation：C11 validator 已拒绝未定义的 `CoreFunctionTailCall(target)`，避免悬空 `return_call` 进入 WAT emission。
	- [x] Core duplicate symbol validation：C11 validator 已拒绝重复 function symbol，避免 namespace/callee lowering 生成 ambiguous WAT labels。
	- [x] standalone tail-call fake path removed：`CoreOpTailCall` 不再 emit `i32.const 0 return` fake function；真实 tail-call 只能通过 `CoreOpFunction` body 的 `CoreExprTailCall*` 进入 `return_call $target` emission，孤立 op 现在返回 backend diagnostic。
	- [x] ContN package WAT parse smoke：C11 gate 现在 parse/validate stackless resume + frame-chain/package layout WAT，明确 multi-shot continuation 是 materialized 例外；真实 capture/frame body extraction 仍 blocked。
	- [x] C11 runnable WAT artifact gate：`level1b:c11-backend` 现在写出 `.scratch/level-1b/c11-backend/*.wat` 与 `.wasm`，并用 wasmtime invoke 覆盖 const、small const、param、branch、tailcall、branch-tailcall、ContN/closure shell；真实 source AST primary path 与 ContN frame body extraction 仍 blocked。
	- [x] C11 source-slice artifact gate：`level-1b/supports/pre-c11-smokes/source_primary_backend.chiba` 现在经窄 source-slice lowering 生成可查看/可运行 WAT，覆盖 `id(x)=x`、small const、`main=id(7)`、literal branch、param branch tailcall；完整 chibacc AST primary path 仍 blocked。
	- [x] stackless resume WAT body threading：C11 stackless resume op 会消费 `StacklessResumeBodyKind` 生成函数 body，不再输出空函数壳；真实 ContN replay frame body 仍是下一层 blocker。
	- [x] backend fake-stub cleanup slice：C11 source contract 已拒绝 boxed Cont1 常量返回、ContN 空 param stub、erased continuation package 空 stub、closure 空 call stub、heap allocation null stub、standalone tailcall `i32.const 0` fake success；`level1b:c11-backend` 现产出 25 个可查看 WAT/wasm artifacts，并包含 boxed Cont1 trap、closure call_ref、heap allocation、erased package、ContN smoke。

- [ ] `level-1b/compiler/source/scan.chiba` 只是 truthfulness scaffold，不是语言设计或未来 frontend；第一优先级是让 chibalex token stream + chibacc AST 成为 C07 source facts 的 primary path，然后删除或降级 `scan.chiba`，不能让 byte-level scanner 长期承担 namespace/item/attribute 语义。
- [ ] attribute grammar 必须先补齐 spec/golden，再实现：
	- 已知 spec：`attributes.md` / `item-attrs.md` 规定统一 `#[attr]`、`#[attr(...)]`、file-level `#![...]`，attribute 至少可挂在函数定义、statement/expression、field/data variant、variant tuple type、namespace、block/unsafe block、lambda、call-site trailing closure 前。
	- 已知缺口：chibalex spec 示例仍只有 `#[ident]` token，chibacc 示例只有 `attrs:item_attr*`，不足以覆盖 `#[attribute(all(someident, a=b, c=[1,2,3,4]))]` 这类 nested/list/named argument 语法。
	- 实现顺序：先在 spec repo 明确 attribute argument AST/grammar（bare name、string/int literal、named arg、nested call、array/list、struct/object），再补 chibalex/chibacc fixtures/goldens，最后用 parser facts 替换 C07 scanner facts。
		- `someident` == `someident = true`
		- 总之可以抄一抄rust
		- 之后我们反正走 proc macro
- [ ] `std.regex` 首发路线改为 Rust-like Thompson NFA/DFA subset，以完成 frontend bootstrap 为先；Perl/PCRE2-compatible VM 作为后续 TODO，不阻塞 C05/C06 自举。
	- 首发必须支持：literal、char class/range、concat、alternation、group/capture bookkeeping、`* + ? {m,n}` greedy/lazy 基础、anchors、UTF-8/codepoint boundary、leftmost/longest tie-break for chibalex。
	- 首发必须拒绝：backref、conditional、recursive pattern、atomic group、possessive、variable-length lookbehind、复杂 lookaround/capture 组合等 PCRE2 VM-only 特性；parse-time hard error，不能半支持。
	- [x] regex unsupported feature policy：std.regex parser 已记录 PCRE-only unsupported matrix 与 parse-time hard-error policy；真实 parser reject 执行仍未完成。
	- [x] regex typed program lowering：`compile_regex` 已从 builtin contract 改成 typed instruction builder，覆盖 literal/any/class/sequence/alternative/capture/non-capture/repeat/assertion，并为 alternation/repeat 生成真实 `RegexSplit` / `RegexJump` labels；capture numbering 已由 state-based compiler 分配。
	- [x] regex source matcher slice：`RegexProgram.match_at` 已从 builtin contract 改成 source-level backtracking VM，覆盖 literal/any/class/assert/accept/Split/Jump/repeat 的 longest endpoint；VM state 现在携带 capture array，Split 分支复制 capture 状态，Enter/Exit 更新 range 并在 Accept 返回 captures。完整 capture rollback/PCRE 级捕获语义仍未完成。
	- [x] UTF-8 XID real table：`std.regex.xiddata` 已从 level0 Unicode 15.1.0 range 表移植为 level-1b method 风格，`Char.is_xid_start/continue` 不再使用 compiler builtin；invalid UTF-8 policy 与 frontend shared decode 仍需 primary parser 接入。
	- PCRE2/Perl 兼容目标保留为 std.regex 后续阶段：单独建 feature matrix、oracle corpus、VM bytecode/backtracking stack/capture rollback 设计后再做。
- [ ] level-1b 最小可执行链必须先复刻 level0 的正确流程，而不是继续扩大 contract gates：
	- `AST -> one-pass CPS + beta`：meta-level continuation `(Val) => CpsExpr`，atom 直接调用 `k(atom)`，只在 call / switch / reset / shift 处物化 IR continuation。
	- `CPS -> BIR/Core blocks`：continuation block / function return / frame-chain lower 到显式 block terminator，`ContN` 先 emit stackless resume frame，再 emit frame-chain，再 emit repeatable package。
	- 这一路只能借鉴 `level0/src/backend/cir/lower.chiba`、`level0/src/backend/bir/lower.chiba` 的流程；不能把 level0 的 numeric bool、旧 branching bug、旧 scanner 语义搬进 level-1b。
- [ ] branching 分析必须作为本轮 primary lowering 的硬门槛：
	- [x] branching gate fixtures：覆盖 if/else、else-if、if-let、match、short-circuit、nested branch；用于防止只看 happy path。
	- [x] branching surface facts：source/typed facts 已记录 if/else/else-if/if-let/match/short-circuit，供 typed lowering 和 CPS join planning 使用。
	- [x] branching CPS join fact：CPS pass 已把 branching surface 转成 `BranchJoinPlan` / `CpsBranchJoinFact`，并要求所有 arm 回到同一 tail；真实表达式级 join continuation body lowering 仍未完成。
	- [x] branch tail-call CPS fact：两臂都是 tail-call 的 if/else 会保留为 `CpsTailBranchCall`，不再在 C09 退化成普通 return fact。
	- [x] branching Core obligation：C11 已把 `CpsBranchJoinFact` 转成 backend-neutral `CoreBranchJoinLoweringObligation`；真实 executable branch block emission 仍未完成。
	- [x] branching miscompile guard：C08 遇到 branching body 会标记 `TypedFunctionBranchJoinPending`，C11 validator 拒绝 fake body emission，避免 `if/else` 被常量竖切误编译。
	- [x] minimal executable branch slice：`if true/false { 42/0 } else { 42/0 }` 已能保留 typed branch body 并 emit executable WAT；变量条件和 nested branch 已覆盖，effect arms 与 if-let 仍走 join obligation / pending blocker。
	- [x] branch literal condition exactness：`if true/false` 窄条件现在要求完整 token，避免 `if truex` / `if falsey` 误进 literal branch lowering。
	- [x] minimal param-condition branch slice：`if flag { ... } else { ... }` 中第一 bool 参数可作为 Wasm i32 条件进入窄 branch lowering；复杂条件仍 pending。
	- [x] minimal branch tail-call slice：`if true/false { f(42/0) } else { g(42/0) }` 已能保留两臂 tail-call 并 emit `return_call` WAT，保证该窄分支 slice 不退化成非尾调用。
	- [x] minimal branch no-arg tail-call slice：`if true/false/flag { f() } else { g() }` 已能保留两臂 no-arg tail-call 并 emit `return_call` WAT，保证普通非 multi-shot 分支调用保持尾位置。
	- [x] minimal branch param0 tail-call slice：`if flag { f(x) } else { g(x) }` 已能保留两臂 param0 tail-call 并 emit `local.get 0 return_call` WAT；复杂表达式参数仍 pending。
	- [x] branch tail-call symbol validation：C08/C11 都会校验 `if/else` 两臂 tail-call target，不让悬空 `return_call` 混进 backend。
	- [x] minimal match fallback slice：`match flag { true => ... _ => ... }` 已能 lower 成 bool branch 并在 WAT 中跑通 true/fallback 两臂；ADT/deep pattern/default exhaustiveness 仍未进入 executable decision tree。
	- [x] minimal nested branch slice：nested `if` 作为 branch arm 已经保留到 executable WAT，并覆盖 outer/inner true/false 组合；复杂 effect/binder join 仍未完成。
	- 不能只看 `if` then/happy path；必须同时覆盖 `else`、`else if`、`if let` 成功/失败分支、`match` 每个 arm、default/fallback、短路逻辑和 nested branch。
	- typed env、pattern binding scope、exhaustiveness/warning、CPS join continuation、branch result type unify 必须一起验收。
	- 所有 branching lowering gate 必须包含 “then/else 都有副作用或不同 binder” 的 fixture，避免再出现看了 if 不看 else、match branching 一坨但漏分支的情况。

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
- 命名边界备注：这里的 **level-1c 指当前 `src/` / `level1c` legacy 产物线**，它只是 first-bootstrap seed / 对拍来源；不是 C12 后继续保留的新主线。C12 验收完成后，`src/` / `level1c` 与 `level0/` 一起删除，最终只保留 `level-1b/` 作为 primary implementation。
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
- 普通 Array / Vec / String / closure env / continuation package 必须保持 Wasm-GC managed object 语义，不退回线性内存或 Node host object。
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
	- **已落地切片**:
		- C08 ADT ctor contract 已记录 canonical tag：`BigCamel` -> `:big_camel`；
		- ctor helper body fact 已记录 `Ctor(args)` 返回 canonical tuple，`_1` 为 tag，payload 从 `_2` 起；
		- tuple field fact 已接入 row/record shape，供后续 C09/C11 消费。
		- C11 已把 ctor helper obligation 降成 executable Core/WAT helper，生成 tuple struct 并通过 `adt-tuple-ctor.wat` 的 tag/payload wasmtime 验证。
		- C11 WAT tuple struct type label 已改为从 `TupleNominalIdentity.tuple_key` 派生并按 canonical tuple key 去重；`adt-tuple-ctor-shared-nominal.wat` 验证两个同形 ctor helper 共享同一个 tuple struct type，而不是按 helper/source occurrence 生成新匿名 struct。
		- C08/C11 已锁定 tuple nominal identity 由有序 semantic type key 序列生成：同顺序同类型复用同一 `compiler.tuple::tuple(...)` nominal，不同顺序生成不同 nominal；`adt-tuple-ctor-order-distinct-nominal.wat` 已验证不同顺序 tuple struct 可查看且可运行。
		- C11 已新增普通 tuple heap field access artifact：`tuple-heap-field-access.wat` 用 Wasm-GC struct 验证 `(A, B)._1/_2` 与三元 tuple field access 的可运行布局；`source-primary-backend.wat` 也已从 source fixture 覆盖 `(41, 1)._1`、`(40, 2)._2`、`(38, 2, 1)._3` 并用 wasmtime 执行。完整 chibacc AST primary tuple literal / field access 仍需接入真实 typed lowering。
	- **仍是 blocker**:
		- source/data parser 尚未真正产出完整 `AdtVariantDecl`；
		- ctor callable symbol / overload resolution / namespace 消歧尚未接到 typed 函数表；
		- `tuple_to_adt` / `adt_to_tuple` intrinsic 仍缺真实 typed identity roundtrip validation。

- [ ] compiler-internal intrinsic surface
	- **目标**:
		- `unsafe_cast[T, F](self: T): F` 明确是 internal / compiler-only intrinsic，不暴露成普通安全 std API；
		- `tuple_to_adt[Tuple, T]` / `adt_to_tuple[T, Tuple]` 明确归为同级 internal bridge，而不是普通库函数语义；
		- `Tuple[T1, T2, T3, T4, ...]` 的类型本体由编译器打洞和 lowering 承认，不要求用户层 std 自己伪装出真正的 tuple type system。
	- **备注**:
		- 这些能力可以有用户可见 surface，但语义来源必须是 compiler intrinsic / builtin contract；
		- 不能靠普通库层“模拟”替代真正的 type/lowering 支持。
	- **已落地切片**:
		- C08 intrinsic obligations 已保存进 `TypedModule`；
		- C09/C11 已线程 tuple bridge identity 到 CPS/Core obligation，并校验 canonical `compiler.intrinsic` namespace。
		- C08-C11 已线程双向 bridge typed roundtrip fact，validator 会拒绝双向 bridge 缺失 `preserves_identity` evidence。
		- C11 已 emit 可执行 eqref identity conversion helpers，并用 `adt-tuple-intrinsic-roundtrip.wat` 验证双向 roundtrip 保持 payload。

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
	- **已落地切片**:
		- C08 typed module 已生成 `ScopeShadowFact`，显式记录 local binding / wildcard discard surface，并固定策略：local shadowing allowed、same-scope duplicate rejected；
		- 真实 block scope graph、pattern binder scope、import shadow matrix runtime/diagnostic 仍未完成。

- [ ] 不再出现 `i64` 和 `1` `0` 还有 `if x != 0` 这种历史遗留代码
	- **进展**:
		- `level1b:truthfulness-audit` 已新增 `historical-numeric-conditional` / `historical-numeric-bool-arm` detector，防止 compiler primary path 继续新增 `if x != 0` / `if x == 1` / raw `=> 0|1` 这类数字布尔编码。
		- `check_unsafe_type` 已把临时 `unsafe_depth != 0` 改成显式 `UnsafeContext` fact；剩余 `i64` / `0` / `1` 需要继续区分 ABI/exit-code/count/index 等合法 target surface 与历史 magic value。

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
	- **已落地切片**:
		- C11 `SymbolDebugManifest` 已可从 validated Core function symbols 生成 `source -> lowered -> $chiba.*` 映射，不再直接 fail 于 manifest generation absent；
		- 完整 import / method / ctor / intrinsic debug path 仍待 namespace/name resolution 接入。

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
	- **进展**:
		- C08 ADT/constructor lowering contract 已新增 `ConstructorMutationObligation` 与 `ConstructorMutateOnceUsedInput` strategy，用于承载 exactly-once input -> mutation lowering 事实；真实 usage fact 接入与 mutation backend lowering 仍未完成。


## Second Bootstrap 剩余项

### C11: wasm-gc Core/backend rewrite 收口

- [ ] level-1b 不可用状态修复 / truthfulness cleanup
	- **现状**: `level-1b/compiler/control/*`、`level-1b/compiler/closure/*`、`level-1b/compiler/backend/*` 中存在大量 contract-only / stub-only 实现：空 facts、直接 `Ok(module)`、只生成 layout 壳、WAT emitter 只输出注释等。
	- **前置**: 语言类 cleanup 必须先对齐 spec repo，尤其 continuation / callable / closure / send / CIR-BIR placement。truthfulness cleanup 不是单纯搜索替换 stub，而是把实现缺口按 spec 硬规则分类。
	- **目标**: level-1b 不再让“接口已存在”伪装成“语义已实现”。所有暂未实现 pass 必须显式 blocker/stub；所有 gate 必须区分 contract smoke 与真实语义验收。
	- **第一轮 audit 清单**:
		- 新增轻量反馈环：`vp run level1b:truthfulness-audit`。该 gate 当前预期失败，用 blocker taxonomy 暴露假成功路径；不要把它并入 expensive bootstrap。
		- 当前基线计数：`legacy-compiler-execution` / `oracle-success-path` 已清零；剩余 `primary-path-blocked` 26；`ok-module-pass-through` / `empty-facts` / `comment-only-backend-output` 已清零为显式 blocker 或 source-contract 拒绝。
		- `level1b:c07-source-driver` 已拒绝 source semantic gate 空 errors pass-through；缺 parsed project facts / source semantic gate scan 时返回 `missing-facts` blocker，不能让 source driver 在无事实输入时冒充可进入 typed 阶段。
		- C07 已新增 byte-level source scanner primitives：`source_starts_with_at`、line advance、first namespace scan、namespace name slice；`ProjectSurface` 现在携带 `SourceProjectFacts`，可由 `scan_project_source_facts` 从已加载 files 派生 header facts（`#![Metal]` / `#![no_prelude_import]`）、doc presence facts（`///` / `#[doc(path=...)]`）、`compile_if` expression slice / top-level shape / predicate classification facts 与 namespace scan facts。这是 source facts primary path 的起点，不是 full parser，仍需接入 project load / compile_if predicate parse / item scan 后才能解除 parser execution blocker。
		- C07 source scanner 现在还派生 line-start item facts（`def` / `type` / `data` / `extern`）；source semantic gate 读取 `ProjectSurface.facts.namespaces/items`：无 namespace 报 `missing-facts: namespace scan found no namespace`，无 item 报 `missing-facts: source item scan absent`，已有 item 后仍 fail-closed 于 `missing-lowering: typed item lowering absent`，不再使用固定 dummy blocker。
		- C07/C08 source item facts now preserve `private` visibility for `private def/type/data/union/interface/extern`; alpha origins and typed item skeletons carry this bit forward for later namespace visibility checks.
		- C07 source semantic gate 现在会拒绝未知 `compile_if` predicate shape（例如 `xor(...)`），避免 unsupported predicate 被 presence/classification facts 洗成可继续编译。
		- C07 source scanner 现在检测 inline namespace block / indented namespace forms，并在 source gate fail-closed 于 `inline namespace block assembly absent`；spec 支持 file-header namespace 与 inline namespace block，当前 transition scanner 只拥有 file-header primary path。
		- C07 source scanner 现在派生 `use` declaration facts（path slice、glob、multi-import marker），为后续 namespace/use resolution primary path 准备输入；真实 import resolution 仍未执行。
		- C07 import policy 现在会构造 source import scope input（owner namespace、explicit uses、prelude policy），并在显式 `use` 或默认 prelude 需要注入时 fail-closed 于 `source import/name resolution absent`，避免跳过 import/name resolution 直接进入 typed lowering。
		- C07 source semantic gate 现在会按 source slice 比较同 namespace 的 item name，拒绝 `invalid-surface: duplicate item in namespace`；真实多 namespace block / import merge 后的完整 duplicate/ambiguous resolution 仍未完成。
		- C07 source item scanner 现在把 line-start `#[compile_if(...)]` 作为 item attribute fact 挂到后续 item；这只是保留条件编译证据，尚未执行 compile_if eval/filter。
		- C07 source semantic gate 现在在 item-level `compile_if` 尚未过滤时 fail-closed 于 `item compile_if filtering absent`，避免被禁用 item 产生 binder/export/backend symbol。
		- C07 source item scanner 现在把 file-header namespace 作为 `owner_namespace` 挂到 item fact；inline namespace 仍 fail-closed，真实 nested namespace ownership 还没完成。
		- C07 source item scanner 现在保留 item header slice 与粗粒度 surface shape（是否有参数、类型标注、body/initializer），为后续 def/static/type/data lowering 提供输入；参数 pattern、type expr、body AST 仍未解析。
		- C07 source item scanner 现在保留 method receiver / operator method / `Self` / pattern parameter / explicit generic parameter surface bits；非 method receiver scope 中出现 `Self` 会 fail-closed 于 `invalid-surface: Self only valid in method receiver scope`，避免 `Self` 被当作普通 top-level type name。
		- C07 source semantic gate 现在拒绝缺少任何扫描到的类型标注的 `extern` item，符合 spec 中 extern ABI 边界必须显式标注的方向；真实 ABI signature parse 仍未完成。
		- C07 source scanner/gate 现在识别 `#[world_local]` 与 header 中的 `Ref[`，并拒绝缺少 world_local 的顶层 static `Ref`；这是 spec 级安全规则的 fail-closed 早期实现，真实 type expr parse 仍未完成。
		- C07 source semantic gate 现在拒绝缺少扫描到的 body/initializer marker 的 `def` item，避免把没有函数体/静态初始化输入的 skeleton 交给 typed lowering。
		- C08 alpha conversion 已开始消费 C07 source item facts：`alpha_convert` 由 `ProjectSurface.facts.items` 派生稳定 binder ids，不再创建空 binder stream；HM + row inference 仍 fail-closed，尚未产 typed item facts。
		- C08 typed item skeleton 现在会把 scanned item 粗分为 function/static value/extern function/nominal type/data/union/interface，避免 `def x` 静态值与 `def f(...)` 函数在 skeleton 阶段混成同一种壳。
		- C08 alpha origins 现在保留 source item kind/name/file/owner_namespace/line/column/private/attributes/surface；type inference 先构造 `TypedItemSkeleton` 并检查 pattern coverage，再 fail-closed 于 source item type expression/body inference absent，避免把只有 binder index 的骨架伪装成 typed module。
		- C08 pattern elaboration 已开始消费 alpha binders：`elaborate_patterns` 由 binder stream 派生稳定 pattern ids，不再创建空 pattern stream；这仍是骨架 facts，不等于完整 pattern AST lowering。
		- C08 typed skeleton 现在在 method-style surface 上 fail-closed 于 `missing-lowering: method receiver Self binding absent`，在 pattern params 上 fail-closed 于 `missing-lowering: function pattern parameter desugaring absent`，在显式泛型 surface 上 fail-closed 于 `missing-lowering: explicit generic parameter binding absent`；template / method-operator / typed-facts gates 已接入 semantic driver，不能继续被注释中的“会运行”冒充。
		- `compiler/lower/ast_to_core.chiba` 已拒绝空 `SurfaceModule` lowering；缺 AST -> Surface item lowering 时返回 `missing-lowering` diagnostic。`level1b:truthfulness-audit` 已补 source gate 空放行、typed inference pass-through、空 Surface lowering 三类回归 detector。
		- `level-1b/src/level1b_main.chiba` 已不再是 `main = 42` magic placeholder；seed entry 只连接 CLI readiness contract，`level1b:smoke` 仍标记 primary compile / WAT emission blocked。
		- `level1b:c09-control-cps` 已改成 fail-closed：缺 answer/control scan、usage subject collection、boundary scan、one-pass CPS beta lowering 时返回 blocker diagnostic；该 gate 当前只通过 source contract，真实 valid/invalid continuation gates 标记为 primary-path-blocked。
		- `level1b:c08-semantic` 已拒绝 `infer_types -> Ok(TypedModule(module))` 这类 untyped alpha module pass-through；缺 HM + row inference over source items 时返回 `missing-facts` blocker，不能让后续 C09/C10/C11 误以为 typed module 已成立。
		- `level1b:c10-closure-package` 已保留 continuation/lambda/closure subject kind，并区分 escaped boxed `Cont1` 与 repeatable `ContN` package；缺 continuation capture extraction 时 fail-closed；该 gate 当前只通过 source contract，closure/directification/nanopass 验收标记为 primary-path-blocked。
		- `level1b:c11-backend` 现已在 source contract 阶段明确拒绝：comment-only `emit_core_op` / fake WAT backend。
		- spec 要求 answer type checking 在 level-1 / CIR 层完成；当前 `check_answer_control` 已改为 fail-closed blocker，尚未生成真实 answer/control facts。
		- spec 要求 continuation boundary / replay / usage 保留 control boundary、answer type、arena/world legality 与 usage；当前 boundary / usage 已改为 fail-closed blocker，replay 仍只有轻量事实壳。
		- C09 replay safety 不再把 usage facts 全部标成 `safe=true`；缺 usage facts 或 capture classification 时 fail-closed，避免 multi-shot replay legality 假通过。
		- C09 replay safety 现在显式区分 replay capture kind：pure value、shared `Ref` cell、non-replay state；shared `Ref` 对 `ContN` 是合法的 shared-reference 语义，不 snapshot / rollback。真实 capture classification 仍 fail-closed。
		- one-pass CPS 当前已不再是单纯 `CpsModule(module)` wrapper：普通函数 tail-form、极窄 tail-call target、branch join fact、ContN materialized exception 已进入事实流；真实 expression CPS transform 与 administrative beta-reduction 仍未完成。
		- 旧 `src/backend/cir/*` 与 level-0 可以作为 legacy reference 借鉴算法、fixture、失败模式；但每次借鉴都必须先过 spec alignment，且 level-1b 重新拥有行为，不能形成 legacy dependency。
		- 旧 `src/backend/cir/cps.chiba` 也只是 L5 wrapper / synthetic continuation package 方向，不是可直接搬运的 spec 级 one-pass CPS + beta 实现。
		- CPS usage 当前已保留 continuation / lambda / closure subject kind，并能对 syntactic no-capture closure 走 direct path；真实 expression-level usage count 与 capture extraction 仍未完成。
		- C10 CPS usage 当前已 fail-closed 于 capturing closure/lambda extraction absent；no-capture directification 只有 syntactic surface 竖切，还没证明完整 closure/lambda 捕获分析。
		- spec 要求 `shift` 捕获 `Cont1`、`shiftn` 捕获 `ContN`，逃逸 `Cont1` boxed 且不升级；当前 control/closure 只按 UseZero/UseOne/UseMany 做壳级 decision，未承载 `Cont1` / `ContN` storage 语义。
		- 已新增 target-independent IR contract：`ContinuationCont1` / `ContinuationContN`、`ContinuationBoxedOneShot`、`ContinuationRepeatableFrameChain`、`ContinuationFact`、sendable callable exclusion hook。
		- continuation package 当前已区分 escaped boxed `Cont1` 与 repeatable `ContN` package；真实 storage/call-site escape analysis 与 capture extraction 仍未完成。
		- closure conversion 当前对 packaged continuation 生成 empty capture fields，未抽取真实 capture set。
		- 已新增 closure/backend contract：`StacklessResumeFunction`、`ContinuationFrame`、`ContinuationLowerBoxedCont1`、`ContinuationLowerRepeatableContN`、`CoreOpStacklessFunction`、`CoreOpContinuationFrameChain`、`CoreOpContNPackage`。
		- C10/C11 source path 已把 `ContinuationSimplification` 转成 `ContinuationLoweringFact` 并贯穿到 backend Core op lowering；boxed `Cont1` 保留 consumed-state，`ContN` lower 为 frame-chain op + repeatable package op，但真实 capture extraction / frame body / WAT emission 仍是 blocker。
		- C10 closure conversion 现在不会为 deleted continuation 生成 backend lowering fact，避免已删除 continuation 以默认 DirectCont1 形态泄漏到 backend。
		- C10 capture model 已区分 `CaptureSharedRefCell`，并把 materialized continuation blocker 拆成 boxed `Cont1` capture extraction 与 `ContN` frame-chain capture extraction；这只提高 fail-closed 诊断精度，尚未实现真实 capture set / frame body 抽取。
		- C11 已修正 erased callable continuation package layout：`ContinuationLowerErasedCallableVariant` 不再复用 closure env layout，而是进入独立 `LayoutContinuationPackage`（tag + payload）；真实 callable ADT dispatch / WAT emission 仍是 blocker。
		- C11 validator 已补 `ContN` ordering invariant：`CoreOpContNPackage` 必须在已见同 owner 的 `CoreOpContinuationFrameChain` 后出现；`CoreOp` 现在保留 owner provenance，防止其他 lowering path 用不相关 frame-chain 绕过 repeatable frame-chain 语义。
		- C11 validator 已补 runtime Core op owner invariant：任何需要 layout 的 materialized Core op 缺 owner provenance 时返回 `CoreMissingOwner`，避免匿名 closure / continuation package 进入 backend emit。
		- C11 Core op 现在保留 runtime state machine fact；boxed `Cont1` 必须携带 `CoreConsumedStateMachine`，其他 op 不得误带 consumed-state，防止 one-shot 语义在 backend 前丢失。
		- C11 Core op 现在保留 `frame_count`，validator 拒绝 zero-frame 的 `ContN` frame-chain，避免 repeatable package 只有壳没有 stackless resume frame。
		- C11 `ContN` lowering 现在会先发出 stackless resume Core ops，再发出 frame-chain 与 repeatable package；validator 要求 frame-chain 前存在匹配数量的 stackless resume ops，且 stackless Core op 也必须保留 owner provenance。
		- `ContN` lowering 验收必须证明：repeatable frame chain 由 stackless resume functions 驱动；frame chain 可重复恢复；捕获 `Ref[T]` 是 shared-reference，不 snapshot / rollback。
		- closure lowering 验收必须证明：no-capture closure 走 direct function / funref / inline；capturing closure 只有逃逸或确需 env 时才 materialize env；env 内 continuation / `Ref[T]` 不被能力洗白。
		- C10 env simplification 现在会生成 backend-consumable `ClosureLoweringFact`：no-capture lifted closure 进入 `ClosureNoCaptureDirect` 且 `env=None`，capturing closure 进入 `ClosureCapturingEnv` 且携带 env；真实 closure subject extraction 与调用点 direct rewrite 仍未接入。
		- C11 Core lowering 现在消费 `ClosureLoweringFact` 而不是旧 simplification 名称：`ClosureNoCaptureDirect` 不产生 closure env Core op，`ClosureCapturingEnv` 才 materialize `CoreOpClosureCall` + env layout；真实 WAT/funref/direct call emission 仍是 blocker。
		- spec 要求 `(A) -> B` 参数位置是 checked-template callable obligation，存储位置 lower 成 erased callable ADT；显式 `cont1` / `contN` storage 不走 erased callable ADT；当前未见真实 callable storage lowering。
		- C09 control IR 现在有 `CallableStorageFact` / `CallableStorageVariant` contract，明确 erased callable storage 至少区分 function / closure / boxed `Cont1` / `ContN`，并提供 sendable callable 排除 continuation variants 的 invariant；真实 callable storage lowering 仍未执行。
		- C08 typed skeleton 现在会从 callable/continuation surface 生成粗粒度 `CallableStorageFact`：普通 storage 包含 function / closure / boxed `Cont1` / `ContN`，`send` callable storage 排除 continuation variants，显式 `cont1` storage 只保留 boxed one-shot `Cont1`，显式 `contN` storage 只保留 repeatable `ContN`；真实 type-expression / body / call-site integration 仍 fail-closed。
		- C08 capability rules 现在有 `check_sendable_callable_storage`，sendable callable storage 若包含 continuation variant 会返回 capability error；下一步是把 C08 callable facts 接入真实 type-expression / call-site / storage lowering，而不是只停在 surface fact。
		- frontend grammar source 已补 `cont1 (A) -> B`、`contN (A) -> B`、`shiftn` contract 与 chibalex/chibacc mini fixtures；生成版 lexer/parser 已刷新，`shift :tag` / `shiftn :tag` label parse 已对齐 lexer 的 `Colon Ident` tokenization，且 AST 保留 tag 名；native chibalex oracle 尚未覆盖 continuation surface keywords。
		- C06 chibacc alternative retry/recovery source 现在使用 `shiftn retry`，与 multi-shot parser retry 语义一致；generated parser runner 的 mini AST evidence 已被 C07 消费，完整 chiba-level1 parser recovery / source-driver AST primary path 仍未接管。
		- C06 chibacc source gate 现在检测 `engine.chiba` 中的 `shift retry` 回归，parser retry 必须保持 multi-shot `shiftn retry`。
		- chibalex 真实实现必须支持 UTF-8 source/identifier scanning；当前 C07 `scan.chiba` 是 ASCII byte-level 过渡 scanner，只能安全识别 ASCII header/keyword facts。若 namespace/item identifier 允许 UTF-8，scanner 必须补 UTF-8 aware path，或在遇到非 ASCII 相关事实时 fail-closed，不能静默漏扫后冒充 source facts 完整。
		- C05 chibalex contract 已固定 UTF-8/XID identifier policy，engine state advance 改为 codepoint offset (`next_char_offset`)；`utf8-ident.chibalex` mini fixture 覆盖 `$XID_START/$XID_CONTINUE`。native `chibalex.o` oracle 现在会实际生成 continuation surface 与 UTF-8 mini fixtures；level-1b chibalex lowering/codegen 已不再返回 contract-only status，真实 generated lexer runner 仍未接管完整 C07 source facts。
		- C05 chibalex mini generator 现在用 UTF-8 lead-byte 长度推进 identifier scan，不再在 UTF-8 identifier 内按单字节推进；真实 XID 表/invalid sequence validation 仍待 primary lexer 接管。
		- C05/C06 generated lexer/parser codegen artifacts 已移除 `ContractOnly` status 与 `missing-lowering` diagnostic；gate 现在拒绝 codegen 再报告 contract-only success。C07 已有 `SourceFrontendEvidence` 合并入口并校验 chibacc-mini AST evidence；剩余 blocker 是完整 chiba-level1 AST primary execution 尚未替换 C07 scanner-derived item/body facts。
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

## 改进点

1. `public use` reexport 暂时不支持, 新增 `public` 关键字可以加在 `def` `use` `type` `data` 之类的前面，但是保持默认 `public` 语义
2. prelude 应该默认就直接 use 一堆 std 和 intrinsics
3. level0 里面的 asm 块在 level-1b 是空缺的，这里我觉得可以打洞 + 不做校验 直接 emit wasm
4. match number string tuple record 的处理

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
