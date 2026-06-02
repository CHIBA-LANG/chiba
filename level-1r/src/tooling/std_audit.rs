#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StdAuditReport {
    pub requirements: Vec<StdRequirement>,
    pub diagnostics: Vec<StdAuditDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StdRequirement {
    pub capability: StdCapability,
    pub classification: StdClassification,
    pub rust_surface: String,
    pub chiba_surface: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StdCapability {
    GrowableSequence,
    OrderedMap,
    OrderedSet,
    StringBuilder,
    Formatting,
    StableHash,
    Utf8Chars,
    RcSharedReference,
    TimeMeasurement,
    FileRead,
    PathJoin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StdClassification {
    ChibaStdFirstBatch,
    RustImplementationConvenience,
    ChibaUnsafeOrMetalBoundary,
    CompilerIntrinsic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdAuditDiagnostic {
    MissingClassification { capability: StdCapability },
    EmptyChibaSurface { capability: StdCapability },
}

pub fn audit_std_dependencies() -> StdAuditReport {
    let mut requirements = vec![
        requirement(
            StdCapability::GrowableSequence,
            StdClassification::ChibaStdFirstBatch,
            "Vec<T>",
            "Vec[T]",
            "all nanopass facts, token streams, regex programs, parser arms, and Core ops need appendable ordered storage",
        ),
        requirement(
            StdCapability::OrderedMap,
            StdClassification::ChibaStdFirstBatch,
            "BTreeMap<K, V>",
            "Map[K, V] with stable iteration",
            "alpha scopes, method indexes, specialization registries, parser rules, and usage maps need deterministic lookup and dumps",
        ),
        requirement(
            StdCapability::OrderedSet,
            StdClassification::ChibaStdFirstBatch,
            "BTreeSet<T>",
            "Set[T] with stable iteration",
            "closure capture analysis and backend linking need deterministic duplicate elimination",
        ),
        requirement(
            StdCapability::StringBuilder,
            StdClassification::ChibaStdFirstBatch,
            "String + push_str",
            "String builder / StrBuf",
            "visual reports, WAT text, manifests, mangled names, and parser diagnostics build incremental strings",
        ),
        requirement(
            StdCapability::Formatting,
            StdClassification::RustImplementationConvenience,
            "fmt::Write + Debug",
            "compiler debug/render traits",
            "Rust derives provide report rendering convenience; Chiba needs explicit render methods, not Rust trait semantics",
        ),
        requirement(
            StdCapability::StableHash,
            StdClassification::CompilerIntrinsic,
            "custom FNV-style stable hash",
            "compiler.intrinsic.stable_hash",
            "layout keys and backend cache keys must be reproducible across hosts and cannot rely on Rust HashMap hashers",
        ),
        requirement(
            StdCapability::Utf8Chars,
            StdClassification::ChibaStdFirstBatch,
            "str::chars",
            "std.regex.utf8 codepoint iterator",
            "lexer and regex baseline operate on Unicode scalar values and must not regress to byte-only scanning",
        ),
        requirement(
            StdCapability::RcSharedReference,
            StdClassification::CompilerIntrinsic,
            "Rc<T>",
            "usage color N lowering evidence",
            "Rc is an audit signal for multi-owner Chiba values, ContN frames, and shared callable storage",
        ),
        requirement(
            StdCapability::TimeMeasurement,
            StdClassification::RustImplementationConvenience,
            "Instant + Duration",
            "nanopass timing hook",
            "pass timing is developer instrumentation; Chiba may expose it behind diagnostics without changing compiler semantics",
        ),
        requirement(
            StdCapability::FileRead,
            StdClassification::ChibaUnsafeOrMetalBoundary,
            "fs::read_to_string",
            "metalstd.file.read_to_string",
            "spec alignment tests and future compiler driver need host file IO through an explicit boundary",
        ),
        requirement(
            StdCapability::PathJoin,
            StdClassification::ChibaUnsafeOrMetalBoundary,
            "PathBuf::join",
            "metalstd.path.join",
            "project/spec discovery needs host path handling isolated from pure Core/CIR semantics",
        ),
    ];
    requirements.sort();
    requirements.dedup_by(|left, right| left.capability == right.capability);

    let diagnostics = requirements
        .iter()
        .flat_map(validate_requirement)
        .collect::<Vec<_>>();

    StdAuditReport {
        requirements,
        diagnostics,
    }
}

fn requirement(
    capability: StdCapability,
    classification: StdClassification,
    rust_surface: &str,
    chiba_surface: &str,
    reason: &str,
) -> StdRequirement {
    StdRequirement {
        capability,
        classification,
        rust_surface: rust_surface.to_string(),
        chiba_surface: chiba_surface.to_string(),
        reason: reason.to_string(),
    }
}

fn validate_requirement(requirement: &StdRequirement) -> Vec<StdAuditDiagnostic> {
    let mut diagnostics = Vec::new();
    if requirement.chiba_surface.is_empty() {
        diagnostics.push(StdAuditDiagnostic::EmptyChibaSurface {
            capability: requirement.capability,
        });
    }
    diagnostics
}

impl StdAuditReport {
    pub fn by_classification(&self, classification: StdClassification) -> Vec<&StdRequirement> {
        self.requirements
            .iter()
            .filter(|requirement| requirement.classification == classification)
            .collect()
    }
}
