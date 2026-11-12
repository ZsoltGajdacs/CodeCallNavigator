//! Architectural classification shared by command-line analysis and native presentation.
mod classify;
mod config;
mod facts;
mod import;
mod load;
mod rules;

pub use classify::{Assignment, LayerAssignments, classify};
pub use config::{
    Configuration, LanguageRules, LayerDefinition, Matcher, PathBase, Selection, parse,
};
pub use facts::{ArchitectureFacts, FACTS_VERSION, FileFacts, file_key};
pub use load::{LoadedConfiguration, load, load_readonly};
