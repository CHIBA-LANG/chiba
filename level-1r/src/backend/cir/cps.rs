use std::fmt;

use crate::ast::{BinaryOp, Literal, Pattern};
use crate::control::ContinuationKind;
use crate::typed::{FieldAccessKind, Type, TypedExpr, TypedExprKind, TypedRecordField};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpsProgram {
    pub term: CpsTerm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpsAtom {
    Var(String),
    Lit(Literal),
    OperatorCallee {
        kind: OperatorKind,
        protocol: String,
        receiver: Box<CpsAtom>,
    },
    FunLambda {
        param: String,
        k_param: String,
        body: Box<CpsTerm>,
    },
    ContLambda {
        param: String,
        body: Box<CpsTerm>,
    },
    Tuple {
        nominal: String,
        fields: Vec<CpsAtom>,
    },
    TupleField {
        tuple: Box<CpsAtom>,
        field: String,
        field_index: usize,
    },
    RecordField {
        record: Box<CpsAtom>,
        field: String,
    },
    Range {
        start: Box<CpsAtom>,
        end: Box<CpsAtom>,
    },
    RecordUpdate {
        base: Box<CpsAtom>,
        layout: String,
        fields: Vec<CpsRecordField>,
    },
    Record {
        layout: String,
        fields: Vec<CpsRecordField>,
    },
    AdtCtor {
        data: String,
        ctor: String,
        variants: Vec<String>,
        args: Vec<CpsAtom>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpsRecordField {
    pub name: String,
    pub value: CpsAtom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpsTerm {
    Halt(CpsAtom),
    AppFun {
        func: CpsAtom,
        args: Vec<CpsAtom>,
        kont: CpsAtom,
    },
    AppCont {
        kont: CpsAtom,
        value: CpsAtom,
    },
    Prompt {
        multi: bool,
        body: Box<CpsTerm>,
    },
    Capture {
        multi: bool,
        binder: String,
        captured: CpsAtom,
        body: Box<CpsTerm>,
    },
    Branch {
        cond: CpsAtom,
        then_term: Box<CpsTerm>,
        else_term: Box<CpsTerm>,
        join: CpsAtom,
    },
    Match {
        scrutinee: CpsAtom,
        arms: Vec<CpsMatchArm>,
        join: CpsAtom,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpsMatchArm {
    pub pattern: Pattern,
    pub body: CpsTerm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorKind {
    Binary(BinaryOp),
    Index,
    IndexSlice,
}

impl OperatorKind {
    pub fn protocol_name(self) -> &'static str {
        match self {
            OperatorKind::Binary(BinaryOp::Add) => "op_add",
            OperatorKind::Binary(BinaryOp::Sub) => "op_sub",
            OperatorKind::Binary(BinaryOp::Mul) => "op_mul",
            OperatorKind::Binary(BinaryOp::Div) => "op_div",
            OperatorKind::Index => "op_index",
            OperatorKind::IndexSlice => "op_index_slice",
        }
    }
}

#[derive(Default)]
struct CpsCtx {
    next_id: usize,
    next_resume_id: usize,
}

impl CpsCtx {
    fn fresh(&mut self, prefix: &str) -> String {
        let id = self.next_id;
        self.next_id += 1;
        format!("{prefix}{id}")
    }

    fn fresh_resume(&mut self) -> String {
        let id = self.next_resume_id;
        self.next_resume_id += 1;
        format!("resume{id}")
    }
}

// Compiler-level continuation. It is evaluated by the Rust transform itself,
// so administrative beta redexes never become object-level CPS nodes.
type MetaKont<'a> = Box<dyn FnOnce(CpsAtom, &mut CpsCtx) -> CpsTerm + 'a>;

pub fn cps_program(expr: &TypedExpr) -> CpsProgram {
    let mut ctx = CpsCtx::default();
    let term = transform(
        expr,
        Box::new(|value, _| CpsTerm::Halt(value)),
        Vec::new(),
        &mut ctx,
    );
    CpsProgram { term }
}

fn transform(
    expr: &TypedExpr,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    match &expr.kind {
        TypedExprKind::Var(name) => k(CpsAtom::Var(name.clone()), ctx),
        TypedExprKind::Lit(lit) => k(CpsAtom::Lit(lit.clone()), ctx),
        TypedExprKind::Lambda { param, body, .. } => {
            let k_param = ctx.fresh("k");
            let body_k = k_param.clone();
            let body = transform(
                body,
                Box::new(|value, _| CpsTerm::AppCont {
                    kont: CpsAtom::Var(body_k),
                    value,
                }),
                controls.clone(),
                ctx,
            );
            k(
                CpsAtom::FunLambda {
                    param: param.clone(),
                    k_param,
                    body: Box::new(body),
                },
                ctx,
            )
        }
        TypedExprKind::Call { callee, args } => transform_call(callee, args, k, controls, ctx),
        TypedExprKind::Tuple { fields, nominal } => {
            transform_tuple(fields, nominal, k, controls, ctx)
        }
        TypedExprKind::Record { fields } => transform_record(fields, k, controls, ctx),
        TypedExprKind::RecordUpdate { base, fields } => {
            transform_record_update(base, fields, k, controls, ctx)
        }
        TypedExprKind::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => transform_adt_ctor(data, ctor, variants, args, k, controls, ctx),
        TypedExprKind::Field {
            receiver,
            name,
            access,
        } => transform(
            receiver,
            Box::new(|value, ctx| {
                let atom = if let FieldAccessKind::TuplePositionalRow { index } = access {
                    CpsAtom::TupleField {
                        tuple: Box::new(value),
                        field: name.clone(),
                        field_index: *index,
                    }
                } else {
                    CpsAtom::RecordField {
                        record: Box::new(value),
                        field: name.clone(),
                    }
                };
                k(atom, ctx)
            }),
            controls,
            ctx,
        ),
        TypedExprKind::MethodCall {
            receiver,
            name,
            args,
        } => {
            let receiver_controls = controls.clone();
            transform(
                receiver,
                Box::new(|receiver, ctx| {
                    let func = CpsAtom::Var(format!("{receiver}.{name}"));
                    transform_call_args(args, 0, Vec::new(), func, k, controls, ctx)
                }),
                receiver_controls,
                ctx,
            )
        }
        TypedExprKind::Index { receiver, index } => {
            let receiver_controls = controls.clone();
            transform(
                receiver,
                Box::new(|receiver, ctx| {
                    let index_controls = controls.clone();
                    transform(
                        index,
                        Box::new(|index, ctx| {
                            let kind = if matches!(index, CpsAtom::Range { .. }) {
                                OperatorKind::IndexSlice
                            } else {
                                OperatorKind::Index
                            };
                            let w = ctx.fresh("w");
                            let kont_body = k(CpsAtom::Var(w.clone()), ctx);
                            CpsTerm::AppFun {
                                func: CpsAtom::OperatorCallee {
                                    kind,
                                    protocol: kind.protocol_name().to_string(),
                                    receiver: Box::new(receiver),
                                },
                                args: vec![index],
                                kont: CpsAtom::ContLambda {
                                    param: w,
                                    body: Box::new(kont_body),
                                },
                            }
                        }),
                        index_controls,
                        ctx,
                    )
                }),
                receiver_controls,
                ctx,
            )
        }
        TypedExprKind::Range { start, end } => {
            let start_controls = controls.clone();
            transform(
                start,
                Box::new(|start, ctx| {
                    let end_controls = controls.clone();
                    transform(
                        end,
                        Box::new(|end, ctx| {
                            k(
                                CpsAtom::Range {
                                    start: Box::new(start),
                                    end: Box::new(end),
                                },
                                ctx,
                            )
                        }),
                        end_controls,
                        ctx,
                    )
                }),
                start_controls,
                ctx,
            )
        }
        TypedExprKind::Binary { op, lhs, rhs } => {
            let kind = OperatorKind::Binary(*op);
            let lhs_controls = controls.clone();
            let rhs_controls = controls;
            transform(
                lhs,
                Box::new(|lhs, ctx| {
                    let rhs_controls = rhs_controls.clone();
                    transform(
                        rhs,
                        Box::new(|rhs, ctx| {
                            let w = ctx.fresh("w");
                            let kont_body = k(CpsAtom::Var(w.clone()), ctx);
                            CpsTerm::AppFun {
                                func: CpsAtom::OperatorCallee {
                                    kind,
                                    protocol: kind.protocol_name().to_string(),
                                    receiver: Box::new(lhs),
                                },
                                args: vec![rhs],
                                kont: CpsAtom::ContLambda {
                                    param: w,
                                    body: Box::new(kont_body),
                                },
                            }
                        }),
                        rhs_controls,
                        ctx,
                    )
                }),
                lhs_controls,
                ctx,
            )
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => transform_if(cond, then_branch, else_branch, k, controls, ctx),
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            let arms = vec![
                crate::typed::TypedMatchArm {
                    pattern: pattern.clone(),
                    body: (*then_branch.clone()),
                },
                crate::typed::TypedMatchArm {
                    pattern: Pattern::Wildcard,
                    body: (*else_branch.clone()),
                },
            ];
            transform_match(scrutinee, &arms, k, controls, ctx)
        }
        TypedExprKind::Match { scrutinee, arms } => {
            transform_match(scrutinee, arms, k, controls, ctx)
        }
        TypedExprKind::Nominal { expr, .. } => {
            let _ = nominal_atom_type(expr);
            transform(expr, k, controls, ctx)
        }
        TypedExprKind::Reset { multi, body } => {
            let mut nested_controls = controls;
            nested_controls.push(if *multi {
                ContinuationKind::ContN
            } else {
                ContinuationKind::Cont1
            });
            let body = transform(body, k, nested_controls, ctx);
            CpsTerm::Prompt {
                multi: *multi,
                body: Box::new(body),
            }
        }
        TypedExprKind::Shift { binder, body } => {
            let kind = controls.last().copied().unwrap_or(ContinuationKind::Cont1);
            let captured_param = ctx.fresh_resume();
            let captured_body = k(CpsAtom::Var(captured_param.clone()), ctx);
            let captured = CpsAtom::ContLambda {
                param: captured_param,
                body: Box::new(captured_body),
            };
            let body = transform(
                body,
                Box::new(|value, _| CpsTerm::Halt(value)),
                controls,
                ctx,
            );
            CpsTerm::Capture {
                multi: kind == ContinuationKind::ContN,
                binder: binder.clone(),
                captured,
                body: Box::new(body),
            }
        }
    }
}

fn transform_if(
    cond: &TypedExpr,
    then_branch: &TypedExpr,
    else_branch: &TypedExpr,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    let cond_controls = controls.clone();
    transform(
        cond,
        Box::new(|cond, ctx| {
            let join_param = ctx.fresh("join");
            let join_name = join_param.clone();
            let join = CpsAtom::ContLambda {
                param: join_param,
                body: Box::new(k(CpsAtom::Var(join_name.clone()), ctx)),
            };
            let then_join = join_name.clone();
            let else_join = join_name;
            let then_term = transform(
                then_branch,
                Box::new(move |value, _| CpsTerm::AppCont {
                    kont: CpsAtom::Var(then_join),
                    value,
                }),
                controls.clone(),
                ctx,
            );
            let else_term = transform(
                else_branch,
                Box::new(move |value, _| CpsTerm::AppCont {
                    kont: CpsAtom::Var(else_join),
                    value,
                }),
                controls,
                ctx,
            );
            CpsTerm::Branch {
                cond,
                then_term: Box::new(then_term),
                else_term: Box::new(else_term),
                join,
            }
        }),
        cond_controls,
        ctx,
    )
}

fn transform_match(
    scrutinee: &TypedExpr,
    arms: &[crate::typed::TypedMatchArm],
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    let scrutinee_controls = controls.clone();
    transform(
        scrutinee,
        Box::new(|scrutinee, ctx| {
            let join_param = ctx.fresh("join");
            let join_name = join_param.clone();
            let join = CpsAtom::ContLambda {
                param: join_param,
                body: Box::new(k(CpsAtom::Var(join_name.clone()), ctx)),
            };
            let mut cps_arms = Vec::new();
            for arm in arms {
                let arm_join = join_name.clone();
                let body = transform(
                    &arm.body,
                    Box::new(move |value, _| CpsTerm::AppCont {
                        kont: CpsAtom::Var(arm_join),
                        value,
                    }),
                    controls.clone(),
                    ctx,
                );
                cps_arms.push(CpsMatchArm {
                    pattern: arm.pattern.clone(),
                    body,
                });
            }
            CpsTerm::Match {
                scrutinee,
                arms: cps_arms,
                join,
            }
        }),
        scrutinee_controls,
        ctx,
    )
}

fn transform_tuple(
    fields: &[TypedExpr],
    nominal: &str,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    transform_tuple_fields(fields, nominal, 0, Vec::new(), k, controls, ctx)
}

fn transform_tuple_fields(
    fields: &[TypedExpr],
    nominal: &str,
    index: usize,
    values: Vec<CpsAtom>,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    if index == fields.len() {
        return k(
            CpsAtom::Tuple {
                nominal: nominal.to_string(),
                fields: values,
            },
            ctx,
        );
    }

    let field_controls = controls.clone();
    transform(
        &fields[index],
        Box::new(move |value, ctx| {
            let mut values = values;
            values.push(value);
            transform_tuple_fields(fields, nominal, index + 1, values, k, controls, ctx)
        }),
        field_controls,
        ctx,
    )
}

fn transform_record(
    fields: &[TypedRecordField],
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    transform_record_fields(fields, 0, Vec::new(), k, controls, ctx)
}

fn transform_record_update(
    base: &TypedExpr,
    fields: &[TypedRecordField],
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    let base_controls = controls.clone();
    transform(
        base,
        Box::new(|base, ctx| {
            transform_record_update_fields(base, fields, 0, Vec::new(), k, controls, ctx)
        }),
        base_controls,
        ctx,
    )
}

fn transform_record_update_fields(
    base: CpsAtom,
    fields: &[TypedRecordField],
    index: usize,
    values: Vec<CpsRecordField>,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    if index == fields.len() {
        let fields = merge_record_update_fields(&base, values);
        let layout = record_layout_name(&fields);
        let atom = match base {
            CpsAtom::Record { .. } => CpsAtom::Record { layout, fields },
            _ => CpsAtom::RecordUpdate {
                base: Box::new(base),
                layout,
                fields,
            },
        };
        return k(atom, ctx);
    }

    let field_controls = controls.clone();
    transform(
        &fields[index].value,
        Box::new(move |value, ctx| {
            let mut values = values;
            values.push(CpsRecordField {
                name: fields[index].name.clone(),
                value,
            });
            transform_record_update_fields(base, fields, index + 1, values, k, controls, ctx)
        }),
        field_controls,
        ctx,
    )
}

fn merge_record_update_fields(base: &CpsAtom, updates: Vec<CpsRecordField>) -> Vec<CpsRecordField> {
    let mut fields = match base {
        CpsAtom::Record { fields, .. } => fields.clone(),
        _ => Vec::new(),
    };
    for update in updates {
        if let Some(existing) = fields.iter_mut().find(|field| field.name == update.name) {
            *existing = update;
        } else {
            fields.push(update);
        }
    }
    fields.sort_by(|left, right| left.name.cmp(&right.name));
    fields
}

fn record_layout_name(fields: &[CpsRecordField]) -> String {
    format!(
        "record::{}",
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>()
            .join("+")
    )
}

fn transform_record_fields(
    fields: &[TypedRecordField],
    index: usize,
    values: Vec<CpsRecordField>,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    if index == fields.len() {
        let mut fields = values;
        fields.sort_by(|left, right| left.name.cmp(&right.name));
        let layout = record_layout_name(&fields);
        return k(CpsAtom::Record { layout, fields }, ctx);
    }

    let field_controls = controls.clone();
    transform(
        &fields[index].value,
        Box::new(move |value, ctx| {
            let mut values = values;
            values.push(CpsRecordField {
                name: fields[index].name.clone(),
                value,
            });
            transform_record_fields(fields, index + 1, values, k, controls, ctx)
        }),
        field_controls,
        ctx,
    )
}

fn transform_call(
    callee: &TypedExpr,
    args: &[TypedExpr],
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    let callee_controls = controls.clone();
    transform(
        callee,
        Box::new(|func, ctx| transform_call_args(args, 0, Vec::new(), func, k, controls, ctx)),
        callee_controls,
        ctx,
    )
}

fn transform_call_args(
    args: &[TypedExpr],
    index: usize,
    values: Vec<CpsAtom>,
    func: CpsAtom,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    if index == args.len() {
        let w = ctx.fresh("w");
        let kont_body = k(CpsAtom::Var(w.clone()), ctx);
        return CpsTerm::AppFun {
            func,
            args: values,
            kont: CpsAtom::ContLambda {
                param: w,
                body: Box::new(kont_body),
            },
        };
    }

    let arg_controls = controls.clone();
    transform(
        &args[index],
        Box::new(move |value, ctx| {
            let mut values = values;
            values.push(value);
            transform_call_args(args, index + 1, values, func, k, controls, ctx)
        }),
        arg_controls,
        ctx,
    )
}

fn transform_adt_ctor(
    data: &str,
    ctor: &str,
    variants: &[String],
    args: &[TypedExpr],
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    transform_adt_ctor_args(data, ctor, variants, args, 0, Vec::new(), k, controls, ctx)
}

fn transform_adt_ctor_args(
    data: &str,
    ctor: &str,
    variants: &[String],
    args: &[TypedExpr],
    index: usize,
    values: Vec<CpsAtom>,
    k: MetaKont<'_>,
    controls: Vec<ContinuationKind>,
    ctx: &mut CpsCtx,
) -> CpsTerm {
    if index == args.len() {
        return k(
            CpsAtom::AdtCtor {
                data: data.to_string(),
                ctor: ctor.to_string(),
                variants: variants.to_vec(),
                args: values,
            },
            ctx,
        );
    }

    let arg_controls = controls.clone();
    transform(
        &args[index],
        Box::new(move |value, ctx| {
            let mut values = values;
            values.push(value);
            transform_adt_ctor_args(
                data,
                ctor,
                variants,
                args,
                index + 1,
                values,
                k,
                controls,
                ctx,
            )
        }),
        arg_controls,
        ctx,
    )
}

fn nominal_atom_type(expr: &TypedExpr) -> Option<&String> {
    match &expr.ty {
        Type::Nominal(name) => Some(name),
        _ => None,
    }
}

impl CpsProgram {
    pub fn contains_administrative_let_cont(&self) -> bool {
        false
    }
}

impl fmt::Display for CpsAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpsAtom::Var(name) => write!(f, "{name}"),
            CpsAtom::Lit(Literal::I64(value)) => write!(f, "{value}"),
            CpsAtom::Lit(Literal::Bool(value)) => write!(f, "{value}"),
            CpsAtom::OperatorCallee {
                protocol, receiver, ..
            } => {
                write!(f, "{protocol}({receiver})")
            }
            CpsAtom::FunLambda {
                param,
                k_param,
                body,
            } => {
                write!(f, "(lambda {param} {k_param}. {body})")
            }
            CpsAtom::ContLambda { param, body } => {
                write!(f, "(cont {param}. {body})")
            }
            CpsAtom::Tuple { nominal, fields } => {
                write!(f, "{nominal}(")?;
                for (index, field) in fields.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "_{}={field}", index + 1)?;
                }
                write!(f, ")")
            }
            CpsAtom::TupleField { tuple, field, .. } => write!(f, "{tuple}.{field}"),
            CpsAtom::RecordField { record, field } => write!(f, "{record}.{field}"),
            CpsAtom::Range { start, end } => write!(f, "{start}..{end}"),
            CpsAtom::RecordUpdate {
                base,
                layout,
                fields,
            } => {
                write!(f, "{layout}{{base={base}")?;
                for field in fields {
                    write!(f, ", {}={}", field.name, field.value)?;
                }
                write!(f, "}}")
            }
            CpsAtom::Record { layout, fields } => {
                write!(f, "{layout}{{")?;
                for (index, field) in fields.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}={}", field.name, field.value)?;
                }
                write!(f, "}}")
            }
            CpsAtom::AdtCtor {
                data, ctor, args, ..
            } => {
                write!(f, "{data}.{ctor}")?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (index, arg) in args.iter().enumerate() {
                        if index > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{arg}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for CpsTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpsTerm::Halt(value) => write!(f, "halt {value}"),
            CpsTerm::AppCont { kont, value } => write!(f, "{kont}({value})"),
            CpsTerm::AppFun { func, args, kont } => {
                for (index, arg) in args.iter().enumerate() {
                    if index == 0 {
                        write!(f, "{func}({arg}")?;
                    } else {
                        write!(f, ", {arg}")?;
                    }
                }
                if args.is_empty() {
                    write!(f, "{func}(")?;
                }
                write!(f, ", {kont})")
            }
            CpsTerm::Prompt { multi, body } => {
                if *multi {
                    write!(f, "resetn {{ {body} }}")
                } else {
                    write!(f, "reset {{ {body} }}")
                }
            }
            CpsTerm::Capture {
                multi,
                binder,
                captured,
                body,
            } => {
                if *multi {
                    write!(f, "shift@contN {binder} captured={captured} {{ {body} }}")
                } else {
                    write!(f, "shift@cont1 {binder} captured={captured} {{ {body} }}")
                }
            }
            CpsTerm::Branch {
                cond,
                then_term,
                else_term,
                join,
            } => {
                write!(
                    f,
                    "if {cond} {{ {then_term} }} else {{ {else_term} }} join {join}"
                )
            }
            CpsTerm::Match {
                scrutinee,
                arms,
                join,
            } => {
                write!(f, "match {scrutinee} {{")?;
                for arm in arms {
                    write!(f, " {} => {}", display_pattern(&arm.pattern), arm.body)?;
                }
                write!(f, " }} join {join}")
            }
        }
    }
}

fn display_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Bind(name) => name.clone(),
        Pattern::Tuple(fields) => {
            let fields = fields
                .iter()
                .map(display_pattern)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({fields})")
        }
        Pattern::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, display_pattern(&field.pattern)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        Pattern::Constructor { data, ctor, args } => {
            let head = match data {
                Some(data) => format!("{data}.{ctor}"),
                None => ctor.clone(),
            };
            if args.is_empty() {
                head
            } else {
                let args = args
                    .iter()
                    .map(display_pattern)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}({args})")
            }
        }
        Pattern::At { name, pattern } => {
            format!("{name} @ {}", display_pattern(pattern))
        }
        Pattern::Lit(Literal::I64(value)) => value.to_string(),
        Pattern::Lit(Literal::Bool(value)) => value.to_string(),
    }
}

impl fmt::Display for CpsProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.term)
    }
}
