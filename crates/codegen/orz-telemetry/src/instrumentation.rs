pub fn install_panic_hook() {}
pub fn layer() -> impl tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync { tracing_subscriber::fmt::layer() }
pub const TARGET: &str = "orz.instrumentation";
pub struct InstrumentationTimer;
impl InstrumentationTimer { pub fn new(_: &str) -> Self { Self } pub fn with_field(&mut self, _: &str, _: &dyn std::fmt::Debug) {} }
pub fn timer(_name: &str) -> InstrumentationTimer { InstrumentationTimer }
