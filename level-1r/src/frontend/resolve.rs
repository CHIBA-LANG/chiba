use std::collections::BTreeMap;

use crate::alpha::{AlphaExpr, AlphaExprKind};
use crate::ast::BinaryOp;
use crate::surface::InterfaceSummary;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolveFacts {
    pub methods: MethodIndex,
    pub names: NameIndex,
    pub resolved_names: Vec<ResolvedName>,
    pub resolved_calls: Vec<ResolvedCall>,
    pub operator_obligations: Vec<OperatorObligation>,
    pub diagnostics: Vec<ResolveDiagnostic>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameIndex {
    functions: BTreeMap<String, Vec<NameCandidate>>,
    constructors: BTreeMap<(String, String), Vec<ConstructorCandidate>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameCandidate {
    pub name: String,
    pub symbol: String,
    pub arity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructorCandidate {
    pub data: String,
    pub ctor: String,
    pub symbol: String,
    pub arity: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MethodIndex {
    methods: BTreeMap<(String, String), Vec<MethodCandidate>>,
    qualified: BTreeMap<String, MethodCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodCandidate {
    pub receiver: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedCall {
    FieldCallable {
        field: String,
    },
    ReceiverMethod {
        receiver: String,
        name: String,
        symbol: String,
    },
    QualifiedCallee {
        path: String,
        symbol: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedName {
    Function {
        name: String,
        symbol: String,
    },
    Constructor {
        data: String,
        ctor: String,
        symbol: String,
        arity: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorObligation {
    pub op: OperatorSurface,
    pub protocol: String,
    pub receiver: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperatorSurface {
    Binary(BinaryOp),
    Index,
    IndexSlice,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveDiagnostic {
    AmbiguousMethod {
        receiver: String,
        name: String,
        candidates: Vec<String>,
    },
    MissingMethod {
        receiver: Option<String>,
        name: String,
    },
    AmbiguousName {
        name: String,
        candidates: Vec<String>,
    },
    FunctionArityMismatch {
        name: String,
        symbol: String,
        expected: usize,
        actual: usize,
    },
    AmbiguousConstructor {
        data: String,
        ctor: String,
        candidates: Vec<String>,
    },
    MissingConstructor {
        data: String,
        ctor: String,
    },
    ConstructorArityMismatch {
        data: String,
        ctor: String,
        expected: usize,
        actual: usize,
    },
}

pub fn resolve_expr(expr: &AlphaExpr, methods: MethodIndex) -> ResolveFacts {
    let mut facts = ResolveFacts {
        methods,
        ..ResolveFacts::default()
    };
    visit(expr, &mut facts);
    facts
}

pub fn resolve_expr_with_names(
    expr: &AlphaExpr,
    methods: MethodIndex,
    names: NameIndex,
) -> ResolveFacts {
    let mut facts = ResolveFacts {
        methods,
        names,
        ..ResolveFacts::default()
    };
    visit(expr, &mut facts);
    facts
}

impl NameIndex {
    pub fn from_interface(interface: &InterfaceSummary) -> Self {
        let mut index = Self::default();
        for function in &interface.functions {
            if function.receiver.is_none() {
                index.add_function_with_arity(
                    &function.source_name,
                    &function.symbol,
                    function.arity,
                );
            }
        }
        for ctor in &interface.constructors {
            if let Some((data, ctor_name)) = data_ctor_from_symbol(&ctor.symbol) {
                index.add_constructor(data, ctor_name, &ctor.symbol, ctor.arity);
            }
        }
        index
    }

    pub fn add_function(&mut self, name: impl Into<String>, symbol: impl Into<String>) {
        self.add_function_with_arity(name, symbol, 0);
    }

    pub fn add_function_with_arity(
        &mut self,
        name: impl Into<String>,
        symbol: impl Into<String>,
        arity: usize,
    ) {
        let name = name.into();
        let symbol = symbol.into();
        self.functions
            .entry(name.clone())
            .or_default()
            .push(NameCandidate { name, symbol, arity });
    }

    pub fn add_constructor(
        &mut self,
        data: impl Into<String>,
        ctor: impl Into<String>,
        symbol: impl Into<String>,
        arity: usize,
    ) {
        let data = data.into();
        let ctor = ctor.into();
        let symbol = symbol.into();
        self.constructors
            .entry((data.clone(), ctor.clone()))
            .or_default()
            .push(ConstructorCandidate {
                data,
                ctor,
                symbol,
                arity,
            });
    }

    fn find_function(&self, name: &str) -> &[NameCandidate] {
        self.functions.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    fn find_constructor(&self, data: &str, ctor: &str) -> &[ConstructorCandidate] {
        self.constructors
            .get(&(data.to_string(), ctor.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

impl MethodIndex {
    pub fn from_interface(interface: &InterfaceSummary) -> Self {
        let mut index = Self::default();
        for function in &interface.functions {
            let Some(receiver) = &function.receiver else {
                continue;
            };
            index.add_candidate(MethodCandidate {
                receiver: receiver.display_name(),
                name: function.source_name.clone(),
                symbol: function.symbol.clone(),
            });
        }
        index
    }

    pub fn add_method(&mut self, receiver: impl Into<String>, name: impl Into<String>) {
        let receiver = receiver.into();
        let name = name.into();
        let symbol = format!("{receiver}.{name}");
        self.add_candidate(MethodCandidate {
            receiver,
            name,
            symbol,
        });
    }

    pub fn add_candidate(&mut self, candidate: MethodCandidate) {
        self.qualified.insert(
            format!("{}.{}", candidate.receiver, candidate.name),
            candidate.clone(),
        );
        self.qualified
            .insert(candidate.symbol.clone(), candidate.clone());
        self.methods
            .entry((candidate.receiver.clone(), candidate.name.clone()))
            .or_default()
            .push(candidate);
    }

    fn find_method(&self, receiver: &str, name: &str) -> &[MethodCandidate] {
        self.methods
            .get(&(receiver.to_string(), name.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn find_qualified(&self, path: &str) -> Option<&MethodCandidate> {
        self.qualified.get(path)
    }
}

fn visit(expr: &AlphaExpr, facts: &mut ResolveFacts) {
    match &expr.kind {
        AlphaExprKind::Var(var) => {
            if var.target.is_none() {
                resolve_var_name(&var.name, facts);
            }
        }
        AlphaExprKind::Lit(_) => {}
        AlphaExprKind::Lambda { body, .. } => visit(body, facts),
        AlphaExprKind::Call { callee, args } => {
            match &callee.kind {
                AlphaExprKind::Var(var) if var.target.is_none() => {
                    resolve_function_call(&var.name, args.len(), facts);
                }
                _ => visit(callee, facts),
            }
            for arg in args {
                visit(arg, facts);
            }
        }
        AlphaExprKind::Tuple(fields) => {
            for field in fields {
                visit(field, facts);
            }
        }
        AlphaExprKind::Record(fields) => {
            for field in fields {
                visit(&field.value, facts);
            }
        }
        AlphaExprKind::RecordUpdate { base, fields } => {
            visit(base, facts);
            for field in fields {
                visit(&field.value, facts);
            }
        }
        AlphaExprKind::AdtCtor {
            data,
            ctor,
            args,
            ..
        } => {
            resolve_constructor_name(data, ctor, args.len(), facts);
            for arg in args {
                visit(arg, facts);
            }
        }
        AlphaExprKind::Field { receiver, .. } => visit(receiver, facts),
        AlphaExprKind::MethodCall {
            receiver,
            name,
            args,
        } => {
            resolve_method_call(receiver, name, facts);
            visit(receiver, facts);
            for arg in args {
                visit(arg, facts);
            }
        }
        AlphaExprKind::Index { receiver, index } => {
            let protocol = if matches!(&index.kind, AlphaExprKind::Range { .. }) {
                "op_index_slice"
            } else {
                "op_index"
            };
            facts.operator_obligations.push(OperatorObligation {
                op: if protocol == "op_index_slice" {
                    OperatorSurface::IndexSlice
                } else {
                    OperatorSurface::Index
                },
                protocol: protocol.to_string(),
                receiver: nominal_name(receiver),
            });
            visit(receiver, facts);
            visit(index, facts);
        }
        AlphaExprKind::Range { start, end } => {
            visit(start, facts);
            visit(end, facts);
        }
        AlphaExprKind::Binary { op, lhs, rhs } => {
            let receiver = nominal_name(lhs);
            facts.operator_obligations.push(OperatorObligation {
                op: OperatorSurface::Binary(op.clone()),
                protocol: operator_protocol(op),
                receiver,
            });
            visit(lhs, facts);
            visit(rhs, facts);
        }
        AlphaExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit(cond, facts);
            visit(then_branch, facts);
            visit(else_branch, facts);
        }
        AlphaExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            visit(scrutinee, facts);
            visit(then_branch, facts);
            visit(else_branch, facts);
        }
        AlphaExprKind::Match { scrutinee, arms } => {
            visit(scrutinee, facts);
            for arm in arms {
                visit(&arm.body, facts);
            }
        }
        AlphaExprKind::Nominal { expr, .. } => visit(expr, facts),
        AlphaExprKind::Reset { body, .. } | AlphaExprKind::Shift { body, .. } => visit(body, facts),
    }
}

fn resolve_var_name(name: &str, facts: &mut ResolveFacts) {
    let candidates = facts.names.find_function(name).to_vec();
    match candidates.as_slice() {
        [] => {}
        [candidate] => facts.resolved_names.push(ResolvedName::Function {
            name: name.to_string(),
            symbol: candidate.symbol.clone(),
        }),
        many => facts.diagnostics.push(ResolveDiagnostic::AmbiguousName {
            name: name.to_string(),
            candidates: many.iter().map(|candidate| candidate.symbol.clone()).collect(),
        }),
    }
}

fn resolve_function_call(name: &str, arity: usize, facts: &mut ResolveFacts) {
    let candidates = facts.names.find_function(name).to_vec();
    match candidates.as_slice() {
        [] => {}
        [candidate] if candidate.arity == arity => facts.resolved_names.push(
            ResolvedName::Function {
                name: name.to_string(),
                symbol: candidate.symbol.clone(),
            },
        ),
        [candidate] => facts.diagnostics.push(ResolveDiagnostic::FunctionArityMismatch {
            name: name.to_string(),
            symbol: candidate.symbol.clone(),
            expected: candidate.arity,
            actual: arity,
        }),
        many => {
            let arity_matches = many
                .iter()
                .filter(|candidate| candidate.arity == arity)
                .collect::<Vec<_>>();
            match arity_matches.as_slice() {
                [candidate] => facts.resolved_names.push(ResolvedName::Function {
                    name: name.to_string(),
                    symbol: candidate.symbol.clone(),
                }),
                [] => facts.diagnostics.push(ResolveDiagnostic::AmbiguousName {
                    name: name.to_string(),
                    candidates: many.iter().map(|candidate| candidate.symbol.clone()).collect(),
                }),
                matches => facts.diagnostics.push(ResolveDiagnostic::AmbiguousName {
                    name: name.to_string(),
                    candidates: matches
                        .iter()
                        .map(|candidate| candidate.symbol.clone())
                        .collect(),
                }),
            }
        }
    }
}

fn resolve_constructor_name(data: &str, ctor: &str, arity: usize, facts: &mut ResolveFacts) {
    let candidates = facts.names.find_constructor(data, ctor).to_vec();
    match candidates.as_slice() {
        [] => {
            if !facts.names.constructors.is_empty() {
                facts.diagnostics.push(ResolveDiagnostic::MissingConstructor {
                    data: data.to_string(),
                    ctor: ctor.to_string(),
                });
            }
        }
        [candidate] if candidate.arity == arity => {
            facts.resolved_names.push(ResolvedName::Constructor {
                data: data.to_string(),
                ctor: ctor.to_string(),
                symbol: candidate.symbol.clone(),
                arity: candidate.arity,
            });
        }
        [candidate] => facts.diagnostics.push(ResolveDiagnostic::ConstructorArityMismatch {
            data: data.to_string(),
            ctor: ctor.to_string(),
            expected: candidate.arity,
            actual: arity,
        }),
        many => facts.diagnostics.push(ResolveDiagnostic::AmbiguousConstructor {
            data: data.to_string(),
            ctor: ctor.to_string(),
            candidates: many.iter().map(|candidate| candidate.symbol.clone()).collect(),
        }),
    }
}

fn resolve_method_call(receiver: &AlphaExpr, name: &str, facts: &mut ResolveFacts) {
    if has_field(receiver, name) {
        facts.resolved_calls.push(ResolvedCall::FieldCallable {
            field: name.to_string(),
        });
        return;
    }

    if let Some(receiver_name) = nominal_name(receiver) {
        let candidates = facts.methods.find_method(&receiver_name, name).to_vec();
        match candidates.as_slice() {
            [] => facts.diagnostics.push(ResolveDiagnostic::MissingMethod {
                receiver: Some(receiver_name),
                name: name.to_string(),
            }),
            [candidate] => facts.resolved_calls.push(ResolvedCall::ReceiverMethod {
                receiver: receiver_name,
                name: name.to_string(),
                symbol: candidate.symbol.clone(),
            }),
            many => facts.diagnostics.push(ResolveDiagnostic::AmbiguousMethod {
                receiver: receiver_name,
                name: name.to_string(),
                candidates: many.iter().map(|candidate| candidate.symbol.clone()).collect(),
            }),
        }
        return;
    }

    let qualified = format!("{}.{}", receiver_path(receiver), name);
    let qualified_candidate = facts.methods.find_qualified(&qualified).cloned();
    if let Some(candidate) = qualified_candidate {
        facts.resolved_calls.push(ResolvedCall::QualifiedCallee {
            path: qualified,
            symbol: candidate.symbol.clone(),
        });
    } else {
        facts.diagnostics.push(ResolveDiagnostic::MissingMethod {
            receiver: None,
            name: name.to_string(),
        });
    }
}

fn has_field(receiver: &AlphaExpr, name: &str) -> bool {
    match &receiver.kind {
        AlphaExprKind::Field { name: field, .. } => field == name,
        _ => false,
    }
}

fn nominal_name(expr: &AlphaExpr) -> Option<String> {
    match &expr.kind {
        AlphaExprKind::Nominal { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn receiver_path(expr: &AlphaExpr) -> String {
    match &expr.kind {
        AlphaExprKind::Var(var) => var.name.clone(),
        AlphaExprKind::Nominal { name, .. } => name.clone(),
        _ => "<expr>".to_string(),
    }
}

fn operator_protocol(op: &BinaryOp) -> String {
    match op {
        BinaryOp::Add => "op_add",
        BinaryOp::Sub => "op_sub",
        BinaryOp::Mul => "op_mul",
        BinaryOp::Div => "op_div",
    }
    .to_string()
}

fn data_ctor_from_symbol(symbol: &str) -> Option<(&str, &str)> {
    let (_, tail) = symbol.rsplit_once("::")?;
    tail.split_once('.')
}
