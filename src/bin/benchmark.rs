//! Reproducible storage/query scale fixture; it performs no source-code analysis.
use anyhow::Result;
use codecallnavigator::{
    graph::{Direction, Filter, GraphIndex},
    model::{Edge, GraphData, Node},
    store::{SqliteStore, SymbolSearch},
};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};

fn fixture(count: usize) -> GraphData {
    let nodes = (0..count)
        .map(|i| Node {
            id: format!("scale/project{}:Method{i:06}", i % 8),
            project: format!("Project{}", i % 8),
            build: "synthetic-scale-fixture".into(),
            language: "fixture".into(),
            kind: "method".into(),
            label: format!("Method{i:06}"),
            signature: format!("Namespace{}.Type{}.Method{i:06}()", i % 32, i % 1000),
            namespace: format!("Namespace{}", i % 32),
            container: format!("Type{}", i % 1000),
            file: format!("src/group{}/Type{}.cs", i % 32, i % 1000),
            line: (i % 500 + 1) as u32,
            ..Default::default()
        })
        .collect();
    let mut edges = Vec::with_capacity(count * 8);
    for i in 0..count {
        for hop in 1..=8 {
            edges.push(Edge {
                evidence: Vec::new(),
                provenance: String::new(),
                from: i as u32,
                to: ((i + hop) % count) as u32,
                kind: "call".into(),
                certainty: "direct".into(),
                count: 1,
            });
        }
    }
    GraphData {
        nodes,
        edges,
        engine_versions: "Synthetic benchmark fixture; no analyzer coverage claim".into(),
        ..Default::default()
    }
}

fn percentile(values: &mut [f64], fraction: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    values[((values.len() as f64 * fraction).ceil() as usize)
        .saturating_sub(1)
        .min(values.len() - 1)]
}

fn measure_queries(store: &SqliteStore, graph: &GraphIndex) -> Result<serde_json::Value> {
    let filter = Filter::default();
    let mut neighbors = Vec::new();
    let mut searches = Vec::new();
    for i in 0..100 {
        let index = (i * 2347) % graph.data.nodes.len();
        let start = Instant::now();
        assert_eq!(
            graph
                .neighbors(index as u32, Direction::Both, &filter)
                .len(),
            16
        );
        neighbors.push(start.elapsed().as_secs_f64() * 1000.0);
        let prefix = if i % 10 == 0 {
            "Method".into()
        } else {
            format!("Method{:03}", index / 1000)
        };
        let start = Instant::now();
        assert!(
            !store
                .search(&SymbolSearch {
                    prefix: &prefix,
                    filter: &filter,
                    limit: 50,
                    offset: 0
                })?
                .is_empty()
        );
        searches.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let start = Instant::now();
    let path = graph
        .shortest_path(0, (graph.data.nodes.len() - 1) as u32, &filter)
        .expect("Connected scale graph");
    let path_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({ "neighborsP95Ms": percentile(&mut neighbors, 0.95),
        "searchP95Ms": percentile(&mut searches, 0.95), "searchMaxMs": percentile(&mut searches, 1.0),
        "longPathMs": path_ms, "longPathNodes": path.len(), "queriesPerKind": 100,
        "searchWorkload": "90 specific method prefixes and 10 prefixes matching all methods" }))
}

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let directory = arguments
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("codecallnavigator-scale"));
    let count = arguments
        .get(1)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(250_000);
    anyhow::ensure!(
        count >= 1000 && count < u32::MAX as usize,
        "Node count must be between1000 andu32MAX"
    );
    let graph = fixture(count);
    let store = SqliteStore::open(&directory)?;
    let start = Instant::now();
    store.publish(&graph, &AtomicBool::new(false))?;
    let publish_seconds = start.elapsed().as_secs_f64();
    drop(graph);
    let start = Instant::now();
    let loaded = store.load_active()?.expect("Published graph");
    let graph = GraphIndex::new(Arc::new(loaded))?;
    let cached_open_seconds = start.elapsed().as_secs_f64();
    let queries = measure_queries(&store, &graph)?;
    let passed = cached_open_seconds < 5.0
        && queries["searchP95Ms"].as_f64().unwrap() < 200.0
        && queries["neighborsP95Ms"].as_f64().unwrap() < 200.0;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({ "nodes": count, "edges": count * 8,
        "publishSeconds": publish_seconds, "cachedOpenSeconds": cached_open_seconds, "queries": queries,
        "targetsPassed": passed, "cacheDirectory": directory, "build": if cfg!(debug_assertions) {"debug"} else {"release"} }))?
    );
    anyhow::ensure!(
        passed,
        "Performance acceptance target missed; see measurements above"
    );
    Ok(())
}
