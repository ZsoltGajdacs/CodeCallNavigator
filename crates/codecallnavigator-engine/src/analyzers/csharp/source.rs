//! Bind compiler-decoded facts to exact original bytes without narrowing graph-only encoding support.
use anyhow::{Context, Result, ensure};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub(super) struct Snapshots {
    files: HashMap<PathBuf, Original>,
    contexts: HashMap<(PathBuf, String), Observation>,
}
enum Original {
    Utf8(String),
    Unavailable(Unavailable),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Unavailable {
    NonUtf8,
    Missing,
    Unreadable,
    CompilerEncoding(u32),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceState {
    ExactUtf8,
    Unavailable(Unavailable),
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Observation {
    state: SourceState,
    code_page: Option<u32>,
    compiler_digest: blake3::Hash,
}
pub(super) enum Binding<'a> {
    Exact(&'a str),
    Unavailable(Unavailable),
}
pub(super) struct CompilerContext<'a> {
    pub file: &'a Path,
    pub assembly: &'a str,
    pub code_page: Option<u32>,
}

impl Unavailable {
    pub(super) fn reason(self) -> String {
        match self {
            Self::NonUtf8 => "original source encoding is not UTF-8".into(),
            Self::Missing => "original source file is unavailable".into(),
            Self::Unreadable => "original source bytes could not be read".into(),
            Self::CompilerEncoding(code_page) => {
                format!("compiler source code page {code_page} is not UTF-8")
            }
        }
    }
}

pub(super) fn path(root: &Path, file: &str) -> PathBuf {
    let path = root.join(file);
    path.canonicalize().unwrap_or(path)
}

impl Snapshots {
    pub(super) fn content(
        &mut self,
        context: CompilerContext<'_>,
        compiler: &str,
    ) -> Result<String> {
        let original = self
            .files
            .entry(context.file.into())
            .or_insert_with(|| Original::read(context.file));
        let binding = original.binding(context.code_page);
        let content = binding.content(compiler, context.file)?.to_owned();
        let observation = Observation {
            state: binding.state(),
            code_page: context.code_page,
            compiler_digest: blake3::hash(compiler.as_bytes()),
        };
        let key = (context.file.into(), context.assembly.into());
        if let Some(previous) = self.contexts.get(&key) {
            ensure!(
                *previous == observation,
                "Conflicting C# compiler source decoding context: {}",
                context.file.display()
            );
        } else {
            self.contexts.insert(key, observation);
        }
        Ok(content)
    }

    pub(super) fn binding(&self, file: &Path, assembly: &str) -> Result<Binding<'_>> {
        let observation = self
            .contexts
            .get(&(file.into(), assembly.into()))
            .with_context(|| {
                format!(
                    "Owned C# source has no compiler snapshot for {assembly}: {}",
                    file.display()
                )
            })?;
        match &observation.state {
            SourceState::Unavailable(reason) => Ok(Binding::Unavailable(*reason)),
            SourceState::ExactUtf8 => match self.files.get(file) {
                Some(Original::Utf8(content)) => Ok(Binding::Exact(content)),
                _ => anyhow::bail!(
                    "Validated C# UTF-8 source snapshot is missing: {}",
                    file.display()
                ),
            },
        }
    }
}

impl Binding<'_> {
    fn content<'a>(&'a self, compiler: &'a str, file: &Path) -> Result<&'a str> {
        match self {
            Self::Exact(content) => {
                ensure!(
                    content.strip_prefix('\u{feff}').unwrap_or(content) == compiler,
                    "C# compiler source does not match original UTF-8 bytes: {}",
                    file.display()
                );
                Ok(content)
            }
            Self::Unavailable(_) => Ok(compiler),
        }
    }
    fn state(&self) -> SourceState {
        match self {
            Self::Exact(_) => SourceState::ExactUtf8,
            Self::Unavailable(reason) => SourceState::Unavailable(*reason),
        }
    }
}

impl Original {
    fn binding(&self, code_page: Option<u32>) -> Binding<'_> {
        if let Some(code_page) = code_page.filter(|page| *page != 65001) {
            return Binding::Unavailable(Unavailable::CompilerEncoding(code_page));
        }
        match self {
            Self::Utf8(content) => Binding::Exact(content),
            Self::Unavailable(reason) => Binding::Unavailable(*reason),
        }
    }
    fn read(file: &Path) -> Self {
        match fs::read(file) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(content) => Self::Utf8(content),
                Err(_) => Self::Unavailable(Unavailable::NonUtf8),
            },
            Err(error) => Self::Unavailable(if error.kind() == std::io::ErrorKind::NotFound {
                Unavailable::Missing
            } else {
                Unavailable::Unreadable
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context(file: &Path) -> CompilerContext<'_> {
        CompilerContext {
            file,
            assembly: "Assembly",
            code_page: None,
        }
    }
    #[test]
    fn exact_source_preserves_bom_and_reads_each_owned_file_once() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("Source.cs");
        fs::write(&file, "\u{feff}class A {}\r\n").unwrap();
        let mut snapshots = Snapshots::default();
        assert_eq!(
            snapshots.content(context(&file), "class A {}\r\n").unwrap(),
            "\u{feff}class A {}\r\n"
        );
        fs::write(&file, "new disk content").unwrap();
        assert_eq!(
            snapshots.content(context(&file), "class A {}\r\n").unwrap(),
            "\u{feff}class A {}\r\n"
        );
        assert!(
            snapshots
                .content(context(&file), "different compiler context")
                .is_err()
        );
    }
    #[test]
    fn non_utf8_and_missing_sources_preserve_compiler_decoded_graph_content() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("Source.cs");
        fs::write(&file, [0xff, 0xfe, b'A', 0]).unwrap();
        let mut snapshots = Snapshots::default();
        assert_eq!(snapshots.content(context(&file), "A").unwrap(), "A");
        assert!(matches!(
            snapshots.binding(&file, "Assembly").unwrap(),
            Binding::Unavailable(Unavailable::NonUtf8)
        ));
        let missing = root.path().join("Generated.cs");
        assert_eq!(
            snapshots
                .content(context(&missing), "class Generated {}")
                .unwrap(),
            "class Generated {}"
        );
        assert!(matches!(
            snapshots.binding(&missing, "Assembly").unwrap(),
            Binding::Unavailable(Unavailable::Missing)
        ));
    }
    #[test]
    fn changed_owned_utf8_compiler_text_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("Source.cs");
        fs::write(&file, "new source").unwrap();
        assert!(
            Snapshots::default()
                .content(context(&file), "old compiler source")
                .is_err()
        );
    }
    #[test]
    fn compiler_encoding_is_context_specific_even_when_original_bytes_are_valid_utf8() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("Source.cs");
        fs::write(&file, "é").unwrap();
        let mut snapshots = Snapshots::default();
        let utf8 = CompilerContext {
            file: &file,
            assembly: "Utf8",
            code_page: Some(65001),
        };
        assert_eq!(snapshots.content(utf8, "é").unwrap(), "é");
        let latin1 = CompilerContext {
            file: &file,
            assembly: "Latin1",
            code_page: Some(28591),
        };
        assert_eq!(snapshots.content(latin1, "Ã©").unwrap(), "Ã©");
        assert!(matches!(
            snapshots.binding(&file, "Utf8").unwrap(),
            Binding::Exact("é")
        ));
        assert!(matches!(
            snapshots.binding(&file, "Latin1").unwrap(),
            Binding::Unavailable(Unavailable::CompilerEncoding(28591))
        ));
    }
    #[test]
    fn repeated_graph_only_contexts_cannot_conflict_on_compiler_text() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("Source.cs");
        fs::write(&file, [0xff, 0xfe, b'A', 0]).unwrap();
        let mut snapshots = Snapshots::default();
        let context = || CompilerContext {
            file: &file,
            assembly: "Assembly",
            code_page: Some(1200),
        };
        assert_eq!(snapshots.content(context(), "A").unwrap(), "A");
        assert_eq!(snapshots.content(context(), "A").unwrap(), "A");
        assert!(
            snapshots
                .content(context(), "conflicting compiler text")
                .is_err()
        );
        assert_eq!(snapshots.content(context(), "A").unwrap(), "A");
    }
}
