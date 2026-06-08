#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProgram {
    pub namespace: Option<NamespaceDecl>,
    pub imports: Vec<UseDecl>,
    pub types: Vec<TypeDecl>,
    pub data: Vec<DataDecl>,
    pub items: Vec<SourceItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamespaceDecl {
    pub path: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseDecl {
    pub path: Vec<String>,
    pub glob: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceItem {
    Def {
        name: String,
        attrs: Vec<ItemAttr>,
        visibility: Visibility,
        receiver: Option<MethodReceiver>,
        generics: Vec<String>,
        generic_params: Vec<GenericParamDecl>,
        params: Vec<ParamDecl>,
        return_type: Option<String>,
        body: Expr,
    },
    ExternDef {
        name: String,
        attrs: Vec<ItemAttr>,
        visibility: Visibility,
        receiver: Option<MethodReceiver>,
        generics: Vec<String>,
        generic_params: Vec<GenericParamDecl>,
        params: Vec<ParamDecl>,
        return_type: Option<String>,
        extern_decl: ExternDecl,
    },
    StaticValue {
        name: String,
        attrs: Vec<ItemAttr>,
        visibility: Visibility,
        ty: Option<String>,
        body: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemAttr {
    Entry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDecl {
    pub name: String,
    pub pattern: Pattern,
    pub ty: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodReceiver {
    pub name: String,
    pub generics: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericParamDecl {
    pub name: String,
    pub bound: Option<GenericBoundDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenericBoundDecl {
    OpenRow(Vec<TypeField>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<TypeField>,
    pub alias_target: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeField {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataDecl {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<DataVariant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataVariant {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExternDecl {
    pub abi: ExternAbi,
    pub symbol: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExternAbi {
    Wasi,
    C,
}

impl ExternDecl {
    pub fn new(abi: ExternAbi, symbol: impl Into<String>) -> Self {
        Self {
            abi,
            symbol: symbol.into(),
        }
    }
}

impl SourceProgram {
    pub fn new(items: Vec<SourceItem>) -> Self {
        Self {
            namespace: None,
            imports: Vec::new(),
            types: Vec::new(),
            data: Vec::new(),
            items,
        }
    }

    pub fn with_surface(
        namespace: Option<NamespaceDecl>,
        imports: Vec<UseDecl>,
        types: Vec<TypeDecl>,
        data: Vec<DataDecl>,
        items: Vec<SourceItem>,
    ) -> Self {
        Self {
            namespace,
            imports,
            types,
            data,
            items,
        }
    }
}

impl Default for SourceProgram {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl NamespaceDecl {
    pub fn new(path: Vec<String>) -> Self {
        Self { path }
    }

    pub fn dotted(&self) -> String {
        self.path.join(".")
    }
}

impl UseDecl {
    pub fn new(path: Vec<String>, glob: bool) -> Self {
        Self { path, glob }
    }

    pub fn dotted(&self) -> String {
        if self.glob {
            format!("{}.*", self.path.join("."))
        } else {
            self.path.join(".")
        }
    }
}

impl DataDecl {
    pub fn new(name: impl Into<String>, generics: Vec<String>, variants: Vec<DataVariant>) -> Self {
        Self {
            name: name.into(),
            generics,
            variants,
        }
    }

    pub fn variant_names(&self) -> Vec<String> {
        self.variants
            .iter()
            .map(|variant| variant.name.clone())
            .collect()
    }
}

impl TypeDecl {
    pub fn new(name: impl Into<String>, generics: Vec<String>, fields: Vec<TypeField>) -> Self {
        Self {
            name: name.into(),
            generics,
            fields,
            alias_target: None,
        }
    }

    pub fn alias(
        name: impl Into<String>,
        generics: Vec<String>,
        alias_target: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            generics,
            fields: Vec::new(),
            alias_target: Some(alias_target.into()),
        }
    }
}

impl TypeField {
    pub fn new(name: impl Into<String>, ty: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ty: ty.into(),
        }
    }
}

impl DataVariant {
    pub fn new(name: impl Into<String>, fields: Vec<String>) -> Self {
        Self {
            name: name.into(),
            fields,
        }
    }
}

impl ParamDecl {
    pub fn new(name: impl Into<String>, ty: Option<String>) -> Self {
        let name = name.into();
        Self {
            pattern: Pattern::bind(name.clone()),
            name,
            ty,
        }
    }

    pub fn untyped(name: impl Into<String>) -> Self {
        Self::new(name, None)
    }

    pub fn pattern(pattern: Pattern, ty: Option<String>) -> Self {
        let name = pattern.primary_binding_name().unwrap_or("_").to_string();
        Self { name, pattern, ty }
    }
}

impl MethodReceiver {
    pub fn new(name: impl Into<String>, generics: Vec<String>) -> Self {
        Self {
            name: name.into(),
            generics,
        }
    }

    pub fn display_name(&self) -> String {
        if self.generics.is_empty() {
            self.name.clone()
        } else {
            format!("{}[{}]", self.name, self.generics.join(","))
        }
    }
}

impl GenericParamDecl {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bound: None,
        }
    }

    pub fn open_row(name: impl Into<String>, fields: Vec<TypeField>) -> Self {
        Self {
            name: name.into(),
            bound: Some(GenericBoundDecl::OpenRow(fields)),
        }
    }
}

impl SourceItem {
    pub fn def(
        name: impl Into<String>,
        generics: Vec<String>,
        params: Vec<ParamDecl>,
        return_type: Option<String>,
        body: Expr,
    ) -> Self {
        Self::Def {
            name: name.into(),
            attrs: Vec::new(),
            visibility: Visibility::Public,
            receiver: None,
            generic_params: generic_param_decls_from_names(&generics),
            generics,
            params,
            return_type,
            body,
        }
    }

    pub fn method_def(
        receiver: MethodReceiver,
        name: impl Into<String>,
        params: Vec<ParamDecl>,
        return_type: Option<String>,
        body: Expr,
    ) -> Self {
        let generic_params = generic_param_decls_from_names(&receiver.generics);
        Self::Def {
            generics: receiver.generics.clone(),
            receiver: Some(receiver),
            name: name.into(),
            attrs: Vec::new(),
            visibility: Visibility::Public,
            params,
            generic_params,
            return_type,
            body,
        }
    }

    pub fn static_value(name: impl Into<String>, ty: Option<String>, body: Expr) -> Self {
        Self::StaticValue {
            name: name.into(),
            attrs: Vec::new(),
            visibility: Visibility::Public,
            ty,
            body,
        }
    }

    pub fn extern_def(
        name: impl Into<String>,
        generics: Vec<String>,
        params: Vec<ParamDecl>,
        return_type: Option<String>,
        extern_decl: ExternDecl,
    ) -> Self {
        Self::ExternDef {
            name: name.into(),
            attrs: Vec::new(),
            visibility: Visibility::Public,
            receiver: None,
            generic_params: generic_param_decls_from_names(&generics),
            generics,
            params,
            return_type,
            extern_decl,
        }
    }

    pub fn with_visibility(self, visibility: Visibility) -> Self {
        match self {
            Self::Def {
                name,
                attrs,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                body,
                ..
            } => Self::Def {
                name,
                attrs,
                visibility,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                body,
            },
            Self::ExternDef {
                name,
                attrs,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                extern_decl,
                ..
            } => Self::ExternDef {
                name,
                attrs,
                visibility,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                extern_decl,
            },
            Self::StaticValue {
                name,
                attrs,
                ty,
                body,
                ..
            } => Self::StaticValue {
                name,
                attrs,
                visibility,
                ty,
                body,
            },
        }
    }

    pub fn with_attrs(self, attrs: Vec<ItemAttr>) -> Self {
        match self {
            Self::Def {
                name,
                visibility,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                body,
                ..
            } => Self::Def {
                name,
                attrs,
                visibility,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                body,
            },
            Self::ExternDef {
                name,
                visibility,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                extern_decl,
                ..
            } => Self::ExternDef {
                name,
                attrs,
                visibility,
                receiver,
                generic_params,
                generics,
                params,
                return_type,
                extern_decl,
            },
            Self::StaticValue {
                name,
                visibility,
                ty,
                body,
                ..
            } => Self::StaticValue {
                name,
                attrs,
                visibility,
                ty,
                body,
            },
        }
    }
}

pub fn generic_param_decls_from_names(names: &[String]) -> Vec<GenericParamDecl> {
    names
        .iter()
        .map(|name| GenericParamDecl::new(name.clone()))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Var(String),
    Lit(Literal),
    Lambda {
        param: String,
        body: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Instantiate {
        callee: Box<Expr>,
        type_args: Vec<String>,
    },
    Tuple(Vec<Expr>),
    SliceLiteral(Vec<Expr>),
    Record(Vec<RecordField>),
    RecordUpdate {
        base: Box<Expr>,
        fields: Vec<RecordField>,
    },
    AdtCtor {
        data: String,
        ctor: String,
        variants: Vec<String>,
        args: Vec<Expr>,
    },
    Field {
        receiver: Box<Expr>,
        name: String,
    },
    MethodCall {
        receiver: Box<Expr>,
        name: String,
        args: Vec<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Index {
        receiver: Box<Expr>,
        index: Box<Expr>,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    IfLet {
        pattern: Pattern,
        scrutinee: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Nominal {
        name: String,
        expr: Box<Expr>,
    },
    Reset {
        multi: bool,
        body: Box<Expr>,
    },
    Shift {
        binder: String,
        body: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordField {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    Wildcard,
    Bind(String),
    Tuple(Vec<Pattern>),
    Record(Vec<RecordPatternField>),
    Constructor {
        data: Option<String>,
        ctor: String,
        args: Vec<Pattern>,
    },
    At {
        name: String,
        pattern: Box<Pattern>,
    },
    Lit(Literal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordPatternField {
    pub name: String,
    pub pattern: Pattern,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    I64(i64),
    Rune(u32),
    Bool(bool),
    String(String),
    CStr(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub fn render_source_expr(expr: &Expr) -> String {
    match expr {
        Expr::Var(name) => name.clone(),
        Expr::Lit(literal) => render_source_literal(literal),
        Expr::Lambda { param, body } => format!("fn {param} => {}", render_source_expr(body)),
        Expr::Call { callee, args } => {
            let args = args
                .iter()
                .map(render_source_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({args})", render_source_expr(callee))
        }
        Expr::Instantiate { callee, type_args } => {
            format!("{}[{}]", render_source_expr(callee), type_args.join(", "))
        }
        Expr::Tuple(fields) => {
            let fields = fields
                .iter()
                .map(render_source_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({fields})")
        }
        Expr::SliceLiteral(items) => {
            let items = items
                .iter()
                .map(render_source_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{items}]")
        }
        Expr::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_source_expr(&field.value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        Expr::RecordUpdate { base, fields } => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_source_expr(&field.value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{} | {fields}}}", render_source_expr(base))
        }
        Expr::AdtCtor {
            data, ctor, args, ..
        } => {
            let args = args
                .iter()
                .map(render_source_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{data}.{ctor}({args})")
        }
        Expr::Field { receiver, name } => format!("{}.{}", render_source_expr(receiver), name),
        Expr::MethodCall {
            receiver,
            name,
            args,
        } => {
            let args = args
                .iter()
                .map(render_source_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}.{}({args})", render_source_expr(receiver), name)
        }
        Expr::Assign { target, value } => {
            format!(
                "{} := {}",
                render_source_expr(target),
                render_source_expr(value)
            )
        }
        Expr::Index { receiver, index } => {
            format!(
                "{}[{}]",
                render_source_expr(receiver),
                render_source_expr(index)
            )
        }
        Expr::Range { start, end } => {
            format!("{}..{}", render_source_expr(start), render_source_expr(end))
        }
        Expr::Binary { op, lhs, rhs } => format!(
            "{} {} {}",
            render_source_expr(lhs),
            render_source_binary_op(*op),
            render_source_expr(rhs)
        ),
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => format!(
            "if {} {{ {} }} else {{ {} }}",
            render_source_expr(cond),
            render_source_expr(then_branch),
            render_source_expr(else_branch)
        ),
        Expr::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => format!(
            "if let {} = {} {{ {} }} else {{ {} }}",
            render_source_pattern(pattern),
            render_source_expr(scrutinee),
            render_source_expr(then_branch),
            render_source_expr(else_branch)
        ),
        Expr::Match { scrutinee, arms } => {
            let arms = arms
                .iter()
                .map(|arm| {
                    format!(
                        "{} => {}",
                        render_source_pattern(&arm.pattern),
                        render_source_expr(&arm.body)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("match {} {{ {arms} }}", render_source_expr(scrutinee))
        }
        Expr::Nominal { name, expr } => format!("{name}({})", render_source_expr(expr)),
        Expr::Reset { multi, body } => {
            let name = if *multi { "resetn" } else { "reset" };
            format!("{name} {{ {} }}", render_source_expr(body))
        }
        Expr::Shift { binder, body } => {
            format!("shift {binder} {{ {} }}", render_source_expr(body))
        }
    }
}

pub fn render_source_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Bind(name) => name.clone(),
        Pattern::Tuple(fields) => {
            let fields = fields
                .iter()
                .map(render_source_pattern)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({fields})")
        }
        Pattern::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_source_pattern(&field.pattern)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        Pattern::Constructor { data, ctor, args } => {
            let args = args
                .iter()
                .map(render_source_pattern)
                .collect::<Vec<_>>()
                .join(", ");
            match data {
                Some(data) => format!("{data}.{ctor}({args})"),
                None => format!("{ctor}({args})"),
            }
        }
        Pattern::At { name, pattern } => {
            format!("{name} @ {}", render_source_pattern(pattern))
        }
        Pattern::Lit(literal) => render_source_literal(literal),
    }
}

pub fn render_source_literal(literal: &Literal) -> String {
    match literal {
        Literal::I64(value) => value.to_string(),
        Literal::Rune(value) => format!("'{}'", char::from_u32(*value).unwrap_or('\u{fffd}')),
        Literal::Bool(value) => value.to_string(),
        Literal::String(value) => format!("{value:?}"),
        Literal::CStr(value) => format!("c{value:?}"),
    }
}

pub fn render_source_binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
    }
}

impl Expr {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }

    pub fn i64(value: i64) -> Self {
        Self::Lit(Literal::I64(value))
    }

    pub fn rune(value: u32) -> Self {
        Self::Lit(Literal::Rune(value))
    }

    pub fn bool(value: bool) -> Self {
        Self::Lit(Literal::Bool(value))
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::Lit(Literal::String(value.into()))
    }

    pub fn cstr(value: impl Into<String>) -> Self {
        Self::Lit(Literal::CStr(value.into()))
    }

    pub fn lambda(param: impl Into<String>, body: Expr) -> Self {
        Self::Lambda {
            param: param.into(),
            body: Box::new(body),
        }
    }

    pub fn call(callee: Expr, arg: Expr) -> Self {
        Self::call_args(callee, vec![arg])
    }

    pub fn call_args(callee: Expr, args: Vec<Expr>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
        }
    }

    pub fn instantiate(callee: Expr, type_args: Vec<String>) -> Self {
        Self::Instantiate {
            callee: Box::new(callee),
            type_args,
        }
    }

    pub fn tuple(fields: Vec<Expr>) -> Self {
        Self::Tuple(fields)
    }

    pub fn slice_literal(items: Vec<Expr>) -> Self {
        Self::SliceLiteral(items)
    }

    pub fn record(fields: Vec<(impl Into<String>, Expr)>) -> Self {
        Self::Record(
            fields
                .into_iter()
                .map(|(name, value)| RecordField {
                    name: name.into(),
                    value,
                })
                .collect(),
        )
    }

    pub fn record_update(base: Expr, fields: Vec<(impl Into<String>, Expr)>) -> Self {
        Self::RecordUpdate {
            base: Box::new(base),
            fields: fields
                .into_iter()
                .map(|(name, value)| RecordField {
                    name: name.into(),
                    value,
                })
                .collect(),
        }
    }

    pub fn adt_ctor(
        data: impl Into<String>,
        ctor: impl Into<String>,
        variants: Vec<impl Into<String>>,
        args: Vec<Expr>,
    ) -> Self {
        Self::AdtCtor {
            data: data.into(),
            ctor: ctor.into(),
            variants: variants.into_iter().map(Into::into).collect(),
            args,
        }
    }

    pub fn field(receiver: Expr, name: impl Into<String>) -> Self {
        Self::Field {
            receiver: Box::new(receiver),
            name: name.into(),
        }
    }

    pub fn method_call(receiver: Expr, name: impl Into<String>, arg: Expr) -> Self {
        Self::method_call_args(receiver, name, vec![arg])
    }

    pub fn method_call_args(receiver: Expr, name: impl Into<String>, args: Vec<Expr>) -> Self {
        Self::MethodCall {
            receiver: Box::new(receiver),
            name: name.into(),
            args,
        }
    }

    pub fn assign(target: Expr, value: Expr) -> Self {
        Self::Assign {
            target: Box::new(target),
            value: Box::new(value),
        }
    }

    pub fn index(receiver: Expr, index: Expr) -> Self {
        Self::Index {
            receiver: Box::new(receiver),
            index: Box::new(index),
        }
    }

    pub fn range(start: Expr, end: Expr) -> Self {
        Self::Range {
            start: Box::new(start),
            end: Box::new(end),
        }
    }

    pub fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Self {
        Self::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn if_else(cond: Expr, then_branch: Expr, else_branch: Expr) -> Self {
        Self::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    pub fn match_expr(scrutinee: Expr, arms: Vec<(Pattern, Expr)>) -> Self {
        Self::Match {
            scrutinee: Box::new(scrutinee),
            arms: arms
                .into_iter()
                .map(|(pattern, body)| MatchArm { pattern, body })
                .collect(),
        }
    }

    pub fn if_let(pattern: Pattern, scrutinee: Expr, then_branch: Expr, else_branch: Expr) -> Self {
        Self::IfLet {
            pattern,
            scrutinee: Box::new(scrutinee),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    pub fn nominal(name: impl Into<String>, expr: Expr) -> Self {
        Self::Nominal {
            name: name.into(),
            expr: Box::new(expr),
        }
    }

    pub fn reset(body: Expr) -> Self {
        Self::Reset {
            multi: false,
            body: Box::new(body),
        }
    }

    pub fn resetn(body: Expr) -> Self {
        Self::Reset {
            multi: true,
            body: Box::new(body),
        }
    }

    pub fn shift(binder: impl Into<String>, body: Expr) -> Self {
        Self::Shift {
            binder: binder.into(),
            body: Box::new(body),
        }
    }
}

impl Pattern {
    pub fn wildcard() -> Self {
        Self::Wildcard
    }

    pub fn bind(name: impl Into<String>) -> Self {
        Self::Bind(name.into())
    }

    pub fn tuple(fields: Vec<Pattern>) -> Self {
        Self::Tuple(fields)
    }

    pub fn record(fields: Vec<(impl Into<String>, Pattern)>) -> Self {
        Self::Record(
            fields
                .into_iter()
                .map(|(name, pattern)| RecordPatternField {
                    name: name.into(),
                    pattern,
                })
                .collect(),
        )
    }

    pub fn ctor(ctor: impl Into<String>, args: Vec<Pattern>) -> Self {
        Self::Constructor {
            data: None,
            ctor: ctor.into(),
            args,
        }
    }

    pub fn qualified_ctor(
        data: impl Into<String>,
        ctor: impl Into<String>,
        args: Vec<Pattern>,
    ) -> Self {
        Self::Constructor {
            data: Some(data.into()),
            ctor: ctor.into(),
            args,
        }
    }

    pub fn at(name: impl Into<String>, pattern: Pattern) -> Self {
        Self::At {
            name: name.into(),
            pattern: Box::new(pattern),
        }
    }

    pub fn lit_i64(value: i64) -> Self {
        Self::Lit(Literal::I64(value))
    }

    pub fn lit_bool(value: bool) -> Self {
        Self::Lit(Literal::Bool(value))
    }

    pub fn primary_binding_name(&self) -> Option<&str> {
        match self {
            Pattern::Bind(name) | Pattern::At { name, .. } => Some(name.as_str()),
            Pattern::Tuple(fields) => fields.iter().find_map(Pattern::primary_binding_name),
            Pattern::Record(fields) => fields
                .iter()
                .find_map(|field| field.pattern.primary_binding_name()),
            Pattern::Constructor { args, .. } => {
                args.iter().find_map(Pattern::primary_binding_name)
            }
            Pattern::Wildcard | Pattern::Lit(_) => None,
        }
    }

    pub fn bindings(&self) -> Vec<String> {
        match self {
            Pattern::Bind(name) => vec![name.clone()],
            Pattern::Tuple(fields) => fields.iter().flat_map(Pattern::bindings).collect(),
            Pattern::Record(fields) => fields
                .iter()
                .flat_map(|field| field.pattern.bindings())
                .collect(),
            Pattern::Constructor { args, .. } => args.iter().flat_map(Pattern::bindings).collect(),
            Pattern::At { name, pattern } => {
                let mut bindings = pattern.bindings();
                bindings.push(name.clone());
                bindings
            }
            Pattern::Wildcard | Pattern::Lit(_) => Vec::new(),
        }
    }
}
