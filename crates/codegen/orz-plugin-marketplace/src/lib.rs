pub mod installer {
    pub fn install(_entry: &super::MarketplaceEntry) -> Result<(), String> { Ok(()) }
}
pub mod git {
    pub fn clone(_url: &str) -> Result<std::path::PathBuf, String> { Err("stub".into()) }
}
pub struct MarketplaceEntry;
pub struct MarketplaceRelativePath;
pub struct MarketplaceSource;
pub enum SourceKind { Git, Local }
pub fn install_resolve(_entry: &MarketplaceEntry) -> Result<(), String> { Ok(()) }
pub fn is_official_source_url(_url: &str) -> bool { false }
pub fn load_extra_sources_from_settings() -> Vec<MarketplaceSource> { vec![] }
pub fn load_sources() -> Vec<MarketplaceSource> { vec![] }
pub fn scan_marketplace(_sources: &[MarketplaceSource]) -> Vec<MarketplaceEntry> { vec![] }
