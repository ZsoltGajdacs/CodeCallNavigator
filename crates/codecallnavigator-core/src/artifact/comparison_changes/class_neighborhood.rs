//! Project snapshot calls onto exact class identities before walking either direction.
use super::{Class, NameDetails, Snapshot};
use crate::{
    model::{RelationshipKind, TargetResolution},
    query::DEFAULT_CLASS_COMPARISON_DEPTH,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Calls = BTreeMap<TargetResolution, u64>;
type Arc = (usize, usize);

#[derive(Default)]
pub(super) struct Neighborhood {
    pub names: BTreeMap<String, NameDetails>,
    pub links: BTreeMap<(String, String), Calls>,
}

impl Neighborhood {
    pub fn new(snapshot: &Snapshot<'_>, root: &Class, depth: u32) -> Self {
        let mut neighborhood = Self::default();
        if snapshot.has_class(root) {
            neighborhood.add_class(snapshot, root);
        }
        if !snapshot.classes.contains_key(root) {
            return neighborhood;
        }
        let calls = ClassCalls::new(snapshot, root, depth);
        for ((from, to), counts) in calls.links {
            let from_id = neighborhood.add_class(snapshot, calls.classes[from]);
            let to_id = neighborhood.add_class(snapshot, calls.classes[to]);
            neighborhood.links.insert((from_id, to_id), counts);
        }
        neighborhood
    }

    fn add_class(&mut self, snapshot: &Snapshot<'_>, class: &Class) -> String {
        let id = class.id();
        self.names
            .entry(id.clone())
            .or_insert_with(|| class.name_details(snapshot.class_visibility(class)));
        id
    }
}

struct ClassCalls<'a> {
    classes: Vec<&'a Class>,
    links: BTreeMap<Arc, Calls>,
}

impl<'a> ClassCalls<'a> {
    fn new(snapshot: &'a Snapshot<'_>, root: &Class, depth: u32) -> Self {
        let classes: Vec<_> = snapshot.classes.keys().collect();
        let root = classes
            .binary_search(&root)
            .expect("the root is a recorded class");
        // Immediate diagrams and previews need only calls incident to the root.
        let one_hop = depth == DEFAULT_CLASS_COMPARISON_DEPTH;
        let mut calls = Self {
            classes,
            links: class_links(snapshot, one_hop.then_some(root)),
        };
        if !one_hop {
            let reachable = calls.neighborhood(root, depth);
            calls.links.retain(|link, _| reachable.contains(link));
        }
        calls
    }

    fn neighborhood(&self, root: usize, depth: u32) -> BTreeSet<Arc> {
        let mut links = BTreeSet::new();
        // Separate walks prevent moving down a caller's other outgoing branches.
        for incoming in [true, false] {
            let mut adjacent = vec![Vec::new(); self.classes.len()];
            for &(from, to) in self.links.keys() {
                let (source, target) = if incoming { (to, from) } else { (from, to) };
                adjacent[source].push((target, (from, to)));
            }
            walk_calls(&adjacent, root, depth, &mut links);
        }
        links
    }
}

fn class_links(snapshot: &Snapshot<'_>, incident_to: Option<usize>) -> BTreeMap<Arc, Calls> {
    let owners = member_owners(snapshot);
    let mut links = BTreeMap::<Arc, Calls>::new();
    for edge in &snapshot.graph.edges {
        let (Some(from), Some(to)) = (owners[edge.from as usize], owners[edge.to as usize]) else {
            continue;
        };
        let in_scope = incident_to.is_none_or(|root| root == from || root == to);
        if from == to || !in_scope {
            continue;
        }
        let classification = edge.classification();
        if classification.kind == RelationshipKind::Call {
            *links
                .entry((from, to))
                .or_default()
                .entry(classification.resolution)
                .or_default() += u64::from(edge.count);
        }
    }
    links
}

fn member_owners(snapshot: &Snapshot<'_>) -> Vec<Option<usize>> {
    let mut owners = vec![None; snapshot.graph.nodes.len()];
    for (index, members) in snapshot.classes.values().enumerate() {
        for &member in members {
            owners[member] = Some(index);
        }
    }
    owners
}

fn walk_calls(adjacent: &[Vec<(usize, Arc)>], root: usize, depth: u32, links: &mut BTreeSet<Arc>) {
    let mut seen = vec![false; adjacent.len()];
    seen[root] = true;
    let mut queue = VecDeque::from([(root, 0)]);
    // Breadth-first ordering makes the first node at the limit the stopping point.
    while let Some((class, distance)) = queue.pop_front().filter(|&(_, distance)| distance < depth)
    {
        for &(neighbor, call) in &adjacent[class] {
            links.insert(call);
            if !std::mem::replace(&mut seen[neighbor], true) {
                queue.push_back((neighbor, distance + 1));
            }
        }
    }
}
