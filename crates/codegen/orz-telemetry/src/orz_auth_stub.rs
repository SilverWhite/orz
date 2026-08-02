use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default)]
pub struct CredentialSnapshot;
#[derive(Debug, Clone)]
pub struct AuthCredentialProvider;
impl AuthCredentialProvider { pub fn new() -> Self { Self } }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAuth;
