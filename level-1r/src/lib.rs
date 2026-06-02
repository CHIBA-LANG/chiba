#[path = "frontend/ast.rs"]
pub mod ast;
#[path = "semantic/alpha.rs"]
pub mod alpha;
#[path = "backend/emit.rs"]
pub mod backend;
pub mod chibalex;
pub mod chibacc;
#[path = "control/closure.rs"]
pub mod closure;
#[path = "control/closure_core_usage.rs"]
pub mod closure_core_usage;
#[path = "control/closure_simplify.rs"]
pub mod closure_simplify;
#[path = "control/check.rs"]
pub mod control;
#[path = "backend/core.rs"]
pub mod core;
#[path = "control/cps.rs"]
pub mod cps;
#[path = "control/cps_usage.rs"]
pub mod cps_usage;
#[path = "tooling/debug.rs"]
pub mod debug;
#[path = "frontend/parser.rs"]
pub mod frontend;
#[path = "control/lambda_lift.rs"]
pub mod lambda_lift;
#[path = "semantic/monomorphize.rs"]
pub mod monomorphize;
#[path = "tooling/nanopass.rs"]
pub mod nanopass;
#[path = "semantic/pattern.rs"]
pub mod pattern;
#[path = "tooling/pipeline.rs"]
pub mod pipeline;
pub mod regex;
#[path = "frontend/resolve.rs"]
pub mod resolve;
#[path = "semantic/specialize.rs"]
pub mod specialize;
#[path = "tooling/std_audit.rs"]
pub mod std_audit;
#[path = "frontend/surface.rs"]
pub mod surface;
#[path = "semantic/template.rs"]
pub mod template;
#[path = "semantic/template_audit.rs"]
pub mod template_audit;
#[path = "semantic/typed.rs"]
pub mod typed;
#[path = "semantic/usage.rs"]
pub mod usage;
#[path = "semantic/usage_audit.rs"]
pub mod usage_audit;

pub use ast::{
    DataDecl, DataVariant, Expr, Literal, NamespaceDecl, ParamDecl, SourceItem, SourceProgram,
    UseDecl,
};
pub use alpha::{alpha_expr, AlphaFacts, BinderId};
pub use cps::{cps_program, CpsProgram};
pub use debug::{render_visual_report, VisualReport};
pub use frontend::{parse_source_program, FrontendError, FrontendOutput};
pub use pipeline::{
    compile_expr, compile_program, compile_program_bundle, compile_source_program_bundle,
    CompileOutput, ProgramCompileOutput, ProgramDefOutput, ProgramDiagnostic, SourceCompileOutput,
};
pub use resolve::{resolve_expr, resolve_expr_with_names, MethodIndex, NameIndex, ResolveFacts};
pub use specialize::{plan_specialization, SpecializationFacts};
pub use surface::{
    build_interface_summary, project_surface, InterfaceSummary, ProjectSurface, SurfaceConstructor,
    SurfaceData, SurfaceDef,
};
pub use template::{analyze_template, TemplateFacts};
pub use template_audit::{audit_checked_templates, TemplateAuditReport};
