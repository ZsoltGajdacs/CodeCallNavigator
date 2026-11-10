//! Architectural classification shared by command-line analysis and native presentation.
mod classify;
mod config;
mod facts;
mod import;
mod load;
mod rules;

pub use classify::classify;
pub use config::{
    Configuration, LanguageRules, LayerDefinition, Matcher, PathBase, Selection, parse,
};
pub use facts::{ArchitectureFacts, FACTS_VERSION, FileFacts, file_key};
pub use load::{LoadedConfiguration, load};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Assignment {
    pub layer: String,
    pub rule: String,
}
#[derive(Clone, Debug, Default)]
pub struct LayerAssignments {
    pub files: HashMap<String, Assignment>,
    pub layers: Vec<LayerDefinition>,
    pub origin: String,
}
