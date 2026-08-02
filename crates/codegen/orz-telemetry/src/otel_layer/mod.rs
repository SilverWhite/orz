pub fn shutdown_otel() {}
pub fn otel_guard() -> OtelGuard { OtelGuard }
pub struct OtelGuard;
impl Drop for OtelGuard { fn drop(&mut self) {} }
pub fn build_otel_layer(_info: OtelClientInfo, _auth: crate::orz_auth_stub::AuthCredentialProvider) -> impl tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync {
    tracing_subscriber::fmt::layer()
}
#[derive(Debug, Clone)]
pub struct OtelClientInfo {
    pub client_name: &'static str,
    pub client_version: &'static str,
    pub service_version: &'static str,
    pub app_entrypoint: &'static str,
}
