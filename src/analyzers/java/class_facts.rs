//! Source facts shared by all methods of an unchanged compiled class.
use super::{build::JavaSourceSet, metadata::ClassMetadata};
use crate::model::Visibility;
use std::path::{Path, PathBuf};

pub(super) struct ClassFacts {
    pub namespace: String,
    pub file: String,
    pub generated_file: bool,
    pub visibility: Option<Visibility>,
}

impl ClassFacts {
    pub fn new(source: &JavaSourceSet, class: &ClassMetadata) -> Self {
        let namespace = class
            .name
            .rsplit_once('.')
            .map(|(namespace, _)| namespace)
            .unwrap_or("")
            .to_owned();
        let file = source_location(source, &namespace, &class.source_file);
        let generated_file = source
            .generated_roots
            .iter()
            .any(|root| Path::new(&file).starts_with(root));
        Self {
            namespace,
            file,
            generated_file,
            visibility: class.visibility,
        }
    }
}

fn source_location(source: &JavaSourceSet, namespace: &str, file: &str) -> String {
    if file.is_empty() || file == "<Unknown>" {
        return String::new();
    }
    let relative = PathBuf::from(namespace.replace('.', "/")).join(file);
    source
        .source_roots
        .iter()
        .chain(&source.generated_roots)
        .map(|root| root.join(&relative))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}
