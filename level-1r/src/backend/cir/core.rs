use crate::ast::render_source_pattern;
use crate::closure::{CaptureFact, ClosureFacts, ClosureStorageKind};
use crate::control::{ContinuationFact, ContinuationKind, ReplaySafety};
use crate::cps::{CpsAtom, CpsProgram, CpsTerm, OperatorKind};
use crate::lambda_lift::LambdaLiftFacts;
use crate::resolve::OperatorSurface;
use crate::specialize::{DischargedObligation, SpecializationFacts};
use crate::template::{dyn_row_contract_key, row_shape_key, DynRowContract, RowShape};
use crate::typed::{
    AggregateBoundary, AggregateKind, BuiltinMethodCall, RangeBoundary, SendColor, TextBoundary,
    TextKind, Type, UsageColor,
};
use crate::usage::UsageFacts;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreProgram {
    pub ops: Vec<CoreOp>,
    pub layouts: Vec<LayoutFact>,
    pub ownership: Vec<OwnershipFact>,
    pub callable_storage: Vec<CallableStorageFact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreOp {
    ReturnValue(CoreValue),
    RuntimeLet {
        binder: String,
        value: CoreValue,
    },
    ReturnBranch {
        cond: CoreValue,
        then_value: CoreValue,
        else_value: CoreValue,
    },
    ReturnMatch {
        scrutinee: CoreValue,
        arms: Vec<CoreMatchArm>,
    },
    TailCall {
        func: String,
        args: Vec<CoreValue>,
    },
    CallableAlias {
        target: String,
        value: CoreValue,
    },
    TailCallResult {
        binder: String,
    },
    DynamicCallableTarget {
        target: String,
    },
    DynRowParamMethodTarget {
        target: String,
        param: String,
        field: String,
    },
    ExternFunctionTarget {
        target: String,
        owner: String,
        symbol: String,
        abi: CoreExternAbi,
        name: String,
        signature: String,
        result_type: Option<Type>,
        result_ref_cell_lane: Option<CoreRefCellLane>,
    },
    Prompt {
        kind: ContinuationKind,
    },
    CaptureContinuation {
        binder: String,
        kind: ContinuationKind,
        captured: CoreCapturedContinuation,
    },
    DirectMethodTarget {
        name: String,
        target: String,
    },
    OperatorTarget {
        protocol: String,
        target: String,
        intrinsic: Option<OperatorIntrinsic>,
    },
    StaticRowAccess {
        field: String,
        layout: String,
    },
    DynRowAdapterAccess {
        subject: String,
        layout: String,
    },
    Branch {
        cond: String,
    },
    Match {
        scrutinee: String,
        patterns: Vec<String>,
    },
    TupleConstruct {
        nominal: String,
        layout: String,
        fields: Vec<String>,
    },
    TupleFieldGet {
        nominal: String,
        layout: String,
        field: String,
        field_index: usize,
    },
    RecordConstruct {
        layout: String,
        fields: Vec<String>,
    },
    RecordUpdate {
        base: String,
        layout: String,
        fields: Vec<String>,
    },
    RecordFieldGet {
        layout: String,
        field: String,
    },
    RangeFieldGet {
        field: RangeField,
    },
    SliceFieldGet {
        field: SliceField,
    },
    AdtConstruct {
        data: String,
        ctor: String,
        variants: Vec<String>,
        args: Vec<String>,
    },
    AdtTupleBridge {
        data: String,
        ctor: String,
        tuple_fields: Vec<String>,
        tuple_to_adt_intrinsic: CompilerIntrinsic,
        adt_to_tuple_intrinsic: CompilerIntrinsic,
    },
    CompilerIntrinsicUse {
        intrinsic: CompilerIntrinsic,
        owner_namespace: String,
        subject: String,
    },
    TargetSpecificTerm {
        term: TargetSpecificCoreTerm,
    },
    LiftedFunction {
        source: String,
        symbol: String,
        env_params: Vec<String>,
        direct: bool,
        param: Option<String>,
        body: Vec<CoreOp>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorIntrinsic {
    I64Add,
    I64Sub,
    I64Mul,
    I64Div,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreExternAbi {
    Wasi,
    C,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreRefCellLane {
    I32,
    ExternRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilerIntrinsic {
    TupleToAdt,
    AdtToTuple,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetSpecificCoreTerm {
    Wasm,
    Wat,
    Binaryen,
    Funcref,
    Eqref,
}

impl TargetSpecificCoreTerm {
    pub fn debug_name(self) -> &'static str {
        match self {
            TargetSpecificCoreTerm::Wasm => "Wasm",
            TargetSpecificCoreTerm::Wat => "WAT",
            TargetSpecificCoreTerm::Binaryen => "Binaryen",
            TargetSpecificCoreTerm::Funcref => "funcref",
            TargetSpecificCoreTerm::Eqref => "eqref",
        }
    }
}

impl CompilerIntrinsic {
    pub fn owner_namespace(self) -> &'static str {
        "compiler.intrinsic"
    }

    pub fn debug_name(self) -> &'static str {
        match self {
            CompilerIntrinsic::TupleToAdt => "compiler.intrinsic.tuple_to_adt",
            CompilerIntrinsic::AdtToTuple => "compiler.intrinsic.adt_to_tuple",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreMatchArm {
    pub pattern: CorePattern,
    pub value: CoreValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreCapturedContinuation {
    pub param: String,
    pub ops: Vec<CoreOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CorePattern {
    Wildcard,
    Bind(String),
    I64(i64),
    Bool(bool),
    Constructor {
        data: Option<String>,
        ctor: String,
        args: Vec<CorePattern>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreValue {
    Unit,
    I64(i64),
    Bool(bool),
    TextLiteral {
        kind: TextKind,
        value: String,
    },
    Var(String),
    Tuple {
        fields: Vec<CoreValue>,
    },
    TupleField {
        tuple: Box<CoreValue>,
        field: String,
        field_index: usize,
    },
    Range {
        start: Box<CoreValue>,
        end: Box<CoreValue>,
    },
    SliceLiteral {
        items: Vec<CoreValue>,
    },
    Record {
        fields: Vec<CoreRecordValueField>,
    },
    RecordUpdate {
        base: Box<CoreValue>,
        fields: Vec<CoreRecordValueField>,
    },
    RecordField {
        record: Box<CoreValue>,
        field: String,
    },
    DynRowPackage {
        payload: Box<CoreValue>,
        fields: Vec<CoreDynRowField>,
    },
    DynRowField {
        package: Box<CoreValue>,
        field: String,
    },
    RangeField {
        range: Box<CoreValue>,
        field: RangeField,
    },
    AggregateField {
        kind: AggregateKind,
        value: Box<CoreValue>,
        field: SliceField,
    },
    TextField {
        kind: TextKind,
        value: Box<CoreValue>,
        field: TextField,
    },
    AggregateIndex {
        kind: AggregateKind,
        value: Box<CoreValue>,
        index: Box<CoreValue>,
    },
    AggregateSlice {
        kind: AggregateKind,
        value: Box<CoreValue>,
        range: Box<CoreValue>,
    },
    TextIndex {
        kind: TextKind,
        value: Box<CoreValue>,
        index: Box<CoreValue>,
    },
    TextSlice {
        kind: TextKind,
        value: Box<CoreValue>,
        range: Box<CoreValue>,
    },
    BuiltinRuntimeCall {
        call: BuiltinMethodCall,
        args: Vec<CoreValue>,
    },
    Adt {
        data: String,
        ctor: String,
        variants: Vec<String>,
        args: Vec<CoreValue>,
    },
    LiftedFunction {
        source: String,
        symbol: String,
    },
    Rendered {
        debug: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreRecordValueField {
    pub name: String,
    pub value: CoreValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreDynRowField {
    pub name: String,
    pub source: CoreDynRowFieldSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreDynRowFieldSource {
    Field,
    ContractObligation {
        ty: Type,
    },
    ReceiverMethod {
        symbol: String,
        runtime_target: String,
        param_ty: Type,
        result_ty: Type,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeField {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceField {
    Len,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextField {
    Len,
}

impl CoreValue {
    pub fn debug_name(&self) -> String {
        match self {
            CoreValue::Unit => "unit".to_string(),
            CoreValue::I64(value) => value.to_string(),
            CoreValue::Bool(value) => value.to_string(),
            CoreValue::TextLiteral { value, .. } => format!("{value:?}"),
            CoreValue::Var(name) => name.clone(),
            CoreValue::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(CoreValue::debug_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({fields})")
            }
            CoreValue::TupleField { tuple, field, .. } => {
                format!("{}.{}", tuple.debug_name(), field)
            }
            CoreValue::Range { start, end } => {
                format!("{}..{}", start.debug_name(), end.debug_name())
            }
            CoreValue::SliceLiteral { items } => {
                let items = items
                    .iter()
                    .map(CoreValue::debug_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{items}]")
            }
            CoreValue::Record { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| format!("{}={}", field.name, field.value.debug_name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{fields}}}")
            }
            CoreValue::RecordUpdate { base, fields } => {
                let fields = fields
                    .iter()
                    .map(|field| format!("{}={}", field.name, field.value.debug_name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{} | {fields}}}", base.debug_name())
            }
            CoreValue::RecordField { record, field } => {
                format!("{}.{}", record.debug_name(), field)
            }
            CoreValue::DynRowPackage { payload, fields } => {
                let fields = fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("dyn{{payload={}, fields=[{fields}]}}", payload.debug_name())
            }
            CoreValue::DynRowField { package, field } => {
                format!("{}.{}", package.debug_name(), field)
            }
            CoreValue::RangeField { range, field } => {
                format!("{}.{}", range.debug_name(), field.source_name())
            }
            CoreValue::AggregateField {
                kind: _,
                value,
                field,
            } => {
                format!("{}.{}", value.debug_name(), field.source_name())
            }
            CoreValue::TextField {
                kind: _,
                value,
                field,
            } => {
                format!("{}.{}", value.debug_name(), field.source_name())
            }
            CoreValue::AggregateIndex {
                kind: _,
                value,
                index,
            } => {
                format!("{}[{}]", value.debug_name(), index.debug_name())
            }
            CoreValue::AggregateSlice {
                kind: _,
                value,
                range,
            } => {
                format!("{}[{}]", value.debug_name(), range.debug_name())
            }
            CoreValue::TextIndex {
                kind: _,
                value,
                index,
            } => {
                format!("{}[{}]", value.debug_name(), index.debug_name())
            }
            CoreValue::TextSlice {
                kind: _,
                value,
                range,
            } => {
                format!("{}[{}]", value.debug_name(), range.debug_name())
            }
            CoreValue::BuiltinRuntimeCall { call, args } => {
                let args = args
                    .iter()
                    .map(CoreValue::debug_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({args})", call.debug_name())
            }
            CoreValue::Adt {
                data, ctor, args, ..
            } => {
                let args = args
                    .iter()
                    .map(CoreValue::debug_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{data}.{ctor}({args})")
            }
            CoreValue::LiftedFunction { symbol, .. } => symbol.clone(),
            CoreValue::Rendered { debug } => debug.clone(),
        }
    }
}

impl RangeField {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

fn range_field_from_boundary(boundary: RangeBoundary) -> RangeField {
    match boundary {
        RangeBoundary::Start => RangeField::Start,
        RangeBoundary::End => RangeField::End,
    }
}

impl SliceField {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Len => "len",
        }
    }
}

fn slice_field_from_boundary(boundary: AggregateBoundary) -> SliceField {
    match boundary {
        AggregateBoundary::Len => SliceField::Len,
    }
}

impl TextField {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Len => "len",
        }
    }
}

fn text_field_from_boundary(boundary: TextBoundary) -> TextField {
    match boundary {
        TextBoundary::Len => TextField::Len,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutFact {
    pub key: String,
    pub hash: u64,
    pub kind: LayoutKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutKind {
    RowShape(RowShape),
    DynRowPackage(DynRowContract),
    ContinuationPackage(ContinuationEnvLayout),
    Cont1StateMachine(ContinuationEnvLayout),
    ClosureEnv(ClosureEnvLayout),
    TupleStruct(TupleLayout),
    RecordStruct(RecordLayout),
    AdtShape(AdtLayout),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationEnvLayout {
    pub binder: String,
    pub kind: ContinuationKind,
    pub fields: Vec<ClosureEnvField>,
    pub replay_safety: ReplaySafety,
    pub clone_on_resume: bool,
    pub consumed_state_machine: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TupleLayout {
    pub nominal: String,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordLayout {
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdtLayout {
    pub data: String,
    pub variants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureEnvLayout {
    pub closure: String,
    pub fields: Vec<ClosureEnvField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureEnvField {
    pub name: String,
    pub usage: UsageColor,
    pub send: SendColor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnershipFact {
    pub subject: String,
    pub kind: OwnershipSubjectKind,
    pub decision: OwnershipDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OwnershipSubjectKind {
    Value,
    Binder,
    DynRowPackage,
    DynRowPayload { send: SendColor },
    Continuation,
    SharedSendValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OwnershipDecision {
    StackValue,
    InplaceReuse,
    Rc,
    Arc,
    StaticData,
    BorrowedView,
    DynPackage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallableStorageFact {
    pub subject: String,
    pub kind: CallableStorageKind,
    pub usage: UsageColor,
    pub send: SendColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CallableStorageKind {
    DirectFn,
    NoCaptureClosure,
    EnvClosure,
    BoxedCont1,
    ContNPackage,
    ErasedCallableAdt,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoreValidation {
    pub diagnostics: Vec<CoreDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreDiagnostic {
    TargetSpecificTerm {
        term: String,
    },
    LayoutHashMismatch {
        key: String,
        expected: u64,
        actual: u64,
    },
    DuplicateLayoutKey {
        key: String,
    },
    MissingContNPackage {
        binder: String,
    },
    UnsafeContNReplayCapture {
        binder: String,
    },
    Cont1HasPackageLayout {
        key: String,
    },
    MissingDynRowLayout {
        layout: String,
    },
    DynRowLayoutKindMismatch {
        layout: String,
    },
    MissingStaticRowLayout {
        layout: String,
    },
    StaticRowLayoutKindMismatch {
        layout: String,
    },
    MissingTupleLayout {
        layout: String,
    },
    TupleLayoutKindMismatch {
        layout: String,
    },
    TupleFieldMissing {
        layout: String,
        field: String,
    },
    MissingRecordLayout {
        layout: String,
    },
    RecordLayoutKindMismatch {
        layout: String,
    },
    RecordFieldMissing {
        layout: String,
        field: String,
    },
    DanglingTailCallTarget {
        target: String,
    },
    EnvClosureMissingLayout {
        subject: String,
    },
    ClosureEnvLayoutHasNoFields {
        layout: String,
    },
    SendableCallableContainsContinuation {
        subject: String,
    },
    Cont1CallableUsedMoreThanOnce {
        subject: String,
    },
    SharedSendSubjectUsesRc {
        subject: String,
    },
    DynPayloadMustUseDynPackage {
        subject: String,
    },
    DuplicateLiftedFunctionSymbol {
        symbol: String,
    },
    LiftedFunctionMissingClosureEnv {
        source: String,
        expected_layout: String,
    },
    CompilerIntrinsicOwnerMismatch {
        intrinsic: CompilerIntrinsic,
        owner_namespace: String,
        expected_owner_namespace: String,
    },
}

pub fn lower_core(cps: &CpsProgram, continuations: &[ContinuationFact]) -> CoreProgram {
    lower_core_with_facts(
        cps,
        continuations,
        &ClosureFacts::default(),
        &[],
        &LambdaLiftFacts::default(),
        &SpecializationFacts::default(),
        &UsageFacts::default(),
    )
}

pub fn lower_core_with_facts(
    cps: &CpsProgram,
    continuations: &[ContinuationFact],
    closures: &ClosureFacts,
    explicit_callable_storage: &[CallableStorageFact],
    lambda_lift: &LambdaLiftFacts,
    specialize: &SpecializationFacts,
    usage: &UsageFacts,
) -> CoreProgram {
    let mut ops = Vec::new();
    lower_term(&cps.term, continuations, lambda_lift, &mut ops);
    let layouts = lower_layouts(&ops, continuations, closures, specialize);
    let ownership = lower_ownership(continuations, specialize, usage);
    let callable_storage =
        lower_callable_storage(continuations, closures, explicit_callable_storage);
    lower_specialization_ops(specialize, &layouts, &mut ops);
    lower_lifted_functions(cps, continuations, lambda_lift, &mut ops);
    CoreProgram {
        ops,
        layouts,
        ownership,
        callable_storage,
    }
}

pub fn validate_core(program: &CoreProgram) -> CoreValidation {
    let mut diagnostics = Vec::new();
    validate_target_neutral(program, &mut diagnostics);
    validate_layouts(program, &mut diagnostics);
    validate_continuation_packages(program, &mut diagnostics);
    validate_tail_calls(program, &mut diagnostics);
    validate_static_row_access(program, &mut diagnostics);
    validate_tuple_field_access(program, &mut diagnostics);
    validate_record_field_access(program, &mut diagnostics);
    validate_dyn_adapter_layouts(program, &mut diagnostics);
    validate_lifted_functions(program, &mut diagnostics);
    validate_callable_storage(program, &mut diagnostics);
    validate_ownership(program, &mut diagnostics);
    validate_compiler_intrinsics(program, &mut diagnostics);
    CoreValidation { diagnostics }
}

impl CoreValidation {
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

fn lower_term(
    term: &CpsTerm,
    continuations: &[ContinuationFact],
    lambda_lift: &LambdaLiftFacts,
    ops: &mut Vec<CoreOp>,
) {
    match term {
        CpsTerm::Halt(atom) => lower_atom_value(atom, lambda_lift, ops),
        CpsTerm::LetRuntime {
            binder,
            value,
            body,
        } => {
            ops.push(CoreOp::RuntimeLet {
                binder: binder.clone(),
                value: core_value_with_lift(value, lambda_lift),
            });
            lower_term(body, continuations, lambda_lift, ops);
        }
        CpsTerm::AppCont { value, .. } => lower_atom_value(value, lambda_lift, ops),
        CpsTerm::AppFun { func, args, kont } => {
            if let Some(inlined) = inline_fun_lambda_call(func, args, kont) {
                lower_term(&inlined, continuations, lambda_lift, ops);
                return;
            }
            if let Some(value) = static_cps_callable_value(func) {
                if let Some(inlined) = inline_fun_lambda_call(value, args, kont) {
                    lower_term(&inlined, continuations, lambda_lift, ops);
                    return;
                }
            }
            let target = render_atom(func);
            lower_callable_target(&target, func, ops);
            let args = lower_call_args(func, args, lambda_lift);
            ops.push(CoreOp::TailCall { func: target, args });
            lower_tailcall_result(kont, ops);
            lower_continuation_atom(kont, continuations, ops);
        }
        CpsTerm::Prompt { multi, body } => {
            ops.push(CoreOp::Prompt {
                kind: if *multi {
                    ContinuationKind::ContN
                } else {
                    ContinuationKind::Cont1
                },
            });
            lower_term(body, continuations, lambda_lift, ops);
        }
        CpsTerm::Capture {
            binder,
            captured,
            body,
            ..
        } => {
            let kind = continuations
                .iter()
                .find(|fact| fact.binder == *binder)
                .map(|fact| fact.kind)
                .unwrap_or(ContinuationKind::Cont1);
            let captured = lower_captured_continuation(captured, continuations);
            ops.push(CoreOp::CaptureContinuation {
                binder: binder.clone(),
                kind,
                captured,
            });
            lower_term(body, continuations, lambda_lift, ops);
        }
        CpsTerm::Branch {
            cond,
            then_term,
            else_term,
            ..
        } => {
            if let (Some(then_value), Some(else_value)) = (
                direct_return_value(then_term),
                direct_return_value(else_term),
            ) {
                ops.push(CoreOp::ReturnBranch {
                    cond: core_value(cond),
                    then_value,
                    else_value,
                });
                return;
            }
            ops.push(CoreOp::Branch {
                cond: render_atom(cond),
            });
            lower_term(then_term, continuations, lambda_lift, ops);
            lower_term(else_term, continuations, lambda_lift, ops);
        }
        CpsTerm::Match {
            scrutinee, arms, ..
        } => {
            if let Some(arms) = direct_match_arms(arms) {
                ops.push(CoreOp::ReturnMatch {
                    scrutinee: core_value(scrutinee),
                    arms,
                });
                return;
            }
            ops.push(CoreOp::Match {
                scrutinee: render_atom(scrutinee),
                patterns: arms
                    .iter()
                    .map(|arm| render_source_pattern(&arm.pattern))
                    .collect(),
            });
            for arm in arms {
                lower_term(&arm.body, continuations, lambda_lift, ops);
            }
        }
    }
}

fn inline_fun_lambda_call(func: &CpsAtom, args: &[CpsAtom], kont: &CpsAtom) -> Option<CpsTerm> {
    let CpsAtom::FunLambda {
        param,
        k_param,
        body,
    } = func
    else {
        return None;
    };
    let [arg] = args else {
        return None;
    };
    Some(substitute_cps_term(
        body,
        &[(param.as_str(), arg), (k_param.as_str(), kont)],
    ))
}

fn substitute_cps_term(term: &CpsTerm, bindings: &[(&str, &CpsAtom)]) -> CpsTerm {
    match term {
        CpsTerm::Halt(atom) => CpsTerm::Halt(substitute_cps_atom(atom, bindings)),
        CpsTerm::LetRuntime {
            binder,
            value,
            body,
        } => {
            let body_bindings = without_cps_binding(bindings, binder);
            CpsTerm::LetRuntime {
                binder: binder.clone(),
                value: substitute_cps_atom(value, bindings),
                body: Box::new(substitute_cps_term(body, &body_bindings)),
            }
        }
        CpsTerm::AppFun { func, args, kont } => CpsTerm::AppFun {
            func: substitute_cps_atom(func, bindings),
            args: args
                .iter()
                .map(|arg| substitute_cps_atom(arg, bindings))
                .collect(),
            kont: substitute_cps_atom(kont, bindings),
        },
        CpsTerm::AppCont { kont, value } => CpsTerm::AppCont {
            kont: substitute_cps_atom(kont, bindings),
            value: substitute_cps_atom(value, bindings),
        },
        CpsTerm::Prompt { multi, body } => CpsTerm::Prompt {
            multi: *multi,
            body: Box::new(substitute_cps_term(body, bindings)),
        },
        CpsTerm::Capture {
            multi,
            binder,
            captured,
            body,
        } => {
            let body_bindings = without_cps_binding(bindings, binder);
            CpsTerm::Capture {
                multi: *multi,
                binder: binder.clone(),
                captured: substitute_cps_atom(captured, bindings),
                body: Box::new(substitute_cps_term(body, &body_bindings)),
            }
        }
        CpsTerm::Branch {
            cond,
            then_term,
            else_term,
            join,
        } => CpsTerm::Branch {
            cond: substitute_cps_atom(cond, bindings),
            then_term: Box::new(substitute_cps_term(then_term, bindings)),
            else_term: Box::new(substitute_cps_term(else_term, bindings)),
            join: substitute_cps_atom(join, bindings),
        },
        CpsTerm::Match {
            scrutinee,
            arms,
            join,
        } => CpsTerm::Match {
            scrutinee: substitute_cps_atom(scrutinee, bindings),
            arms: arms
                .iter()
                .map(|arm| crate::cps::CpsMatchArm {
                    pattern: arm.pattern.clone(),
                    body: substitute_cps_term(&arm.body, bindings),
                })
                .collect(),
            join: substitute_cps_atom(join, bindings),
        },
    }
}

fn substitute_cps_atom(atom: &CpsAtom, bindings: &[(&str, &CpsAtom)]) -> CpsAtom {
    if let CpsAtom::Var(name) = atom {
        if let Some((_, value)) = bindings.iter().find(|(binding, _)| *binding == name) {
            return (*value).clone();
        }
    }
    match atom {
        CpsAtom::Var(_) | CpsAtom::Lit(_) => atom.clone(),
        CpsAtom::OperatorCallee {
            kind,
            protocol,
            receiver,
        } => CpsAtom::OperatorCallee {
            kind: *kind,
            protocol: protocol.clone(),
            receiver: Box::new(substitute_cps_atom(receiver, bindings)),
        },
        CpsAtom::FunLambda {
            param,
            k_param,
            body,
        } => {
            let body_bindings = without_cps_bindings(bindings, &[param.as_str(), k_param.as_str()]);
            CpsAtom::FunLambda {
                param: param.clone(),
                k_param: k_param.clone(),
                body: Box::new(substitute_cps_term(body, &body_bindings)),
            }
        }
        CpsAtom::ContLambda { param, body } => {
            let body_bindings = without_cps_binding(bindings, param);
            CpsAtom::ContLambda {
                param: param.clone(),
                body: Box::new(substitute_cps_term(body, &body_bindings)),
            }
        }
        CpsAtom::Tuple { nominal, fields } => CpsAtom::Tuple {
            nominal: nominal.clone(),
            fields: fields
                .iter()
                .map(|field| substitute_cps_atom(field, bindings))
                .collect(),
        },
        CpsAtom::SliceLiteral { items } => CpsAtom::SliceLiteral {
            items: items
                .iter()
                .map(|item| substitute_cps_atom(item, bindings))
                .collect(),
        },
        CpsAtom::TupleField {
            tuple,
            field,
            field_index,
        } => CpsAtom::TupleField {
            tuple: Box::new(substitute_cps_atom(tuple, bindings)),
            field: field.clone(),
            field_index: *field_index,
        },
        CpsAtom::RecordField { record, field } => CpsAtom::RecordField {
            record: Box::new(substitute_cps_atom(record, bindings)),
            field: field.clone(),
        },
        CpsAtom::DynRowPackage { payload, fields } => CpsAtom::DynRowPackage {
            payload: Box::new(substitute_cps_atom(payload, bindings)),
            fields: fields.clone(),
        },
        CpsAtom::DynRowField { package, field } => CpsAtom::DynRowField {
            package: Box::new(substitute_cps_atom(package, bindings)),
            field: field.clone(),
        },
        CpsAtom::Range { start, end } => CpsAtom::Range {
            start: Box::new(substitute_cps_atom(start, bindings)),
            end: Box::new(substitute_cps_atom(end, bindings)),
        },
        CpsAtom::RangeField { range, boundary } => CpsAtom::RangeField {
            range: Box::new(substitute_cps_atom(range, bindings)),
            boundary: *boundary,
        },
        CpsAtom::AggregateField {
            kind,
            value,
            boundary,
        } => CpsAtom::AggregateField {
            kind: *kind,
            value: Box::new(substitute_cps_atom(value, bindings)),
            boundary: *boundary,
        },
        CpsAtom::TextField {
            kind,
            value,
            boundary,
        } => CpsAtom::TextField {
            kind: *kind,
            value: Box::new(substitute_cps_atom(value, bindings)),
            boundary: *boundary,
        },
        CpsAtom::AggregateIndex { kind, value, index } => CpsAtom::AggregateIndex {
            kind: *kind,
            value: Box::new(substitute_cps_atom(value, bindings)),
            index: Box::new(substitute_cps_atom(index, bindings)),
        },
        CpsAtom::AggregateSlice { kind, value, range } => CpsAtom::AggregateSlice {
            kind: *kind,
            value: Box::new(substitute_cps_atom(value, bindings)),
            range: Box::new(substitute_cps_atom(range, bindings)),
        },
        CpsAtom::TextIndex { kind, value, index } => CpsAtom::TextIndex {
            kind: *kind,
            value: Box::new(substitute_cps_atom(value, bindings)),
            index: Box::new(substitute_cps_atom(index, bindings)),
        },
        CpsAtom::TextSlice { kind, value, range } => CpsAtom::TextSlice {
            kind: *kind,
            value: Box::new(substitute_cps_atom(value, bindings)),
            range: Box::new(substitute_cps_atom(range, bindings)),
        },
        CpsAtom::BuiltinRuntimeCall { call, args } => CpsAtom::BuiltinRuntimeCall {
            call: *call,
            args: args
                .iter()
                .map(|arg| substitute_cps_atom(arg, bindings))
                .collect(),
        },
        CpsAtom::RecordUpdate {
            base,
            layout,
            fields,
        } => CpsAtom::RecordUpdate {
            base: Box::new(substitute_cps_atom(base, bindings)),
            layout: layout.clone(),
            fields: substitute_cps_record_fields(fields, bindings),
        },
        CpsAtom::Record { layout, fields } => CpsAtom::Record {
            layout: layout.clone(),
            fields: substitute_cps_record_fields(fields, bindings),
        },
        CpsAtom::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => CpsAtom::AdtCtor {
            data: data.clone(),
            ctor: ctor.clone(),
            variants: variants.clone(),
            args: args
                .iter()
                .map(|arg| substitute_cps_atom(arg, bindings))
                .collect(),
        },
    }
}

fn without_cps_binding<'a>(
    bindings: &[(&'a str, &'a CpsAtom)],
    shadowed: &str,
) -> Vec<(&'a str, &'a CpsAtom)> {
    without_cps_bindings(bindings, &[shadowed])
}

fn without_cps_bindings<'a>(
    bindings: &[(&'a str, &'a CpsAtom)],
    shadowed: &[&str],
) -> Vec<(&'a str, &'a CpsAtom)> {
    bindings
        .iter()
        .copied()
        .filter(|(binding, _)| !shadowed.iter().any(|candidate| candidate == binding))
        .collect()
}

fn substitute_cps_record_fields(
    fields: &[crate::cps::CpsRecordField],
    bindings: &[(&str, &CpsAtom)],
) -> Vec<crate::cps::CpsRecordField> {
    fields
        .iter()
        .map(|field| crate::cps::CpsRecordField {
            name: field.name.clone(),
            value: substitute_cps_atom(&field.value, bindings),
        })
        .collect()
}

fn lower_captured_continuation(
    captured: &CpsAtom,
    continuations: &[ContinuationFact],
) -> CoreCapturedContinuation {
    if let CpsAtom::ContLambda { param, body } = captured {
        let mut ops = Vec::new();
        lower_term(body, continuations, &LambdaLiftFacts::default(), &mut ops);
        CoreCapturedContinuation {
            param: param.clone(),
            ops,
        }
    } else {
        let param = render_atom(captured);
        CoreCapturedContinuation {
            param: param.clone(),
            ops: vec![CoreOp::ReturnValue(CoreValue::Var(param))],
        }
    }
}

fn lower_tailcall_result(kont: &CpsAtom, ops: &mut Vec<CoreOp>) {
    if let CpsAtom::ContLambda { param, .. } = kont {
        ops.push(CoreOp::TailCallResult {
            binder: param.clone(),
        });
    }
}

fn lower_call_args(
    func: &CpsAtom,
    args: &[CpsAtom],
    lambda_lift: &LambdaLiftFacts,
) -> Vec<CoreValue> {
    match func {
        CpsAtom::OperatorCallee { receiver, .. } => std::iter::once(core_value(receiver))
            .chain(args.iter().map(core_value))
            .collect(),
        CpsAtom::DynRowField { package, field } => {
            if let Some((payload, _)) = static_cps_dyn_receiver_method(package, field) {
                return std::iter::once(core_value(payload))
                    .chain(args.iter().map(core_value))
                    .collect();
            }
            args.iter()
                .map(|arg| core_callable_arg_value(arg, lambda_lift))
                .collect()
        }
        _ => args
            .iter()
            .map(|arg| core_callable_arg_value(arg, lambda_lift))
            .collect(),
    }
}

fn core_callable_arg_value(atom: &CpsAtom, lambda_lift: &LambdaLiftFacts) -> CoreValue {
    core_value_with_lift(atom, lambda_lift)
}

fn core_value_with_lift(atom: &CpsAtom, lambda_lift: &LambdaLiftFacts) -> CoreValue {
    let CpsAtom::FunLambda { param, .. } = atom else {
        return core_value(atom);
    };
    lambda_lift
        .functions
        .iter()
        .find(|function| function.source == format!("closure::{param}"))
        .map(|function| CoreValue::LiftedFunction {
            source: function.source.clone(),
            symbol: function.symbol.clone(),
        })
        .unwrap_or_else(|| core_value(atom))
}

fn lower_callable_target(target: &str, atom: &CpsAtom, ops: &mut Vec<CoreOp>) {
    match atom {
        CpsAtom::OperatorCallee { kind, protocol, .. } => ops.push(CoreOp::OperatorTarget {
            protocol: protocol.clone(),
            target: target.to_string(),
            intrinsic: operator_intrinsic(*kind),
        }),
        CpsAtom::RecordField { record, field } => {
            if let Some(value) = static_cps_record_field_value(record, field) {
                ops.push(CoreOp::CallableAlias {
                    target: target.to_string(),
                    value: core_value(value),
                });
            }
            ops.push(CoreOp::DynamicCallableTarget {
                target: target.to_string(),
            });
        }
        CpsAtom::DynRowField { package, field } => {
            if let Some((_, runtime_target)) = static_cps_dyn_receiver_method(package, field) {
                ops.push(CoreOp::CallableAlias {
                    target: target.to_string(),
                    value: CoreValue::Var(runtime_target.to_string()),
                });
            } else if let Some(param) = dyn_row_method_package_var(package) {
                ops.push(CoreOp::DynRowParamMethodTarget {
                    target: target.to_string(),
                    param: param.to_string(),
                    field: field.clone(),
                });
            }
            ops.push(CoreOp::DynamicCallableTarget {
                target: target.to_string(),
            });
        }
        _ => ops.push(CoreOp::DynamicCallableTarget {
            target: target.to_string(),
        }),
    }
}

fn dyn_row_method_package_var<'a>(package: &'a CpsAtom) -> Option<&'a str> {
    match package {
        CpsAtom::Var(param) => Some(param.as_str()),
        CpsAtom::RecordField { record, field } => {
            let CpsAtom::Var(param) = static_cps_record_field_value(record, field)? else {
                return None;
            };
            Some(param.as_str())
        }
        _ => None,
    }
}

fn static_cps_record_field_value<'a>(record: &'a CpsAtom, name: &str) -> Option<&'a CpsAtom> {
    let CpsAtom::Record { fields, .. } = record else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

fn static_cps_callable_value(atom: &CpsAtom) -> Option<&CpsAtom> {
    match atom {
        CpsAtom::RecordField { record, field } => static_cps_record_field_value(record, field),
        _ => None,
    }
}

fn static_cps_dyn_receiver_method<'a>(
    package: &'a CpsAtom,
    name: &str,
) -> Option<(&'a CpsAtom, &'a str)> {
    let CpsAtom::DynRowPackage { payload, fields } = package else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .and_then(|field| match &field.source {
            crate::cps::CpsDynRowFieldSource::ReceiverMethod { runtime_target, .. } => {
                Some((payload.as_ref(), runtime_target.as_str()))
            }
            crate::cps::CpsDynRowFieldSource::Field
            | crate::cps::CpsDynRowFieldSource::ContractObligation { .. } => None,
        })
}

fn direct_return_value(term: &CpsTerm) -> Option<CoreValue> {
    match term {
        CpsTerm::Halt(atom) | CpsTerm::AppCont { value: atom, .. } => Some(core_value(atom)),
        _ => None,
    }
}

fn direct_match_arms(arms: &[crate::cps::CpsMatchArm]) -> Option<Vec<CoreMatchArm>> {
    arms.iter()
        .map(|arm| {
            Some(CoreMatchArm {
                pattern: core_pattern(&arm.pattern)?,
                value: direct_return_value(&arm.body)?,
            })
        })
        .collect()
}

fn core_pattern(pattern: &crate::ast::Pattern) -> Option<CorePattern> {
    match pattern {
        crate::ast::Pattern::Wildcard => Some(CorePattern::Wildcard),
        crate::ast::Pattern::Bind(name) => Some(CorePattern::Bind(name.clone())),
        crate::ast::Pattern::Lit(crate::ast::Literal::I64(value)) => Some(CorePattern::I64(*value)),
        crate::ast::Pattern::Lit(crate::ast::Literal::Bool(value)) => {
            Some(CorePattern::Bool(*value))
        }
        crate::ast::Pattern::Constructor { data, ctor, args } => Some(CorePattern::Constructor {
            data: data.clone(),
            ctor: ctor.clone(),
            args: args.iter().map(core_pattern).collect::<Option<Vec<_>>>()?,
        }),
        _ => None,
    }
}

fn lower_continuation_atom(
    atom: &CpsAtom,
    continuations: &[ContinuationFact],
    ops: &mut Vec<CoreOp>,
) {
    if let CpsAtom::ContLambda { body, .. } = atom {
        lower_term(body, continuations, &LambdaLiftFacts::default(), ops);
    }
}

fn lower_specialization_ops(
    specialize: &SpecializationFacts,
    layouts: &[LayoutFact],
    ops: &mut Vec<CoreOp>,
) {
    for item in &specialize.work_items {
        for obligation in &item.obligations {
            match obligation {
                DischargedObligation::Method {
                    name,
                    target: Some(target),
                } => ops.push(CoreOp::DirectMethodTarget {
                    name: name.clone(),
                    target: target.clone(),
                }),
                DischargedObligation::Function { .. }
                | DischargedObligation::Static { .. }
                | DischargedObligation::Constructor { .. } => {}
                DischargedObligation::Operator {
                    op,
                    protocol,
                    receiver: Some(receiver),
                } => ops.push(CoreOp::OperatorTarget {
                    protocol: protocol.clone(),
                    target: format!("{receiver}.{protocol}"),
                    intrinsic: operator_surface_intrinsic(op),
                }),
                DischargedObligation::DynAdapter { contract } => {
                    if let Some(layout) = layouts
                        .iter()
                        .find(|layout| {
                            matches!(&layout.kind, LayoutKind::DynRowPackage(candidate) if candidate == contract)
                        })
                        .map(|layout| layout.key.clone())
                    {
                        ops.push(CoreOp::DynRowAdapterAccess {
                            subject: row_shape_key(&contract.shape),
                            layout,
                        });
                    }
                }
                DischargedObligation::Field { field, shape } => {
                    if let Some(layout) = layouts
                        .iter()
                        .find(|layout| {
                            matches!(&layout.kind, LayoutKind::RowShape(candidate) if candidate == shape)
                        })
                        .map(|layout| layout.key.clone())
                    {
                        ops.push(CoreOp::StaticRowAccess {
                            field: field.clone(),
                            layout,
                        });
                    }
                }
                DischargedObligation::Method { target: None, .. }
                | DischargedObligation::Operator { receiver: None, .. } => {}
            }
        }
    }
}

fn operator_intrinsic(kind: OperatorKind) -> Option<OperatorIntrinsic> {
    match kind {
        OperatorKind::Binary(crate::ast::BinaryOp::Add) => Some(OperatorIntrinsic::I64Add),
        OperatorKind::Binary(crate::ast::BinaryOp::Sub) => Some(OperatorIntrinsic::I64Sub),
        OperatorKind::Binary(crate::ast::BinaryOp::Mul) => Some(OperatorIntrinsic::I64Mul),
        OperatorKind::Binary(crate::ast::BinaryOp::Div) => Some(OperatorIntrinsic::I64Div),
        OperatorKind::Index | OperatorKind::IndexSlice => None,
    }
}

fn operator_surface_intrinsic(op: &OperatorSurface) -> Option<OperatorIntrinsic> {
    match op {
        OperatorSurface::Binary(crate::ast::BinaryOp::Add) => Some(OperatorIntrinsic::I64Add),
        OperatorSurface::Binary(crate::ast::BinaryOp::Sub) => Some(OperatorIntrinsic::I64Sub),
        OperatorSurface::Binary(crate::ast::BinaryOp::Mul) => Some(OperatorIntrinsic::I64Mul),
        OperatorSurface::Binary(crate::ast::BinaryOp::Div) => Some(OperatorIntrinsic::I64Div),
        OperatorSurface::Index | OperatorSurface::IndexSlice => None,
    }
}

fn lower_lifted_functions(
    cps: &CpsProgram,
    continuations: &[ContinuationFact],
    lambda_lift: &LambdaLiftFacts,
    ops: &mut Vec<CoreOp>,
) {
    for function in &lambda_lift.functions {
        let (param, body) = lowered_lifted_body(cps, function, continuations, lambda_lift);
        ops.push(CoreOp::LiftedFunction {
            source: function.source.clone(),
            symbol: function.symbol.clone(),
            env_params: function.env_params.clone(),
            direct: function.direct,
            param,
            body,
        });
    }
}

fn lowered_lifted_body(
    cps: &CpsProgram,
    function: &crate::lambda_lift::LiftedFunctionFact,
    continuations: &[ContinuationFact],
    lambda_lift: &LambdaLiftFacts,
) -> (Option<String>, Vec<CoreOp>) {
    let Some(lambda) = find_cps_fun_lambda(&cps.term, function) else {
        return (None, Vec::new());
    };
    let CpsAtom::FunLambda {
        param,
        k_param,
        body,
    } = lambda
    else {
        unreachable!("find_cps_fun_lambda only returns function lambdas");
    };
    let return_body = return_from_cps_cont(body, k_param);
    let mut ops = Vec::new();
    lower_term(&return_body, continuations, lambda_lift, &mut ops);
    (Some(param.clone()), ops)
}

fn find_cps_fun_lambda<'a>(
    term: &'a CpsTerm,
    function: &crate::lambda_lift::LiftedFunctionFact,
) -> Option<&'a CpsAtom> {
    match term {
        CpsTerm::Halt(atom) | CpsTerm::AppCont { value: atom, .. } => {
            find_cps_fun_lambda_atom(atom, function)
        }
        CpsTerm::LetRuntime { value, body, .. } => find_cps_fun_lambda_atom(value, function)
            .or_else(|| find_cps_fun_lambda(body, function)),
        CpsTerm::AppFun { func, args, kont } => find_cps_fun_lambda_atom(func, function)
            .or_else(|| {
                args.iter()
                    .find_map(|arg| find_cps_fun_lambda_atom(arg, function))
            })
            .or_else(|| find_cps_fun_lambda_atom(kont, function)),
        CpsTerm::Prompt { body, .. } => find_cps_fun_lambda(body, function),
        CpsTerm::Capture { captured, body, .. } => find_cps_fun_lambda_atom(captured, function)
            .or_else(|| find_cps_fun_lambda(body, function)),
        CpsTerm::Branch {
            cond,
            then_term,
            else_term,
            join,
        } => find_cps_fun_lambda_atom(cond, function)
            .or_else(|| find_cps_fun_lambda(then_term, function))
            .or_else(|| find_cps_fun_lambda(else_term, function))
            .or_else(|| find_cps_fun_lambda_atom(join, function)),
        CpsTerm::Match {
            scrutinee,
            arms,
            join,
        } => find_cps_fun_lambda_atom(scrutinee, function)
            .or_else(|| {
                arms.iter()
                    .find_map(|arm| find_cps_fun_lambda(&arm.body, function))
            })
            .or_else(|| find_cps_fun_lambda_atom(join, function)),
    }
}

fn find_cps_fun_lambda_atom<'a>(
    atom: &'a CpsAtom,
    function: &crate::lambda_lift::LiftedFunctionFact,
) -> Option<&'a CpsAtom> {
    match atom {
        CpsAtom::FunLambda { param, body, .. } => {
            if function.source == format!("closure::{param}") {
                Some(atom)
            } else {
                find_cps_fun_lambda(body, function)
            }
        }
        CpsAtom::ContLambda { body, .. } => find_cps_fun_lambda(body, function),
        CpsAtom::OperatorCallee { receiver, .. }
        | CpsAtom::TupleField {
            tuple: receiver, ..
        }
        | CpsAtom::RecordField {
            record: receiver, ..
        }
        | CpsAtom::DynRowPackage {
            payload: receiver, ..
        }
        | CpsAtom::DynRowField {
            package: receiver, ..
        }
        | CpsAtom::RangeField {
            range: receiver, ..
        }
        | CpsAtom::AggregateField {
            value: receiver, ..
        }
        | CpsAtom::TextField {
            value: receiver, ..
        } => find_cps_fun_lambda_atom(receiver, function),
        CpsAtom::Tuple { fields, .. } | CpsAtom::SliceLiteral { items: fields } => fields
            .iter()
            .find_map(|field| find_cps_fun_lambda_atom(field, function)),
        CpsAtom::Range { start, end } => find_cps_fun_lambda_atom(start, function)
            .or_else(|| find_cps_fun_lambda_atom(end, function)),
        CpsAtom::AggregateIndex { value, index, .. } | CpsAtom::TextIndex { value, index, .. } => {
            find_cps_fun_lambda_atom(value, function)
                .or_else(|| find_cps_fun_lambda_atom(index, function))
        }
        CpsAtom::AggregateSlice { value, range, .. } | CpsAtom::TextSlice { value, range, .. } => {
            find_cps_fun_lambda_atom(value, function)
                .or_else(|| find_cps_fun_lambda_atom(range, function))
        }
        CpsAtom::BuiltinRuntimeCall { args, .. } | CpsAtom::AdtCtor { args, .. } => args
            .iter()
            .find_map(|arg| find_cps_fun_lambda_atom(arg, function)),
        CpsAtom::RecordUpdate { base, fields, .. } => find_cps_fun_lambda_atom(base, function)
            .or_else(|| {
                fields
                    .iter()
                    .find_map(|field| find_cps_fun_lambda_atom(&field.value, function))
            }),
        CpsAtom::Record { fields, .. } => fields
            .iter()
            .find_map(|field| find_cps_fun_lambda_atom(&field.value, function)),
        CpsAtom::Var(_) | CpsAtom::Lit(_) => None,
    }
}

fn return_from_cps_cont(term: &CpsTerm, k_param: &str) -> CpsTerm {
    match term {
        CpsTerm::AppCont {
            kont: CpsAtom::Var(kont),
            value,
        } if kont == k_param => CpsTerm::Halt(value.clone()),
        CpsTerm::Halt(atom) => CpsTerm::Halt(atom.clone()),
        CpsTerm::LetRuntime {
            binder,
            value,
            body,
        } => CpsTerm::LetRuntime {
            binder: binder.clone(),
            value: value.clone(),
            body: Box::new(return_from_cps_cont(body, k_param)),
        },
        CpsTerm::AppFun { func, args, kont } => CpsTerm::AppFun {
            func: func.clone(),
            args: args.clone(),
            kont: return_from_cps_cont_atom(kont, k_param),
        },
        CpsTerm::AppCont { kont, value } => CpsTerm::AppCont {
            kont: kont.clone(),
            value: value.clone(),
        },
        CpsTerm::Prompt { multi, body } => CpsTerm::Prompt {
            multi: *multi,
            body: Box::new(return_from_cps_cont(body, k_param)),
        },
        CpsTerm::Capture {
            multi,
            binder,
            captured,
            body,
        } => CpsTerm::Capture {
            multi: *multi,
            binder: binder.clone(),
            captured: captured.clone(),
            body: Box::new(return_from_cps_cont(body, k_param)),
        },
        CpsTerm::Branch {
            cond,
            then_term,
            else_term,
            join,
        } => CpsTerm::Branch {
            cond: cond.clone(),
            then_term: Box::new(return_from_cps_cont(then_term, k_param)),
            else_term: Box::new(return_from_cps_cont(else_term, k_param)),
            join: join.clone(),
        },
        CpsTerm::Match {
            scrutinee,
            arms,
            join,
        } => CpsTerm::Match {
            scrutinee: scrutinee.clone(),
            arms: arms
                .iter()
                .map(|arm| crate::cps::CpsMatchArm {
                    pattern: arm.pattern.clone(),
                    body: return_from_cps_cont(&arm.body, k_param),
                })
                .collect(),
            join: join.clone(),
        },
    }
}

fn return_from_cps_cont_atom(atom: &CpsAtom, k_param: &str) -> CpsAtom {
    match atom {
        CpsAtom::ContLambda { param, body } => CpsAtom::ContLambda {
            param: param.clone(),
            body: Box::new(return_from_cps_cont(body, k_param)),
        },
        _ => atom.clone(),
    }
}

fn validate_target_neutral(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::TargetSpecificTerm { term } = op {
            diagnostics.push(CoreDiagnostic::TargetSpecificTerm {
                term: term.debug_name().to_string(),
            });
        }
    }
}

fn validate_layouts(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    let mut keys = Vec::new();
    for layout in &program.layouts {
        let expected = stable_hash(&layout.key);
        if layout.hash != expected {
            diagnostics.push(CoreDiagnostic::LayoutHashMismatch {
                key: layout.key.clone(),
                expected,
                actual: layout.hash,
            });
        }
        if keys.contains(&layout.key) {
            diagnostics.push(CoreDiagnostic::DuplicateLayoutKey {
                key: layout.key.clone(),
            });
        } else {
            keys.push(layout.key.clone());
        }
        if matches!(&layout.kind, LayoutKind::ContinuationPackage(env) if env.kind == ContinuationKind::Cont1)
        {
            diagnostics.push(CoreDiagnostic::Cont1HasPackageLayout {
                key: layout.key.clone(),
            });
        }
        if matches!(&layout.kind, LayoutKind::ClosureEnv(ClosureEnvLayout { fields, .. }) if fields.is_empty())
        {
            diagnostics.push(CoreDiagnostic::ClosureEnvLayoutHasNoFields {
                layout: layout.key.clone(),
            });
        }
    }
}

fn validate_continuation_packages(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::CaptureContinuation {
            binder,
            kind: ContinuationKind::ContN,
            ..
        } = op
        {
            let has_package = program.layouts.iter().any(|layout| {
                matches!(
                    &layout.kind,
                    LayoutKind::ContinuationPackage(env)
                        if env.kind == ContinuationKind::ContN && env.binder == *binder
                )
            });
            if !has_package {
                diagnostics.push(CoreDiagnostic::MissingContNPackage {
                    binder: binder.clone(),
                });
            } else if program.layouts.iter().any(|layout| {
                matches!(
                    &layout.kind,
                    LayoutKind::ContinuationPackage(env)
                        if env.kind == ContinuationKind::ContN
                            && env.binder == *binder
                            && env.replay_safety == ReplaySafety::Unsafe
                )
            }) {
                diagnostics.push(CoreDiagnostic::UnsafeContNReplayCapture {
                    binder: binder.clone(),
                });
            }
        }
    }
}

fn validate_tail_calls(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        validate_tail_call_op(program, op, diagnostics);
    }
}

fn validate_tail_call_op(
    program: &CoreProgram,
    op: &CoreOp,
    diagnostics: &mut Vec<CoreDiagnostic>,
) {
    match op {
        CoreOp::TailCall { func, .. } if !is_known_tail_target(program, func) => {
            diagnostics.push(CoreDiagnostic::DanglingTailCallTarget {
                target: func.clone(),
            });
        }
        CoreOp::CaptureContinuation { captured, .. } => {
            for op in &captured.ops {
                validate_tail_call_op(program, op, diagnostics);
            }
        }
        _ => {}
    }
}

fn is_known_tail_target(program: &CoreProgram, func: &str) -> bool {
    program.ops.iter().any(|op| op_has_tail_target(op, func))
}

fn op_has_tail_target(op: &CoreOp, func: &str) -> bool {
    match op {
        CoreOp::DynamicCallableTarget { target } => target == func,
        CoreOp::DynRowParamMethodTarget { target, .. } => target == func,
        CoreOp::CallableAlias { target, .. } => target == func,
        CoreOp::ExternFunctionTarget { target, .. } => target == func,
        CoreOp::DirectMethodTarget { target, .. } => target == func,
        CoreOp::OperatorTarget { target, .. } => target == func,
        CoreOp::LiftedFunction { symbol, .. } => symbol == func,
        CoreOp::CaptureContinuation { captured, .. } => {
            captured.ops.iter().any(|op| op_has_tail_target(op, func))
        }
        CoreOp::ReturnValue(_)
        | CoreOp::RuntimeLet { .. }
        | CoreOp::TargetSpecificTerm { .. }
        | CoreOp::ReturnBranch { .. }
        | CoreOp::ReturnMatch { .. }
        | CoreOp::TupleConstruct { .. }
        | CoreOp::TupleFieldGet { .. }
        | CoreOp::RecordConstruct { .. }
        | CoreOp::RecordUpdate { .. }
        | CoreOp::RecordFieldGet { .. }
        | CoreOp::RangeFieldGet { .. }
        | CoreOp::SliceFieldGet { .. }
        | CoreOp::AdtConstruct { .. }
        | CoreOp::AdtTupleBridge { .. }
        | CoreOp::CompilerIntrinsicUse { .. }
        | CoreOp::TailCallResult { .. }
        | CoreOp::TailCall { .. }
        | CoreOp::Prompt { .. }
        | CoreOp::Branch { .. }
        | CoreOp::Match { .. }
        | CoreOp::StaticRowAccess { .. }
        | CoreOp::DynRowAdapterAccess { .. } => false,
    }
}

fn validate_record_field_access(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::RecordFieldGet { layout, field } = op {
            match program.layouts.iter().find(|fact| fact.key == *layout) {
                Some(fact) => match &fact.kind {
                    LayoutKind::RecordStruct(record) if record.fields.contains(field) => {}
                    LayoutKind::RecordStruct(_) => {
                        diagnostics.push(CoreDiagnostic::RecordFieldMissing {
                            layout: layout.clone(),
                            field: field.clone(),
                        })
                    }
                    _ => diagnostics.push(CoreDiagnostic::RecordLayoutKindMismatch {
                        layout: layout.clone(),
                    }),
                },
                None => diagnostics.push(CoreDiagnostic::MissingRecordLayout {
                    layout: layout.clone(),
                }),
            }
        }
    }
}

fn validate_static_row_access(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::StaticRowAccess { layout, .. } = op {
            match program.layouts.iter().find(|fact| fact.key == *layout) {
                Some(fact) if matches!(&fact.kind, LayoutKind::RowShape(_)) => {}
                Some(_) => diagnostics.push(CoreDiagnostic::StaticRowLayoutKindMismatch {
                    layout: layout.clone(),
                }),
                None => diagnostics.push(CoreDiagnostic::MissingStaticRowLayout {
                    layout: layout.clone(),
                }),
            }
        }
    }
}

fn validate_tuple_field_access(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::TupleFieldGet { layout, field, .. } = op {
            match program.layouts.iter().find(|fact| fact.key == *layout) {
                Some(fact) => match &fact.kind {
                    LayoutKind::TupleStruct(tuple) if tuple.fields.contains(field) => {}
                    LayoutKind::TupleStruct(_) => {
                        diagnostics.push(CoreDiagnostic::TupleFieldMissing {
                            layout: layout.clone(),
                            field: field.clone(),
                        })
                    }
                    _ => diagnostics.push(CoreDiagnostic::TupleLayoutKindMismatch {
                        layout: layout.clone(),
                    }),
                },
                None => diagnostics.push(CoreDiagnostic::MissingTupleLayout {
                    layout: layout.clone(),
                }),
            }
        }
    }
}

fn validate_dyn_adapter_layouts(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        if let CoreOp::DynRowAdapterAccess { layout, .. } = op {
            match program.layouts.iter().find(|fact| fact.key == *layout) {
                Some(fact) if matches!(&fact.kind, LayoutKind::DynRowPackage(_)) => {}
                Some(_) => diagnostics.push(CoreDiagnostic::DynRowLayoutKindMismatch {
                    layout: layout.clone(),
                }),
                None => diagnostics.push(CoreDiagnostic::MissingDynRowLayout {
                    layout: layout.clone(),
                }),
            }
        }
    }
}

fn validate_lifted_functions(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    let mut symbols = Vec::new();
    for op in &program.ops {
        if let CoreOp::LiftedFunction {
            source,
            symbol,
            env_params,
            direct,
            ..
        } = op
        {
            if symbols.contains(symbol) {
                diagnostics.push(CoreDiagnostic::DuplicateLiftedFunctionSymbol {
                    symbol: symbol.clone(),
                });
            } else {
                symbols.push(symbol.clone());
            }

            if !direct && !env_params.is_empty() {
                let expected_layout = format!("closure-env::{source}");
                let has_env_layout = program.layouts.iter().any(|layout| {
                    layout.key == expected_layout
                        && matches!(&layout.kind, LayoutKind::ClosureEnv(_))
                });
                if !has_env_layout {
                    diagnostics.push(CoreDiagnostic::LiftedFunctionMissingClosureEnv {
                        source: source.clone(),
                        expected_layout,
                    });
                }
            }
        }
    }
}

fn validate_callable_storage(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for fact in &program.callable_storage {
        let continuation_variant = matches!(
            fact.kind,
            CallableStorageKind::BoxedCont1 | CallableStorageKind::ContNPackage
        );
        if fact.send == SendColor::Send && continuation_variant {
            diagnostics.push(CoreDiagnostic::SendableCallableContainsContinuation {
                subject: fact.subject.clone(),
            });
        }
        if fact.kind == CallableStorageKind::BoxedCont1 && fact.usage == UsageColor::Many {
            diagnostics.push(CoreDiagnostic::Cont1CallableUsedMoreThanOnce {
                subject: fact.subject.clone(),
            });
        }
        if fact.kind == CallableStorageKind::EnvClosure {
            let expected_layout = format!("closure-env::{}", fact.subject);
            let has_layout = program.layouts.iter().any(|layout| {
                layout.key == expected_layout && matches!(&layout.kind, LayoutKind::ClosureEnv(_))
            });
            if !has_layout {
                diagnostics.push(CoreDiagnostic::EnvClosureMissingLayout {
                    subject: fact.subject.clone(),
                });
            }
        }
    }
}

fn validate_ownership(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for fact in &program.ownership {
        if fact.kind == OwnershipSubjectKind::SharedSendValue
            && fact.decision == OwnershipDecision::Rc
        {
            diagnostics.push(CoreDiagnostic::SharedSendSubjectUsesRc {
                subject: fact.subject.clone(),
            });
        }
        if fact.kind == OwnershipSubjectKind::DynRowPackage
            && fact.decision != OwnershipDecision::DynPackage
        {
            diagnostics.push(CoreDiagnostic::DynPayloadMustUseDynPackage {
                subject: fact.subject.clone(),
            });
        }
    }
}

fn validate_compiler_intrinsics(program: &CoreProgram, diagnostics: &mut Vec<CoreDiagnostic>) {
    for op in &program.ops {
        let CoreOp::CompilerIntrinsicUse {
            intrinsic,
            owner_namespace,
            ..
        } = op
        else {
            continue;
        };
        let expected = intrinsic.owner_namespace();
        if owner_namespace != expected {
            diagnostics.push(CoreDiagnostic::CompilerIntrinsicOwnerMismatch {
                intrinsic: *intrinsic,
                owner_namespace: owner_namespace.clone(),
                expected_owner_namespace: expected.to_string(),
            });
        }
    }
}

fn render_atom(atom: &CpsAtom) -> String {
    match atom {
        CpsAtom::Var(name) => name.clone(),
        CpsAtom::Lit(crate::ast::Literal::I64(value)) => value.to_string(),
        CpsAtom::Lit(crate::ast::Literal::Rune(value)) => format!("{value}rune"),
        CpsAtom::Lit(crate::ast::Literal::Bool(value)) => value.to_string(),
        CpsAtom::Lit(crate::ast::Literal::String(value)) => format!("{value:?}"),
        CpsAtom::Lit(crate::ast::Literal::CStr(value)) => format!("c{value:?}"),
        CpsAtom::OperatorCallee {
            protocol, receiver, ..
        } => {
            format!("{protocol}({receiver})")
        }
        CpsAtom::FunLambda { param, .. } => format!("lambda#{param}"),
        CpsAtom::ContLambda { param, .. } => format!("cont#{param}"),
        CpsAtom::Tuple { nominal, fields } => format!(
            "{nominal}({})",
            fields
                .iter()
                .map(render_atom)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CpsAtom::TupleField { tuple, field, .. } => format!("{}.{}", render_atom(tuple), field),
        CpsAtom::SliceLiteral { items } => format!(
            "[{}]",
            items.iter().map(render_atom).collect::<Vec<_>>().join(", ")
        ),
        CpsAtom::Range { start, end } => format!("{}..{}", render_atom(start), render_atom(end)),
        CpsAtom::RangeField { range, boundary } => {
            format!("{}.{}", render_atom(range), boundary.source_name())
        }
        CpsAtom::AggregateField {
            kind: _,
            value,
            boundary,
        } => {
            format!("{}.{}", render_atom(value), boundary.source_name())
        }
        CpsAtom::TextField {
            kind: _,
            value,
            boundary,
        } => {
            format!("{}.{}", render_atom(value), boundary.source_name())
        }
        CpsAtom::AggregateIndex {
            kind: _,
            value,
            index,
        } => {
            format!("{}[{}]", render_atom(value), render_atom(index))
        }
        CpsAtom::AggregateSlice {
            kind: _,
            value,
            range,
        } => {
            format!("{}[{}]", render_atom(value), render_atom(range))
        }
        CpsAtom::TextIndex {
            kind: _,
            value,
            index,
        } => {
            format!("{}[{}]", render_atom(value), render_atom(index))
        }
        CpsAtom::TextSlice {
            kind: _,
            value,
            range,
        } => {
            format!("{}[{}]", render_atom(value), render_atom(range))
        }
        CpsAtom::BuiltinRuntimeCall { call, args } => {
            format!(
                "{}({})",
                call.debug_name(),
                args.iter().map(render_atom).collect::<Vec<_>>().join(", ")
            )
        }
        CpsAtom::Record { layout, fields } => format!(
            "{layout}{{{}}}",
            fields
                .iter()
                .map(|field| format!("{}={}", field.name, render_atom(&field.value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CpsAtom::RecordField { record, field } => format!("{}.{}", render_atom(record), field),
        CpsAtom::DynRowPackage { payload, fields } => format!(
            "dyn{{payload={}, fields=[{}]}}",
            render_atom(payload),
            fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CpsAtom::DynRowField { package, field } => {
            format!("{}.{}", render_atom(package), field)
        }
        CpsAtom::RecordUpdate {
            base,
            layout,
            fields,
        } => format!(
            "{layout}{{base={}, {}}}",
            render_atom(base),
            fields
                .iter()
                .map(|field| format!("{}={}", field.name, render_atom(&field.value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CpsAtom::AdtCtor {
            data, ctor, args, ..
        } => format!(
            "{data}.{ctor}({})",
            args.iter().map(render_atom).collect::<Vec<_>>().join(", ")
        ),
    }
}

fn core_value(atom: &CpsAtom) -> CoreValue {
    match atom {
        CpsAtom::Lit(crate::ast::Literal::I64(value)) => CoreValue::I64(*value),
        CpsAtom::Lit(crate::ast::Literal::Rune(value)) => CoreValue::I64(i64::from(*value)),
        CpsAtom::Lit(crate::ast::Literal::Bool(value)) => CoreValue::Bool(*value),
        CpsAtom::Lit(crate::ast::Literal::String(value)) => CoreValue::TextLiteral {
            kind: TextKind::String,
            value: value.clone(),
        },
        CpsAtom::Lit(crate::ast::Literal::CStr(value)) => CoreValue::TextLiteral {
            kind: TextKind::CStr,
            value: value.clone(),
        },
        CpsAtom::Var(name) if name == "Unit" || name == "unit" => CoreValue::Unit,
        CpsAtom::Var(name) => CoreValue::Var(name.clone()),
        CpsAtom::OperatorCallee { .. } => CoreValue::Rendered {
            debug: render_atom(atom),
        },
        CpsAtom::Tuple { fields, .. } => CoreValue::Tuple {
            fields: fields.iter().map(core_value).collect(),
        },
        CpsAtom::SliceLiteral { items } => CoreValue::SliceLiteral {
            items: items.iter().map(core_value).collect(),
        },
        CpsAtom::TupleField {
            tuple,
            field,
            field_index,
        } => CoreValue::TupleField {
            tuple: Box::new(core_value(tuple)),
            field: field.clone(),
            field_index: *field_index,
        },
        CpsAtom::Range { start, end } => CoreValue::Range {
            start: Box::new(core_value(start)),
            end: Box::new(core_value(end)),
        },
        CpsAtom::Record { fields, .. } => CoreValue::Record {
            fields: fields
                .iter()
                .map(|field| CoreRecordValueField {
                    name: field.name.clone(),
                    value: core_value(&field.value),
                })
                .collect(),
        },
        CpsAtom::RecordUpdate { base, fields, .. } => CoreValue::RecordUpdate {
            base: Box::new(core_value(base)),
            fields: fields
                .iter()
                .map(|field| CoreRecordValueField {
                    name: field.name.clone(),
                    value: core_value(&field.value),
                })
                .collect(),
        },
        CpsAtom::RangeField { range, boundary } => CoreValue::RangeField {
            range: Box::new(core_value(range)),
            field: range_field_from_boundary(*boundary),
        },
        CpsAtom::AggregateField {
            kind,
            value,
            boundary,
        } => CoreValue::AggregateField {
            kind: *kind,
            value: Box::new(core_value(value)),
            field: slice_field_from_boundary(*boundary),
        },
        CpsAtom::TextField {
            kind,
            value,
            boundary,
        } => CoreValue::TextField {
            kind: *kind,
            value: Box::new(core_value(value)),
            field: text_field_from_boundary(*boundary),
        },
        CpsAtom::AggregateIndex { kind, value, index } => CoreValue::AggregateIndex {
            kind: *kind,
            value: Box::new(core_value(value)),
            index: Box::new(core_value(index)),
        },
        CpsAtom::AggregateSlice { kind, value, range } => CoreValue::AggregateSlice {
            kind: *kind,
            value: Box::new(core_value(value)),
            range: Box::new(core_value(range)),
        },
        CpsAtom::TextIndex { kind, value, index } => CoreValue::TextIndex {
            kind: *kind,
            value: Box::new(core_value(value)),
            index: Box::new(core_value(index)),
        },
        CpsAtom::TextSlice { kind, value, range } => CoreValue::TextSlice {
            kind: *kind,
            value: Box::new(core_value(value)),
            range: Box::new(core_value(range)),
        },
        CpsAtom::BuiltinRuntimeCall { call, args } => CoreValue::BuiltinRuntimeCall {
            call: *call,
            args: args.iter().map(core_value).collect(),
        },
        CpsAtom::RecordField { record, field } => CoreValue::RecordField {
            record: Box::new(core_value(record)),
            field: field.clone(),
        },
        CpsAtom::DynRowPackage { payload, fields } => CoreValue::DynRowPackage {
            payload: Box::new(core_value(payload)),
            fields: fields
                .iter()
                .map(|field| CoreDynRowField {
                    name: field.name.clone(),
                    source: match &field.source {
                        crate::cps::CpsDynRowFieldSource::Field => CoreDynRowFieldSource::Field,
                        crate::cps::CpsDynRowFieldSource::ContractObligation { ty } => {
                            CoreDynRowFieldSource::ContractObligation { ty: ty.clone() }
                        }
                        crate::cps::CpsDynRowFieldSource::ReceiverMethod {
                            symbol,
                            runtime_target,
                            param_ty,
                            result_ty,
                        } => CoreDynRowFieldSource::ReceiverMethod {
                            symbol: symbol.clone(),
                            runtime_target: runtime_target.clone(),
                            param_ty: param_ty.clone(),
                            result_ty: result_ty.clone(),
                        },
                    },
                })
                .collect(),
        },
        CpsAtom::DynRowField { package, field } => CoreValue::DynRowField {
            package: Box::new(core_value(package)),
            field: field.clone(),
        },
        CpsAtom::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => CoreValue::Adt {
            data: data.clone(),
            ctor: ctor.clone(),
            variants: variants.clone(),
            args: args.iter().map(core_value).collect(),
        },
        _ => CoreValue::Rendered {
            debug: render_atom(atom),
        },
    }
}

fn lower_atom_value(atom: &CpsAtom, lambda_lift: &LambdaLiftFacts, ops: &mut Vec<CoreOp>) {
    match atom {
        CpsAtom::Tuple { nominal, fields } => {
            let layout = format!("tuple::{nominal}");
            ops.push(CoreOp::TupleConstruct {
                nominal: nominal.clone(),
                layout,
                fields: fields.iter().map(render_atom).collect(),
            });
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::TupleField {
            tuple,
            field,
            field_index,
        } => {
            if let CpsAtom::Tuple { nominal, fields } = tuple.as_ref() {
                ops.push(CoreOp::TupleConstruct {
                    nominal: nominal.clone(),
                    layout: format!("tuple::{nominal}"),
                    fields: fields.iter().map(render_atom).collect(),
                });
                ops.push(CoreOp::TupleFieldGet {
                    nominal: nominal.clone(),
                    layout: format!("tuple::{nominal}"),
                    field: field.clone(),
                    field_index: *field_index,
                });
            }
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::SliceLiteral { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::Range { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::BuiltinRuntimeCall { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::RangeField { boundary, .. } => {
            ops.push(CoreOp::RangeFieldGet {
                field: range_field_from_boundary(*boundary),
            });
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::AggregateField { boundary, .. } => {
            ops.push(CoreOp::SliceFieldGet {
                field: slice_field_from_boundary(*boundary),
            });
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::TextField { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::AggregateIndex { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::AggregateSlice { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::TextIndex { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::TextSlice { .. } => {
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::Record { layout, fields } => {
            ops.push(CoreOp::RecordConstruct {
                layout: layout.clone(),
                fields: fields.iter().map(|field| field.name.clone()).collect(),
            });
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::RecordUpdate {
            base,
            layout,
            fields,
        } => {
            ops.push(CoreOp::RecordUpdate {
                base: render_atom(base),
                layout: layout.clone(),
                fields: fields.iter().map(|field| field.name.clone()).collect(),
            });
            lower_atom_value(base, lambda_lift, ops);
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::RecordField { record, field } => {
            if let CpsAtom::Record { layout, fields } = record.as_ref() {
                ops.push(CoreOp::RecordConstruct {
                    layout: layout.clone(),
                    fields: fields.iter().map(|field| field.name.clone()).collect(),
                });
                ops.push(CoreOp::RecordFieldGet {
                    layout: layout.clone(),
                    field: field.clone(),
                });
            }
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::DynRowPackage { payload, .. } => {
            lower_atom_value(payload, lambda_lift, ops);
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::DynRowField { package, .. } => {
            lower_atom_value(package, lambda_lift, ops);
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        CpsAtom::AdtCtor {
            data,
            ctor,
            variants,
            args,
        } => {
            ops.push(CoreOp::AdtConstruct {
                data: data.clone(),
                ctor: ctor.clone(),
                variants: variants.clone(),
                args: args.iter().map(render_atom).collect(),
            });
            ops.push(CoreOp::AdtTupleBridge {
                data: data.clone(),
                ctor: ctor.clone(),
                tuple_fields: std::iter::once(format!(":{ctor}"))
                    .chain(args.iter().map(render_atom))
                    .collect(),
                tuple_to_adt_intrinsic: CompilerIntrinsic::TupleToAdt,
                adt_to_tuple_intrinsic: CompilerIntrinsic::AdtToTuple,
            });
            ops.push(CoreOp::CompilerIntrinsicUse {
                intrinsic: CompilerIntrinsic::TupleToAdt,
                owner_namespace: CompilerIntrinsic::TupleToAdt.owner_namespace().to_string(),
                subject: format!("{data}.{ctor}"),
            });
            ops.push(CoreOp::CompilerIntrinsicUse {
                intrinsic: CompilerIntrinsic::AdtToTuple,
                owner_namespace: CompilerIntrinsic::AdtToTuple.owner_namespace().to_string(),
                subject: format!("{data}.{ctor}"),
            });
            ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift)));
        }
        _ => ops.push(CoreOp::ReturnValue(core_value_with_lift(atom, lambda_lift))),
    }
}

fn lower_layouts(
    ops: &[CoreOp],
    continuations: &[ContinuationFact],
    closures: &ClosureFacts,
    specialize: &SpecializationFacts,
) -> Vec<LayoutFact> {
    let mut layouts = Vec::new();
    for item in &specialize.work_items {
        for shape in &item.key.normalized_shapes {
            let key = row_shape_layout_key(shape);
            layouts.push(LayoutFact {
                hash: stable_hash(&key),
                key,
                kind: LayoutKind::RowShape(shape.clone()),
            });
        }
        for contract in &item.key.dyn_contracts {
            let key = dyn_row_layout_key(contract);
            layouts.push(LayoutFact {
                hash: stable_hash(&key),
                key,
                kind: LayoutKind::DynRowPackage(contract.clone()),
            });
        }
    }
    for fact in continuations {
        let key = continuation_layout_key(fact.kind, &fact.binder);
        let env = continuation_env_layout(fact);
        layouts.push(LayoutFact {
            hash: stable_hash(&key),
            key,
            kind: match fact.kind {
                ContinuationKind::Cont1 => LayoutKind::Cont1StateMachine(env),
                ContinuationKind::ContN => LayoutKind::ContinuationPackage(env),
            },
        });
    }
    collect_tuple_layouts(&mut layouts, ops);
    collect_record_layouts(&mut layouts, ops);
    collect_adt_layouts(&mut layouts, ops);
    for closure in &closures.closures {
        if closure.storage == ClosureStorageKind::EnvClosure {
            let env = closure_env_layout(&closure.param, &closure.captures);
            let key = format!("closure-env::closure::{}", closure.param);
            layouts.push(LayoutFact {
                hash: stable_hash(&key),
                key,
                kind: LayoutKind::ClosureEnv(env),
            });
        }
    }
    layouts
}

fn continuation_layout_key(kind: ContinuationKind, binder: &str) -> String {
    format!("continuation::{}::{binder}", continuation_kind_key(kind))
}

fn continuation_kind_key(kind: ContinuationKind) -> &'static str {
    match kind {
        ContinuationKind::Cont1 => "cont1",
        ContinuationKind::ContN => "contn",
    }
}

fn row_shape_layout_key(shape: &RowShape) -> String {
    format!("row::{}", row_shape_key(shape).replace('|', "::"))
}

fn dyn_row_layout_key(contract: &DynRowContract) -> String {
    format!(
        "dyn-row::{}",
        dyn_row_contract_key(contract).replace('|', "::")
    )
}

fn collect_adt_layouts(layouts: &mut Vec<LayoutFact>, ops: &[CoreOp]) {
    for op in ops {
        let CoreOp::AdtConstruct { data, variants, .. } = op else {
            continue;
        };
        let key = format!("adt::{data}");
        if layouts.iter().any(|fact| fact.key == key) {
            continue;
        }
        layouts.push(LayoutFact {
            key: key.clone(),
            hash: stable_hash(&key),
            kind: LayoutKind::AdtShape(AdtLayout {
                data: data.clone(),
                variants: variants.clone(),
            }),
        });
    }
}

fn collect_record_layouts(layouts: &mut Vec<LayoutFact>, ops: &[CoreOp]) {
    for op in ops {
        let (layout, fields) = match op {
            CoreOp::RecordConstruct { layout, fields } => (layout, fields),
            CoreOp::RecordUpdate { layout, fields, .. } => (layout, fields),
            CoreOp::RecordFieldGet { .. } => continue,
            _ => continue,
        };
        if layouts.iter().any(|fact| fact.key == *layout) {
            continue;
        }
        layouts.push(LayoutFact {
            key: layout.clone(),
            hash: stable_hash(layout),
            kind: LayoutKind::RecordStruct(RecordLayout {
                fields: fields.clone(),
            }),
        });
    }
}

fn collect_tuple_layouts(layouts: &mut Vec<LayoutFact>, ops: &[CoreOp]) {
    for op in ops {
        let (nominal, layout, fields_len) = match op {
            CoreOp::TupleConstruct {
                nominal,
                layout,
                fields,
            } => (nominal, layout, fields.len()),
            CoreOp::TupleFieldGet { .. } => continue,
            _ => continue,
        };
        if layouts.iter().any(|fact| fact.key == *layout) {
            continue;
        }
        let field_names = (1..=fields_len)
            .map(|index| format!("_{index}"))
            .collect::<Vec<_>>();
        layouts.push(LayoutFact {
            key: layout.clone(),
            hash: stable_hash(layout),
            kind: LayoutKind::TupleStruct(TupleLayout {
                nominal: nominal.clone(),
                fields: field_names,
            }),
        });
    }
}

fn lower_ownership(
    continuations: &[ContinuationFact],
    specialize: &SpecializationFacts,
    usage: &UsageFacts,
) -> Vec<OwnershipFact> {
    let mut facts = Vec::new();
    for (name, count) in &usage.vars {
        facts.push(OwnershipFact {
            subject: format!("var::{name}"),
            kind: OwnershipSubjectKind::Value,
            decision: ownership_from_usage(count.color(), SendColor::Obligation),
        });
    }
    for (binder, count) in &usage.binders {
        facts.push(OwnershipFact {
            subject: format!("binder::{binder}"),
            kind: OwnershipSubjectKind::Binder,
            decision: ownership_from_usage(count.color(), SendColor::Obligation),
        });
    }
    for item in &specialize.work_items {
        for contract in &item.key.dyn_contracts {
            facts.push(OwnershipFact {
                subject: format!("dyn::{}", dyn_row_contract_key(contract)),
                kind: OwnershipSubjectKind::DynRowPackage,
                decision: OwnershipDecision::DynPackage,
            });
            facts.push(OwnershipFact {
                subject: format!("dyn-payload::{}", dyn_row_contract_key(contract)),
                kind: OwnershipSubjectKind::DynRowPayload {
                    send: contract.send,
                },
                decision: ownership_from_usage(contract.payload_usage, contract.send),
            });
        }
    }
    for fact in continuations {
        facts.push(OwnershipFact {
            subject: format!("continuation::{}", fact.binder),
            kind: OwnershipSubjectKind::Continuation,
            decision: match fact.kind {
                ContinuationKind::Cont1 => OwnershipDecision::StackValue,
                ContinuationKind::ContN => OwnershipDecision::DynPackage,
            },
        });
    }
    facts
}

fn lower_callable_storage(
    continuations: &[ContinuationFact],
    closures: &ClosureFacts,
    explicit: &[CallableStorageFact],
) -> Vec<CallableStorageFact> {
    let mut facts = Vec::new();
    facts.push(CallableStorageFact {
        subject: "top-level-def".to_string(),
        kind: CallableStorageKind::DirectFn,
        usage: UsageColor::Many,
        send: SendColor::Send,
    });
    for closure in &closures.closures {
        let has_captures = !closure.captures.is_empty();
        facts.push(CallableStorageFact {
            subject: format!("closure::{}", closure.param),
            kind: match closure.storage {
                ClosureStorageKind::DirectNoCapture => CallableStorageKind::NoCaptureClosure,
                ClosureStorageKind::EnvClosure => CallableStorageKind::EnvClosure,
            },
            usage: if has_captures {
                UsageColor::Many
            } else {
                UsageColor::One
            },
            send: if has_captures {
                SendColor::Obligation
            } else {
                SendColor::Send
            },
        });
    }
    for fact in continuations {
        facts.push(CallableStorageFact {
            subject: format!("continuation::{}", fact.binder),
            kind: match fact.kind {
                ContinuationKind::Cont1 => CallableStorageKind::BoxedCont1,
                ContinuationKind::ContN => CallableStorageKind::ContNPackage,
            },
            usage: fact.usage,
            send: SendColor::NotSend,
        });
    }
    facts.extend(explicit.iter().cloned());
    if !continuations.is_empty() || !closures.closures.is_empty() {
        facts.push(CallableStorageFact {
            subject: "callable-storage::erased".to_string(),
            kind: CallableStorageKind::ErasedCallableAdt,
            usage: UsageColor::Many,
            send: SendColor::Obligation,
        });
    }
    facts
}

fn closure_env_layout(closure: &str, captures: &[CaptureFact]) -> ClosureEnvLayout {
    let fields = captures
        .iter()
        .map(|capture| ClosureEnvField {
            name: capture.name.clone(),
            usage: capture.usage,
            send: SendColor::Obligation,
        })
        .collect::<Vec<_>>();
    ClosureEnvLayout {
        closure: format!("closure::{closure}"),
        fields,
    }
}

fn continuation_env_layout(fact: &ContinuationFact) -> ContinuationEnvLayout {
    ContinuationEnvLayout {
        binder: fact.binder.clone(),
        kind: fact.kind,
        fields: Vec::new(),
        replay_safety: fact.replay_safety,
        clone_on_resume: fact.kind == ContinuationKind::ContN,
        consumed_state_machine: fact.kind == ContinuationKind::Cont1,
    }
}

fn ownership_from_usage(usage: UsageColor, send: SendColor) -> OwnershipDecision {
    match (usage, send) {
        (UsageColor::One, _) => OwnershipDecision::StackValue,
        (UsageColor::Many, SendColor::Send) => OwnershipDecision::Arc,
        (UsageColor::Many, SendColor::NotSend | SendColor::Obligation) => OwnershipDecision::Rc,
        (UsageColor::Obligation, _) => OwnershipDecision::BorrowedView,
    }
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_layout_nominal_is_not_inferred_from_layout_key_shape() {
        let ops = vec![CoreOp::TupleConstruct {
            nominal: "SourceTuple".to_string(),
            layout: "opaque-layout-key".to_string(),
            fields: vec!["1".to_string()],
        }];
        let mut layouts = Vec::new();

        collect_tuple_layouts(&mut layouts, &ops);

        assert_eq!(
            layouts,
            vec![LayoutFact {
                key: "opaque-layout-key".to_string(),
                hash: stable_hash("opaque-layout-key"),
                kind: LayoutKind::TupleStruct(TupleLayout {
                    nominal: "SourceTuple".to_string(),
                    fields: vec!["_1".to_string()],
                }),
            }]
        );
    }
}
