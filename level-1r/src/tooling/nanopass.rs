use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PassReport {
    pub events: Vec<PassEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassEvent {
    pub name: &'static str,
    pub input: &'static str,
    pub output: &'static str,
    pub elapsed: Duration,
}

impl PassReport {
    pub fn record<T>(
        &mut self,
        name: &'static str,
        input: &'static str,
        output: &'static str,
        run: impl FnOnce() -> T,
    ) -> T {
        let started = Instant::now();
        let value = run();
        self.events.push(PassEvent {
            name,
            input,
            output,
            elapsed: started.elapsed(),
        });
        value
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.events.iter().map(|event| event.name).collect()
    }
}
