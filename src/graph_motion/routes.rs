//! Route presence is keyed by endpoints and classification, never layout-local indices.
use super::Spring;
use crate::projection::{GraphView, ViewEdge};
use codecallnavigator::model::RelationshipClassification;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key(String, String, RelationshipClassification, usize);

#[derive(Clone)]
pub(super) struct Captured {
    key: Key,
    pub edge: ViewEdge,
    anchors: [[f32; 4]; 2],
}

pub(super) fn capture(view: &GraphView) -> Vec<Captured> {
    let mut counts = HashMap::new();
    view.edges
        .iter()
        .map(|edge| {
            let base = (
                view.nodes[edge.from].key.clone(),
                view.nodes[edge.to].key.clone(),
                edge.classification,
            );
            let occurrence = counts.entry(base.clone()).or_insert(0);
            let key = Key(base.0, base.1, base.2, *occurrence);
            *occurrence += 1;
            Captured {
                key,
                edge: edge.clone(),
                anchors: [rect(&view.nodes[edge.from]), rect(&view.nodes[edge.to])],
            }
        })
        .collect()
}

fn rect(node: &crate::projection::ViewNode) -> [f32; 4] {
    [
        node.position[0],
        node.position[1],
        node.size[0],
        node.size[1],
    ]
}
struct Route {
    captured: Captured,
    opacity: Spring,
    points: Vec<[f32; 2]>,
}

#[derive(Default)]
pub(super) struct Routes {
    incoming: Vec<Route>,
    outgoing: Vec<Route>,
}

fn incoming_opacity(
    key: &Key,
    departing: &HashSet<Key>,
    existed: &HashSet<Key>,
    previous: &HashMap<Key, Spring>,
) -> Spring {
    if departing.contains(key) {
        return Spring::new(0.0, 1.0);
    }
    previous
        .get(key)
        .copied()
        .unwrap_or_else(|| Spring::new(if existed.contains(key) { 1.0 } else { 0.0 }, 1.0))
}

impl Routes {
    pub fn begin(&mut self, before: Vec<Captured>, view: &GraphView) {
        let old: HashMap<_, _> = self
            .incoming
            .drain(..)
            .map(|route| (route.captured.key.clone(), route.opacity))
            .collect();
        let existed: HashSet<_> = before.iter().map(|route| route.key.clone()).collect();
        let targets = capture(view);
        // Keep partially visible historical routes until they finish fading.
        let unchanged: HashMap<_, _> = targets.iter().map(|n| (&n.key, &n.edge.points)).collect();
        let mut outgoing: Vec<_> = self
            .outgoing
            .drain(..)
            .filter(|r| r.opacity.value > 0.001)
            .collect();
        for captured in before {
            let same = unchanged
                .get(&captured.key)
                .is_some_and(|points| **points == captured.edge.points);
            if !same {
                let mut opacity = old
                    .get(&captured.key)
                    .copied()
                    .unwrap_or(Spring::new(1.0, 0.0));
                opacity.target = 0.0;
                outgoing.push(Route {
                    points: captured.edge.points.clone(),
                    captured,
                    opacity,
                });
            }
        }
        let departing: HashSet<_> = outgoing
            .iter()
            .map(|old| old.captured.key.clone())
            .collect();
        self.incoming = targets
            .into_iter()
            .map(|captured| {
                let opacity = incoming_opacity(&captured.key, &departing, &existed, &old);
                Route {
                    points: captured.edge.points.clone(),
                    captured,
                    opacity,
                }
            })
            .collect();
        // Bound retained presentation data even under continuous adversarial navigation.
        outgoing.sort_by(|a, b| b.opacity.value.total_cmp(&a.opacity.value));
        outgoing.truncate(6000);
        self.outgoing = outgoing;
    }
    pub fn apply(&mut self, view: &GraphView) {
        let rectangles: HashMap<_, _> = view
            .nodes
            .iter()
            .map(|n| (n.key.as_str(), rect(n)))
            .collect();
        for route in &mut self.outgoing {
            let from = rectangles
                .get(route.captured.key.0.as_str())
                .copied()
                .unwrap_or(route.captured.anchors[0]);
            let to = rectangles
                .get(route.captured.key.1.as_str())
                .copied()
                .unwrap_or(route.captured.anchors[1]);
            let Some(first) = route.points.first().copied() else {
                continue;
            };
            let Some(last) = route.points.last().copied() else {
                continue;
            };
            let delta = |anchor: [f32; 4], now: [f32; 4], port: [f32; 2]| -> [f32; 2] {
                std::array::from_fn(|axis| {
                    now[axis] - anchor[axis]
                        + (port[axis] - anchor[axis]) / anchor[axis + 2].max(1.0)
                            * (now[axis + 2] - anchor[axis + 2])
                })
            };
            let a = delta(route.captured.anchors[0], from, first);
            let b = delta(route.captured.anchors[1], to, last);
            let count = route.points.len().saturating_sub(1).max(1) as f32;
            for (i, (point, base)) in route
                .captured
                .edge
                .points
                .iter_mut()
                .zip(&route.points)
                .enumerate()
            {
                let t = i as f32 / count;
                for axis in 0..2 {
                    point[axis] = base[axis] + (1.0 - t) * a[axis] + t * b[axis];
                }
            }
        }
    }
    pub fn advance(&mut self, dt: f32) -> bool {
        self.incoming
            .iter_mut()
            .chain(&mut self.outgoing)
            .fold(false, |active, r| r.opacity.step(dt) || active)
    }
    pub fn opacity(&self, index: usize) -> f32 {
        self.incoming.get(index).map_or(1.0, |r| r.opacity.value)
    }
    pub fn outgoing(&self) -> impl Iterator<Item = (&ViewEdge, f32)> {
        self.outgoing
            .iter()
            .map(|r| (&r.captured.edge, r.opacity.value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::ViewNode;
    fn graph(reverse: bool, shift: f32) -> GraphView {
        let nodes = vec![
            ViewNode {
                key: "a".into(),
                position: [shift, 0.0],
                size: [100.0, 60.0],
                ..Default::default()
            },
            ViewNode {
                key: "b".into(),
                position: [shift, 200.0],
                size: [100.0, 60.0],
                ..Default::default()
            },
        ];
        let edges = vec![ViewEdge {
            from: if reverse { 1 } else { 0 },
            to: if reverse { 0 } else { 1 },
            points: vec![[shift + 50.0, 60.0], [shift + 50.0, 200.0]],
            ..Default::default()
        }];
        GraphView {
            nodes,
            edges,
            ..Default::default()
        }
    }
    #[test]
    fn route_matching_uses_endpoint_identity_and_crossfades_changed_geometry() {
        let mut routes = Routes::default();
        let before = graph(false, 0.0);
        let after = graph(false, 100.0);
        routes.begin(capture(&before), &after);
        assert_eq!(routes.opacity(0), 0.0);
        assert_eq!(routes.outgoing().count(), 1);
        routes.advance(0.03);
        assert!(routes.opacity(0) > 0.0 && routes.opacity(0) < 1.0);
        assert!(routes.outgoing().next().unwrap().1 < 1.0);
        let changed = graph(true, 100.0);
        assert_ne!(capture(&after)[0].key, capture(&changed)[0].key);
    }
}
