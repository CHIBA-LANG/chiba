use std::collections::BTreeMap;

use crate::alpha::{AlphaExpr, AlphaExprKind};
use crate::ast::BinaryOp;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolveFacts {
    pub methods: MethodIndex,
    pub resolved_calls: Vec<ResolvedCall>,
    pub operator_obligations: Vec<OperatorObligation>,
    pub diagnostics: Vec<ResolveDiagnostic>,
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
pub struct OperatorObligation {
    pub op: BinaryOp,
    pub protocol: String,
    pub receiver: Option<String>,
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
}

pub fn resolve_expr(expr: &AlphaExpr, methods: MethodIndex) -> ResolveFacts {
    let mut facts = ResolveFacts {
        methods,
        ..ResolveFacts::default()
    };
    visit(expr, &mut facts);
    facts
}

impl MethodIndex {
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
        AlphaExprKind::Var(_) | AlphaExprKind::Lit(_) => {}
        AlphaExprKind::Lambda { body, .. } => visit(body, facts),
        AlphaExprKind::Call { callee, arg } => {
            visit(callee, facts);
            visit(arg, facts);
        }
        AlphaExprKind::Field { receiver, .. } => visit(receiver, facts),
        AlphaExprKind::MethodCall {
            receiver,
            name,
            arg,
        } => {
            resolve_method_call(receiver, name, facts);
            visit(receiver, facts);
            visit(arg, facts);
        }
        AlphaExprKind::Binary { op, lhs, rhs } => {
            let receiver = nominal_name(lhs);
            facts.operator_obligations.push(OperatorObligation {
                op: op.clone(),
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
