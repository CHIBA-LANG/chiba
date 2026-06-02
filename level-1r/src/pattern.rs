use crate::ast::{Literal, Pattern};
use crate::typed::{TypedExpr, TypedExprKind, Type};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatternFacts {
    pub matches: Vec<MatchExhaustivenessFact>,
    pub envs: Vec<PatternEnvFact>,
    pub diagnostics: Vec<PatternDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchExhaustivenessFact {
    pub scrutinee_type: Type,
    pub covered_literals: Vec<Literal>,
    pub covered_constructors: Vec<String>,
    pub has_wildcard: bool,
    pub exhaustive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternEnvFact {
    pub bindings: Vec<String>,
    pub success_branch_binds: bool,
    pub failure_branch_binds: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternDiagnostic {
    NonExhaustiveMatch {
        scrutinee_type: Type,
        missing: Vec<Pattern>,
    },
    DuplicateBinding {
        name: String,
    },
    ChainedAtPattern {
        name: String,
    },
}

pub fn analyze_patterns(expr: &TypedExpr) -> PatternFacts {
    let mut facts = PatternFacts::default();
    visit(expr, &mut facts);
    facts
}

fn visit(expr: &TypedExpr, facts: &mut PatternFacts) {
    match &expr.kind {
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
        TypedExprKind::Lambda { body, .. } => visit(body, facts),
        TypedExprKind::Call { callee, args } => {
            visit(callee, facts);
            for arg in args {
                visit(arg, facts);
            }
        }
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                visit(field, facts);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                visit(&field.value, facts);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            visit(base, facts);
            for field in fields {
                visit(&field.value, facts);
            }
        }
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                visit(arg, facts);
            }
        }
        TypedExprKind::Field { receiver, .. } => visit(receiver, facts),
        TypedExprKind::MethodCall { receiver, arg, .. } => {
            visit(receiver, facts);
            visit(arg, facts);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            visit(lhs, facts);
            visit(rhs, facts);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit(cond, facts);
            visit(then_branch, facts);
            visit(else_branch, facts);
        }
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            visit(scrutinee, facts);
            visit(then_branch, facts);
            visit(else_branch, facts);
            diagnose_duplicate_bindings(pattern, facts);
            diagnose_chained_at_patterns(pattern, facts);
            facts.envs.push(PatternEnvFact {
                bindings: pattern_bindings(pattern),
                success_branch_binds: true,
                failure_branch_binds: false,
            });
        }
        TypedExprKind::Match { scrutinee, arms } => {
            visit(scrutinee, facts);
            for arm in arms {
                visit(&arm.body, facts);
                diagnose_duplicate_bindings(&arm.pattern, facts);
                diagnose_chained_at_patterns(&arm.pattern, facts);
            }
            let fact = exhaustiveness(scrutinee, arms);
            if !fact.exhaustive {
                facts.diagnostics.push(PatternDiagnostic::NonExhaustiveMatch {
                    scrutinee_type: fact.scrutinee_type.clone(),
                    missing: missing_patterns(&fact),
                });
            }
            facts.matches.push(fact);
        }
        TypedExprKind::Nominal { expr, .. } => visit(expr, facts),
        TypedExprKind::Reset { body, .. } => visit(body, facts),
        TypedExprKind::Shift { body, .. } => visit(body, facts),
    }
}

fn exhaustiveness(
    scrutinee: &TypedExpr,
    arms: &[crate::typed::TypedMatchArm],
) -> MatchExhaustivenessFact {
    let mut covered_literals = Vec::new();
    let mut covered_constructors = Vec::new();
    let mut has_wildcard = false;
    for arm in arms {
        match coverage_pattern(&arm.pattern) {
            Pattern::Wildcard => has_wildcard = true,
            Pattern::Bind(_) => has_wildcard = true,
            Pattern::Tuple(_) => {}
            Pattern::Record(_) => {}
            Pattern::Constructor { ctor, .. } if !covered_constructors.contains(ctor) => {
                covered_constructors.push(ctor.clone());
            }
            Pattern::Constructor { .. } => {}
            Pattern::At { pattern, .. } => match pattern.as_ref() {
                Pattern::Wildcard | Pattern::Bind(_) => has_wildcard = true,
                Pattern::Constructor { ctor, .. } if !covered_constructors.contains(ctor) => {
                    covered_constructors.push(ctor.clone());
                }
                Pattern::Lit(lit) if !covered_literals.contains(lit) => {
                    covered_literals.push(lit.clone());
                }
                Pattern::Lit(_)
                | Pattern::Tuple(_)
                | Pattern::Record(_)
                | Pattern::Constructor { .. }
                | Pattern::At { .. } => {}
            },
            Pattern::Lit(lit) if !covered_literals.contains(lit) => {
                covered_literals.push(lit.clone());
            }
            Pattern::Lit(_) => {}
        }
    }
    let exhaustive = has_wildcard
        || bool_is_exhaustive(&scrutinee.ty, &covered_literals)
        || adt_is_exhaustive(&scrutinee.ty, &covered_constructors);
    MatchExhaustivenessFact {
        scrutinee_type: scrutinee.ty.clone(),
        covered_literals,
        covered_constructors,
        has_wildcard,
        exhaustive,
    }
}

fn coverage_pattern(pattern: &Pattern) -> &Pattern {
    match pattern {
        Pattern::At { pattern, .. } => coverage_pattern(pattern),
        _ => pattern,
    }
}

fn bool_is_exhaustive(ty: &Type, covered: &[Literal]) -> bool {
    if *ty != Type::Bool {
        return false;
    }
    covered.contains(&Literal::Bool(true)) && covered.contains(&Literal::Bool(false))
}

fn adt_is_exhaustive(ty: &Type, covered: &[String]) -> bool {
    let Type::Adt { variants, .. } = ty else {
        return false;
    };
    variants.iter().all(|variant| covered.contains(variant))
}

fn missing_patterns(fact: &MatchExhaustivenessFact) -> Vec<Pattern> {
    if fact.has_wildcard {
        return vec![];
    }
    match fact.scrutinee_type {
        Type::Bool => {
            let mut missing = Vec::new();
            if !fact.covered_literals.contains(&Literal::Bool(true)) {
                missing.push(Pattern::Lit(Literal::Bool(true)));
            }
            if !fact.covered_literals.contains(&Literal::Bool(false)) {
                missing.push(Pattern::Lit(Literal::Bool(false)));
            }
            missing
        }
        Type::Adt { name, variants } => variants
            .iter()
            .filter(|variant| !fact.covered_constructors.contains(variant))
            .map(|variant| Pattern::qualified_ctor(name.clone(), variant.clone(), vec![]))
            .collect(),
        _ => vec![Pattern::Wildcard],
    }
}

fn pattern_bindings(pattern: &Pattern) -> Vec<String> {
    match pattern {
        Pattern::Bind(name) => vec![name.clone()],
        Pattern::Tuple(fields) => fields
            .iter()
            .flat_map(pattern_bindings)
            .collect::<Vec<_>>(),
        Pattern::Record(fields) => fields
            .iter()
            .flat_map(|field| pattern_bindings(&field.pattern))
            .collect::<Vec<_>>(),
        Pattern::Constructor { args, .. } => args
            .iter()
            .flat_map(pattern_bindings)
            .collect::<Vec<_>>(),
        Pattern::At { name, pattern } => {
            let mut bindings = pattern_bindings(pattern);
            bindings.push(name.clone());
            bindings
        }
        Pattern::Wildcard | Pattern::Lit(_) => vec![],
    }
}

fn diagnose_duplicate_bindings(pattern: &Pattern, facts: &mut PatternFacts) {
    let mut seen = Vec::new();
    let mut duplicates = Vec::new();
    collect_duplicate_bindings(pattern, &mut seen, &mut duplicates);
    for name in duplicates {
        facts
            .diagnostics
            .push(PatternDiagnostic::DuplicateBinding { name });
    }
}

fn collect_duplicate_bindings(
    pattern: &Pattern,
    seen: &mut Vec<String>,
    duplicates: &mut Vec<String>,
) {
    match pattern {
        Pattern::Bind(name) => {
            if seen.contains(name) && !duplicates.contains(name) {
                duplicates.push(name.clone());
            } else {
                seen.push(name.clone());
            }
        }
        Pattern::Tuple(fields) => {
            for field in fields {
                collect_duplicate_bindings(field, seen, duplicates);
            }
        }
        Pattern::Record(fields) => {
            for field in fields {
                collect_duplicate_bindings(&field.pattern, seen, duplicates);
            }
        }
        Pattern::Constructor { args, .. } => {
            for arg in args {
                collect_duplicate_bindings(arg, seen, duplicates);
            }
        }
        Pattern::At { name, pattern } => {
            collect_duplicate_bindings(pattern, seen, duplicates);
            if seen.contains(name) && !duplicates.contains(name) {
                duplicates.push(name.clone());
            } else {
                seen.push(name.clone());
            }
        }
        Pattern::Wildcard | Pattern::Lit(_) => {}
    }
}

fn diagnose_chained_at_patterns(pattern: &Pattern, facts: &mut PatternFacts) {
    match pattern {
        Pattern::At { pattern, .. } => {
            if let Pattern::At { name, .. } = pattern.as_ref() {
                facts.diagnostics.push(PatternDiagnostic::ChainedAtPattern {
                    name: name.clone(),
                });
            }
            diagnose_chained_at_patterns(pattern, facts);
        }
        Pattern::Tuple(fields) => {
            for field in fields {
                diagnose_chained_at_patterns(field, facts);
            }
        }
        Pattern::Constructor { args, .. } => {
            for arg in args {
                diagnose_chained_at_patterns(arg, facts);
            }
        }
        Pattern::Record(fields) => {
            for field in fields {
                diagnose_chained_at_patterns(&field.pattern, facts);
            }
        }
        Pattern::Wildcard | Pattern::Bind(_) | Pattern::Lit(_) => {}
    }
}
