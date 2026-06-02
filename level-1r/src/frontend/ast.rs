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
        receiver: Option<MethodReceiver>,
        generics: Vec<String>,
        params: Vec<ParamDecl>,
        return_type: Option<String>,
        body: Expr,
    },
    StaticValue {
        name: String,
        ty: Option<String>,
        body: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDecl {
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodReceiver {
    pub name: String,
    pub generics: Vec<String>,
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
    pub fn new(
        name: impl Into<String>,
        generics: Vec<String>,
        variants: Vec<DataVariant>,
    ) -> Self {
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
    pub fn new(
        name: impl Into<String>,
        generics: Vec<String>,
        fields: Vec<TypeField>,
    ) -> Self {
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
        Self {
            name: name.into(),
            ty,
        }
    }

    pub fn untyped(name: impl Into<String>) -> Self {
        Self::new(name, None)
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
            receiver: None,
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
        Self::Def {
            generics: receiver.generics.clone(),
            receiver: Some(receiver),
            name: name.into(),
            params,
            return_type,
            body,
        }
    }

    pub fn static_value(name: impl Into<String>, ty: Option<String>, body: Expr) -> Self {
        Self::StaticValue {
            name: name.into(),
            ty,
            body,
        }
    }
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
    Bool(bool),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl Expr {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }

    pub fn i64(value: i64) -> Self {
        Self::Lit(Literal::I64(value))
    }

    pub fn bool(value: bool) -> Self {
        Self::Lit(Literal::Bool(value))
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

    pub fn qualified_ctor(data: impl Into<String>, ctor: impl Into<String>, args: Vec<Pattern>) -> Self {
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
}
