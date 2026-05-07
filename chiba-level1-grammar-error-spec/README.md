# chiba-level1 grammar error spec

这组样例专门用于测试两件事：

1. `chibalex` 必须能稳定产出 token。
2. `chibacc` 必须在语法阶段报错，而不是挂死、越界或误接受。

文件按两类组织：

- `01` 到 `18`：局部错误，理论上适合做错误恢复，parser 应该尽量继续看到后续结构。
- `19` 到 `100`：结构性错误，parser 可以报错后尽早停止，不强求局部修复。

建议验证方式：

1. 先跑 lexer，确认没有 `LexError`。
2. 再跑 parser，记录是“拒绝并恢复”还是“拒绝并终止”。
3. 后续如果要给 acc 增加恢复测试，可以为每个样例再补一个 `.parser.spec` 或 `.error.expect`。

样例说明：

- `01-test.chiba`：`let` 缺右侧表达式。应报错，可恢复到后续 `return`。
- `02-test.chiba`：函数参数之间缺逗号。应报错，可恢复到函数体。
- `03-test.chiba`：记录字面量字段缺值。应报错，可恢复到后续语句。
- `04-test.chiba`：`if` 缺条件表达式。应报错，可恢复到 `else` 或后续 block。
- `05-test.chiba`：`match` 分支缺 `=>`。应报错，可恢复到下一个 arm。
- `06-test.chiba`：调用参数表中出现空参数。应报错，可恢复到 `)` 后继续。
- `07-test.chiba`：多余的 `else`。应报错，不必强求局部恢复。
- `08-test.chiba`：表达式尾随二元运算符直到 block 结束。应报错，不必强求局部恢复。
- `09-test.chiba`：`def` 缺函数体。应报错，不必强求继续接受同一 item。
- `10-test.chiba`：未闭合 block 后直接开始下一个 `def`。应报错，若能恢复最好，但允许直接终止。
- `11-test.chiba`：两条 `let` 写在同一行且没有分隔。应报错，可恢复到后续 `return`。
- `12-test.chiba`：多一个右括号。应报错，可恢复到语句尾。
- `13-test.chiba`：少一个右括号。应报错，可恢复到 block 末尾。
- `14-test.chiba`：索引表达式多一个右方括号。应报错，可恢复到语句尾。
- `15-test.chiba`：泛型参数列表缺 `]`。应报错，可恢复到 item 尾部。
- `16-test.chiba`：调用参数缺逗号。应报错，可恢复到 `)` 后继续。
- `17-test.chiba`：tuple/record 混合里缺逗号。应报错，可恢复到后续字段。
- `18-test.chiba`：match 模式参数缺逗号。应报错，可恢复到下一个 arm。
- `19-test.chiba`：多一个右花括号。应报错，不必强求局部恢复。
- `20-test.chiba`：block 少一个右花括号直到文件结束。应报错，不必强求局部恢复。
- `21-test.chiba`：列表索引少一个右方括号并直接跟成员访问。应报错，不必强求局部恢复。
- `22-test.chiba`：泛型路径中多一个 `]`。应报错，不必强求局部恢复。
- `23-test.chiba`：函数参数列表以逗号开头。应报错，不必强求局部恢复。
- `24-test.chiba`：函数参数列表以逗号结尾后再接参数。应报错，不必强求局部恢复。
- `25-test.chiba`：记录更新里字段之间缺逗号。应报错，不必强求局部恢复。
- `26-test.chiba`：类型参数里缺逗号且嵌套泛型连在一起。应报错，不必强求局部恢复。
- `27-test.chiba`：`if` 条件右括号多一个而 block 正常。应报错，不必强求局部恢复。
- `28-test.chiba`：`use demo.io.{...}` 这类导入缺右花括号。应报错，不必强求局部恢复。
- `29-test.chiba`：把 `def` 误写成 `de`。lexer 会把它当普通标识符，parser 应报错。
- `30-test.chiba`：把 `def` 误写成 `func`。lexer 会通过，parser 应报错。
- `31-test.chiba`：`let` 使用 `true` 作为变量名。应报错。
- `32-test.chiba`：`let` 使用 `false` 作为变量名。应报错。
- `33-test.chiba`：`let` 使用 `if` 作为变量名。应报错。
- `34-test.chiba`：`def` 使用关键字作为函数名。应报错。
- `35-test.chiba`：重复 `private` 修饰符。应报错。
- `36-test.chiba`：修饰符和 `def` 顺序错误。应报错。
- `37-test.chiba`：`return` 后直接跟 `let` 声明。应报错。
- `38-test.chiba`：`break` 后多出额外表达式。应报错。
- `39-test.chiba`：`continue` 后多出额外标记。应报错。
- `40-test.chiba`：`if let` 缺模式右侧 `=`。应报错。
- `41-test.chiba`：`if let` 缺条件表达式。应报错。
- `42-test.chiba`：`for` 条件头里多一个分号。应报错。
- `43-test.chiba`：`for` 标签位置错误。应报错。
- `44-test.chiba`：属性 `#[]` 缺右方括号。应报错。
- `45-test.chiba`：属性参数列表缺逗号。应报错。
- `46-test.chiba`：`use` 路径中多一个点。应报错。
- `47-test.chiba`：`match` 缺匹配表达式。应报错。
- `48-test.chiba`：`match` 缺左花括号。应报错。
- `49-test.chiba`：lambda 参数列表缺 `|`。应报错。
- `50-test.chiba`：record update 缺 `|`。应报错。
- `51-test.chiba`：泛型参数列表缺逗号。应报错。
- `52-test.chiba`：类型注解里的泛型缺右方括号。应报错。
- `53-test.chiba`：类型参数列表以逗号开头。应报错。
- `54-test.chiba`：模式 `Some(,)` 含空位。应报错。
- `55-test.chiba`：模式里重复 `@` 绑定。应报错。
- `56-test.chiba`：记录模式字段缺值模式。应报错。
- `57-test.chiba`：调用参数表只有逗号。应报错。
- `58-test.chiba`：管道运算符连续出现两次。应报错。
- `59-test.chiba`：成员访问里出现双点。应报错。
- `60-test.chiba`：方法调用链里双点后接调用。应报错。
- `61-test.chiba`：索引后紧跟额外 `[`。应报错。
- `62-test.chiba`：二元运算符连续出现。应报错。
- `63-test.chiba`：前缀运算符后直接接 `)`。应报错。
- `64-test.chiba`：闭包参数列表多一个逗号。应报错。
- `65-test.chiba`：泛型调用缺右方括号后直接开始调用。应报错。
- `66-test.chiba`：`type` 声明缺名字。应报错。
- `67-test.chiba`：`type` 别名缺右侧类型。应报错。
- `68-test.chiba`：`data` 声明缺名字。应报错。
- `69-test.chiba`：`data` 泛型头缺右方括号。应报错。
- `70-test.chiba`：`data` 构造器列表缺右花括号。应报错。
- `71-test.chiba`：`union` 声明缺字段类型。应报错。
- `72-test.chiba`：`union` 字段之间缺逗号。应报错。
- `73-test.chiba`：方法定义缺 `.` 左侧类型。应报错。
- `74-test.chiba`：方法定义缺函数名。应报错。
- `75-test.chiba`：`extern` ABI 字符串后多一个字符串。应报错。
- `76-test.chiba`：`extern` 缺 ABI 字符串。应报错。
- `77-test.chiba`：`asm` 约束列表缺右括号。应报错。
- `78-test.chiba`：`asm` 输入列表字段缺寄存器名。应报错。
- `79-test.chiba`：`asm` 缺 `=>`。应报错。
- `80-test.chiba`：`reset` 标签位置错误。应报错。
- `81-test.chiba`：`shift` 缺 continuation 变量。应报错。
- `82-test.chiba`：`shift` 标签和 continuation 顺序错误。应报错。
- `83-test.chiba`：`break` 标签缺冒号。应报错。
- `84-test.chiba`：`continue` 标签缺冒号。应报错。
- `85-test.chiba`：`namespace` 路径以点开头。应报错。
- `86-test.chiba`：`namespace` 路径双点。应报错。
- `87-test.chiba`：`use` 分组导入里以逗号开头。应报错。
- `88-test.chiba`：`use` 分组导入里双逗号。应报错。
- `89-test.chiba`：`use` 通配符前多一个点。应报错。
- `90-test.chiba`：属性目标和 item 之间缺目标。应报错。
- `91-test.chiba`：属性列表为空但后续 item 头坏掉。应报错。
- `92-test.chiba`：属性参数列表以逗号开头。应报错。
- `93-test.chiba`：unsafe block 头多一个表达式。应报错。
- `94-test.chiba`：`return` 类型注解缺类型。应报错。
- `95-test.chiba`：函数参数类型注解缺类型。应报错。
- `96-test.chiba`：函数参数名缺失只剩类型。应报错。
- `97-test.chiba`：record 字段名重复冒号。应报错。
- `98-test.chiba`：tuple 尾部出现双逗号。应报错。
- `99-test.chiba`：slice 范围双 `..` 连写。应报错。
- `100-test.chiba`：调用后 trailing closure 前多一个属性头。应报错。
- `101-test.chiba`：`via` 后缺 namespace 路径。应报错。
- `102-test.chiba`：泛型 row bound 字段缺类型。应报错。
- `103-test.chiba`：泛型调用类型参数列表缺逗号。应报错。