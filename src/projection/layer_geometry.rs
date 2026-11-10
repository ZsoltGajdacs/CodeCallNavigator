//! Graphviz constraints for architecture bands and independent flow columns.
use super::GraphView;
use std::collections::HashMap;

struct Column {
    bands: Vec<usize>,
    support: bool,
}

pub(super) fn constraints(view: &GraphView, source: &mut String) {
    source.push_str("graph [newrank=true,compound=true];\n");
    let columns = columns(view);
    let mut starts = Vec::new();
    for (index, column) in columns.iter().enumerate() {
        let representatives = column_constraints(view, column, index, source);
        if let Some(&first) = representatives.first() {
            let beside = column
                .bands
                .first()
                .and_then(|&band| view.layer_bands[band].beside.as_deref());
            if let Some(target) = beside.and_then(|name| first_in_band(view, name)) {
                same_rank(source, &[target, first]);
            } else {
                starts.push(first);
            }
        }
    }
    same_rank(source, &starts);
    trailing_constraints(view, source);
}

fn column_constraints(
    view: &GraphView,
    column: &Column,
    index: usize,
    source: &mut String,
) -> Vec<usize> {
    source.push_str(&format!(
        "subgraph cluster_flow_{index} {{ label=\"\"; color=\"transparent\"; margin=24;\n"
    ));
    let mut representatives = Vec::new();
    for &band in &column.bands {
        let members = members(view, &view.layer_bands[band].name);
        for member in &members {
            source.push_str(&format!("n{member} [group=\"flow_{index}\"];\n"));
        }
        if column.support {
            representatives.extend(members);
        } else {
            same_rank(source, &members);
            if let Some(&first) = members.first() {
                representatives.push(first);
            }
        }
    }
    for pair in representatives.windows(2) {
        source.push_str(&format!(
            "n{} -> n{} [style=invis,weight=100,minlen=2];\n",
            pair[0], pair[1]
        ));
    }
    source.push_str("}\n");
    representatives
}

fn members(view: &GraphView, name: &str) -> Vec<usize> {
    view.nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (node.layer.as_deref() == Some(name)).then_some(index))
        .collect()
}

fn first_in_band(view: &GraphView, name: &str) -> Option<usize> {
    view.nodes
        .iter()
        .position(|node| node.layer.as_deref() == Some(name))
}

fn same_rank(source: &mut String, members: &[usize]) {
    if members.is_empty() {
        return;
    }
    source.push_str("{rank=same; ");
    for member in members {
        source.push_str(&format!("n{member}; "));
    }
    source.push_str("}\n");
    for pair in members.windows(2) {
        source.push_str(&format!(
            "n{} -> n{} [style=invis,weight=1000];\n",
            pair[0], pair[1]
        ));
    }
}

fn columns(view: &GraphView) -> Vec<Column> {
    let mut columns = vec![Column {
        bands: Vec::new(),
        support: false,
    }];
    let mut flows = HashMap::new();
    for (index, band) in view.layer_bands.iter().enumerate() {
        let column = if band.support {
            columns.push(Column {
                bands: Vec::new(),
                support: true,
            });
            columns.len() - 1
        } else if let Some(flow) = &band.flow {
            *flows.entry(flow.clone()).or_insert_with(|| {
                columns.push(Column {
                    bands: Vec::new(),
                    support: false,
                });
                columns.len() - 1
            })
        } else {
            0
        };
        columns[column].bands.push(index);
    }
    columns
}

fn trailing_constraints(view: &GraphView, source: &mut String) {
    let Some(trailing) = first_in_band(view, "Tests").or_else(|| first_in_band(view, "Other"))
    else {
        return;
    };
    let mut ends = std::collections::BTreeMap::new();
    for band in view
        .layer_bands
        .iter()
        .filter(|band| !band.support && !matches!(band.name.as_str(), "Tests" | "Other"))
    {
        if let Some(first) = first_in_band(view, &band.name) {
            ends.insert(band.flow.clone(), first);
        }
    }
    for end in ends.values() {
        source.push_str(&format!(
            "n{end} -> n{trailing} [style=invis,weight=100,minlen=2];\n"
        ));
    }
}
