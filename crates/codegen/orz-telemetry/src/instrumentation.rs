use tracing::Span;

pub fn install_panic_hook() {}

pub fn layer() -> impl tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync {
    tracing_subscriber::fmt::layer()
}

pub const TARGET: &str = "orz.instrumentation";

pub struct InstrumentationTimer {
    name: String,
}

impl InstrumentationTimer {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
    pub fn new_with_span(name: &str, _mode: InstrumentationMode, _entered: Option<tracing::span::EnteredSpan>) -> Self {
        Self { name: name.to_string() }
    }
    pub fn with_field(&mut self, _key: &str, _value: &dyn std::fmt::Debug) {}
    pub fn stop(&self) {}
}

pub fn timer(name: &str) -> InstrumentationTimer {
    InstrumentationTimer::new(name)
}
pub fn timer_with_target(name: &str, _target: &str) -> InstrumentationTimer {
    InstrumentationTimer::new(name)
}
pub fn timer_with_module(name: &str, _module: &str) -> InstrumentationTimer {
    InstrumentationTimer::new(name)
}

pub enum InstrumentationMode {
    Full,
    Minimal,
    Off,
    Chrome,
}

pub fn current_mode() -> InstrumentationMode {
    InstrumentationMode::Off
}
