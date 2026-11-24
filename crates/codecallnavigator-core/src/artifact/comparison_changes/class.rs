//! Exact native class ownership and the saved TypeScript semantic owner chain.
use super::{NameDetails, stable_id};
use crate::{
    architecture::FileFacts,
    artifact::identity,
    model::{GraphData, Node, Visibility},
    search::ClassIdentity,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Class {
    pub owner: ClassIdentity,
    pub typescript_file: Option<String>,
    semantic: Option<String>,
}

impl Class {
    pub fn from_node(node: &Node) -> Option<Self> {
        if node.language != "typescript" {
            return (!node.container.is_empty()).then(|| Self {
                owner: ClassIdentity::from_node(node),
                typescript_file: None,
                semantic: None,
            });
        }
        let symbol = identity::portable_semantic(node, (&node.build, &node.file))?;
        let (semantic, name) = typescript_owner(symbol)?;
        Some(Self {
            owner: ClassIdentity {
                build: node.build.clone(),
                project: node.project.clone(),
                container: name,
            },
            typescript_file: Some(node.file.clone()),
            semantic: Some(semantic),
        })
    }

    pub fn from_declaration(file: &FileFacts, name: &str) -> Self {
        Self {
            owner: ClassIdentity {
                build: file.build.clone(),
                project: file.project.clone(),
                container: name.into(),
            },
            typescript_file: (file.language == "typescript").then(|| file.file.clone()),
            semantic: None,
        }
    }

    pub fn name(&self) -> &str {
        simple_name(&self.owner.container)
    }

    pub fn label(&self) -> &str {
        if self.typescript_file.is_some() {
            &self.owner.container
        } else {
            self.name()
        }
    }

    pub fn visibility(&self, graph: &GraphData, members: &[usize]) -> Option<Visibility> {
        if self.typescript_file.is_some() {
            return None;
        }
        members
            .iter()
            .find_map(|&index| graph.nodes[index].container_visibility)
    }

    pub fn name_details(&self, visibility: Option<Visibility>) -> NameDetails {
        NameDetails {
            label: self.label().to_owned(),
            description: self.owner.container.clone(),
            visibility,
        }
    }

    pub fn id(&self) -> String {
        let semantic = self.semantic.as_deref().unwrap_or(&self.owner.container);
        stable_id(
            "class",
            &[
                &self.owner.build,
                &self.owner.project,
                self.typescript_file.as_deref().unwrap_or_default(),
                semantic,
            ],
        )
    }

    /// A file-only declaration can confirm that a named class still exists if
    /// the other snapshot has no callable carrying its TypeScript owner key.
    pub fn matches_declaration(&self, declaration: &Self) -> bool {
        self.owner == declaration.owner && self.typescript_file == declaration.typescript_file
    }

    pub fn can_confirm(&self, recorded: &Self) -> bool {
        self == recorded || (self.semantic.is_none() && recorded.matches_declaration(self))
    }
}

pub(super) fn simple_name(name: &str) -> &str {
    crate::search::simple_class_name(name)
}

fn typescript_owner(mut symbol: String) -> Option<(String, String)> {
    // The adapter serializes each key as [base, occurrence], where base is
    // [owner, syntax kind, declaration name/role, signature/token fingerprint].
    for _ in 0..64 {
        let (base, _): (String, u32) = serde_json::from_str(&symbol).ok()?;
        let [owner, kind, name, signature]: [String; 4] = serde_json::from_str(&base).ok()?;
        if matches!(
            kind.as_str(),
            "ClassDeclaration" | "ClassExpression" | "InterfaceDeclaration"
        ) {
            return signature.is_empty().then_some((symbol, name));
        }
        symbol = owner;
    }
    None
}
