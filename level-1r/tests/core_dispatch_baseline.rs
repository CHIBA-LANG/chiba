use chiba_level1r::core::{lower_core_with_facts, CoreOp};
use chiba_level1r::cps::{CpsAtom, CpsProgram, CpsTerm};
use chiba_level1r::closure::ClosureFacts;
use chiba_level1r::lambda_lift::LambdaLiftFacts;
use chiba_level1r::specialize::plan_specialization;
use chiba_level1r::template::{
    canonical_open_row, dyn_row_contract, ShapeType, TemplateFacts, TemplateObligation,
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
        op: chiba_level1r::ast::BinaryOp::Add,
        protocol: "op_add".to_string(),
        receiver: Some("Vec2".to_string()),
    });
    let specialize = plan_specialization("add_vec2", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );

    assert!(core.ops.contains(&CoreOp::OperatorTarget {
        protocol: "op_add".to_string(),
        target: "Vec2.op_add".to_string(),
    }));
}

#[test]
fn dyn_row_adapter_access_refs_dyn_package_layout() {
    let mut template = TemplateFacts::default();
    let contract = dyn_row_contract(vec![("name", ShapeType::Named("String".to_string()))]);
    template.dyn_contracts.push(contract.clone());
    template
        .obligations
        .push(TemplateObligation::DynAdapter { contract });
    let specialize = plan_specialization("render_dyn", &template);
    let cps = halt_unit();

    let core = lower_core_with_facts(
        &cps,
        &[],
        &ClosureFacts::default(),
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );
    let expected_shape =
        canonical_open_row(vec![("name", ShapeType::Named("String".to_string()))]);

    assert!(core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::DynRowAdapterAccess { subject, layout }
                if subject == &format!("{expected_shape:?}") && layout.starts_with("dyn-row::")
        )
    }));
}

#[test]
fn field_obligation_enters_core_as_static_row_access() {
    let mut template = TemplateFacts::default();
    let shape = canonical_open_row(vec![("name", ShapeType::Unknown)]);
    template.row_shapes.push(shape);
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
        &LambdaLiftFacts::default(),
        &specialize,
        &UsageFacts::default(),
    );

    assert!(core.ops.iter().any(|op| {
        matches!(
            op,
            CoreOp::StaticRowAccess { field, layout }
                if field == "name" && layout.starts_with("row::")
        )
    }));
}

fn halt_unit() -> CpsProgram {
    CpsProgram {
        term: CpsTerm::Halt(CpsAtom::Var("unit".to_string())),
    }
}
