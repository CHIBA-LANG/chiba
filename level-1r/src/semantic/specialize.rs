use std::collections::BTreeMap;

use crate::control::ContinuationKind;
use crate::resolve::OperatorSurface;
use crate::template::{
    DynRowContract, RowShape, TemplateFacts, TemplateInstantiation, TemplateObligation,
    TemplateParam,
};
use crate::typed::{SendColor, UsageColor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecializationFacts {
    pub work_items: Vec<SpecializationWorkItem>,
    pub registry: InstantiationRegistry,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpecializationKey {
    pub generic_symbol: String,
    pub template_params: Vec<TemplateParam>,
    pub explicit_instantiations: Vec<TemplateInstantiation>,
    pub concrete_nominals: Vec<String>,
    pub normalized_shapes: Vec<RowShape>,
    pub capabilities: CapabilityFacts,
    pub continuations: Vec<ContinuationKind>,
    pub dyn_contracts: Vec<DynRowContract>,
    pub abi_mode: AbiMode,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityFacts {
    pub usage: Vec<UsageColor>,
    pub send: Vec<SendColor>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AbiMode {
    #[default]
    Chiba,
    Wasi,
    Env,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecializationWorkItem {
    pub key: SpecializationKey,
    pub obligations: Vec<DischargedObligation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DischargedObligation {
    Field {
        field: String,
        shape: RowShape,
    },
    Method {
        name: String,
        target: Option<String>,
    },
    Function {
        name: String,
        target: String,
    },
    Static {
        name: String,
        target: String,
    },
    Constructor {
        data: String,
        ctor: String,
        target: String,
        arity: usize,
    },
    Operator {
        op: OperatorSurface,
        protocol: String,
        receiver: Option<String>,
    },
    DynAdapter {
        contract: DynRowContract,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstantiationRegistry {
    entries: BTreeMap<SpecializationKey, InstantiationStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstantiationStatus {
    InProgress,
    Done { artifact: String },
    Failed { diagnostic: String },
}

pub fn plan_specialization(
    generic_symbol: impl Into<String>,
    template: &TemplateFacts,
) -> SpecializationFacts {
    let key = specialization_key(generic_symbol, template);
    let obligations = discharge_obligations(template);
    let mut registry = InstantiationRegistry::default();
    registry.request(key.clone());
    SpecializationFacts {
        work_items: vec![SpecializationWorkItem { key, obligations }],
        registry,
    }
}

pub fn specialization_key(
    generic_symbol: impl Into<String>,
    template: &TemplateFacts,
) -> SpecializationKey {
    let mut normalized_shapes = template.row_shapes.clone();
    normalized_shapes.sort();
    normalized_shapes.dedup();

    let mut concrete_nominals = Vec::new();
    let mut dyn_contracts = template.dyn_contracts.clone();
    dyn_contracts.sort();
    dyn_contracts.dedup();

    for obligation in &template.obligations {
        match obligation {
            TemplateObligation::Method {
                receiver: Some(receiver),
                ..
            }
            | TemplateObligation::Function {
                resolved: receiver, ..
            }
            | TemplateObligation::Static {
                resolved: receiver, ..
            }
            | TemplateObligation::Constructor {
                resolved: receiver, ..
            }
            | TemplateObligation::Operator {
                receiver: Some(receiver),
                ..
            } => concrete_nominals.push(receiver.clone()),
            _ => {}
        }
    }
    concrete_nominals.sort();
    concrete_nominals.dedup();

    SpecializationKey {
        generic_symbol: generic_symbol.into(),
        template_params: template.explicit_params.clone(),
        explicit_instantiations: template.explicit_instantiations.clone(),
        concrete_nominals,
        normalized_shapes,
        capabilities: CapabilityFacts {
            usage: dyn_contracts
                .iter()
                .map(|contract| contract.payload_usage)
                .collect(),
            send: dyn_contracts.iter().map(|contract| contract.send).collect(),
        },
        continuations: Vec::new(),
        dyn_contracts,
        abi_mode: AbiMode::Chiba,
    }
}

pub fn discharge_obligations(template: &TemplateFacts) -> Vec<DischargedObligation> {
    template
        .obligations
        .iter()
        .map(|obligation| match obligation {
            TemplateObligation::Field { shape, field } => DischargedObligation::Field {
                field: field.clone(),
                shape: shape.clone(),
            },
            TemplateObligation::Method { name, resolved, .. } => DischargedObligation::Method {
                name: name.clone(),
                target: resolved.clone(),
            },
            TemplateObligation::Function { name, resolved } => DischargedObligation::Function {
                name: name.clone(),
                target: resolved.clone(),
            },
            TemplateObligation::Static { name, resolved } => DischargedObligation::Static {
                name: name.clone(),
                target: resolved.clone(),
            },
            TemplateObligation::Constructor {
                data,
                ctor,
                resolved,
                arity,
            } => DischargedObligation::Constructor {
                data: data.clone(),
                ctor: ctor.clone(),
                target: resolved.clone(),
                arity: *arity,
            },
            TemplateObligation::Operator {
                op,
                protocol,
                receiver,
            } => DischargedObligation::Operator {
                op: op.clone(),
                protocol: protocol.clone(),
                receiver: receiver.clone(),
            },
            TemplateObligation::DynAdapter { contract } => DischargedObligation::DynAdapter {
                contract: contract.clone(),
            },
        })
        .collect()
}

impl InstantiationRegistry {
    pub fn request(&mut self, key: SpecializationKey) -> InstantiationStatus {
        self.entries
            .entry(key)
            .or_insert(InstantiationStatus::InProgress)
            .clone()
    }

    pub fn finish(&mut self, key: SpecializationKey, artifact: impl Into<String>) {
        self.entries.insert(
            key,
            InstantiationStatus::Done {
                artifact: artifact.into(),
            },
        );
    }

    pub fn fail(&mut self, key: SpecializationKey, diagnostic: impl Into<String>) {
        self.entries.insert(
            key,
            InstantiationStatus::Failed {
                diagnostic: diagnostic.into(),
            },
        );
    }

    pub fn status(&self, key: &SpecializationKey) -> Option<&InstantiationStatus> {
        self.entries.get(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
