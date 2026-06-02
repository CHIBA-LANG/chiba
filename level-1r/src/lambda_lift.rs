use crate::closure::{ClosureFacts, ClosureStorageKind};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LambdaLiftFacts {
    pub functions: Vec<LiftedFunctionFact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiftedFunctionFact {
    pub source: String,
    pub symbol: String,
    pub env_params: Vec<String>,
    pub direct: bool,
}

pub fn lift_lambdas(closures: &ClosureFacts) -> LambdaLiftFacts {
    let functions = closures
        .closures
        .iter()
        .enumerate()
        .map(|(index, closure)| {
            let source = format!("closure::{}", closure.param);
            let symbol = lifted_symbol(index, &source);
            let env_params = closure
                .captures
                .iter()
                .map(|capture| capture.name.clone())
                .collect::<Vec<_>>();
            LiftedFunctionFact {
                source,
                symbol,
                env_params,
                direct: closure.storage == ClosureStorageKind::DirectNoCapture,
            }
        })
        .collect::<Vec<_>>();
    LambdaLiftFacts { functions }
}

fn lifted_symbol(index: usize, source: &str) -> String {
    let clean = source
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("lift::{index:04}::{clean}")
}
