use std::collections::BTreeMap;

use crate::control::ContinuationKind;
use crate::cps::{CpsAtom, CpsProgram, CpsTerm};
use crate::typed::UsageColor;
use crate::usage::UseCount;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CpsUsageFacts {
    pub continuations: BTreeMap<String, CpsContinuationUsage>,
    pub function_lambdas: BTreeMap<String, UseCount>,
    pub continuation_lambdas: BTreeMap<String, UseCount>,
    pub diagnostics: Vec<CpsUsageDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpsContinuationUsage {
    pub kind: ContinuationKind,
    pub count: UseCount,
    pub materialization: ContinuationMaterialization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContinuationMaterialization {
    Dead,
    InlineSingleUse,
    OneShotStateMachine,
    MultiResumePackage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpsUsageDiagnostic {
    Cont1ResumedMoreThanOnce { binder: String, count: UseCount },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContinuationSimplificationFacts {
    pub decisions: BTreeMap<String, ContinuationMaterialization>,
}

pub fn analyze_cps_usage(program: &CpsProgram) -> CpsUsageFacts {
    let mut facts = CpsUsageFacts::default();
    visit_term(&program.term, &mut facts);
    facts
}

pub fn simplify_continuations(usage: &CpsUsageFacts) -> ContinuationSimplificationFacts {
    let decisions = usage
        .continuations
        .iter()
        .map(|(name, usage)| (name.clone(), usage.materialization))
        .collect::<BTreeMap<_, _>>();
    ContinuationSimplificationFacts { decisions }
}

fn visit_term(term: &CpsTerm, facts: &mut CpsUsageFacts) {
    match term {
        CpsTerm::Halt(atom) => visit_atom(atom, facts),
        CpsTerm::AppFun { func, args, kont } => {
            visit_atom(func, facts);
            for arg in args {
                visit_atom(arg, facts);
            }
            visit_atom(kont, facts);
        }
        CpsTerm::AppCont { kont, value } => {
            visit_atom(kont, facts);
            visit_atom(value, facts);
        }
        CpsTerm::Prompt { body, .. } => visit_term(body, facts),
        CpsTerm::Capture {
            multi,
            binder,
            captured,
            body,
        } => {
            let kind = if *multi {
                ContinuationKind::ContN
            } else {
                ContinuationKind::Cont1
            };
            let count = count_binder_uses(body, binder);
            if kind == ContinuationKind::Cont1 && count == UseCount::Many {
                facts
                    .diagnostics
                    .push(CpsUsageDiagnostic::Cont1ResumedMoreThanOnce {
                        binder: binder.clone(),
                        count,
                    });
            }
            facts.continuations.insert(
                binder.clone(),
                CpsContinuationUsage {
                    kind,
                    count,
                    materialization: materialization(kind, count),
                },
            );
            visit_atom(captured, facts);
            visit_term(body, facts);
        }
        CpsTerm::Branch {
            cond,
            then_term,
            else_term,
            join,
        } => {
            visit_atom(cond, facts);
            visit_term(then_term, facts);
            visit_term(else_term, facts);
            visit_atom(join, facts);
        }
        CpsTerm::Match {
            scrutinee,
            arms,
            join,
        } => {
            visit_atom(scrutinee, facts);
            for arm in arms {
                visit_term(&arm.body, facts);
            }
            visit_atom(join, facts);
        }
    }
}

fn visit_atom(atom: &CpsAtom, facts: &mut CpsUsageFacts) {
    match atom {
        CpsAtom::Var(_) | CpsAtom::Lit(_) => {}
        CpsAtom::OperatorCallee { receiver, .. } => visit_atom(receiver, facts),
        CpsAtom::FunLambda { param, body, .. } => {
            bump(&mut facts.function_lambdas, param);
            visit_term(body, facts);
        }
        CpsAtom::ContLambda { param, body } => {
            bump(&mut facts.continuation_lambdas, param);
            visit_term(body, facts);
        }
        CpsAtom::Tuple { fields, .. } => {
            for field in fields {
                visit_atom(field, facts);
            }
        }
        CpsAtom::SliceLiteral { items } => {
            for item in items {
                visit_atom(item, facts);
            }
        }
        CpsAtom::TupleField { tuple, .. } => visit_atom(tuple, facts),
        CpsAtom::Range { start, end } => {
            visit_atom(start, facts);
            visit_atom(end, facts);
        }
        CpsAtom::RangeField { range, .. } => visit_atom(range, facts),
        CpsAtom::AggregateField { value, .. } => visit_atom(value, facts),
        CpsAtom::TextField { value, .. } => visit_atom(value, facts),
        CpsAtom::AggregateIndex { value, index, .. } => {
            visit_atom(value, facts);
            visit_atom(index, facts);
        }
        CpsAtom::AggregateSlice { value, range, .. } => {
            visit_atom(value, facts);
            visit_atom(range, facts);
        }
        CpsAtom::TextIndex { value, index, .. } => {
            visit_atom(value, facts);
            visit_atom(index, facts);
        }
        CpsAtom::TextSlice { value, range, .. } => {
            visit_atom(value, facts);
            visit_atom(range, facts);
        }
        CpsAtom::BuiltinRuntimeCall { args, .. } => {
            for arg in args {
                visit_atom(arg, facts);
            }
        }
        CpsAtom::Record { fields, .. } => {
            for field in fields {
                visit_atom(&field.value, facts);
            }
        }
        CpsAtom::RecordField { record, .. } => visit_atom(record, facts),
        CpsAtom::RecordUpdate { base, fields, .. } => {
            visit_atom(base, facts);
            for field in fields {
                visit_atom(&field.value, facts);
            }
        }
        CpsAtom::AdtCtor { args, .. } => {
            for arg in args {
                visit_atom(arg, facts);
            }
        }
    }
}

fn count_binder_uses(term: &CpsTerm, binder: &str) -> UseCount {
    let mut count = UseCount::Zero;
    count_term_refs(term, binder, &mut count);
    count
}

fn count_term_refs(term: &CpsTerm, binder: &str, count: &mut UseCount) {
    match term {
        CpsTerm::Halt(atom) => count_atom_refs(atom, binder, count),
        CpsTerm::AppFun { func, args, kont } => {
            count_atom_refs(func, binder, count);
            for arg in args {
                count_atom_refs(arg, binder, count);
            }
            count_atom_refs(kont, binder, count);
        }
        CpsTerm::AppCont { kont, value } => {
            count_atom_refs(kont, binder, count);
            count_atom_refs(value, binder, count);
        }
        CpsTerm::Prompt { body, .. } => count_term_refs(body, binder, count),
        CpsTerm::Capture { captured, body, .. } => {
            count_atom_refs(captured, binder, count);
            count_term_refs(body, binder, count);
        }
        CpsTerm::Branch {
            cond,
            then_term,
            else_term,
            join,
        } => {
            count_atom_refs(cond, binder, count);
            count_term_refs(then_term, binder, count);
            count_term_refs(else_term, binder, count);
            count_atom_refs(join, binder, count);
        }
        CpsTerm::Match {
            scrutinee,
            arms,
            join,
        } => {
            count_atom_refs(scrutinee, binder, count);
            for arm in arms {
                count_term_refs(&arm.body, binder, count);
            }
            count_atom_refs(join, binder, count);
        }
    }
}

fn count_atom_refs(atom: &CpsAtom, binder: &str, count: &mut UseCount) {
    match atom {
        CpsAtom::Var(name) if name == binder => *count = count.bump(),
        CpsAtom::Var(_) | CpsAtom::Lit(_) => {}
        CpsAtom::OperatorCallee { receiver, .. } => count_atom_refs(receiver, binder, count),
        CpsAtom::FunLambda { body, .. } | CpsAtom::ContLambda { body, .. } => {
            count_term_refs(body, binder, count);
        }
        CpsAtom::Tuple { fields, .. } => {
            for field in fields {
                count_atom_refs(field, binder, count);
            }
        }
        CpsAtom::SliceLiteral { items } => {
            for item in items {
                count_atom_refs(item, binder, count);
            }
        }
        CpsAtom::TupleField { tuple, .. } => count_atom_refs(tuple, binder, count),
        CpsAtom::Range { start, end } => {
            count_atom_refs(start, binder, count);
            count_atom_refs(end, binder, count);
        }
        CpsAtom::RangeField { range, .. } => count_atom_refs(range, binder, count),
        CpsAtom::AggregateField { value, .. } => count_atom_refs(value, binder, count),
        CpsAtom::TextField { value, .. } => count_atom_refs(value, binder, count),
        CpsAtom::AggregateIndex { value, index, .. } => {
            count_atom_refs(value, binder, count);
            count_atom_refs(index, binder, count);
        }
        CpsAtom::AggregateSlice { value, range, .. } => {
            count_atom_refs(value, binder, count);
            count_atom_refs(range, binder, count);
        }
        CpsAtom::TextIndex { value, index, .. } => {
            count_atom_refs(value, binder, count);
            count_atom_refs(index, binder, count);
        }
        CpsAtom::TextSlice { value, range, .. } => {
            count_atom_refs(value, binder, count);
            count_atom_refs(range, binder, count);
        }
        CpsAtom::BuiltinRuntimeCall { args, .. } => {
            for arg in args {
                count_atom_refs(arg, binder, count);
            }
        }
        CpsAtom::Record { fields, .. } => {
            for field in fields {
                count_atom_refs(&field.value, binder, count);
            }
        }
        CpsAtom::RecordField { record, .. } => count_atom_refs(record, binder, count),
        CpsAtom::RecordUpdate { base, fields, .. } => {
            count_atom_refs(base, binder, count);
            for field in fields {
                count_atom_refs(&field.value, binder, count);
            }
        }
        CpsAtom::AdtCtor { args, .. } => {
            for arg in args {
                count_atom_refs(arg, binder, count);
            }
        }
    }
}

fn materialization(kind: ContinuationKind, count: UseCount) -> ContinuationMaterialization {
    match (kind, count) {
        (_, UseCount::Zero) => ContinuationMaterialization::Dead,
        (ContinuationKind::Cont1, UseCount::One) => ContinuationMaterialization::InlineSingleUse,
        (ContinuationKind::Cont1, UseCount::Many) => {
            ContinuationMaterialization::OneShotStateMachine
        }
        (ContinuationKind::ContN, UseCount::One | UseCount::Many) => {
            ContinuationMaterialization::MultiResumePackage
        }
    }
}

fn bump(map: &mut BTreeMap<String, UseCount>, name: &str) {
    let current = map.get(name).copied().unwrap_or(UseCount::Zero);
    map.insert(name.to_string(), current.bump());
}

impl CpsContinuationUsage {
    pub fn color(&self) -> UsageColor {
        self.count.color()
    }
}
