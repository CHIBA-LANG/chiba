use crate::alpha::{AlphaExpr, AlphaExprKind};
use crate::ast::BinaryOp;
use crate::resolve::{OperatorObligation, ResolveFacts, ResolvedCall};
use crate::typed::{SendColor, UsageColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateFacts {
    pub row_shapes: Vec<RowShape>,
    pub obligations: Vec<TemplateObligation>,
    pub dyn_contracts: Vec<DynRowContract>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowField {
    pub name: String,
    pub ty: ShapeType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShapeType {
    Unknown,
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowShape {
    pub openness: RowOpenness,
    pub fields: Vec<RowField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RowOpenness {
    Open,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemplateObligation {
    Field {
        shape: RowShape,
        field: String,
    },
    Method {
        receiver: Option<String>,
        name: String,
        resolved: Option<String>,
    },
    Operator {
        op: BinaryOp,
        protocol: String,
        receiver: Option<String>,
    },
    DynAdapter {
        contract: DynRowContract,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynRowContract {
    pub shape: RowShape,
    pub payload_usage: UsageColor,
    pub send: SendColor,
    pub adapter: DynAdapterKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DynAdapterKind {
    StaticToDynPackage,
    DynAdapterAccess,
}

pub fn analyze_template(expr: &AlphaExpr, resolve: &ResolveFacts) -> TemplateFacts {
    let mut facts = TemplateFacts::default();
    collect_expr_obligations(expr, &mut facts);
    collect_resolve_obligations(resolve, &mut facts);
    facts
}

pub fn canonical_open_row(fields: Vec<(&str, ShapeType)>) -> RowShape {
    canonical_row(RowOpenness::Open, fields)
}

pub fn canonical_closed_row(fields: Vec<(&str, ShapeType)>) -> RowShape {
    canonical_row(RowOpenness::Closed, fields)
}

pub fn dyn_row_contract(fields: Vec<(&str, ShapeType)>) -> DynRowContract {
    DynRowContract {
        shape: canonical_open_row(fields),
        payload_usage: UsageColor::Many,
        send: SendColor::Obligation,
        adapter: DynAdapterKind::StaticToDynPackage,
    }
}

fn canonical_row(openness: RowOpenness, fields: Vec<(&str, ShapeType)>) -> RowShape {
    let mut fields = fields
        .into_iter()
        .map(|(name, ty)| RowField {
            name: name.to_string(),
            ty,
        })
        .collect::<Vec<_>>();
    fields.sort();
    fields.dedup_by(|a, b| a.name == b.name);
    RowShape { openness, fields }
}

fn collect_expr_obligations(expr: &AlphaExpr, facts: &mut TemplateFacts) {
    match &expr.kind {
        AlphaExprKind::Var(_) | AlphaExprKind::Lit(_) => {}
        AlphaExprKind::Lambda { body, .. } => collect_expr_obligations(body, facts),
        AlphaExprKind::Call { callee, arg } => {
            collect_expr_obligations(callee, facts);
            collect_expr_obligations(arg, facts);
        }
        AlphaExprKind::Tuple(fields) => {
            for field in fields {
                collect_expr_obligations(field, facts);
            }
        }
        AlphaExprKind::Record(fields) => {
            let shape = canonical_closed_row(
                fields
                    .iter()
                    .map(|field| (field.name.as_str(), ShapeType::Unknown))
                    .collect(),
            );
            facts.row_shapes.push(shape);
            for field in fields {
                collect_expr_obligations(&field.value, facts);
            }
        }
        AlphaExprKind::Field { receiver, name } => {
            let shape = canonical_open_row(vec![(name.as_str(), ShapeType::Unknown)]);
            facts.row_shapes.push(shape.clone());
            facts.obligations.push(TemplateObligation::Field {
                shape,
                field: name.clone(),
            });
            collect_expr_obligations(receiver, facts);
        }
        AlphaExprKind::MethodCall { receiver, arg, .. } => {
            collect_expr_obligations(receiver, facts);
            collect_expr_obligations(arg, facts);
        }
        AlphaExprKind::Binary { lhs, rhs, .. } => {
            collect_expr_obligations(lhs, facts);
            collect_expr_obligations(rhs, facts);
        }
        AlphaExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_expr_obligations(cond, facts);
            collect_expr_obligations(then_branch, facts);
            collect_expr_obligations(else_branch, facts);
        }
        AlphaExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_obligations(scrutinee, facts);
            collect_expr_obligations(then_branch, facts);
            collect_expr_obligations(else_branch, facts);
        }
        AlphaExprKind::Match { scrutinee, arms } => {
            collect_expr_obligations(scrutinee, facts);
            for arm in arms {
                collect_expr_obligations(&arm.body, facts);
            }
        }
        AlphaExprKind::Nominal { expr, .. } => collect_expr_obligations(expr, facts),
        AlphaExprKind::Reset { body, .. } | AlphaExprKind::Shift { body, .. } => {
            collect_expr_obligations(body, facts);
        }
    }
}

fn collect_resolve_obligations(resolve: &ResolveFacts, facts: &mut TemplateFacts) {
    for call in &resolve.resolved_calls {
        match call {
            ResolvedCall::FieldCallable { field } => {
                let shape = canonical_open_row(vec![(field.as_str(), ShapeType::Unknown)]);
                facts.row_shapes.push(shape.clone());
                facts.obligations.push(TemplateObligation::Field {
                    shape,
                    field: field.clone(),
                });
            }
            ResolvedCall::ReceiverMethod {
                receiver,
                name,
                symbol,
            } => facts.obligations.push(TemplateObligation::Method {
                receiver: Some(receiver.clone()),
                name: name.clone(),
                resolved: Some(symbol.clone()),
            }),
            ResolvedCall::QualifiedCallee { path, symbol } => {
                facts.obligations.push(TemplateObligation::Method {
                    receiver: None,
                    name: path.clone(),
                    resolved: Some(symbol.clone()),
                });
            }
        }
    }

    for OperatorObligation {
        op,
        protocol,
        receiver,
    } in &resolve.operator_obligations
    {
        facts.obligations.push(TemplateObligation::Operator {
            op: op.clone(),
            protocol: protocol.clone(),
            receiver: receiver.clone(),
        });
    }
}
