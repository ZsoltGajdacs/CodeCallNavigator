use crate::theme;
use crate::{app::State, projection::ViewNode};
use rune::TextStyle;
use rune::{Cx, Drag, El, MagnifyEvent, Phase, WheelEvent, canvas, col, el, text};
use rune_kit::Kit;
use serde::{Deserialize, Serialize};

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
    let mut content = el().key("graph-world").w_full().h_full();
    let shapes = crate::edges::render(&state.graph_view, &state.nav.camera, state.viewport, k);
    content = content.child(canvas(shapes).w_full().h_full().inert());
    if !state.view_is_current() {
        content = content.inert().opacity(0.45);
    }
    for (index, node) in state.graph_view.nodes.iter().enumerate() {
        if state.nav.camera.shows(node, state.viewport) {
            let card = node_card(state, node, index, k);
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
    el().key("graph-canvas")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .clip_soft(0.0)
        .fill(theme::canvas(k))
        .on_layout(|state: &mut State, rect| state.viewport = [rect[2], rect[3]])
        .on_wheel(|state: &mut State, event: WheelEvent, _| {
            if event.mods.cmd {
                state
                    .nav
                    .camera
                    .zoom_at([event.x, event.y], (-event.dy / 240.0).exp());
            } else {
                state.nav.camera.pan[0] += event.dx;
                state.nav.camera.pan[1] += event.dy;
            }
            state.dirty_session = true;
            true
        })
        .on_magnify(|state: &mut State, event: MagnifyEvent, _| {
            state
                .nav
                .camera
                .zoom_at([event.x, event.y], 1.0 + event.delta);
            state.dirty_session = true;
            true
        })
        .on_drag(|state: &mut State, drag: &mut Drag, _| {
            state.nav.camera.drag(drag);
            state.dirty_session = true;
        })
        .child(content)
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

fn node_card(state: &State, node: &ViewNode, index: usize, k: &Kit) -> El {
    let selected = node
        .member
        .is_some_and(|member| state.selected() == Some(member));
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
    let key = node.key.clone();
    let member = node.member;
    let detail = node_detail(node);
    col()
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
        .role("button")
        .label(format!("{} · {}", node.label, detail))
        .focusable(true)
        .on_press(move |state: &mut State| state.activate(key.clone(), member))
        .on_double_press(move |state: &mut State| {
            if let Some(member) = member {
                state.select(member, true);
            }
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
        )
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
