//! Searching

use crate::config::Config;

/// A search index manager
///
/// This is clone-safe: cloning this struct will return references to the same object
#[derive(Clone)]
pub struct SearchIndex {}

impl SearchIndex {
    /// Create a new instance of the search index
    pub fn new(config: Config) {}
}
