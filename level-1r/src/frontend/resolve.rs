use std::collections::BTreeMap;

use crate::alpha::{AlphaExpr, AlphaExprKind};
use crate::ast::{BinaryOp, Visibility};
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
    pub owner: String,
    pub visibility: Visibility,
    pub kind: NameCandidateKind,
    pub arity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameCandidateKind {
    Function,
    Static,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructorCandidate {
    pub data: String,
    pub ctor: String,
    pub symbol: String,
    pub owner: String,
    pub arity: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MethodIndex {
    methods: BTreeMap<(String, String), Vec<MethodCandidate>>,
    qualified: BTreeMap<String, Vec<MethodCandidate>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodCandidate {
    pub receiver: String,
    pub name: String,
    pub symbol: String,
    pub owner: String,
    pub visibility: Visibility,
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
    Static {
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
        Self::from_interface_for_namespace(interface, &interface.namespace)
    }

    pub fn from_interface_for_namespace(
        interface: &InterfaceSummary,
        current_namespace: &str,
    ) -> Self {
        let mut index = Self::default();
        let mut name_candidates = Vec::new();
        for function in &interface.functions {
            if function.receiver.is_none() {
                if !is_visible_from(function.visibility, &function.owner, current_namespace) {
                    continue;
                }
                name_candidates.push(NameCandidate {
                    name: function.source_name.clone(),
                    symbol: function.symbol.clone(),
                    owner: function.owner.clone(),
                    visibility: function.visibility,
                    kind: NameCandidateKind::Function,
                    arity: function.arity,
                });
            }
        }
        for static_value in &interface.statics {
            if !is_visible_from(
                static_value.visibility,
                &static_value.owner,
                current_namespace,
            ) {
                continue;
            }
            name_candidates.push(NameCandidate {
                name: static_value.source_name.clone(),
                symbol: static_value.symbol.clone(),
                owner: static_value.owner.clone(),
                visibility: static_value.visibility,
                kind: NameCandidateKind::Static,
                arity: 0,
            });
        }
        for candidate in visible_name_candidates(name_candidates, current_namespace) {
            index
                .functions
                .entry(candidate.name.clone())
                .or_default()
                .push(candidate);
        }
        for ctor in visible_constructors(interface, current_namespace) {
            index.add_constructor(
                &ctor.data,
                &ctor.name,
                &ctor.symbol,
                &ctor.owner,
                ctor.arity,
            );
        }
        index
    }

    pub fn add_function(&mut self, name: impl Into<String>, symbol: impl Into<String>) {
        let symbol = symbol.into();
        self.add_function_with_arity(name, symbol, "root", Visibility::Public, 0);
    }

    pub fn add_static(
        &mut self,
        name: impl Into<String>,
        symbol: impl Into<String>,
        owner: impl Into<String>,
        visibility: Visibility,
    ) {
        let name = name.into();
        let symbol = symbol.into();
        let owner = owner.into();
        self.functions
            .entry(name.clone())
            .or_default()
            .push(NameCandidate {
                name,
                symbol,
                owner,
                visibility,
                kind: NameCandidateKind::Static,
                arity: 0,
            });
    }

    pub fn add_function_with_arity(
        &mut self,
        name: impl Into<String>,
        symbol: impl Into<String>,
        owner: impl Into<String>,
        visibility: Visibility,
        arity: usize,
    ) {
        let name = name.into();
        let symbol = symbol.into();
        let owner = owner.into();
        self.functions
            .entry(name.clone())
            .or_default()
            .push(NameCandidate {
                name,
                symbol,
                owner,
                visibility,
                kind: NameCandidateKind::Function,
                arity,
            });
    }

    pub fn add_constructor(
        &mut self,
        data: impl Into<String>,
        ctor: impl Into<String>,
        symbol: impl Into<String>,
        owner: impl Into<String>,
        arity: usize,
    ) {
        let data = data.into();
        let ctor = ctor.into();
        let symbol = symbol.into();
        let owner = owner.into();
        self.constructors
            .entry((data.clone(), ctor.clone()))
            .or_default()
            .push(ConstructorCandidate {
                data,
                ctor,
                symbol,
                owner,
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

fn visible_name_candidates(
    candidates: Vec<NameCandidate>,
    current_namespace: &str,
) -> Vec<NameCandidate> {
    let mut by_name = BTreeMap::<String, Vec<NameCandidate>>::new();
    for candidate in candidates {
        by_name
            .entry(candidate.name.clone())
            .or_default()
            .push(candidate);
    }
    by_name
        .into_values()
        .flat_map(|candidates| {
            let local = candidates
                .iter()
                .filter(|candidate| candidate.owner == current_namespace)
                .cloned()
                .collect::<Vec<_>>();
            if local.is_empty() {
                candidates
            } else {
                local
            }
        })
        .collect()
}

fn visible_constructors<'a>(
    interface: &'a InterfaceSummary,
    current_namespace: &str,
) -> Vec<&'a crate::surface::InterfaceConstructor> {
    let mut by_data_ctor =
        BTreeMap::<(String, String), Vec<&crate::surface::InterfaceConstructor>>::new();
    for ctor in &interface.constructors {
        by_data_ctor
            .entry((ctor.data.clone(), ctor.name.clone()))
            .or_default()
            .push(ctor);
    }
    by_data_ctor
        .into_values()
        .flat_map(|candidates| {
            let local = candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.owner == current_namespace)
                .collect::<Vec<_>>();
            if local.is_empty() {
                candidates
            } else {
                local
            }
        })
        .collect()
}

impl MethodIndex {
    pub fn from_interface(interface: &InterfaceSummary) -> Self {
        Self::from_interface_for_namespace(interface, &interface.namespace)
    }

    pub fn from_interface_for_namespace(
        interface: &InterfaceSummary,
        current_namespace: &str,
    ) -> Self {
        let mut index = Self::default();
        for function in visible_method_functions(interface, current_namespace) {
            let Some(receiver) = &function.receiver else {
                continue;
            };
            index.add_candidate(MethodCandidate {
                receiver: receiver.display_name(),
                name: function.source_name.clone(),
                symbol: function.symbol.clone(),
                owner: function.owner.clone(),
                visibility: function.visibility,
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
            owner: "root".to_string(),
            visibility: Visibility::Public,
        });
    }

    pub fn add_candidate(&mut self, candidate: MethodCandidate) {
        self.add_qualified_candidate(
            format!("{}.{}", candidate.receiver, candidate.name),
            &candidate,
        );
        self.add_qualified_candidate(candidate.symbol.clone(), &candidate);
        self.methods
            .entry((candidate.receiver.clone(), candidate.name.clone()))
            .or_default()
            .push(candidate);
    }

    fn add_qualified_candidate(&mut self, path: String, candidate: &MethodCandidate) {
        let candidates = self.qualified.entry(path).or_default();
        if !candidates
            .iter()
            .any(|existing| existing.symbol == candidate.symbol)
        {
            candidates.push(candidate.clone());
        }
    }

    fn find_method(&self, receiver: &str, name: &str) -> &[MethodCandidate] {
        self.methods
            .get(&(receiver.to_string(), name.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn find_qualified(&self, path: &str) -> &[MethodCandidate] {
        self.qualified.get(path).map(Vec::as_slice).unwrap_or(&[])
    }
}

fn visible_method_functions<'a>(
    interface: &'a InterfaceSummary,
    current_namespace: &str,
) -> Vec<&'a crate::surface::InterfaceFunction> {
    let mut by_receiver_name =
        BTreeMap::<(String, String), Vec<&crate::surface::InterfaceFunction>>::new();
    for function in &interface.functions {
        let Some(receiver) = &function.receiver else {
            continue;
        };
        if !is_visible_from(function.visibility, &function.owner, current_namespace) {
            continue;
        }
        by_receiver_name
            .entry((receiver.display_name(), function.source_name.clone()))
            .or_default()
            .push(function);
    }
    by_receiver_name
        .into_values()
        .flat_map(|candidates| {
            let local = candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.owner == current_namespace)
                .collect::<Vec<_>>();
            if local.is_empty() {
                candidates
            } else {
                local
            }
        })
        .collect()
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
        AlphaExprKind::SliceLiteral(items) => {
            for item in items {
                visit(item, facts);
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
            data, ctor, args, ..
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
        AlphaExprKind::Assign { target, value } => {
            visit(target, facts);
            visit(value, facts);
        }
        AlphaExprKind::Index { receiver, index } => {
            let op = if matches!(&index.kind, AlphaExprKind::Range { .. }) {
                OperatorSurface::IndexSlice
            } else {
                OperatorSurface::Index
            };
            facts.operator_obligations.push(OperatorObligation {
                protocol: operator_surface_protocol(&op),
                op,
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
        [candidate] => push_resolved_name(name, candidate, facts),
        many => facts.diagnostics.push(ResolveDiagnostic::AmbiguousName {
            name: name.to_string(),
            candidates: many
                .iter()
                .map(|candidate| candidate.symbol.clone())
                .collect(),
        }),
    }
}

fn resolve_function_call(name: &str, arity: usize, facts: &mut ResolveFacts) {
    let candidates = facts
        .names
        .find_function(name)
        .iter()
        .filter(|candidate| candidate.kind == NameCandidateKind::Function)
        .cloned()
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => {}
        [candidate] if candidate.arity == arity => {
            facts.resolved_names.push(ResolvedName::Function {
                name: name.to_string(),
                symbol: candidate.symbol.clone(),
            })
        }
        [candidate] => facts
            .diagnostics
            .push(ResolveDiagnostic::FunctionArityMismatch {
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
                    candidates: many
                        .iter()
                        .map(|candidate| candidate.symbol.clone())
                        .collect(),
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

fn push_resolved_name(name: &str, candidate: &NameCandidate, facts: &mut ResolveFacts) {
    match candidate.kind {
        NameCandidateKind::Function => facts.resolved_names.push(ResolvedName::Function {
            name: name.to_string(),
            symbol: candidate.symbol.clone(),
        }),
        NameCandidateKind::Static => facts.resolved_names.push(ResolvedName::Static {
            name: name.to_string(),
            symbol: candidate.symbol.clone(),
        }),
    }
}

fn resolve_constructor_name(data: &str, ctor: &str, arity: usize, facts: &mut ResolveFacts) {
    let candidates = facts.names.find_constructor(data, ctor).to_vec();
    match candidates.as_slice() {
        [] => {
            if !facts.names.constructors.is_empty() {
                facts
                    .diagnostics
                    .push(ResolveDiagnostic::MissingConstructor {
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
        [candidate] => facts
            .diagnostics
            .push(ResolveDiagnostic::ConstructorArityMismatch {
                data: data.to_string(),
                ctor: ctor.to_string(),
                expected: candidate.arity,
                actual: arity,
            }),
        many => facts
            .diagnostics
            .push(ResolveDiagnostic::AmbiguousConstructor {
                data: data.to_string(),
                ctor: ctor.to_string(),
                candidates: many
                    .iter()
                    .map(|candidate| candidate.symbol.clone())
                    .collect(),
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
                candidates: many
                    .iter()
                    .map(|candidate| candidate.symbol.clone())
                    .collect(),
            }),
        }
        return;
    }

    let qualified = format!("{}.{}", receiver_path(receiver), name);
    let candidates = facts.methods.find_qualified(&qualified).to_vec();
    match candidates.as_slice() {
        [] => facts.diagnostics.push(ResolveDiagnostic::MissingMethod {
            receiver: None,
            name: name.to_string(),
        }),
        [candidate] => facts.resolved_calls.push(ResolvedCall::QualifiedCallee {
            path: qualified,
            symbol: candidate.symbol.clone(),
        }),
        many => facts.diagnostics.push(ResolveDiagnostic::AmbiguousMethod {
            receiver: receiver_path(receiver),
            name: name.to_string(),
            candidates: many
                .iter()
                .map(|candidate| candidate.symbol.clone())
                .collect(),
        }),
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
    operator_surface_protocol(&OperatorSurface::Binary(op.clone()))
}

fn operator_surface_protocol(op: &OperatorSurface) -> String {
    match op {
        OperatorSurface::Binary(BinaryOp::Add) => "op_add",
        OperatorSurface::Binary(BinaryOp::Sub) => "op_sub",
        OperatorSurface::Binary(BinaryOp::Mul) => "op_mul",
        OperatorSurface::Binary(BinaryOp::Div) => "op_div",
        OperatorSurface::Index => "op_index",
        OperatorSurface::IndexSlice => "op_index_slice",
    }
    .to_string()
}

fn is_visible_from(visibility: Visibility, owner: &str, current_namespace: &str) -> bool {
    matches!(visibility, Visibility::Public) || owner == current_namespace
}
