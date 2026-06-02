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
    BoxedOneShot,
    MultiResumePackage,
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
        CpsTerm::AppFun { func, arg, kont } => {
            visit_atom(func, facts);
            visit_atom(arg, facts);
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
            body,
        } => {
            let kind = if *multi {
                ContinuationKind::ContN
            } else {
                ContinuationKind::Cont1
            };
            let count = count_binder_uses(body, binder);
            facts.continuations.insert(
                binder.clone(),
                CpsContinuationUsage {
                    kind,
                    count,
                    materialization: materialization(kind, count),
                },
            );
            visit_term(body, facts);
        }
    }
}

fn visit_atom(atom: &CpsAtom, facts: &mut CpsUsageFacts) {
    match atom {
        CpsAtom::Var(_) | CpsAtom::Lit(_) => {}
        CpsAtom::FunLambda { param, body, .. } => {
            bump(&mut facts.function_lambdas, param);
            visit_term(body, facts);
        }
        CpsAtom::ContLambda { param, body } => {
            bump(&mut facts.continuation_lambdas, param);
            visit_term(body, facts);
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
        CpsTerm::AppFun { func, arg, kont } => {
            count_atom_refs(func, binder, count);
            count_atom_refs(arg, binder, count);
            count_atom_refs(kont, binder, count);
        }
        CpsTerm::AppCont { kont, value } => {
            count_atom_refs(kont, binder, count);
            count_atom_refs(value, binder, count);
        }
        CpsTerm::Prompt { body, .. } => count_term_refs(body, binder, count),
        CpsTerm::Capture { body, .. } => count_term_refs(body, binder, count),
    }
}

fn count_atom_refs(atom: &CpsAtom, binder: &str, count: &mut UseCount) {
    match atom {
        CpsAtom::Var(name) if name == binder => *count = count.bump(),
        CpsAtom::Var(_) | CpsAtom::Lit(_) => {}
        CpsAtom::FunLambda { body, .. } | CpsAtom::ContLambda { body, .. } => {
            count_term_refs(body, binder, count);
        }
    }
}

fn materialization(kind: ContinuationKind, count: UseCount) -> ContinuationMaterialization {
    match (kind, count) {
        (_, UseCount::Zero) => ContinuationMaterialization::Dead,
        (ContinuationKind::Cont1, UseCount::One) => ContinuationMaterialization::InlineSingleUse,
        (ContinuationKind::Cont1, UseCount::Many) => ContinuationMaterialization::BoxedOneShot,
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
