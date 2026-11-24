//! Render the same class layout with the before or after relationship facts.
use super::panes::{empty, heading};
use crate::{
    app::State,
    flow_comparison::{ClassPair, SideId, class_present},
    flow_view::address::Address,
    projection::labels::{self, Label},
    theme,
};
use codecallnavigator::{
    artifact::comparison_changes::{ClassNode, GraphChange},
    model::TargetResolution,
    search::simple_class_name,
};
use rune::{Color, El, Key, Shape, canvas, col, el, text};
use rune_kit::Kit;
use std::collections::BTreeMap;

fn class_label(node: &ClassNode, side: SideId) -> Label {
    let name = node
        .short_name
        .as_deref()
        .unwrap_or_else(|| simple_class_name(&node.name));
    let visibility = match side {
        SideId::Before => node.before_visibility,
        SideId::After => node.after_visibility,
    };
    labels::named(name, &node.name, visibility)
}

pub(super) fn pane(content: El, state: &State, side: SideId, k: &Kit) -> El {
    let (title, address) = match side {
        SideId::Before => ("Before", Address::Before),
        SideId::After => ("After", Address::After),
    };
    let workspace = &state.flow_comparison;
    let Some(pair) = workspace.class_pair() else {
        return content.child(heading(title, None, k)).child(empty(
            address,
            &workspace.detail_message,
            k,
        ));
    };
    let name = class_label(&pair.graph.nodes[pair.root], side);
    let content = content.child(heading(title, Some(&name), k));
    if !pair.has_root(side) {
        return content.child(empty(address, "Class missing in this revision", k));
    }
    content.child(
        ClassCanvas {
            pair,
            side,
            address,
            viewport: workspace.viewports[side.index()],
            kit: k,
        }
        .view(),
    )
}

struct ClassCanvas<'a> {
    pair: &'a ClassPair,
    side: SideId,
    address: Address,
    viewport: [f32; 2],
    kit: &'a Kit,
}

impl ClassCanvas<'_> {
    fn view(&self) -> El {
        let Self {
            pair,
            address,
            side,
            kit,
            ..
        } = *self;
        let id = pair.graph.root.clone();
        el().key(address.key("class-canvas"))
            .motion(rune::Spring::NONE)
            .grow(1.0)
            .w_full()
            .min_w(0.0)
            .clip_soft(0.0)
            .fill(theme::canvas(kit))
            .on_layout(move |s: &mut State, r| {
                s.flow_comparison
                    .measure_class_pane(side, &id, [r[2], r[3]]);
            })
            .on_wheel(move |s: &mut State, event, _| {
                if let Some(camera) = s.flow_comparison.class_camera_mut(side) {
                    camera.zoom_at([event.x, event.y], (event.dy / 240.0).exp());
                }
                true
            })
            .on_magnify(move |s: &mut State, event, _| {
                if let Some(camera) = s.flow_comparison.class_camera_mut(side) {
                    camera.zoom_at([event.x, event.y], 1.0 + event.delta);
                }
                true
            })
            .on_drag(move |s: &mut State, drag, _| {
                if let Some(camera) = s.flow_comparison.class_camera_mut(side) {
                    camera.drag(drag);
                }
            })
            .child(self.connectors())
            .children(self.cards())
            .child(notice(pair, kit))
    }

    fn cards(&self) -> impl Iterator<Item = El> + '_ {
        let Self {
            pair,
            side,
            viewport,
            ..
        } = *self;
        let camera = &pair.cameras[side.index()];
        pair.graph
            .nodes
            .iter()
            .zip(&pair.layout.nodes)
            .enumerate()
            .filter_map(move |(index, (node, geometry))| {
                if !class_present(node, side) || !camera.shows(geometry, viewport) {
                    return None;
                }
                let point = camera.screen(geometry.position);
                let size = geometry.size.map(|length| length * camera.zoom);
                Some(self.card(node, index == pair.root, CardBounds { point, size }))
            })
    }

    fn card(&self, node: &ClassNode, root: bool, bounds: CardBounds) -> El {
        let Self {
            side,
            address,
            kit: k,
            ..
        } = *self;
        let CardBounds { point, size } = bounds;
        let anchor = address.key(&format!("class-{}", node.id));
        let border = status_color(node.status, k).unwrap_or({
            if root {
                k.c.text_accent
            } else {
                k.c.std_border
            }
        });
        let keyboard_anchor = anchor.clone();
        let name = class_label(node, side);
        let mut control = col()
            .key(anchor.clone())
            .absolute(point[0], point[1])
            .size(size[0], size[1])
            .center()
            .px(6.0)
            .gap(2.0)
            .radius(6.0)
            .border(if root { 2.0 } else { 1.0 }, border)
            .fill(k.c.surface_raised)
            .role("button")
            .label(&name.text)
            .help(&name.description)
            .focusable(true)
            .child(
                text(&name.text)
                    .key(address.key(&format!("class-name-{}", node.id)))
                    .w_full()
                    .truncate()
                    .font(k.ty.small_medium)
                    .color(k.c.text_base),
            );
        if let Some(status) = status_label(node.status) {
            control = control.child(text(status).font(k.ty.small).color(border));
        }
        control
            .on_hover_change(move |s: &mut State, inside, tx| {
                s.flow_comparison.hover_canvas_source(&anchor, inside, tx);
            })
            .on_key(move |s: &mut State, event, _| {
                if event.key != Key::Enter {
                    return false;
                }
                s.flow_comparison.focus_canvas_source(&keyboard_anchor);
                true
            })
    }

    fn connectors(&self) -> El {
        let Self {
            pair,
            side,
            address,
            viewport,
            kit: k,
        } = *self;
        let camera = &pair.cameras[side.index()];
        let mut shape = Shape::new();
        let mut labels = el().w_full().h_full().inert();
        for (link, edge) in pair.graph.links.iter().zip(&pair.layout.edges) {
            let counts = match side {
                SideId::Before => &link.before,
                SideId::After => &link.after,
            };
            if counts.is_empty() {
                continue;
            }
            let points: Vec<_> = edge
                .points
                .iter()
                .map(|&point| camera.screen(point))
                .collect();
            if points.len() < 2 || !crate::edges::visible(&points, viewport) {
                continue;
            }
            let uncertain = counts
                .keys()
                .any(|resolution| *resolution != TargetResolution::Direct);
            let color = status_color(link.status, k).unwrap_or_else(|| {
                if uncertain {
                    theme::uncertain(k)
                } else {
                    k.c.text_muted
                }
            });
            shape = crate::edges::route(shape, &points, color, uncertain);
            let [x, y] = points[points.len() / 2];
            labels = labels.child(
                text(link_label(counts, link.status))
                    .key(address.key(&format!("class-link-{}-{}", link.from, link.to)))
                    .absolute(x + 5.0, y - 9.0)
                    .font(k.ty.small)
                    .color(color)
                    .fill(theme::canvas(k))
                    .px(3.0),
            );
        }
        labels.child(canvas(shape).w_full().h_full())
    }
}

struct CardBounds {
    point: [f32; 2],
    size: [f32; 2],
}

fn notice(pair: &ClassPair, k: &Kit) -> El {
    if pair.has_changes() {
        return el();
    }
    text("No class-level call changes")
        .absolute(8.0, 8.0)
        .font(k.ty.small)
        .color(k.c.text_muted)
        .inert()
}

fn status_label(status: GraphChange) -> Option<&'static str> {
    match status {
        GraphChange::Unchanged => None,
        GraphChange::Added => Some("Added"),
        GraphChange::Removed => Some("Removed"),
        GraphChange::Modified => Some("Modified"),
    }
}

fn status_color(status: GraphChange, k: &Kit) -> Option<Color> {
    match status {
        GraphChange::Unchanged => None,
        GraphChange::Added => Some(k.c.success.fg),
        GraphChange::Removed => Some(k.c.danger.fg),
        GraphChange::Modified => Some(k.c.warning.fg),
    }
}

fn link_label(counts: &BTreeMap<TargetResolution, u64>, status: GraphChange) -> String {
    let mut parts: Vec<_> = status_label(status)
        .map(str::to_owned)
        .into_iter()
        .collect();
    parts.extend(counts.iter().map(|(resolution, count)| {
        let label = match resolution {
            TargetResolution::Direct => "direct",
            TargetResolution::VirtualDispatch => "virtual",
            TargetResolution::Possible => "possible",
            TargetResolution::Other => "other",
        };
        format!("{count} {label}")
    }));
    parts.join(" · ")
}
