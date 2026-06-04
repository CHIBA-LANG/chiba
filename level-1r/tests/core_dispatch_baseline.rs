use chiba_level1r::closure::ClosureFacts;
use chiba_level1r::core::{
    lower_core_with_facts, CoreOp, CoreValue, LayoutKind, OperatorIntrinsic,
};
use chiba_level1r::cps::{CpsAtom, CpsProgram, CpsTerm, OperatorKind};
use chiba_level1r::lambda_lift::LambdaLiftFacts;
use chiba_level1r::specialize::plan_specialization;
use chiba_level1r::template::{
    canonical_open_row, dyn_row_contract, row_shape_key, ShapeType, TemplateFacts,
    TemplateObligation,
};
use chiba_level1r::usage::UsageFacts;

#[test]
fn discharged_method_target_enters_core_without_wasm_terms() {
    let mut template = TemplateFacts::default();
    template.obligations.push(TemplateObligation::Method {
        receiver: Some("Vec2".to_string()),
        name: "norm".to_string(),
        resolved: Some("math.Vec2.norm".to_string()),
    });
    let specialize = plan_specialization("call_norm", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );

    assert!(core.ops.contains(&CoreOp::DirectMethodTarget {
        name: "norm".to_string(),
        target: "math.Vec2.norm".to_string(),
    }));
    assert!(!format!("{core:#?}").contains("funcref"));
}

#[test]
fn operator_obligation_enters_core_as_method_protocol_target() {
    let mut template = TemplateFacts::default();
    template.obligations.push(TemplateObligation::Operator {
        op: chiba_level1r::resolve::OperatorSurface::Binary(chiba_level1r::ast::BinaryOp::Add),
        protocol: "renamed-template-add".to_string(),
        receiver: Some("Vec2".to_string()),
    });
    let specialize = plan_specialization("add_vec2", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );

    assert!(core.ops.contains(&CoreOp::OperatorTarget {
        protocol: "renamed-template-add".to_string(),
        target: "Vec2.renamed-template-add".to_string(),
        intrinsic: Some(OperatorIntrinsic::I64Add),
    }));
}

#[test]
fn operator_intrinsic_uses_structured_kind_not_protocol_debug_string() {
    let cps = CpsProgram {
        term: CpsTerm::AppFun {
            func: CpsAtom::OperatorCallee {
                kind: OperatorKind::Binary(chiba_level1r::ast::BinaryOp::Add),
                protocol: "renamed-debug-add".to_string(),
                receiver: Box::new(CpsAtom::Var("lhs".to_string())),
            },
            args: vec![CpsAtom::Var("rhs".to_string())],
            kont: CpsAtom::ContLambda {
                param: "sum".to_string(),
                body: Box::new(CpsTerm::Halt(CpsAtom::Var("sum".to_string()))),
            },
        },
    };

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &plan_specialization("add", &TemplateFacts::default()),
        &UsageFacts::default(),
    );

    assert!(core.ops.contains(&CoreOp::OperatorTarget {
        protocol: "renamed-debug-add".to_string(),
        target: "renamed-debug-add(lhs)".to_string(),
        intrinsic: Some(OperatorIntrinsic::I64Add),
    }));
    assert!(core.ops.contains(&CoreOp::TailCall {
        func: "renamed-debug-add(lhs)".to_string(),
        args: vec![
            CoreValue::Var("lhs".to_string()),
            CoreValue::Var("rhs".to_string())
        ],
    }));
}

#[test]
fn dyn_row_adapter_access_refs_dyn_package_layout() {
    let mut template = TemplateFacts::default();
    let contract = dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]);
    template.dyn_contracts.push(contract.clone());
    template.obligations.push(TemplateObligation::DynAdapter {
        contract: contract.clone(),
    });
    let specialize = plan_specialization("render_dyn", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );
    let expected_shape = canonical_open_row(vec![("name", ShapeType::Named("String".to_string()))]);
    let expected_subject = row_shape_key(&expected_shape);
    let expected_layout = core
        .layouts
        .iter()
        .find_map(|layout| {
            matches!(&layout.kind, LayoutKind::DynRowPackage(candidate) if candidate == &contract)
                .then(|| layout.key.clone())
        })
        .expect("dyn row package layout fact");

    assert!(core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::DynRowAdapterAccess { subject, layout }
                if subject == &expected_subject && layout == &expected_layout
        )
    }));
}

#[test]
fn field_obligation_enters_core_as_static_row_access() {
    let mut template = TemplateFacts::default();
    let shape = canonical_open_row(vec![("name", ShapeType::Unknown)]);
    template.row_shapes.push(shape.clone());
    template.obligations.push(TemplateObligation::Field {
        shape: canonical_open_row(vec![("name", ShapeType::Unknown)]),
        field: "name".to_string(),
    });
    let specialize = plan_specialization("get_name", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );
    let expected_layout = core
        .layouts
        .iter()
        .find_map(|layout| {
            matches!(&layout.kind, LayoutKind::RowShape(candidate) if candidate == &shape)
                .then(|| layout.key.clone())
        })
        .expect("static row layout fact");

    assert!(core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::StaticRowAccess { field, layout }
                if field == "name" && layout == &expected_layout
        )
    }));
}

#[test]
fn row_and_dyn_dispatch_do_not_invent_layout_keys_from_debug_strings() {
    let mut template = TemplateFacts::default();
    let shape = canonical_open_row(vec![("name", ShapeType::Unknown)]);
    let contract = dyn_row_contract(vec![("name", ShapeType::Unknown)]);
    template.obligations.push(TemplateObligation::Field {
        shape: shape.clone(),
        field: "name".to_string(),
    });
    template
        .obligations
        .push(TemplateObligation::DynAdapter { contract });
    let specialize = plan_specialization("missing_layout_facts", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );

    assert!(core
        .ops
        .iter()
        .all(|op| !matches!(op, CoreOp::StaticRowAccess { .. })));
    assert!(core
        .ops
        .iter()
        .all(|op| !matches!(op, CoreOp::DynRowAdapterAccess { .. })));
}

#[test]
fn row_and_dyn_layout_keys_are_structured_not_rust_debug_shapes() {
    let mut template = TemplateFacts::default();
    let shape = canonical_open_row(vec![("name", ShapeType::Named("String".to_string()))]);
    let contract = dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]);
    let expected_contract = contract.clone();
    template.row_shapes.push(shape.clone());
    template.dyn_contracts.push(contract.clone());
    template.obligations.push(TemplateObligation::Field {
        shape: shape.clone(),
        field: "name".to_string(),
    });
    template
        .obligations
        .push(TemplateObligation::DynAdapter { contract });
    let specialize = plan_specialization("structured_layout_keys", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );
    let row_layout = core
        .layouts
        .iter()
        .find(
            |layout| matches!(&layout.kind, LayoutKind::RowShape(candidate) if candidate == &shape),
        )
        .expect("row layout");
    let dyn_layout = core
		.layouts
		.iter()
		.find(|layout| {
			matches!(&layout.kind, LayoutKind::DynRowPackage(candidate) if candidate == &expected_contract)
		})
		.expect("dyn row layout");

    assert_eq!(row_layout.key, "row::open::name:String");
    assert_eq!(
        dyn_layout.key,
        "dyn-row::open::name:String;usage=N;send=obligation;adapter=static-to-dyn-package"
    );
    assert!(!row_layout.key.contains("RowShape"));
    assert!(!dyn_layout.key.contains("DynRowContract"));
    assert!(core.ops.iter().any(|op| {
        matches!(op, CoreOp::StaticRowAccess { layout, .. } if layout == &row_layout.key)
    }));
    assert!(core.ops.iter().any(|op| {
        matches!(op, CoreOp::DynRowAdapterAccess { layout, .. } if layout == &dyn_layout.key)
    }));
}

fn halt_unit() -> CpsProgram {
    CpsProgram {
        term: CpsTerm::Halt(CpsAtom::Var("unit".to_string())),
    }
}
