//! Current recorded inputs plus recursive file contents, separate from certification.
use super::{Paths, Reason, provenance};
use crate::fingerprint::{self, ContentObservation, InputKind};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    path: String,
    excluded_children: Vec<String>,
}

#[derive(serde::Serialize)]
struct Value {
    kind: Option<InputKind>,
    hash: String,
    link: Option<String>,
}

pub(super) struct Revision {
    pub revision: String,
    pub complete: bool,
    pub reasons: Vec<Reason>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Scope {
    path: PathBuf,
    excluded_children: Vec<String>,
}

impl Scope {
    fn outputs(&self) -> Vec<PathBuf> {
        self.excluded_children
            .iter()
            .map(|child| self.path.join(child))
            .collect()
    }
}

pub(super) struct Inputs<'a> {
    paths: &'a Paths,
    cancel: &'a AtomicBool,
    values: BTreeMap<Key, Value>,
    scopes: BTreeSet<Scope>,
    complete: bool,
    reasons: Vec<Reason>,
    ignored_outputs: Vec<PathBuf>,
}

impl<'a> Inputs<'a> {
    pub fn excluding(paths: &'a Paths, cancel: &'a AtomicBool, outputs: &[PathBuf]) -> Self {
        Self {
            paths,
            cancel,
            values: BTreeMap::new(),
            scopes: BTreeSet::new(),
            complete: true,
            reasons: Vec::new(),
            ignored_outputs: outputs.into(),
        }
    }

    pub fn observe(&self, path: &Path, excluded: &[String]) -> Result<ContentObservation> {
        fingerprint::observe_content_cancellable(path, excluded, self.cancel)
    }

    pub fn check(&self) -> Result<()> {
        ensure!(
            !self.cancel.load(Ordering::Acquire),
            "Freshness observation cancelled"
        );
        Ok(())
    }

    pub fn unavailable_root(&mut self, root: &str) {
        self.complete = false;
        self.values.insert(
            Key {
                path: format!("@{root}/"),
                excluded_children: Vec::new(),
            },
            Value {
                kind: None,
                hash: "unavailable_root".into(),
                link: None,
            },
        );
    }

    pub fn record(
        &mut self,
        input: &provenance::Input,
        local: &Path,
        current: Result<&ContentObservation, &anyhow::Error>,
    ) -> Result<()> {
        self.check()?;
        if current.is_ok_and(|value| value.kind == InputKind::Directory) {
            self.scopes.insert(Scope {
                path: local.into(),
                excluded_children: input.excluded_children.clone(),
            });
        }
        self.insert(
            Key {
                path: input.path.clone(),
                excluded_children: input.excluded_children.clone(),
            },
            local,
            current,
        );
        Ok(())
    }

    fn insert(
        &mut self,
        key: Key,
        local: &Path,
        current: Result<&ContentObservation, &anyhow::Error>,
    ) {
        let value = match current {
            Ok(current) => Value {
                kind: Some(current.kind),
                hash: current.hash.clone(),
                link: self.link(local),
            },
            Err(_) => {
                self.complete = false;
                Value {
                    kind: None,
                    hash: "unreadable_input".into(),
                    link: None,
                }
            }
        };
        self.values.insert(key, value);
    }

    fn link(&mut self, local: &Path) -> Option<String> {
        match fs::symlink_metadata(local) {
            Ok(metadata) if metadata.is_symlink() => match fs::read_link(local) {
                Ok(target) => Some(
                    blake3::hash(target.as_os_str().as_encoded_bytes())
                        .to_hex()
                        .to_string(),
                ),
                Err(_) => {
                    self.issue("unreadable_link", local);
                    None
                }
            },
            Ok(_) => None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => {
                self.issue("unreadable_input", local);
                None
            }
        }
    }

    fn issue(&mut self, code: &str, local: &Path) {
        self.complete = false;
        self.reasons.push(Reason {
            code: code.into(),
            path: self.paths.portable(&local.to_string_lossy()).ok(),
        });
    }

    pub fn finish(mut self) -> Result<Revision> {
        self.check()?;
        for scope in self.scopes.clone() {
            let outputs = scope.outputs();
            let mut traversal_outputs = outputs.clone();
            traversal_outputs.extend(self.ignored_outputs.clone());
            let paths = fingerprint::observation_paths(
                std::slice::from_ref(&scope.path),
                &traversal_outputs,
                self.cancel,
            )
            .context("Cannot reconcile observed input scopes")?;
            for path in paths {
                self.observe_path(&path, &outputs)?;
            }
        }
        self.check()?;
        let mut hasher = blake3::Hasher::new();
        for (key, value) in &self.values {
            hasher.update(&serde_json::to_vec(&(
                &key.path,
                &key.excluded_children,
                value,
            ))?);
        }
        Ok(Revision {
            revision: hasher.finalize().to_hex().to_string(),
            complete: self.complete,
            reasons: self.reasons,
        })
    }

    fn unfollowed_directory_link(&self, local: &Path) -> bool {
        if self.scopes.iter().any(|scope| scope.path == local) {
            return false;
        }
        if !fs::symlink_metadata(local).is_ok_and(|metadata| metadata.is_symlink()) {
            return false;
        }
        local.is_dir()
    }

    fn observe_path(&mut self, local: &Path, outputs: &[PathBuf]) -> Result<()> {
        self.check()?;
        let path = self.paths.portable(&local.to_string_lossy())?;
        let mut excluded_children = fingerprint::excluded_children(local, outputs);
        excluded_children.extend(super::super::refresh_inputs::direct_children(
            local,
            &self.ignored_outputs,
        ));
        excluded_children.sort();
        excluded_children.dedup();
        let key = Key {
            path,
            excluded_children,
        };
        if self.unfollowed_directory_link(local) {
            self.issue("unfollowed_directory_link", local);
        }
        if !self.values.contains_key(&key) {
            let current = self.observe(local, &key.excluded_children);
            if current.is_err() {
                self.issue("unreadable_input", local);
            }
            self.insert(key, local, current.as_ref());
        }
        Ok(())
    }
}
