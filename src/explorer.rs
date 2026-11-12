//! Lazy desktop filesystem workspace, independent of graph navigation and rendering.
mod loader;
mod selection;
use selection::SelectionToken;
#[cfg(test)]
use selection::selected_files;
#[cfg(test)]
mod tests;

use crate::app::State;
use loader::{Entry, Kind};
use rune::{Latest, Tx};
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub const ROW_HEIGHT: f32 = 26.0;

#[derive(Clone, Debug)]
pub enum RowKind {
    Directory,
    File,
    Symlink,
    Loading,
    Empty,
    Error,
}
#[derive(Clone, Debug)]
pub struct Row {
    pub path: String,
    pub label: String,
    pub depth: usize,
    pub kind: RowKind,
}
impl Row {
    fn note(path: &str, depth: usize, kind: RowKind, label: String) -> Self {
        Self {
            path: path.into(),
            depth,
            kind,
            label,
        }
    }

    pub fn is_file(&self) -> bool {
        matches!(self.kind, RowKind::File | RowKind::Symlink)
    }
    pub fn key(&self) -> String {
        format!("explorer-row-{}-{}", self.path, self.kind_tag())
    }
    fn kind_tag(&self) -> &str {
        match self.kind {
            RowKind::Directory | RowKind::File | RowKind::Symlink => "entry",
            RowKind::Loading => "loading",
            RowKind::Empty => "empty",
            RowKind::Error => "error",
        }
    }
}

enum Directory {
    Loading,
    Ready(Vec<Entry>),
    Failed(String),
}
struct Output {
    generation: u64,
    path: String,
    canonical_root: Option<String>,
    result: Result<Vec<Entry>, String>,
}

pub struct Workspace {
    pub root: String,
    canonical_root: Option<String>,
    pub rows: Vec<Row>,
    pub expanded: BTreeSet<String>,
    pub selected: BTreeSet<String>,
    pub serial: u64,
    directories: HashMap<String, Directory>,
    queue: VecDeque<String>,
    job: Latest<Output>,
    cancelled: Arc<AtomicBool>,
    generation: u64,
    observed_root: String,
    root_deadline: Option<f64>,
    selection_token: Option<SelectionToken>,
    resolved_files: BTreeSet<String>,
    reveal: bool,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            root: String::new(),
            canonical_root: None,
            rows: Vec::new(),
            expanded: BTreeSet::new(),
            selected: BTreeSet::new(),
            serial: 0,
            directories: HashMap::new(),
            queue: VecDeque::new(),
            job: Latest::default(),
            cancelled: Arc::new(AtomicBool::new(false)),
            generation: 0,
            observed_root: String::new(),
            root_deadline: None,
            selection_token: None,
            resolved_files: BTreeSet::new(),
            reveal: false,
        }
    }
}
impl Workspace {
    /// Translate the selected-root alias to the analyzer's canonical root without I/O.
    pub(crate) fn graph_path(&self, logical: &str) -> String {
        match &self.canonical_root {
            Some(canonical) => replace_prefix(logical, &self.root, canonical),
            None => logical.into(),
        }
    }
    fn logical_path(&self, graph: &str) -> String {
        if loader::contains(&self.root, graph) {
            return graph.into();
        }
        match &self.canonical_root {
            Some(canonical) => replace_prefix(graph, canonical, &self.root),
            None => graph.into(),
        }
    }
    pub(crate) fn graph_selected(&mut self) {
        self.selection_token = None;
    }
    fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.job.cancel();
        self.cancelled = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        self.queue.clear();
    }
    fn queue_directory(&mut self, path: String) {
        if self.directories.contains_key(&path) {
            return;
        }
        self.directories.insert(path.clone(), Directory::Loading);
        self.queue.push_back(path);
    }
    pub fn refresh(&mut self) {
        self.cancel();
        self.directories.clear();
        if !self.root.is_empty() {
            self.queue_directory(self.root.clone());
        }
        self.reveal = true;
        self.rebuild();
    }
    pub fn toggle(&mut self, path: String) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }
        self.rebuild();
    }
    pub fn select(&mut self, path: String) {
        self.selected = BTreeSet::from([path]);
        self.serial += 1;
    }
    pub fn retry(&mut self, path: String) {
        self.directories.remove(&path);
        self.queue_directory(path);
        self.rebuild();
    }
    fn rebuild(&mut self) {
        let mut projection = TreeRows::default();
        if !self.root.is_empty() {
            self.append_directory(&self.root, 0, &mut projection);
        }
        for path in projection.missing {
            self.queue_directory(path);
        }
        self.rows = projection.visible;
        self.serial += 1;
    }
    fn append_directory(&self, path: &str, depth: usize, rows: &mut TreeRows) {
        match self.directories.get(path) {
            Some(Directory::Ready(entries)) if entries.is_empty() => rows.visible.push(Row::note(
                path,
                depth,
                RowKind::Empty,
                "Empty folder".into(),
            )),
            Some(Directory::Ready(entries)) => self.append_entries(entries, depth, rows),
            Some(Directory::Failed(error)) => rows.visible.push(Row::note(
                path,
                depth,
                RowKind::Error,
                format!("{error} · Retry"),
            )),
            status => {
                if status.is_none() {
                    rows.missing.push(path.into());
                }
                rows.visible
                    .push(Row::note(path, depth, RowKind::Loading, "Loading…".into()));
            }
        }
    }
    fn append_entries(&self, entries: &[Entry], depth: usize, rows: &mut TreeRows) {
        for entry in entries {
            rows.visible.push(Row {
                path: entry.path.clone(),
                label: entry.label.clone(),
                depth,
                kind: match entry.kind {
                    Kind::Directory => RowKind::Directory,
                    Kind::File => RowKind::File,
                    Kind::Symlink => RowKind::Symlink,
                },
            });
            if entry.kind == Kind::Directory && self.expanded.contains(&entry.path) {
                self.append_directory(&entry.path, depth + 1, rows);
            }
        }
    }
    fn highlight(&mut self, files: BTreeSet<String>) {
        self.selected = files
            .into_iter()
            .map(|file| self.logical_path(&file))
            .filter(|file| loader::contains(&self.root, file))
            .collect();
        for file in &self.selected {
            let mut parent = Path::new(file).parent();
            while let Some(path) = parent {
                let path = path.to_string_lossy().into_owned();
                if path == self.root || !loader::contains(&self.root, &path) {
                    break;
                }
                self.expanded.insert(path);
                parent = parent.and_then(Path::parent);
            }
        }
        self.reveal = true;
        self.rebuild();
    }
    fn accept(&mut self, output: Output) {
        if output.generation != self.generation {
            return;
        }
        if output.path == self.root && self.canonical_root != output.canonical_root {
            self.canonical_root = output.canonical_root;
            self.graph_selected();
        }
        match output.result {
            Ok(entries) => {
                self.prune_missing(&output.path, &entries);
                self.directories
                    .insert(output.path, Directory::Ready(entries));
            }
            Err(error) => {
                self.directories
                    .insert(output.path, Directory::Failed(error));
            }
        }
        self.rebuild();
    }
    fn prune_missing(&mut self, directory: &str, entries: &[Entry]) {
        let present: BTreeSet<_> = entries.iter().map(|entry| entry.path.as_str()).collect();
        let retained = |path: &String| {
            let Ok(relative) = Path::new(path).strip_prefix(directory) else {
                return true;
            };
            let Some(first) = relative.components().next() else {
                return true;
            };
            let child = Path::new(directory).join(first);
            present.contains(child.to_string_lossy().as_ref())
        };
        self.expanded.retain(retained);
        self.selected.retain(retained);
    }
}

pub fn poll(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    let root = if state.scene && state.root.text.ends_with(" · no project scanned") {
        ""
    } else {
        state.root.text.trim()
    };
    let mut changed = state.explorer.observe_root(root, time);
    changed |= state.explorer.poll_root(time, tx);
    changed |= selection::sync(state);
    if let Some(output) = state.explorer.job.take() {
        state.explorer.accept(output);
        changed = true;
    }
    start_next(state);
    if state.ui.sidebar.is_open() {
        state.explorer.reveal_selection(tx);
    }
    changed
}
impl Workspace {
    fn observe_root(&mut self, root: &str, time: f64) -> bool {
        if root == self.observed_root {
            return false;
        }
        self.cancel();
        self.observed_root = root.into();
        self.root.clear();
        self.canonical_root = None;
        self.directories.clear();
        self.rows.clear();
        self.expanded.clear();
        self.selected.clear();
        self.selection_token = None;
        self.root_deadline = (!root.is_empty()).then_some(time + 0.15);
        self.serial += 1;
        true
    }
    fn poll_root(&mut self, time: f64, tx: &mut Tx) -> bool {
        let Some(deadline) = self.root_deadline else {
            return false;
        };
        if time < deadline {
            tx.wake_after(Duration::from_secs_f64(deadline - time));
            return false;
        }
        self.root_deadline = None;
        self.root = crate::source_path::identity(&self.observed_root);
        self.refresh();
        true
    }
    fn path_pending(&self, file: &str) -> bool {
        let Ok(relative) = Path::new(file).strip_prefix(&self.root) else {
            return false;
        };
        let mut directory = Path::new(&self.root).to_path_buf();
        for component in relative.components() {
            let Some(Directory::Ready(entries)) =
                self.directories.get(directory.to_string_lossy().as_ref())
            else {
                return !matches!(
                    self.directories.get(directory.to_string_lossy().as_ref()),
                    Some(Directory::Failed(_))
                );
            };
            directory.push(component);
            let Some(entry) = entries
                .iter()
                .find(|entry| Path::new(&entry.path) == directory)
            else {
                return false;
            };
            if entry.kind != Kind::Directory {
                return false;
            }
        }
        false
    }
    fn reveal_selection(&mut self, tx: &mut Tx) {
        if !self.reveal || self.selected.iter().any(|path| self.path_pending(path)) {
            return;
        }
        let Some(row) = self
            .rows
            .iter()
            .find(|row| self.selected.contains(&row.path) && row.is_file())
        else {
            return;
        };
        tx.scroll_into_view_in("explorer-scroll", row.key());
        self.reveal = false;
    }
}
#[derive(Default)]
struct TreeRows {
    visible: Vec<Row>,
    missing: Vec<String>,
}

fn start_next(state: &mut State) {
    if state.explorer.job.is_pending() {
        return;
    }
    let Some(path) = state.explorer.queue.pop_front() else {
        return;
    };
    if let Some(waker) = &state.jobs.waker {
        state.explorer.job.set_waker(waker.clone());
    }
    let generation = state.explorer.generation;
    let cancelled = state.explorer.cancelled.clone();
    let root_request = path == state.explorer.root;
    state.explorer.job.request(path, move |path| Output {
        generation,
        canonical_root: root_request
            .then(|| std::fs::canonicalize(&path).ok())
            .flatten()
            .map(|root| crate::source_path::identity(&root.to_string_lossy())),
        result: loader::read(&path, &cancelled),
        path,
    });
}

fn replace_prefix(path: &str, from: &str, to: &str) -> String {
    match Path::new(path).strip_prefix(from) {
        Ok(relative) => Path::new(to).join(relative).to_string_lossy().into_owned(),
        Err(_) => path.into(),
    }
}
