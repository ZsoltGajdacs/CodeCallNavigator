//! Transient displayed geometry. Graph semantics and target layout remain unchanged.
mod bands;
mod routes;
use crate::{camera::Camera, projection::GraphView};
use std::{cell::Cell, collections::HashMap, time::Duration};

#[derive(Clone, Copy, Debug)]
struct Spring {
    value: f32,
    velocity: f32,
    target: f32,
}

impl Spring {
    fn new(value: f32, target: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
            target,
        }
    }
    fn step(&mut self, dt: f32) -> bool {
        (self.value, self.velocity) =
            rune::Spring::new(0.30, 0.0).sample(self.value, self.velocity, self.target, dt);
        if (self.value - self.target).abs() < 0.01 && self.velocity.abs() < 0.02 {
            self.value = self.target;
            self.velocity = 0.0;
            false
        } else {
            true
        }
    }
}

struct Node {
    key: String,
    rect: [Spring; 4],
    opacity: Spring,
}

pub struct Snapshot {
    rectangles: HashMap<String, [f32; 4]>,
    camera: Camera,
    nodes: Vec<crate::projection::ViewNode>,
    edges: Vec<routes::Captured>,
    bands: Vec<crate::projection::LayerBand>,
}

impl Snapshot {
    pub fn capture(view: &GraphView, camera: &Camera) -> Self {
        Self {
            rectangles: view
                .nodes
                .iter()
                .map(|n| {
                    (
                        n.key.clone(),
                        [n.position[0], n.position[1], n.size[0], n.size[1]],
                    )
                })
                .collect(),
            camera: camera.clone(),
            // Departing cards only paint their label and rectangle. Do not clone
            // potentially large method/file inventories merely to fade a card.
            nodes: view
                .nodes
                .iter()
                .map(|node| crate::projection::ViewNode {
                    key: node.key.clone(),
                    label: node.label.clone(),
                    position: node.position,
                    size: node.size,
                    ..Default::default()
                })
                .collect(),
            edges: routes::capture(view),
            bands: view.layer_bands.clone(),
        }
    }
}

struct Ghost {
    node: crate::projection::ViewNode,
    opacity: Spring,
}

#[derive(Default)]
pub struct Motion {
    nodes: Vec<Node>,
    ghosts: Vec<Ghost>,
    routes: Vec<Vec<[f32; 2]>>,
    route_presence: routes::Routes,
    bands: bands::Bands,
    camera: Option<[Spring; 3]>,
    last_time: Option<f64>,
    pub reduced: Cell<bool>,
}

impl Motion {
    pub fn begin(&mut self, before: Snapshot, view: &mut GraphView, camera: &mut Camera) {
        if before.rectangles.is_empty() || view.nodes.is_empty() {
            self.clear();
            return;
        }
        let previous_ghosts: HashMap<_, _> = self
            .ghosts
            .drain(..)
            .map(|g| (g.node.key.clone(), g))
            .collect();
        let previous: HashMap<_, _> = self.nodes.drain(..).map(|n| (n.key.clone(), n)).collect();
        self.nodes = view
            .nodes
            .iter()
            .map(|n| retarget_node(n, &before, &previous, &previous_ghosts))
            .collect();
        let retained: std::collections::HashSet<_> =
            view.nodes.iter().map(|n| n.key.as_str()).collect();
        self.ghosts = before
            .nodes
            .into_iter()
            .filter(|n| !retained.contains(n.key.as_str()))
            .map(|node| {
                let opacity = {
                    let mut opacity = previous
                        .get(&node.key)
                        .map(|n| n.opacity)
                        .unwrap_or(Spring::new(1.0, 0.0));
                    opacity.target = 0.0;
                    opacity
                };
                Ghost { node, opacity }
            })
            .chain(
                previous_ghosts
                    .into_values()
                    .filter(|g| !retained.contains(g.node.key.as_str())),
            )
            .take(1500)
            .collect();
        self.route_presence.begin(before.edges, view);
        self.bands.begin(before.bands, &view.layer_bands);
        self.routes = view.edges.iter().map(|e| e.points.clone()).collect();
        let origin = [
            before.camera.pan[0],
            before.camera.pan[1],
            before.camera.zoom,
        ];
        let target = [camera.pan[0], camera.pan[1], camera.zoom];
        let previous_camera = self.camera.take();
        self.camera = Some(std::array::from_fn(|i| {
            let mut track = Spring::new(origin[i], target[i]);
            if let Some(previous) = previous_camera {
                track.velocity = previous[i].velocity;
            }
            track
        }));
        if self.reduced.get() {
            self.advance(10.0);
        }
        self.apply(view, camera);
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.ghosts.clear();
        self.routes.clear();
        self.route_presence = Default::default();
        self.bands = Default::default();
        self.camera = None;
        self.last_time = None;
    }

    pub fn ghosts(&self) -> impl Iterator<Item = (&crate::projection::ViewNode, f32)> {
        self.ghosts.iter().map(|g| (&g.node, g.opacity.value))
    }

    pub fn outgoing_routes(&self) -> impl Iterator<Item = (&crate::projection::ViewEdge, f32)> {
        self.route_presence.outgoing()
    }
    pub fn edge_opacity(&self, index: usize) -> f32 {
        self.route_presence.opacity(index)
    }

    pub fn interrupt_camera(&mut self) {
        self.camera = None;
    }
    pub fn opacity(&self, index: usize) -> f32 {
        self.nodes.get(index).map_or(1.0, |n| n.opacity.value)
    }

    pub fn tick(&mut self, time: f64, view: &mut GraphView, camera: &mut Camera) -> bool {
        if self.nodes.is_empty() {
            return false;
        }
        // Navigation restoration and framing can write a camera outside gesture handlers.
        // A changed presented value transfers ownership back to that writer immediately.
        if self.camera_overridden(camera) {
            self.interrupt_camera();
        }
        let dt = self
            .last_time
            .replace(time)
            .map_or(1.0 / 60.0, |last| (time - last).clamp(0.0, 0.1) as f32);
        let active = self.advance(if self.reduced.get() { 10.0 } else { dt });
        self.apply(view, camera);
        if !active {
            self.clear();
        }
        true
    }

    fn camera_overridden(&self, camera: &Camera) -> bool {
        self.camera.as_ref().is_some_and(|tracks| {
            [camera.pan[0], camera.pan[1], camera.zoom]
                != tracks.each_ref().map(|track| track.value)
        })
    }

    fn advance(&mut self, dt: f32) -> bool {
        let node_tracks = self.nodes.iter_mut().flat_map(|node| {
            node.rect
                .iter_mut()
                .chain(std::iter::once(&mut node.opacity))
        });
        let camera_tracks = self.camera.iter_mut().flat_map(|tracks| tracks.iter_mut());
        let ghost_tracks = self.ghosts.iter_mut().map(|ghost| &mut ghost.opacity);
        let geometry_active = node_tracks
            .chain(camera_tracks)
            .chain(ghost_tracks)
            .fold(false, |active, track| track.step(dt) || active);
        self.route_presence.advance(dt) | self.bands.advance(dt) | geometry_active
    }

    fn apply(&mut self, view: &mut GraphView, camera: &mut Camera) {
        for (node, track) in view.nodes.iter_mut().zip(&self.nodes) {
            node.position = [track.rect[0].value, track.rect[1].value];
            node.size = [track.rect[2].value, track.rect[3].value];
        }
        for (edge, points) in view.edges.iter_mut().zip(&self.routes) {
            let from = &self.nodes[edge.from];
            let to = &self.nodes[edge.to];
            let Some(first) = points.first() else {
                continue;
            };
            let Some(last) = points.last() else {
                continue;
            };
            let from_delta = endpoint_delta(from, *first);
            let to_delta = endpoint_delta(to, *last);
            let count = points.len().saturating_sub(1).max(1) as f32;
            for (i, (point, target)) in edge.points.iter_mut().zip(points).enumerate() {
                let t = i as f32 / count;
                for axis in 0..2 {
                    point[axis] = target[axis] + (1.0 - t) * from_delta[axis] + t * to_delta[axis];
                }
            }
        }
        self.bands.apply(&mut view.layer_bands);
        self.route_presence.apply(view);
        if let Some(tracks) = &self.camera {
            camera.pan = [tracks[0].value, tracks[1].value];
            camera.zoom = tracks[2].value;
        }
    }
}

fn retarget_node(
    n: &crate::projection::ViewNode,
    before: &Snapshot,
    previous: &HashMap<String, Node>,
    previous_ghosts: &HashMap<String, Ghost>,
) -> Node {
    let target = [n.position[0], n.position[1], n.size[0], n.size[1]];
    let origin = before
        .rectangles
        .get(&n.key)
        .copied()
        .or_else(|| {
            previous_ghosts.get(&n.key).map(|g| {
                [
                    g.node.position[0],
                    g.node.position[1],
                    g.node.size[0],
                    g.node.size[1],
                ]
            })
        })
        .unwrap_or(target);
    let mut rect = std::array::from_fn(|i| Spring::new(origin[i], target[i]));
    if let Some(old) = previous.get(&n.key) {
        for i in 0..4 {
            rect[i].velocity = old.rect[i].velocity;
        }
    }
    let opacity = previous
        .get(&n.key)
        .map(|old| old.opacity)
        .or_else(|| previous_ghosts.get(&n.key).map(|g| g.opacity))
        .map(|mut opacity| {
            opacity.target = 1.0;
            opacity
        })
        .unwrap_or_else(|| {
            Spring::new(
                if before.rectangles.contains_key(&n.key) {
                    1.0
                } else {
                    0.0
                },
                1.0,
            )
        });
    Node {
        key: n.key.clone(),
        rect,
        opacity,
    }
}

/// Preserve each Graphviz port's relative position while its owning rectangle changes.
fn endpoint_delta(node: &Node, endpoint: [f32; 2]) -> [f32; 2] {
    std::array::from_fn(|axis| {
        let position = node.rect[axis];
        let size = node.rect[axis + 2];
        let fraction = (endpoint[axis] - position.target) / size.target.max(1.0);
        position.value - position.target + fraction * (size.value - size.target)
    })
}

pub fn poll(state: &mut crate::app::State, time: f64, tx: &mut rune::Tx) -> bool {
    let changed = state
        .graph_motion
        .tick(time, &mut state.graph_view, &mut state.nav.camera);
    if changed {
        *state.edge_geometry.get_mut() = Default::default();
        if !state.graph_motion.nodes.is_empty() {
            tx.wake_after(Duration::from_secs_f64(1.0 / 120.0));
        } else {
            // Persist the final camera once, never transient animation frames.
            state.dirty_session = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::ViewNode;
    fn view(x: f32) -> GraphView {
        GraphView {
            nodes: vec![ViewNode {
                key: "same".into(),
                position: [x, 0.0],
                size: [100.0, 60.0],
                ..Default::default()
            }],
            ..Default::default()
        }
    }
    #[test]
    fn retargeting_starts_at_displayed_position_and_keeps_velocity() {
        let mut motion = Motion::default();
        let mut camera = Camera::default();
        let mut target = view(200.0);
        motion.begin(
            Snapshot::capture(&view(0.0), &camera),
            &mut target,
            &mut camera,
        );
        motion.tick(0.0, &mut target, &mut camera);
        let displayed = target.nodes[0].position[0];
        assert!(displayed > 0.0 && displayed < 200.0);
        let before = Snapshot::capture(&target, &camera);
        target = view(400.0);
        motion.begin(before, &mut target, &mut camera);
        assert_eq!(target.nodes[0].position[0], displayed);
        assert!(motion.nodes[0].rect[0].velocity > 0.0);
        motion.tick(0.02, &mut target, &mut camera);
        assert!(target.nodes[0].position[0] > displayed);
        for i in 1..240 {
            motion.tick(i as f64 / 60.0, &mut target, &mut camera);
        }
        assert_eq!(target.nodes[0].position[0], 400.0);
        assert!(motion.nodes.is_empty());
    }
    #[test]
    fn reduced_motion_installs_target_without_travel() {
        let mut motion = Motion::default();
        motion.reduced.set(true);
        let mut camera = Camera::default();
        let mut target = view(200.0);
        motion.begin(
            Snapshot::capture(&view(0.0), &camera),
            &mut target,
            &mut camera,
        );
        motion.tick(0.0, &mut target, &mut camera);
        assert_eq!(target.nodes[0].position[0], 200.0);
        assert!(motion.nodes.is_empty());
    }
    #[test]
    fn changing_size_keeps_edge_ports_attached() {
        let node = Node {
            key: "n".into(),
            rect: [
                Spring::new(20.0, 100.0),
                Spring::new(30.0, 200.0),
                Spring::new(60.0, 120.0),
                Spring::new(40.0, 80.0),
            ],
            opacity: Spring::new(1.0, 1.0),
        };
        let port = [220.0, 240.0];
        let delta = endpoint_delta(&node, port);
        assert_eq!([port[0] + delta[0], port[1] + delta[1]], [80.0, 50.0]);
    }

    #[test]
    fn external_camera_override_during_motion_preserves_position_and_zoom() {
        let mut motion = Motion::default();
        let mut camera = Camera::default();
        let mut target = view(200.0);
        motion.begin(
            Snapshot::capture(&view(0.0), &camera),
            &mut target,
            &mut camera,
        );
        motion.tick(0.0, &mut target, &mut camera);
        camera.pan = [91.0, -47.0];
        camera.zoom = 0.67;
        for frame in 1..240 {
            motion.tick(frame as f64 / 60.0, &mut target, &mut camera);
        }
        assert_eq!(camera.pan, [91.0, -47.0]);
        assert_eq!(camera.zoom, 0.67);
        assert_eq!(target.nodes[0].position[0], 200.0);
        assert!(motion.nodes.is_empty());
    }

    #[test]
    fn camera_interruption_preserves_manual_position_while_nodes_finish() {
        let mut motion = Motion::default();
        let mut camera = Camera::default();
        let mut target = view(200.0);
        motion.begin(
            Snapshot::capture(&view(0.0), &camera),
            &mut target,
            &mut camera,
        );
        motion.interrupt_camera();
        camera.pan = [42.0, 73.0];
        motion.tick(0.0, &mut target, &mut camera);
        assert_eq!(camera.pan, [42.0, 73.0]);
        assert!(target.nodes[0].position[0] > 0.0);
    }

    #[test]
    fn removed_nodes_fade_as_noninteractive_presentation_ghosts() {
        let mut motion = Motion::default();
        let mut camera = Camera::default();
        let before = view(0.0);
        let mut target = view(20.0);
        target.nodes[0].key = "replacement".into();
        motion.begin(
            Snapshot::capture(&before, &camera),
            &mut target,
            &mut camera,
        );
        motion.tick(0.0, &mut target, &mut camera);
        assert_eq!(motion.ghosts().next().unwrap().0.key, "same");
        assert!(motion.ghosts().next().unwrap().1 < 1.0);
        assert!(motion.opacity(0) > 0.0 && motion.opacity(0) < 1.0);
    }
    #[test]
    fn rapid_retarget_preserves_departure_opacity_and_revives_displayed_node() {
        let mut motion = Motion::default();
        let mut camera = Camera::default();
        let original = view(0.0);
        let mut target = view(200.0);
        target.nodes[0].key = "other".into();
        motion.begin(
            Snapshot::capture(&original, &camera),
            &mut target,
            &mut camera,
        );
        motion.tick(0.0, &mut target, &mut camera);
        let opacity = motion.ghosts().next().unwrap().1;
        let snapshot = Snapshot::capture(&target, &camera);
        target = view(400.0);
        target.nodes[0].key = "third".into();
        motion.begin(snapshot, &mut target, &mut camera);
        assert_eq!(
            motion.ghosts().find(|(n, _)| n.key == "same").unwrap().1,
            opacity
        );
        let snapshot = Snapshot::capture(&target, &camera);
        target = view(600.0);
        motion.begin(snapshot, &mut target, &mut camera);
        assert_eq!(target.nodes[0].position, [0.0, 0.0]);
        assert_eq!(motion.opacity(0), opacity);
    }
}
