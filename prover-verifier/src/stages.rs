//! Optional per-stage proving times, enabled by the `stage-profile` feature.
//!
//! Winterfell wraps its proving stages in `tracing` spans. A small subscriber sums the time spent
//! inside each span by name. Spans nest, so totals are inclusive of their children.

#[cfg(feature = "stage-profile")]
mod collector {
    use std::{
        collections::BTreeMap,
        sync::Mutex,
        time::{Duration, Instant},
    };

    use tracing::{
        span::{Attributes, Id, Record},
        Event, Metadata, Subscriber,
    };

    #[derive(Default)]
    struct State {
        names: Vec<&'static str>,
        started: Vec<Option<Instant>>,
        totals: BTreeMap<&'static str, Duration>,
    }

    static STATE: Mutex<Option<State>> = Mutex::new(None);

    struct StageSubscriber;

    impl Subscriber for StageSubscriber {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, attributes: &Attributes<'_>) -> Id {
            let mut guard = STATE.lock().unwrap();
            let state = guard.get_or_insert_with(State::default);
            state.names.push(attributes.metadata().name());
            state.started.push(None);
            Id::from_u64(state.names.len() as u64)
        }

        fn record(&self, _span: &Id, _values: &Record<'_>) {}
        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
        fn event(&self, _event: &Event<'_>) {}

        fn enter(&self, span: &Id) {
            let mut guard = STATE.lock().unwrap();
            let state = guard.get_or_insert_with(State::default);
            state.started[span.into_u64() as usize - 1] = Some(Instant::now());
        }

        fn exit(&self, span: &Id) {
            let now = Instant::now();
            let mut guard = STATE.lock().unwrap();
            let state = guard.get_or_insert_with(State::default);
            let index = span.into_u64() as usize - 1;
            if let Some(started) = state.started[index].take() {
                *state.totals.entry(state.names[index]).or_default() += now - started;
            }
        }
    }

    pub fn install() {
        tracing::subscriber::set_global_default(StageSubscriber)
            .expect("the stage profiler is installed once");
    }

    pub fn reset() {
        if let Some(state) = STATE.lock().unwrap().as_mut() {
            state.totals.clear();
        }
    }

    pub fn totals_ns() -> Option<BTreeMap<&'static str, u128>> {
        let guard = STATE.lock().unwrap();
        let state = guard.as_ref()?;
        Some(
            state
                .totals
                .iter()
                .map(|(name, total)| (*name, total.as_nanos()))
                .collect(),
        )
    }
}

#[cfg(feature = "stage-profile")]
pub use collector::{install, reset, totals_ns};

#[cfg(not(feature = "stage-profile"))]
pub fn install() {}

#[cfg(not(feature = "stage-profile"))]
pub fn reset() {}

#[cfg(not(feature = "stage-profile"))]
pub fn totals_ns() -> Option<std::collections::BTreeMap<&'static str, u128>> {
    None
}
