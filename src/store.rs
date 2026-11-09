use crate::{
    graph::Filter,
    model::{Edge, GraphData, Node},
};
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

/// Text and pagination for a symbol search within one graph generation.
pub struct SymbolSearch<'a> {
    pub prefix: &'a str,
    pub filter: &'a Filter,
    pub limit: usize,
    pub offset: usize,
}

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(0);

const SCHEMA: &str = "
CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE nodes (
 idx INTEGER PRIMARY KEY, id TEXT NOT NULL UNIQUE, project TEXT NOT NULL, build TEXT NOT NULL,
 language TEXT NOT NULL, kind TEXT NOT NULL, label TEXT NOT NULL, signature TEXT NOT NULL,
 namespace TEXT NOT NULL, container TEXT NOT NULL, file TEXT NOT NULL, line INTEGER NOT NULL,
 is_test INTEGER NOT NULL, is_generated INTEGER NOT NULL, body_unavailable INTEGER NOT NULL
);
CREATE TABLE edges (idx INTEGER PRIMARY KEY, from_idx INTEGER NOT NULL, to_idx INTEGER NOT NULL,
 kind TEXT NOT NULL, certainty TEXT NOT NULL, count INTEGER NOT NULL,
 FOREIGN KEY(from_idx) REFERENCES nodes(idx), FOREIGN KEY(to_idx) REFERENCES nodes(idx));
CREATE INDEX edges_from ON edges(from_idx);
CREATE INDEX edges_to ON edges(to_idx);
CREATE VIRTUAL TABLE node_search USING fts5(label, signature, namespace, container, file,
 tokenize='unicode61', prefix='2 3 4');
";

pub struct SqliteStore {
    root: PathBuf,
    writer: Mutex<()>,
}

pub type SQLiteStore = SqliteStore;

impl SqliteStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        fs::create_dir_all(root.as_ref())?;
        let store = Self {
            root: root.as_ref().to_owned(),
            writer: Mutex::new(()),
        };
        let connection = store.catalog()?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS catalog (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        Ok(store)
    }

    fn catalog(&self) -> Result<Connection> {
        let connection = Connection::open(self.root.join("catalog.sqlite3"))?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(connection)
    }

    pub fn active_generation(&self) -> Result<Option<String>> {
        let name: Option<String> = self
            .catalog()?
            .query_row("SELECT value FROM catalog WHERE key='active'", [], |row| {
                row.get(0)
            })
            .optional()?;
        if let Some(name) = &name {
            self.generation_path(name)?;
        }
        Ok(name)
    }

    fn generation_path(&self, name: &str) -> Result<PathBuf> {
        ensure!(
            name.starts_with("generation-")
                && name.ends_with(".sqlite3")
                && name
                    .chars()
                    .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '.'),
            "Invalid generation identity"
        );
        Ok(self.root.join(name))
    }

    /// Cancellation is honored until the catalog publication transaction commits.
    pub fn publish(&self, graph: &GraphData, cancelled: &AtomicBool) -> Result<String> {
        let _writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("Graph writer lock poisoned"))?;
        check_cancelled(cancelled)?;
        graph.validate()?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let sequence = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "generation-{stamp}-{}-{sequence}.sqlite3",
            std::process::id()
        );
        let path = self.root.join(&name);
        if let Err(error) = self.write_generation(&path, graph, cancelled) {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        // Retain a fully written file after a catalog error: an I/O failure at
        // commit can leave its outcome uncertain, and deleting could break an
        // already published pointer. Unreferenced generations are harmless.
        self.activate(&name, cancelled)?;
        Ok(name)
    }

    fn write_generation(
        &self,
        path: &Path,
        graph: &GraphData,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        let mut connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(SCHEMA)?;
        write_nodes(&transaction, graph, cancelled)?;
        write_edges(&transaction, graph, cancelled)?;
        let metadata = serde_json::json!({ "diagnostics": graph.diagnostics, "roots": graph.roots,
            "fingerprints": graph.fingerprints, "engine_versions": graph.engine_versions,
            "entry_points": graph.entry_points, "architecture": graph.architecture, "flow_inventory": graph.flow_inventory, "method_policy": crate::method_policy::VERSION,
            "edge_provenance": graph.edges.iter().map(|edge| &edge.provenance).collect::<Vec<_>>() });
        transaction.execute(
            "INSERT INTO metadata VALUES ('graph', ?1)",
            [metadata.to_string()],
        )?;
        transaction.execute(
            "INSERT INTO metadata VALUES ('method_policy', ?1)",
            [crate::method_policy::VERSION.to_string()],
        )?;
        check_cancelled(cancelled)?;
        transaction.commit()?;
        connection.close().map_err(|(_, error)| error)?;
        fs::File::open(path)?.sync_all()?;
        Ok(())
    }

    fn activate(&self, name: &str, cancelled: &AtomicBool) -> Result<()> {
        let mut catalog = self.catalog()?;
        let transaction = catalog.transaction()?;
        transaction.execute(
            "INSERT INTO catalog VALUES ('active', ?1)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [name],
        )?;
        check_cancelled(cancelled)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_active(&self) -> Result<Option<GraphData>> {
        let Some(generation) = self.active_generation()? else {
            return Ok(None);
        };
        self.load_generation(&generation).map(Some)
    }

    pub fn load_generation(&self, generation: &str) -> Result<GraphData> {
        let (connection, metadata) = self.read_generation(generation)?;
        let mut graph = GraphData {
            diagnostics: serde_json::from_value(metadata["diagnostics"].clone())?,
            roots: serde_json::from_value(metadata["roots"].clone())?,
            fingerprints: serde_json::from_value(metadata["fingerprints"].clone())?,
            engine_versions: serde_json::from_value(metadata["engine_versions"].clone())?,
            entry_points: serde_json::from_value(metadata["entry_points"].clone())?,
            architecture: serde_json::from_value(metadata["architecture"].clone())?,
            flow_inventory: serde_json::from_value(metadata["flow_inventory"].clone())?,
            ..Default::default()
        };
        graph.nodes = connection
            .prepare("SELECT * FROM nodes ORDER BY idx")?
            .query_map([], read_node)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        graph.edges = connection
            .prepare("SELECT from_idx,to_idx,kind,certainty,count FROM edges ORDER BY idx")?
            .query_map([], |row| {
                Ok(Edge {
                    provenance: String::new(),
                    from: row.get(0)?,
                    to: row.get(1)?,
                    kind: row.get(2)?,
                    certainty: row.get(3)?,
                    count: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if let Some(values) = metadata.get("edge_provenance") {
            let provenance: Vec<String> = serde_json::from_value(values.clone())?;
            ensure!(
                provenance.len() == graph.edges.len(),
                "Edge provenance count differs from graph"
            );
            for (edge, value) in graph.edges.iter_mut().zip(provenance) {
                edge.provenance = value;
            }
        }
        graph.validate()?;
        Ok(graph)
    }

    /// All generation readers enforce the same persisted graph policy.
    fn read_generation(&self, generation: &str) -> Result<(Connection, Value)> {
        let connection = self.open_generation(generation)?;
        let metadata: String =
            connection.query_row("SELECT value FROM metadata WHERE key='graph'", [], |row| {
                row.get(0)
            })?;
        let metadata: Value = serde_json::from_str(&metadata)?;
        ensure!(
            metadata["method_policy"].as_u64() == Some(crate::method_policy::VERSION.into()),
            "Refresh this workspace: its cached method graph uses an older policy"
        );
        Ok((connection, metadata))
    }

    fn open_generation(&self, generation: &str) -> Result<Connection> {
        let path = self.generation_path(generation)?;
        let connection =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let policy: Option<String> = connection
            .query_row(
                "SELECT value FROM metadata WHERE key='method_policy'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let policy = match policy {
            Some(value) => {
                crate::metrics::count("storage.policy_scalar_reads", 1);
                value.parse::<u32>().ok()
            }
            None => {
                crate::metrics::count("storage.policy_legacy_reads", 1);
                #[derive(serde::Deserialize)]
                struct LegacyPolicy {
                    method_policy: Option<u32>,
                }
                let metadata: String = connection.query_row(
                    "SELECT value FROM metadata WHERE key='graph'",
                    [],
                    |row| row.get(0),
                )?;
                serde_json::from_str::<LegacyPolicy>(&metadata)?.method_policy
            }
        };
        ensure!(
            policy == Some(crate::method_policy::VERSION),
            "Refresh this workspace: its cached method graph uses an older policy"
        );
        Ok(connection)
    }

    pub fn search(&self, request: &SymbolSearch<'_>) -> Result<Vec<u32>> {
        let Some(generation) = self.active_generation()? else {
            return Ok(Vec::new());
        };
        self.search_generation(&generation, request)
    }

    pub fn search_generation(
        &self,
        generation: &str,
        request: &SymbolSearch<'_>,
    ) -> Result<Vec<u32>> {
        let _stage = crate::metrics::stage("storage.search");
        let SymbolSearch {
            prefix,
            filter,
            limit,
            offset,
        } = request;
        let Some(query) = prefix_query(prefix) else {
            return Ok(Vec::new());
        };
        let connection = self.open_generation(generation)?;
        let mut statement = connection.prepare(
            "SELECT n.idx FROM node_search JOIN nodes n ON n.idx=node_search.rowid-1
            WHERE node_search MATCH ?1 AND (?2 OR NOT n.is_test) AND (?3 OR NOT n.is_generated)
            AND (?4 OR NOT n.body_unavailable) ORDER BY bm25(node_search),n.idx LIMIT ?5 OFFSET ?6",
        )?;
        Ok(statement
            .query_map(
                params![
                    query,
                    filter.include_tests,
                    filter.include_generated,
                    filter.include_unavailable,
                    (*limit).clamp(1, 1000) as i64,
                    (*offset).min(i64::MAX as usize) as i64
                ],
                |row| row.get(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_session(&self, session: &Value) -> Result<()> {
        self.catalog()?.execute(
            "INSERT INTO catalog VALUES ('session', ?1)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [session.to_string()],
        )?;
        Ok(())
    }

    pub fn load_session(&self) -> Result<Option<Value>> {
        let value: Option<String> = self
            .catalog()?
            .query_row("SELECT value FROM catalog WHERE key='session'", [], |row| {
                row.get(0)
            })
            .optional()?;
        value
            .map(|json| serde_json::from_str(&json).context("Invalid stored navigation session"))
            .transpose()
    }
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Acquire) {
        bail!("Analysis cancelled before publication");
    }
    Ok(())
}

fn write_nodes(
    transaction: &rusqlite::Transaction<'_>,
    graph: &GraphData,
    cancelled: &AtomicBool,
) -> Result<()> {
    let mut nodes = transaction
        .prepare("INSERT INTO nodes VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)")?;
    let mut search = transaction.prepare("INSERT INTO node_search(rowid,label,signature,namespace,container,file) VALUES (?1,?2,?3,?4,?5,?6)")?;
    for (index, node) in graph.nodes.iter().enumerate() {
        if index % 1024 == 0 {
            check_cancelled(cancelled)?;
        }
        nodes.execute(params![
            index as i64,
            node.id,
            node.project,
            node.build,
            node.language,
            node.kind,
            node.label,
            node.signature,
            node.namespace,
            node.container,
            node.file,
            node.line,
            node.is_test,
            node.is_generated,
            node.body_unavailable
        ])?;
        search.execute(params![
            index as i64 + 1,
            node.label,
            node.signature,
            node.namespace,
            node.container,
            node.file
        ])?;
    }
    Ok(())
}

fn write_edges(
    transaction: &rusqlite::Transaction<'_>,
    graph: &GraphData,
    cancelled: &AtomicBool,
) -> Result<()> {
    let mut statement = transaction.prepare("INSERT INTO edges VALUES (?1,?2,?3,?4,?5,?6)")?;
    for (index, edge) in graph.edges.iter().enumerate() {
        if index % 4096 == 0 {
            check_cancelled(cancelled)?;
        }
        statement.execute(params![
            index as i64,
            edge.from,
            edge.to,
            edge.kind,
            edge.certainty,
            edge.count
        ])?;
    }
    Ok(())
}

fn read_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<Node> {
    Ok(Node {
        id: row.get(1)?,
        project: row.get(2)?,
        build: row.get(3)?,
        language: row.get(4)?,
        kind: row.get(5)?,
        label: row.get(6)?,
        signature: row.get(7)?,
        namespace: row.get(8)?,
        container: row.get(9)?,
        file: row.get(10)?,
        line: row.get(11)?,
        is_test: row.get(12)?,
        is_generated: row.get(13)?,
        body_unavailable: row.get(14)?,
    })
}

fn prefix_query(prefix: &str) -> Option<String> {
    let tokens: Vec<String> = prefix
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .take(8)
        .map(|token| format!("\"{token}\"*"))
        .collect();
    (!tokens.is_empty()).then(|| tokens.join(" AND "))
}
