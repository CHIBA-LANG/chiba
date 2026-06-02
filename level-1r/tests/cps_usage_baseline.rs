use chiba_level1r::control::ContinuationKind;
use chiba_level1r::cps::{CpsAtom, CpsProgram, CpsTerm};
use chiba_level1r::cps_usage::{
    analyze_cps_usage, simplify_continuations, ContinuationMaterialization,
};
use chiba_level1r::usage::UseCount;
use chiba_level1r::{compile_expr, Expr};

#[test]
fn captured_continuation_unused_is_dead_after_cps_usage() {
    let output = compile_expr(&Expr::reset(Expr::shift("k", Expr::i64(0))));
    let usage = output.cps_usage.continuations.get("k").unwrap();

    assert_eq!(usage.kind, ContinuationKind::Cont1);
    assert_eq!(usage.count, UseCount::Zero);
    assert_eq!(usage.materialization, ContinuationMaterialization::Dead);
    assert_eq!(
        output.continuation_simplification.decisions.get("k"),
        Some(&ContinuationMaterialization::Dead)
    );
}

#[test]
fn cont1_single_resume_is_inline_single_use() {
    let cps = CpsProgram {
        term: CpsTerm::Capture {
            multi: false,
            binder: "k".to_string(),
            body: Box::new(CpsTerm::AppCont {
                kont: CpsAtom::Var("k".to_string()),
                value: CpsAtom::Lit(chiba_level1r::Literal::I64(1)),
            }),
        },
    };

    let usage = analyze_cps_usage(&cps);

    assert_eq!(usage.continuations["k"].count, UseCount::One);
    assert_eq!(
        usage.continuations["k"].materialization,
        ContinuationMaterialization::InlineSingleUse
    );
    assert_eq!(
        simplify_continuations(&usage).decisions["k"],
        ContinuationMaterialization::InlineSingleUse
    );
}

#[test]
fn cont1_many_resume_is_boxed_one_shot_state_machine() {
    let cps = CpsProgram {
        term: CpsTerm::Capture {
            multi: false,
            binder: "k".to_string(),
            body: Box::new(CpsTerm::AppFun {
                func: CpsAtom::Var("f".to_string()),
                args: vec![CpsAtom::Var("k".to_string())],
                kont: CpsAtom::ContLambda {
                    param: "w".to_string(),
                    body: Box::new(CpsTerm::AppCont {
                        kont: CpsAtom::Var("k".to_string()),
                        value: CpsAtom::Var("w".to_string()),
                    }),
                },
            }),
        },
    };

    let usage = analyze_cps_usage(&cps);

    assert_eq!(usage.continuations["k"].count, UseCount::Many);
    assert_eq!(
        usage.continuations["k"].materialization,
        ContinuationMaterialization::BoxedOneShot
    );
}

#[test]
fn contn_single_or_many_resume_uses_multi_resume_package() {
    let single = CpsProgram {
        term: CpsTerm::Capture {
            multi: true,
            binder: "retry".to_string(),
            body: Box::new(CpsTerm::Halt(CpsAtom::Var("retry".to_string()))),
        },
    };

    let usage = analyze_cps_usage(&single);

    assert_eq!(usage.continuations["retry"].count, UseCount::One);
    assert_eq!(
        usage.continuations["retry"].materialization,
        ContinuationMaterialization::MultiResumePackage
    );
}

#[test]
fn cps_usage_counts_object_level_lambdas() {
    let output = compile_expr(&Expr::call(Expr::var("f"), Expr::var("x")));

    assert_eq!(output.cps_usage.continuation_lambdas.len(), 1);
    assert!(output.render_visual().contains("cps-usage:"));
    assert!(output
        .render_visual()
        .contains("continuation-simplification:"));
}
