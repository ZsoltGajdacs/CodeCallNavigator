//! Resolve one presentation level for a focused neighborhood before traversal.
use super::{
    FocusTarget, GroupKind, Grouping, Request,
    groups::{self, Group},
};
use codecallnavigator::{graph::GraphIndex, model::Node};

#[derive(Clone, Copy)]
pub(super) enum PresentationLevel {
    Project,
    Namespace,
    Folder(usize),
    Class,
    Layer,
    Method,
}

impl PresentationLevel {
    fn of(group: &Group) -> Self {
        match group.kind {
            GroupKind::Project => Self::Project,
            GroupKind::Namespace => Self::Namespace,
            GroupKind::Folder => Self::Folder(group.folder_depth),
            GroupKind::Class => Self::Class,
            GroupKind::Layer => Self::Layer,
        }
    }

    pub(super) fn representative(self, node: &Node, request: &Request) -> Option<Group> {
        let kind = match self {
            Self::Method => return None,
            Self::Folder(depth) => return Some(groups::folder_at_depth(node, request, depth)),
            Self::Project => GroupKind::Project,
            Self::Namespace => GroupKind::Namespace,
            Self::Class => GroupKind::Class,
            Self::Layer => GroupKind::Layer,
        };
        groups::groups(node, request)
            .into_iter()
            .find(|group| group.kind == kind)
    }

    fn children(self, graph: &GraphIndex, request: &Request, members: &[u32]) -> Self {
        match self {
            Self::Layer => Self::Project,
            Self::Project => match request.grouping {
                Grouping::Namespace => Self::Namespace,
                Grouping::Folder => Self::Folder(1),
                _ => Self::Class,
            },
            Self::Folder(depth)
                if members.iter().any(|&index| {
                    groups::folder_depth(&graph.data.nodes[index as usize]) > depth
                }) =>
            {
                Self::Folder(depth + 1)
            }
            Self::Class | Self::Method => Self::Method,
            Self::Namespace | Self::Folder(_) => Self::Class,
        }
    }
}

pub(super) struct FocusProjection {
    pub level: PresentationLevel,
    pub members: Vec<u32>,
}

impl FocusProjection {
    pub(super) fn resolve(
        graph: &GraphIndex,
        request: &Request,
        nodes: &[u32],
        target: &FocusTarget,
    ) -> Option<Self> {
        if let FocusTarget::Method(identity) = target {
            let member = nodes
                .iter()
                .copied()
                .find(|&index| graph.data.nodes[index as usize].id == *identity)?;
            return Some(Self {
                level: PresentationLevel::Method,
                members: vec![member],
            });
        }
        let (FocusTarget::Group(key)
        | FocusTarget::GroupChildren(key)
        | FocusTarget::ClassMethods(key)) = target
        else {
            unreachable!("method focus resolved above")
        };
        let matches: Vec<_> = nodes
            .iter()
            .filter_map(|&index| {
                groups::find(&graph.data.nodes[index as usize], request, key)
                    .map(|group| (index, group))
            })
            .collect();
        let mut level = PresentationLevel::of(&matches.first()?.1);
        let members: Vec<_> = matches.into_iter().map(|(index, _)| index).collect();
        match target {
            FocusTarget::ClassMethods(_) => level = PresentationLevel::Method,
            FocusTarget::GroupChildren(_) => level = level.children(graph, request, &members),
            _ => {}
        }
        Some(Self { level, members })
    }
}
