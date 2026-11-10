//! Job-scoped measurements shared by native UI, CLI and analyzer adapters.
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Stage {
    pub calls: u64,
    pub milliseconds: f64,
}

/// Stage durations can nest; they must not be added to obtain total wall time.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub complete: bool,
    pub elapsed_milliseconds: f64,
    pub stages: BTreeMap<String, Stage>,
    pub counters: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub struct Metrics {
    started: Instant,
    report: Arc<Mutex<Report>>,
}

thread_local! {
    static CURRENT: RefCell<Option<Metrics>> = const { RefCell::new(None) };
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            report: Arc::new(Mutex::new(Report {
                schema: "codegraph.performance.v1",
                complete: false,
                elapsed_milliseconds: 0.0,
                stages: BTreeMap::new(),
                counters: BTreeMap::new(),
            })),
        }
    }
}

impl Metrics {
    /// Enter on each worker thread that contributes to this analysis.
    pub fn enter(&self) -> Scope {
        let previous = CURRENT.with(|current| current.replace(Some(self.clone())));
        Scope { previous }
    }

    pub fn report(&self) -> Report {
        let mut report = self.report.lock().expect("metrics lock").clone();
        if !report.complete {
            report.elapsed_milliseconds = self.started.elapsed().as_secs_f64() * 1000.0;
        }
        report
    }

    pub fn finish(&self) {
        let mut report = self.report.lock().expect("metrics lock");
        if !report.complete {
            report.elapsed_milliseconds = self.started.elapsed().as_secs_f64() * 1000.0;
            report.complete = true;
        }
    }
}

pub struct Scope {
    previous: Option<Metrics>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|current| current.replace(self.previous.take()));
    }
}

pub struct Timer {
    measurement: Option<(Metrics, &'static str, Instant)>,
}

pub fn stage(name: &'static str) -> Timer {
    Timer {
        measurement: CURRENT.with(|current| {
            current
                .borrow()
                .clone()
                .map(|metrics| (metrics, name, Instant::now()))
        }),
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let Some((metrics, name, started)) = self.measurement.take() {
            let mut report = metrics.report.lock().expect("metrics lock");
            if report.complete {
                return;
            }
            let stage = report.stages.entry(name.into()).or_default();
            stage.calls += 1;
            stage.milliseconds += started.elapsed().as_secs_f64() * 1000.0;
        }
    }
}

pub fn count(name: &str, amount: u64) {
    CURRENT.with(|current| {
        if let Some(metrics) = current.borrow().as_ref() {
            let mut report = metrics.report.lock().expect("metrics lock");
            if !report.complete {
                *report.counters.entry(name.into()).or_default() += amount;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_jobs_restore_nested_scopes_and_collect_worker_measurements() {
        let first = Metrics::default();
        let second = Metrics::default();
        {
            let _scope = first.enter();
            count("files", 2);
            {
                let _scope = second.enter();
                let _timer = stage("compile");
                count("files", 3);
            }
            count("files", 5);
        }
        let worker = first.clone();
        std::thread::spawn(move || {
            let _scope = worker.enter();
            let _timer = stage("normalize");
            count("files", 1);
        })
        .join()
        .unwrap();
        count("files", 100);
        assert_eq!(first.report().counters["files"], 8);
        assert_eq!(second.report().counters["files"], 3);
        assert_eq!(first.report().stages["normalize"].calls, 1);
        assert!(!first.report().stages.contains_key("compile"));
        assert_eq!(second.report().stages["compile"].calls, 1);
        first.finish();
        let elapsed = first.report().elapsed_milliseconds;
        first.finish();
        assert_eq!(first.report().elapsed_milliseconds, elapsed);
        assert!(first.report().complete);
        {
            let _scope = first.enter();
            let _timer = stage("retry");
            count("files", 100);
        }
        assert_eq!(first.report().counters["files"], 8);
        assert!(!first.report().stages.contains_key("retry"));
    }
}
