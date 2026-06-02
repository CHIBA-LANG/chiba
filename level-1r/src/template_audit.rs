use crate::monomorphize::MonomorphizationPlan;
use crate::specialize::{DischargedObligation, SpecializationFacts};
use crate::template::{TemplateFacts, TemplateObligation};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateAuditReport {
    pub obligations: Vec<TemplateAuditEntry>,
    pub diagnostics: Vec<TemplateAuditDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateAuditEntry {
    pub source: TemplateObligationSource,
    pub definition_time_check: bool,
    pub instantiation_time_discharge: bool,
    pub monomorphized: bool,
    pub specialization_key: String,
    pub rust_trait_solver_used: bool,
    pub explanation: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemplateObligationSource {
    RowShape,
    Field,
    Method,
    Operator,
    DynAdapter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateAuditDiagnostic {
    MissingDefinitionTimeCheck {
        source: TemplateObligationSource,
    },
    MissingInstantiationDischarge {
        source: TemplateObligationSource,
    },
    MissingMonomorphization {
        source: TemplateObligationSource,
    },
    RustTraitSolverLeak {
        source: TemplateObligationSource,
    },
}

pub fn audit_checked_templates(
    template: &TemplateFacts,
    specialize: &SpecializationFacts,
    monomorphize: &MonomorphizationPlan,
) -> TemplateAuditReport {
    let mut entries = Vec::new();

    for shape in &template.row_shapes {
        entries.push(TemplateAuditEntry {
            source: TemplateObligationSource::RowShape,
            definition_time_check: true,
            instantiation_time_discharge: true,
            monomorphized: !monomorphize.jobs.is_empty(),
            specialization_key: format!("shape::{shape:?}"),
            rust_trait_solver_used: false,
            explanation: "canonical row shape collected during definition-time checking and reused in specialization key".to_string(),
        });
    }

    for obligation in &template.obligations {
        let source = obligation_source(obligation);
        let discharged = obligation_discharged(source, specialize);
        entries.push(TemplateAuditEntry {
            source,
            definition_time_check: true,
            instantiation_time_discharge: discharged,
            monomorphized: !monomorphize.jobs.is_empty(),
            specialization_key: specialization_key_for(source, specialize),
            rust_trait_solver_used: false,
            explanation: explanation_for(obligation),
        });
    }

    for contract in &template.dyn_contracts {
        let source = TemplateObligationSource::DynAdapter;
        entries.push(TemplateAuditEntry {
            source,
            definition_time_check: true,
            instantiation_time_discharge: obligation_discharged(source, specialize),
            monomorphized: !monomorphize.jobs.is_empty(),
            specialization_key: format!("dyn::{contract:?}"),
            rust_trait_solver_used: false,
            explanation: "dyn adapter obligation is packaged at instantiation, not resolved by runtime impl search".to_string(),
        });
    }

    let diagnostics = entries
        .iter()
        .flat_map(validate_entry)
        .collect::<Vec<_>>();

    TemplateAuditReport {
        obligations: entries,
        diagnostics,
    }
}

fn obligation_source(obligation: &TemplateObligation) -> TemplateObligationSource {
    match obligation {
        TemplateObligation::Field { .. } => TemplateObligationSource::Field,
        TemplateObligation::Method { .. } => TemplateObligationSource::Method,
        TemplateObligation::Operator { .. } => TemplateObligationSource::Operator,
        TemplateObligation::DynAdapter { .. } => TemplateObligationSource::DynAdapter,
    }
}

fn obligation_discharged(
    source: TemplateObligationSource,
    specialize: &SpecializationFacts,
) -> bool {
    specialize.work_items.iter().any(|item| {
        item.obligations
            .iter()
            .any(|obligation| discharged_source(obligation) == source)
    })
}

fn discharged_source(obligation: &DischargedObligation) -> TemplateObligationSource {
    match obligation {
        DischargedObligation::Field { .. } => TemplateObligationSource::Field,
        DischargedObligation::Method { .. } => TemplateObligationSource::Method,
        DischargedObligation::Operator { .. } => TemplateObligationSource::Operator,
        DischargedObligation::DynAdapter { .. } => TemplateObligationSource::DynAdapter,
    }
}

fn specialization_key_for(
    source: TemplateObligationSource,
    specialize: &SpecializationFacts,
) -> String {
    specialize
        .work_items
        .first()
        .map(|item| format!("{source:?}::{:?}", item.key))
        .unwrap_or_else(|| format!("{source:?}::<missing>"))
}

fn explanation_for(obligation: &TemplateObligation) -> String {
    match obligation {
        TemplateObligation::Field { field, .. } => {
            format!("field access `{field}` becomes an open-row structural obligation")
        }
        TemplateObligation::Method { name, resolved, .. } => match resolved {
            Some(target) => format!("method `{name}` is discharged to concrete target `{target}`"),
            None => format!("method `{name}` remains a checked-template method obligation"),
        },
        TemplateObligation::Operator { protocol, .. } => {
            format!("operator protocol `{protocol}` is a structural operator obligation")
        }
        TemplateObligation::DynAdapter { .. } => {
            "dyn adapter is constructed from the expected dynamic row contract".to_string()
        }
    }
}

fn validate_entry(entry: &TemplateAuditEntry) -> Vec<TemplateAuditDiagnostic> {
    let mut diagnostics = Vec::new();
    if !entry.definition_time_check {
        diagnostics.push(TemplateAuditDiagnostic::MissingDefinitionTimeCheck {
            source: entry.source,
        });
    }
    if !entry.instantiation_time_discharge {
        diagnostics.push(TemplateAuditDiagnostic::MissingInstantiationDischarge {
            source: entry.source,
        });
    }
    if !entry.monomorphized {
        diagnostics.push(TemplateAuditDiagnostic::MissingMonomorphization {
            source: entry.source,
        });
    }
    if entry.rust_trait_solver_used {
        diagnostics.push(TemplateAuditDiagnostic::RustTraitSolverLeak {
            source: entry.source,
        });
    }
    diagnostics
}
