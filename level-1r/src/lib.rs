pub mod ast;
pub mod alpha;
pub mod backend;
pub mod chibalex;
pub mod chibacc;
pub mod closure;
pub mod closure_core_usage;
pub mod closure_simplify;
pub mod control;
pub mod core;
pub mod cps;
pub mod cps_usage;
pub mod debug;
pub mod frontend;
pub mod lambda_lift;
pub mod monomorphize;
pub mod nanopass;
pub mod pattern;
pub mod pipeline;
pub mod regex;
pub mod resolve;
pub mod specialize;
pub mod std_audit;
pub mod surface;
pub mod template;
pub mod template_audit;
pub mod typed;
pub mod usage;
pub mod usage_audit;

pub use ast::{
    DataDecl, DataVariant, Expr, Literal, NamespaceDecl, ParamDecl, SourceProgram, UseDecl,
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
