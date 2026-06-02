use chiba_level1r::closure::ClosureStorageKind;
use chiba_level1r::control::ContinuationKind;
use chiba_level1r::core::{
    validate_core, CallableStorageFact, CallableStorageKind, CoreDiagnostic, CoreProgram,
};
use chiba_level1r::typed::{SendColor, UsageColor};
use chiba_level1r::{compile_expr, Expr};

#[test]
fn no_capture_closure_lowers_to_sendable_direct_callable_storage() {
    let output = compile_expr(&Expr::lambda("x", Expr::var("x")));

    assert_eq!(
        output.closure.closures[0].storage,
        ClosureStorageKind::DirectNoCapture
    );
    assert!(output.core.callable_storage.contains(&CallableStorageFact {
        subject: "closure::x".to_string(),
        kind: CallableStorageKind::NoCaptureClosure,
        usage: UsageColor::One,
        send: SendColor::Send,
    }));
}

#[test]
fn capturing_closure_lowers_to_env_callable_storage_with_send_obligation() {
    let output = compile_expr(&Expr::lambda(
        "x",
        Expr::lambda("y", Expr::call(Expr::var("x"), Expr::var("y"))),
    ));

    assert!(output.core.callable_storage.contains(&CallableStorageFact {
        subject: "closure::y".to_string(),
        kind: CallableStorageKind::EnvClosure,
        usage: UsageColor::Many,
        send: SendColor::Obligation,
    }));
}

#[test]
fn cont1_and_contn_lower_to_distinct_not_send_callable_storage() {
    let cont1 = compile_expr(&Expr::reset(Expr::shift("k", Expr::i64(0))));
    let contn = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert!(cont1.core.callable_storage.contains(&CallableStorageFact {
        subject: "continuation::k".to_string(),
        kind: CallableStorageKind::BoxedCont1,
        usage: UsageColor::One,
        send: SendColor::NotSend,
    }));
    assert!(contn.core.callable_storage.contains(&CallableStorageFact {
        subject: "continuation::retry".to_string(),
        kind: CallableStorageKind::ContNPackage,
        usage: UsageColor::Many,
        send: SendColor::NotSend,
    }));
}

#[test]
fn erased_callable_adt_fact_appears_when_callable_variants_exist() {
    let output = compile_expr(&Expr::resetn(Expr::shift("retry", Expr::i64(0))));

    assert!(output.core.callable_storage.contains(&CallableStorageFact {
        subject: "callable-storage::erased".to_string(),
        kind: CallableStorageKind::ErasedCallableAdt,
        usage: UsageColor::Many,
        send: SendColor::Obligation,
    }));
}

#[test]
fn core_validator_rejects_sendable_continuation_callable_storage() {
    let program = CoreProgram {
        ops: vec![],
        layouts: vec![],
        ownership: vec![],
        callable_storage: vec![CallableStorageFact {
            subject: "continuation::retry".to_string(),
            kind: CallableStorageKind::ContNPackage,
            usage: UsageColor::Many,
            send: SendColor::Send,
        }],
    };

    assert_eq!(
        validate_core(&program).diagnostics,
        vec![CoreDiagnostic::SendableCallableContainsContinuation {
            subject: "continuation::retry".to_string()
        }]
    );
}

#[test]
fn continuation_callable_kind_is_delimiter_driven_not_storage_guessed() {
    let cont1 = compile_expr(&Expr::reset(Expr::shift("k", Expr::i64(0))));
    let contn = compile_expr(&Expr::resetn(Expr::shift("k", Expr::i64(0))));

    assert_eq!(cont1.control.continuations[0].kind, ContinuationKind::Cont1);
    assert_eq!(contn.control.continuations[0].kind, ContinuationKind::ContN);
}
