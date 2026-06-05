use crate::ast::{render_source_expr, Expr};
use crate::control::{ContinuationKind, ControlFacts};
use crate::core::{CoreProgram, OwnershipDecision};
use crate::typed::{Type, TypedExpr, UsageColor};
use crate::usage::UsageFacts;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsageAuditReport {
    pub entries: Vec<UsageAuditEntry>,
    pub diagnostics: Vec<UsageAuditDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageAuditEntry {
    pub subject: String,
    pub source_signature: String,
    pub typed_signature: String,
    pub usage_signature: String,
    pub rust_reference_signature: String,
    pub rust_reference_ownership: RustReferenceOwnership,
    pub usage: UsageColor,
    pub ownership: Option<OwnershipDecision>,
    pub aligned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustReferenceOwnership {
    MoveOnly,
    SharedRc,
    Obligation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageAuditDiagnostic {
    RcRequiresManyUsage {
        subject: String,
        usage: UsageColor,
    },
    ManyUsageNeedsSharedRustReference {
        subject: String,
        rust_reference_signature: String,
    },
}

pub fn audit_usage_lowering(
    source: &Expr,
    typed: &TypedExpr,
    usage: &UsageFacts,
    control: &ControlFacts,
    core: &CoreProgram,
) -> UsageAuditReport {
    let mut report = UsageAuditReport::default();
    collect_var_entries(source, typed, usage, core, &mut report);
    collect_continuation_entries(control, core, &mut report);
    report
}

fn collect_var_entries(
    source: &Expr,
    typed: &TypedExpr,
    usage: &UsageFacts,
    core: &CoreProgram,
    report: &mut UsageAuditReport,
) {
    for (name, count) in &usage.vars {
        let subject = format!("var::{name}");
        let color = count.color();
        let ty = type_for_var(typed, name).unwrap_or(Type::Unknown);
        let ownership = ownership_for(core, &subject);
        let entry = audit_entry(
            subject,
            render_source_expr(source),
            format!("{name}: {}", render_type(&ty)),
            format!("{name}: {} {}", render_usage(color), render_type(&ty)),
            rust_reference(color, &ty).with_binding(name),
            color,
            ownership,
        );
        push_entry(entry, report);
    }
}

fn collect_continuation_entries(
    control: &ControlFacts,
    core: &CoreProgram,
    report: &mut UsageAuditReport,
) {
    for fact in &control.continuations {
        let subject = format!("continuation::{}", fact.binder);
        let ty = match fact.kind {
            ContinuationKind::Cont1 => Type::Nominal("Cont1".to_string()),
            ContinuationKind::ContN => Type::Nominal("ContN".to_string()),
        };
        let ownership = ownership_for(core, &subject);
        let entry = audit_entry(
            subject,
            format!("shift {}", fact.binder),
            format!("{}: {}", fact.binder, render_type(&ty)),
            format!(
                "{}: {} {}",
                fact.binder,
                render_usage(fact.usage),
                render_type(&ty)
            ),
            continuation_rust_reference(fact.kind).with_binding(&fact.binder),
            fact.usage,
            ownership,
        );
        push_entry(entry, report);
    }
}

fn audit_entry(
    subject: String,
    source_signature: String,
    typed_signature: String,
    usage_signature: String,
    rust_reference: RustReference,
    usage: UsageColor,
    ownership: Option<OwnershipDecision>,
) -> UsageAuditEntry {
    let aligned = rust_reference_aligns_with_usage(rust_reference.ownership, usage);
    let rust_reference_ownership = rust_reference.ownership;
    let rust_reference_signature = rust_reference.signature();
    UsageAuditEntry {
        subject,
        source_signature,
        typed_signature,
        usage_signature,
        rust_reference_signature,
        rust_reference_ownership,
        usage,
        ownership,
        aligned,
    }
}

fn push_entry(entry: UsageAuditEntry, report: &mut UsageAuditReport) {
    if entry.rust_reference_ownership == RustReferenceOwnership::SharedRc
        && entry.usage != UsageColor::Many
    {
        report
            .diagnostics
            .push(UsageAuditDiagnostic::RcRequiresManyUsage {
                subject: entry.subject.clone(),
                usage: entry.usage,
            });
    }
    if entry.usage == UsageColor::Many
        && entry.rust_reference_ownership != RustReferenceOwnership::SharedRc
    {
        report
            .diagnostics
            .push(UsageAuditDiagnostic::ManyUsageNeedsSharedRustReference {
                subject: entry.subject.clone(),
                rust_reference_signature: entry.rust_reference_signature.clone(),
            });
    }
    report.entries.push(entry);
}

fn rust_reference_aligns_with_usage(ownership: RustReferenceOwnership, usage: UsageColor) -> bool {
    match usage {
        UsageColor::One => ownership != RustReferenceOwnership::SharedRc,
        UsageColor::Many => ownership == RustReferenceOwnership::SharedRc,
        UsageColor::Obligation => true,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RustReference {
    ty: String,
    ownership: RustReferenceOwnership,
}

impl RustReference {
    fn with_binding(self, name: &str) -> Self {
        Self {
            ty: format!("let {name}: {}", self.ty),
            ownership: self.ownership,
        }
    }

    fn signature(self) -> String {
        self.ty
    }
}

fn rust_reference(usage: UsageColor, ty: &Type) -> RustReference {
    let rendered = render_type(ty);
    match usage {
        UsageColor::One => RustReference {
            ty: rendered,
            ownership: RustReferenceOwnership::MoveOnly,
        },
        UsageColor::Many => RustReference {
            ty: format!("Rc<{rendered}>"),
            ownership: RustReferenceOwnership::SharedRc,
        },
        UsageColor::Obligation => RustReference {
            ty: format!("MaybeRc<{rendered}>"),
            ownership: RustReferenceOwnership::Obligation,
        },
    }
}

fn continuation_rust_reference(kind: ContinuationKind) -> RustReference {
    match kind {
        ContinuationKind::Cont1 => RustReference {
            ty: "Cont1Frame".to_string(),
            ownership: RustReferenceOwnership::MoveOnly,
        },
        ContinuationKind::ContN => RustReference {
            ty: "Rc<ContNFrame>".to_string(),
            ownership: RustReferenceOwnership::SharedRc,
        },
    }
}

fn render_usage(color: UsageColor) -> &'static str {
    match color {
        UsageColor::One => "1",
        UsageColor::Many => "N",
        UsageColor::Obligation => "?",
    }
}

fn render_type(ty: &Type) -> String {
    match ty {
        Type::Unknown => "Unknown".to_string(),
        Type::I64 => "i64".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Tuple(fields) => {
            let fields = fields
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({fields})")
        }
        Type::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_type(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        Type::Adt { name, .. } | Type::Nominal(name) => name.clone(),
        Type::Func(arg, ret) => format!("({}) -> {}", render_type(arg), render_type(ret)),
        Type::Continuation {
            multi,
            input,
            answer,
        } => {
            let kind = if *multi { "contN" } else { "cont1" };
            format!("{kind} ({}) -> {}", render_type(input), render_type(answer))
        }
    }
}

fn type_for_var(expr: &TypedExpr, name: &str) -> Option<Type> {
    match &expr.kind {
        crate::typed::TypedExprKind::Var(var) if var == name => Some(expr.ty.clone()),
        crate::typed::TypedExprKind::Var(_) | crate::typed::TypedExprKind::Lit(_) => None,
        crate::typed::TypedExprKind::Lambda { body, .. } => type_for_var(body, name),
        crate::typed::TypedExprKind::Call { callee, args } => type_for_var(callee, name)
            .or_else(|| args.iter().find_map(|arg| type_for_var(arg, name))),
        crate::typed::TypedExprKind::Tuple { fields, .. } => {
            fields.iter().find_map(|field| type_for_var(field, name))
        }
        crate::typed::TypedExprKind::SliceLiteral { items, .. } => {
            items.iter().find_map(|item| type_for_var(item, name))
        }
        crate::typed::TypedExprKind::Record { fields } => fields
            .iter()
            .find_map(|field| type_for_var(&field.value, name)),
        crate::typed::TypedExprKind::RecordUpdate { base, fields } => type_for_var(base, name)
            .or_else(|| {
                fields
                    .iter()
                    .find_map(|field| type_for_var(&field.value, name))
            }),
        crate::typed::TypedExprKind::AdtCtor { args, .. } => {
            args.iter().find_map(|arg| type_for_var(arg, name))
        }
        crate::typed::TypedExprKind::Field { receiver, .. } => type_for_var(receiver, name),
        crate::typed::TypedExprKind::MethodCall { receiver, args, .. } => {
            type_for_var(receiver, name)
                .or_else(|| args.iter().find_map(|arg| type_for_var(arg, name)))
        }
        crate::typed::TypedExprKind::Index { receiver, index } => {
            type_for_var(receiver, name).or_else(|| type_for_var(index, name))
        }
        crate::typed::TypedExprKind::Range { start, end } => {
            type_for_var(start, name).or_else(|| type_for_var(end, name))
        }
        crate::typed::TypedExprKind::Binary { lhs, rhs, .. } => {
            type_for_var(lhs, name).or_else(|| type_for_var(rhs, name))
        }
        crate::typed::TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => type_for_var(cond, name)
            .or_else(|| type_for_var(then_branch, name))
            .or_else(|| type_for_var(else_branch, name)),
        crate::typed::TypedExprKind::IfLet {
            scrutinee,
            then_branch,
            else_branch,
            ..
        } => type_for_var(scrutinee, name)
            .or_else(|| type_for_var(then_branch, name))
            .or_else(|| type_for_var(else_branch, name)),
        crate::typed::TypedExprKind::Match { scrutinee, arms } => type_for_var(scrutinee, name)
            .or_else(|| arms.iter().find_map(|arm| type_for_var(&arm.body, name))),
        crate::typed::TypedExprKind::Nominal { expr, .. } => type_for_var(expr, name),
        crate::typed::TypedExprKind::Reset { body, .. }
        | crate::typed::TypedExprKind::Shift { body, .. } => type_for_var(body, name),
    }
}

fn ownership_for(core: &CoreProgram, subject: &str) -> Option<OwnershipDecision> {
    core.ownership
        .iter()
        .find(|fact| fact.subject == subject)
        .map(|fact| fact.decision)
}
