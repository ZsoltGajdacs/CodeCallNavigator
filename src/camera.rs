use crate::theme;
use crate::{
    app::State,
    projection::{GraphView, ViewEdge, ViewNode},
};
use rune::TextStyle;
use rune::{Cx, Drag, El, MagnifyEvent, Phase, WheelEvent, canvas, col, el, text};
use rune_kit::Kit;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct PendingCenter {
    pub deadline: Option<f64>,
    pub interval: f64,
}

#[cfg(all(target_os = "macos", not(test)))]
fn double_click_interval() -> f64 {
    // SAFETY: Called by the UI event handler on the main thread, as in Rune.
    unsafe { objc2_app_kit::NSEvent::doubleClickInterval() }
}

#[cfg(any(not(target_os = "macos"), test))]
fn double_click_interval() -> f64 {
    0.5
}

pub fn poll_center(state: &mut State, time: f64, tx: &mut rune::Tx) -> bool {
    let Some(pending) = &mut state.pending_center else {
        return false;
    };
    let deadline = *pending.deadline.get_or_insert(time + pending.interval);
    if time < deadline {
        tx.wake_after(Duration::from_secs_f64(deadline - time));
        return false;
    }
    state.pending_center = None;
    let Some(node) = state
        .graph_view
        .nodes
        .iter()
        .find(|node| Some(&node.key) == state.visual_selection.as_ref())
    else {
        return false;
    };
    let center = [
        node.position[0] + node.size[0] / 2.0,
        node.position[1] + node.size[1] / 2.0,
    ];
    state.nav.camera.pan = [
        state.viewport[0] / 2.0 - center[0] * state.nav.camera.zoom,
        state.viewport[1] / 2.0 - center[1] * state.nav.camera.zoom,
    ];
    state.dirty_session = true;
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub pan: [f32; 2],
    pub zoom: f32,
    #[serde(skip)]
    drag_start: [f32; 2],
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: [24.0, 24.0],
            zoom: 1.0,
            drag_start: [0.0; 2],
        }
    }
}

impl Camera {
    pub fn sanitize(&mut self) {
        if !self.zoom.is_finite() {
            self.zoom = 1.0;
        }
        self.zoom = self.zoom.clamp(0.08, 3.0);
        if !self.pan.iter().all(|value| value.is_finite()) {
            self.pan = [24.0, 24.0];
        }
    }
    pub fn screen(&self, point: [f32; 2]) -> [f32; 2] {
        [
            point[0] * self.zoom + self.pan[0],
            point[1] * self.zoom + self.pan[1],
        ]
    }

    pub fn zoom_at(&mut self, point: [f32; 2], factor: f32) {
        let world = [
            (point[0] - self.pan[0]) / self.zoom,
            (point[1] - self.pan[1]) / self.zoom,
        ];
        self.zoom = (self.zoom * factor).clamp(0.08, 3.0);
        self.pan = [
            point[0] - world[0] * self.zoom,
            point[1] - world[1] * self.zoom,
        ];
    }

    pub fn fit(&mut self, world: [f32; 2], viewport: [f32; 2]) {
        self.zoom = ((viewport[0] - 48.0) / world[0].max(1.0))
            .min((viewport[1] - 48.0) / world[1].max(1.0))
            .clamp(0.08, 1.2);
        self.pan = [
            (viewport[0] - world[0] * self.zoom) / 2.0,
            (viewport[1] - world[1] * self.zoom) / 2.0,
        ];
    }

    pub fn drag(&mut self, drag: &Drag) {
        if drag.phase == Phase::Down {
            self.drag_start = self.pan;
        }
        self.pan = if drag.cancelled() {
            self.drag_start
        } else {
            [self.drag_start[0] + drag.dx, self.drag_start[1] + drag.dy]
        };
    }

    pub fn shows(&self, node: &ViewNode, viewport: [f32; 2]) -> bool {
        let origin = self.screen(node.position);
        origin[0] + node.size[0] * self.zoom >= 0.0
            && origin[1] + node.size[1] * self.zoom >= 0.0
            && origin[0] <= viewport[0]
            && origin[1] <= viewport[1]
    }
}

pub fn graph_canvas(state: &State, _: &mut Cx<State>, k: &Kit) -> El {
    el().key("graph-canvas")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .clip_soft(0.0)
        .fill(theme::canvas(k))
        .on_layout(|state: &mut State, rect| {
            state.viewport = [rect[2], rect[3]];
            state.source_popup.canvas = [rect[0], rect[1]];
        })
        .on_wheel(|state: &mut State, event: WheelEvent, _| {
            state.pending_center = None;
            state
                .nav
                .camera
                .zoom_at([event.x, event.y], (-event.dy / 240.0).exp());
            state.dirty_session = true;
            true
        })
        .on_magnify(|state: &mut State, event: MagnifyEvent, _| {
            state.pending_center = None;
            state
                .nav
                .camera
                .zoom_at([event.x, event.y], 1.0 + event.delta);
            state.dirty_session = true;
            true
        })
        .on_drag(|state: &mut State, drag: &mut Drag, _| {
            state.pending_center = None;
            state.nav.camera.drag(drag);
            state.dirty_session = true;
        })
        .child(graph_content(state, k))
}

pub(crate) struct Emphasis {
    selected: Option<usize>,
    related: Vec<bool>,
}

impl Emphasis {
    fn new(view: &GraphView, key: Option<&str>) -> Self {
        let selected = key.and_then(|key| view.nodes.iter().position(|node| node.key == key));
        let mut related = vec![false; view.nodes.len()];
        if let Some(index) = selected {
            related[index] = true;
            for edge in &view.edges {
                if edge.from == index || edge.to == index {
                    related[edge.from] = true;
                    related[edge.to] = true;
                }
            }
        }
        Self { selected, related }
    }

    fn node_dimmed(&self, index: usize) -> bool {
        self.selected.is_some() && !self.related[index]
    }

    pub(crate) fn edge_dimmed(&self, edge: &ViewEdge) -> bool {
        self.selected
            .is_some_and(|index| edge.from != index && edge.to != index)
    }
}

fn graph_content(state: &State, k: &Kit) -> El {
    let mut content = el().key("graph-world").w_full().h_full();
    let emphasis = Emphasis::new(&state.graph_view, state.visual_selection.as_deref());
    let shapes = crate::edges::render(state, &emphasis, k);
    content = content.child(canvas(shapes).w_full().h_full().inert());
    if !state.view_is_current() {
        content = content.inert().opacity(0.45);
    }
    for (index, node) in state.graph_view.nodes.iter().enumerate() {
        if state.nav.camera.shows(node, state.viewport) {
            let card = node_card(state, index, &emphasis, k);
            content = content.child(if state.view_is_current() {
                card
            } else {
                card.inert()
            });
        }
    }
    if state.graph_view.nodes.is_empty() {
        content = content.child(empty_canvas(state, k));
    }
    content
}

fn empty_canvas(state: &State, k: &Kit) -> El {
    let (title, description) = empty_message(state);
    col()
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .center()
        .padding(24.0)
        .gap(12.0)
        .child(
            text(title)
                .font(TextStyle::new(20.0, 600.0))
                .color(k.c.text_base)
                .max_w(580.0)
                .wrap(),
        )
        .child(
            text(description)
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .max_w(580.0)
                .wrap(),
        )
}

fn empty_message(state: &State) -> (&str, &str) {
    if state.graph.is_none() {
        return (
            "See how your application connects",
            "Choose a folder, select its builds, then generate a call graph.",
        );
    }
    if state.jobs.layout.is_pending() || state.needs_layout {
        return (
            "Preparing graph view…",
            "The complete analysis is available while the visible layout is prepared.",
        );
    }
    if !state.warning.is_empty() {
        return (
            "Graph view unavailable",
            "Your last successful analysis is preserved. Read the message in the inspector, or change the graph filters.",
        );
    }
    (
        "No symbols in this view",
        "Adjust the filters, search for a method, or return to Overview.",
    )
}

fn node_card(state: &State, index: usize, emphasis: &Emphasis, k: &Kit) -> El {
    let node = &state.graph_view.nodes[index];
    let selected = emphasis.selected == Some(index);
    let accent = k.c.text_accent;
    let border = if node.cyclic {
        theme::uncertain(k)
    } else if selected {
        accent
    } else {
        k.c.text_muted.alpha(0.3)
    };
    let point = state.nav.camera.screen(node.position);
    let scale = state.nav.camera.zoom;
    let detail = node_detail(node);
    let card = col()
        .key(format!("graph-node-{index}"))
        .absolute(point[0], point[1])
        .size(node.size[0] * scale, node.size[1] * scale)
        .radius(9.0)
        .border(if selected { 2.0 } else { 1.0 }, border)
        .fill(if selected {
            accent.alpha(0.16)
        } else if node.application_start {
            accent.alpha(0.10)
        } else {
            k.c.layer_01
        })
        .padding((10.0 * scale).max(3.0))
        .gap(3.0)
        .clip_rounded(9.0)
        .opacity(if emphasis.node_dimmed(index) {
            0.48
        } else {
            1.0
        })
        .child(
            text(&node.label)
                .font(theme::mono((14.0 * scale).max(8.0), 600.0))
                .color(k.c.text_base)
                .truncate(),
        )
        .child(
            text(detail)
                .font(TextStyle::new((11.0 * scale).max(7.0), 400.0))
                .color(k.c.text_muted)
                .truncate(),
        );
    node_actions(card, node)
}

fn node_actions(card: El, node: &ViewNode) -> El {
    let key = node.key.clone();
    let drill_key = key.clone();
    let member = node.member;
    let hover_node = node.clone();
    let detail = node_detail(node);
    card.on_hover_change(move |s: &mut State, inside, tx| {
        s.hover_source(hover_node.clone(), inside, tx)
    })
    .role("button")
    .label(format!("{} · {}", node.label, detail))
    .focusable(true)
    .on_press_tx(move |state: &mut State, tx| {
        let interval = double_click_interval();
        state.select_visible(key.clone(), member, interval);
        tx.wake_after(Duration::from_secs_f64(interval));
    })
    .on_double_press(move |state: &mut State| {
        state.pending_center = None;
        if let Some(member) = member {
            state.select(member, true);
        } else {
            state.activate(drill_key.clone(), None);
        }
    })
}

fn node_detail(node: &ViewNode) -> String {
    if node.member.is_none() {
        format!(
            "{} symbols · {} links inside",
            node.count, node.internal_edges
        )
    } else if node.application_start {
        format!("Application start · {}", node.detail)
    } else {
        node.detail.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emphasis_keeps_only_direct_neighbors_and_incident_edges_bright() {
        let view = GraphView {
            nodes: (0..5)
                .map(|i| ViewNode {
                    key: i.to_string(),
                    ..Default::default()
                })
                .collect(),
            edges: [(0, 1), (2, 0), (1, 2), (1, 3)]
                .into_iter()
                .map(|(from, to)| ViewEdge {
                    from,
                    to,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let emphasis = Emphasis::new(&view, Some("0"));
        assert_eq!(
            (0..5).map(|i| emphasis.node_dimmed(i)).collect::<Vec<_>>(),
            [false, false, false, true, true]
        );
        assert_eq!(
            view.edges
                .iter()
                .map(|edge| emphasis.edge_dimmed(edge))
                .collect::<Vec<_>>(),
            [false, false, true, true]
        );
        let none = Emphasis::new(&view, Some("missing"));
        assert!((0..5).all(|i| !none.node_dimmed(i)));
        assert!(view.edges.iter().all(|edge| !none.edge_dimmed(edge)));
    }
    #[test]
    fn zoom_keeps_the_world_point_under_the_pointer() {
        let mut camera = Camera::default();
        let point = [150.0, 100.0];
        let shown = camera.screen(point);
        camera.zoom_at(shown, 2.0);
        assert_eq!(camera.screen(point), shown);
        camera.fit([1000.0, 500.0], [600.0, 400.0]);
        assert!(camera.zoom > 0.0 && camera.zoom < 1.0);
    }
}
