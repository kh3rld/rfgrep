//! Core search engine implementation
use crate::metrics::Metrics;
use crate::search::cache::SearchCache;
use crate::search::plugins::PluginManager;
use std::sync::Arc;

/// Main search engine that coordinates all search operations
pub struct SearchEngine {
    pub metrics: Arc<Metrics>,
    pub plugin_manager: PluginManager,
    pub cache: SearchCache,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new(metrics: Arc<Metrics>) -> crate::error::Result<Self> {
        Ok(Self {
            metrics,
            plugin_manager: PluginManager::new()?,
            cache: SearchCache::new(),
        })
    }
}
