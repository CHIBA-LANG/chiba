use std::collections::BTreeMap;

use crate::specialize::{SpecializationFacts, SpecializationKey};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MonomorphizationPlan {
    pub jobs: Vec<MonomorphizationJob>,
    pub duplicates: Vec<DuplicateInstantiation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonomorphizationJob {
    pub key: SpecializationKey,
    pub artifact: String,
    pub status: MonomorphizationStatus,
    pub call_sites: Vec<String>,
    pub definition_note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateInstantiation {
    pub key: SpecializationKey,
    pub call_site: String,
    pub joined_artifact: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonomorphizationStatus {
    Scheduled,
    InProgress,
    Failed {
        call_site: String,
        definition_note: String,
        diagnostic: String,
    },
}

pub fn schedule_monomorphization(facts: &SpecializationFacts) -> MonomorphizationPlan {
    let mut by_key = BTreeMap::<SpecializationKey, MonomorphizationJob>::new();
    let mut duplicates = Vec::new();

    for (index, item) in facts.work_items.iter().enumerate() {
        let call_site = format!("call-site::{index}");
        let artifact = artifact_name(&item.key);
        match by_key.get_mut(&item.key) {
            Some(existing) => {
                existing.call_sites.push(call_site.clone());
                duplicates.push(DuplicateInstantiation {
                    key: item.key.clone(),
                    call_site,
                    joined_artifact: existing.artifact.clone(),
                });
            }
            None => {
                by_key.insert(
                    item.key.clone(),
                    MonomorphizationJob {
                        definition_note: definition_note(&item.key),
                        key: item.key.clone(),
                        artifact,
                        status: MonomorphizationStatus::Scheduled,
                        call_sites: vec![call_site],
                    },
                );
            }
        }
    }

    MonomorphizationPlan {
        jobs: by_key.into_values().collect(),
        duplicates,
    }
}

pub fn mark_in_progress(plan: &mut MonomorphizationPlan, key: &SpecializationKey) {
    if let Some(job) = plan.jobs.iter_mut().find(|job| &job.key == key) {
        job.status = MonomorphizationStatus::InProgress;
    }
}

pub fn fail_job(
    plan: &mut MonomorphizationPlan,
    key: &SpecializationKey,
    call_site: impl Into<String>,
    diagnostic: impl Into<String>,
) {
    if let Some(job) = plan.jobs.iter_mut().find(|job| &job.key == key) {
        job.status = MonomorphizationStatus::Failed {
            call_site: call_site.into(),
            definition_note: job.definition_note.clone(),
            diagnostic: diagnostic.into(),
        };
    }
}

fn artifact_name(key: &SpecializationKey) -> String {
    let mut text = format!("mono::{}", key.generic_symbol);
    if !key.template_params.is_empty() {
        text.push_str("::params=");
        text.push_str(&stable_hash(&format!("{:?}", key.template_params)).to_string());
    }
    if !key.explicit_instantiations.is_empty() {
        text.push_str("::typeargs=");
        text.push_str(&stable_hash(&format!("{:?}", key.explicit_instantiations)).to_string());
    }
    if !key.concrete_nominals.is_empty() {
        text.push_str("::nominal=");
        text.push_str(&key.concrete_nominals.join("+"));
    }
    if !key.normalized_shapes.is_empty() {
        text.push_str("::shape=");
        text.push_str(&stable_hash(&format!("{:?}", key.normalized_shapes)).to_string());
    }
    if !key.dyn_contracts.is_empty() {
        text.push_str("::dyn=");
        text.push_str(&stable_hash(&format!("{:?}", key.dyn_contracts)).to_string());
    }
    text.push_str("::abi=");
    text.push_str(&format!("{:?}", key.abi_mode));
    text
}

fn definition_note(key: &SpecializationKey) -> String {
    format!("definition-site::{}", key.generic_symbol)
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
