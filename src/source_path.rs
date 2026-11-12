//! Lexical source identities: filesystem aliases are never resolved on the UI thread.
use std::path::{Component, Path, PathBuf};

pub(crate) fn identity(value: &str) -> String {
    let path = Path::new(value);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexical_identity_preserves_aliases_and_resolves_components() {
        assert_eq!(
            identity("/repo/src/.././alias/File.cs"),
            "/repo/alias/File.cs"
        );
        assert_eq!(identity("/../../repo/a.cs"), "/repo/a.cs");
        assert!(Path::new(&identity("src/main.rs")).is_absolute());
    }
}
