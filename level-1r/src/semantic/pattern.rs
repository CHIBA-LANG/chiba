use crate::ast::{Literal, ParamDecl, Pattern};
use crate::typed::{Type, TypeContext, TypedExpr, TypedExprKind};

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
    analyze_patterns_with_context(expr, &TypeContext::new())
}

pub fn analyze_patterns_with_context(expr: &TypedExpr, context: &TypeContext) -> PatternFacts {
    let mut facts = PatternFacts::default();
    visit(expr, &mut facts, context);
    facts
}

pub fn analyze_patterns_with_params(expr: &TypedExpr, params: &[ParamDecl]) -> PatternFacts {
    analyze_patterns_with_params_and_context(expr, params, &TypeContext::new())
}

pub fn analyze_patterns_with_params_and_context(
    expr: &TypedExpr,
    params: &[ParamDecl],
    context: &TypeContext,
) -> PatternFacts {
    let mut facts = PatternFacts::default();
    for param in params {
        analyze_param_pattern(param, &mut facts);
    }
    visit(expr, &mut facts, context);
    facts
}

pub fn analyze_param_patterns(params: &[ParamDecl]) -> PatternFacts {
    let mut facts = PatternFacts::default();
    for param in params {
        analyze_param_pattern(param, &mut facts);
    }
    facts
}

fn analyze_param_pattern(param: &ParamDecl, facts: &mut PatternFacts) {
    diagnose_duplicate_bindings(&param.pattern, facts);
    diagnose_chained_at_patterns(&param.pattern, facts);
    let bindings = pattern_bindings(&param.pattern);
    if !bindings.is_empty() {
        facts.envs.push(PatternEnvFact {
            bindings,
            success_branch_binds: true,
            failure_branch_binds: false,
        });
    }
}

fn visit(expr: &TypedExpr, facts: &mut PatternFacts, context: &TypeContext) {
    match &expr.kind {
        TypedExprKind::Var(_) | TypedExprKind::Lit(_) => {}
        TypedExprKind::Lambda { body, .. } => visit(body, facts, context),
        TypedExprKind::Call { callee, args } => {
            visit(callee, facts, context);
            for arg in args {
                visit(arg, facts, context);
            }
        }
        TypedExprKind::Tuple { fields, .. } => {
            for field in fields {
                visit(field, facts, context);
            }
        }
        TypedExprKind::SliceLiteral { items, .. } => {
            for item in items {
                visit(item, facts, context);
            }
        }
        TypedExprKind::Record { fields } => {
            for field in fields {
                visit(&field.value, facts, context);
            }
        }
        TypedExprKind::RecordUpdate { base, fields } => {
            visit(base, facts, context);
            for field in fields {
                visit(&field.value, facts, context);
            }
        }
        TypedExprKind::DynRowPackage { payload, .. } => visit(payload, facts, context),
        TypedExprKind::DynRowField { package, .. } => visit(package, facts, context),
        TypedExprKind::AdtCtor { args, .. } => {
            for arg in args {
                visit(arg, facts, context);
            }
        }
        TypedExprKind::AdtToTuple { value, .. } => visit(value, facts, context),
        TypedExprKind::TupleToAdt { value, .. } => visit(value, facts, context),
        TypedExprKind::Field { receiver, .. } => visit(receiver, facts, context),
        TypedExprKind::MethodCall { receiver, args, .. } => {
            visit(receiver, facts, context);
            for arg in args {
                visit(arg, facts, context);
            }
        }
        TypedExprKind::Assign { target, value, .. } => {
            visit(target, facts, context);
            visit(value, facts, context);
        }
        TypedExprKind::Index {
            receiver, index, ..
        } => {
            visit(receiver, facts, context);
            visit(index, facts, context);
        }
        TypedExprKind::Range { start, end } => {
            visit(start, facts, context);
            visit(end, facts, context);
        }
        TypedExprKind::Binary { lhs, rhs, .. } => {
            visit(lhs, facts, context);
            visit(rhs, facts, context);
        }
        TypedExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            visit(cond, facts, context);
            visit(then_branch, facts, context);
            visit(else_branch, facts, context);
        }
        TypedExprKind::IfLet {
            pattern,
            scrutinee,
            then_branch,
            else_branch,
        } => {
            visit(scrutinee, facts, context);
            visit(then_branch, facts, context);
            visit(else_branch, facts, context);
            diagnose_duplicate_bindings(pattern, facts);
            diagnose_chained_at_patterns(pattern, facts);
            facts.envs.push(PatternEnvFact {
                bindings: pattern_bindings(pattern),
                success_branch_binds: true,
                failure_branch_binds: false,
            });
        }
        TypedExprKind::Match { scrutinee, arms } => {
            visit(scrutinee, facts, context);
            for arm in arms {
                visit(&arm.body, facts, context);
                diagnose_duplicate_bindings(&arm.pattern, facts);
                diagnose_chained_at_patterns(&arm.pattern, facts);
                let bindings = pattern_bindings(&arm.pattern);
                if !bindings.is_empty() {
                    facts.envs.push(PatternEnvFact {
                        bindings,
                        success_branch_binds: true,
                        failure_branch_binds: false,
                    });
                }
            }
            let fact = exhaustiveness(scrutinee, arms, context);
            if !fact.exhaustive {
                facts
                    .diagnostics
                    .push(PatternDiagnostic::NonExhaustiveMatch {
                        scrutinee_type: fact.scrutinee_type.clone(),
                        missing: missing_patterns(&fact),
                    });
            }
            facts.matches.push(fact);
        }
        TypedExprKind::Nominal { expr, .. } => visit(expr, facts, context),
        TypedExprKind::Reset { body, .. } => visit(body, facts, context),
        TypedExprKind::Shift { body, .. } => visit(body, facts, context),
    }
}

fn exhaustiveness(
    scrutinee: &TypedExpr,
    arms: &[crate::typed::TypedMatchArm],
    context: &TypeContext,
) -> MatchExhaustivenessFact {
    let mut covered_literals = Vec::new();
    let mut covered_constructors = Vec::new();
    let mut has_wildcard = false;
    for arm in arms {
        match coverage_pattern(&arm.pattern) {
            pattern if pattern_covers_all(pattern) => has_wildcard = true,
            Pattern::Wildcard | Pattern::Bind(_) => unreachable!("covered by pattern_covers_all"),
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
    let structurally_exhaustive = patterns_cover_type(
        &arms
            .iter()
            .map(|arm| arm.pattern.clone())
            .collect::<Vec<_>>(),
        &scrutinee.ty,
        context,
    );
    let exhaustive = has_wildcard
        || bool_is_exhaustive(&scrutinee.ty, &covered_literals)
        || adt_is_exhaustive(&scrutinee.ty, &covered_constructors, context)
        || structurally_exhaustive;
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

fn pattern_covers_all(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Wildcard | Pattern::Bind(_) => true,
        Pattern::At { pattern, .. } => pattern_covers_all(pattern),
        Pattern::Tuple(fields) => fields.iter().all(pattern_covers_all),
        Pattern::Record(fields) => fields
            .iter()
            .all(|field| pattern_covers_all(&field.pattern)),
        Pattern::Constructor { .. } | Pattern::Lit(_) => false,
    }
}

fn bool_is_exhaustive(ty: &Type, covered: &[Literal]) -> bool {
    if *ty != Type::Bool {
        return false;
    }
    covered.contains(&Literal::Bool(true)) && covered.contains(&Literal::Bool(false))
}

fn adt_is_exhaustive(ty: &Type, covered: &[String], context: &TypeContext) -> bool {
    context
        .adt_variants_for_type(ty)
        .is_some_and(|(_, variants)| variants.iter().all(|variant| covered.contains(variant)))
}

fn missing_patterns(fact: &MatchExhaustivenessFact) -> Vec<Pattern> {
    if fact.has_wildcard {
        return vec![];
    }
    match &fact.scrutinee_type {
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

fn patterns_cover_type(patterns: &[Pattern], ty: &Type, context: &TypeContext) -> bool {
    if patterns.iter().any(pattern_covers_all) {
        return true;
    }
    match ty {
        Type::Bool => {
            pattern_list_covers_literal(patterns, &Literal::Bool(true), ty, context)
                && pattern_list_covers_literal(patterns, &Literal::Bool(false), ty, context)
        }
        Type::Tuple(fields) => tuple_patterns_cover_fields(patterns, fields, context),
        _ => {
            let Some((_, variants)) = context.adt_variants_for_type(ty) else {
                return false;
            };
            variants.iter().all(|variant| {
                let variant_patterns = patterns
                    .iter()
                    .filter_map(|pattern| constructor_payload_patterns(pattern, variant))
                    .collect::<Vec<_>>();
                if variant_patterns.is_empty() {
                    return false;
                }
                let payload_tys = context
                    .constructor_payload_types_for_pattern(ty, None, variant)
                    .unwrap_or_default();
                if payload_tys.is_empty() {
                    return true;
                }
                payload_tys.iter().enumerate().all(|(index, payload_ty)| {
                    let nested = variant_patterns
                        .iter()
                        .filter_map(|payloads| payloads.get(index).cloned())
                        .collect::<Vec<_>>();
                    patterns_cover_type(&nested, payload_ty, context)
                })
            })
        }
    }
}

fn tuple_patterns_cover_fields(
    patterns: &[Pattern],
    fields: &[Type],
    context: &TypeContext,
) -> bool {
    let tuples = patterns
        .iter()
        .map(coverage_pattern)
        .filter_map(|pattern| match pattern {
            Pattern::Tuple(items) if items.len() == fields.len() => Some(items.as_slice()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if tuples.is_empty() {
        return false;
    }
    tuple_patterns_cover_prefix(&tuples, fields, context)
}

fn tuple_patterns_cover_prefix(
    tuples: &[&[Pattern]],
    fields: &[Type],
    context: &TypeContext,
) -> bool {
    let Some((field_ty, rest_tys)) = fields.split_first() else {
        return true;
    };
    let Some(cases) = pattern_domain_cases(field_ty, context) else {
        return tuples.iter().any(|tuple| {
            let Some((head, tail)) = tuple.split_first() else {
                return false;
            };
            pattern_covers_all(head) && tuple_tail_covers_unbounded(tail, rest_tys, context)
        });
    };
    cases.iter().all(|case| {
        let narrowed = tuples
            .iter()
            .filter_map(|tuple| {
                let (head, tail) = tuple.split_first()?;
                pattern_matches_case(head, case, field_ty, context).then_some(tail)
            })
            .collect::<Vec<_>>();
        !narrowed.is_empty() && tuple_patterns_cover_prefix(&narrowed, rest_tys, context)
    })
}

fn tuple_tail_covers_unbounded(tail: &[Pattern], fields: &[Type], context: &TypeContext) -> bool {
    if tail.len() != fields.len() {
        return false;
    }
    tail.iter().zip(fields).all(|(pattern, ty)| {
        pattern_covers_all(pattern) || patterns_cover_type(&[pattern.clone()], ty, context)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PatternDomainCase {
    Literal(Literal),
    Constructor(String),
}

fn pattern_domain_cases(ty: &Type, context: &TypeContext) -> Option<Vec<PatternDomainCase>> {
    match ty {
        Type::Bool => Some(vec![
            PatternDomainCase::Literal(Literal::Bool(true)),
            PatternDomainCase::Literal(Literal::Bool(false)),
        ]),
        _ => context.adt_variants_for_type(ty).map(|(_, variants)| {
            variants
                .into_iter()
                .map(PatternDomainCase::Constructor)
                .collect()
        }),
    }
}

fn pattern_matches_case(
    pattern: &Pattern,
    case: &PatternDomainCase,
    ty: &Type,
    context: &TypeContext,
) -> bool {
    if pattern_covers_all(pattern) {
        return true;
    }
    match case {
        PatternDomainCase::Literal(lit) => match coverage_pattern(pattern) {
            Pattern::Lit(candidate) => candidate == lit,
            _ => false,
        },
        PatternDomainCase::Constructor(ctor) => {
            let Some(payloads) = constructor_payload_patterns(pattern, ctor) else {
                return false;
            };
            let payload_tys = context
                .constructor_payload_types_for_pattern(ty, None, ctor)
                .unwrap_or_default();
            payload_tys.iter().enumerate().all(|(index, payload_ty)| {
                payloads.get(index).is_some_and(|nested| {
                    patterns_cover_type(&[nested.clone()], payload_ty, context)
                })
            })
        }
    }
}

fn pattern_list_covers_literal(
    patterns: &[Pattern],
    lit: &Literal,
    ty: &Type,
    context: &TypeContext,
) -> bool {
    patterns.iter().any(|pattern| {
        pattern_matches_case(
            pattern,
            &PatternDomainCase::Literal(lit.clone()),
            ty,
            context,
        )
    })
}

fn constructor_payload_patterns<'a>(pattern: &'a Pattern, ctor: &str) -> Option<&'a [Pattern]> {
    match coverage_pattern(pattern) {
        Pattern::Constructor {
            ctor: candidate,
            args,
            ..
        } if candidate == ctor => Some(args.as_slice()),
        _ => None,
    }
}

fn pattern_bindings(pattern: &Pattern) -> Vec<String> {
    match pattern {
        Pattern::Bind(name) => vec![name.clone()],
        Pattern::Tuple(fields) => fields.iter().flat_map(pattern_bindings).collect::<Vec<_>>(),
        Pattern::Record(fields) => fields
            .iter()
            .flat_map(|field| pattern_bindings(&field.pattern))
            .collect::<Vec<_>>(),
        Pattern::Constructor { args, .. } => {
            args.iter().flat_map(pattern_bindings).collect::<Vec<_>>()
        }
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
                facts
                    .diagnostics
                    .push(PatternDiagnostic::ChainedAtPattern { name: name.clone() });
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
