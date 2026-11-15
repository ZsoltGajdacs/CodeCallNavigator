//! Structural identities never depend on spans, graph step IDs or traversal ordinals.
use crate::flow::{
    CallSite, CallTarget, FunctionFlowIR,
    parts::{Part, Parts},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IrField {
    Body,
    Value,
    Condition,
    ThenBranch,
    ElseBranch,
    Update,
    Subject,
    Cases,
    Resources,
    Catches,
    Finally,
    Cleanup,
    Regions,
    Items,
    Left,
    Right,
    WhenTrue,
    WhenFalse,
    Test,
    Calls,
}
impl IrField {
    fn list(self) -> bool {
        matches!(
            self,
            Self::Body
                | Self::ThenBranch
                | Self::ElseBranch
                | Self::Update
                | Self::Cases
                | Self::Resources
                | Self::Catches
                | Self::Finally
                | Self::Cleanup
                | Self::Regions
                | Self::Items
                | Self::Calls
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum IrPathComponent {
    Field { field: IrField },
    Index { index: usize },
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IrPath(pub Vec<IrPathComponent>);
impl IrPath {
    pub fn field(mut self, field: IrField) -> Self {
        self.0.push(IrPathComponent::Field { field });
        self
    }
    pub fn index(mut self, index: usize) -> Self {
        self.0.push(IrPathComponent::Index { index });
        self
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.0.first()
                == Some(&IrPathComponent::Field {
                    field: IrField::Body
                }),
            "IR path must start at body"
        );
        let mut list = false;
        for component in &self.0 {
            match component {
                IrPathComponent::Field { field } => {
                    ensure!(!list, "IR list path is missing its index");
                    list = field.list();
                }
                IrPathComponent::Index { .. } => {
                    ensure!(list, "IR index has no list field");
                    list = false;
                }
            }
        }
        Ok(())
    }
    fn ends_field(&self, field: IrField) -> bool {
        self.0.last() == Some(&IrPathComponent::Field { field })
    }
    fn ends_element(&self, field: IrField) -> bool {
        matches!(self.0.as_slice(), [.., IrPathComponent::Field{field:found}, IrPathComponent::Index{..}] if *found==field)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PendingRoute {
    Normal,
    Return,
    Throw,
    Break { label: Option<String> },
    Continue { label: Option<String> },
    Yield,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CleanupSite {
    Finally {
        block: IrPath,
    },
    Resource {
        resource: IrPath,
    },
    /// A cleanup list without per-resource acquisition (Try.resources is empty).
    Cleanup {
        block: IrPath,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CleanupCopy {
    pub site: CleanupSite,
    pub route: PendingRoute,
}
impl CleanupCopy {
    fn validate(&self) -> Result<()> {
        let valid = match &self.site {
            CleanupSite::Finally { block } => {
                block.validate()?;
                block.ends_field(IrField::Finally)
            }
            CleanupSite::Cleanup { block } => {
                block.validate()?;
                block.ends_field(IrField::Cleanup)
            }
            CleanupSite::Resource { resource } => {
                resource.validate()?;
                resource.ends_element(IrField::Resources)
            }
        };
        ensure!(
            valid,
            "Cleanup copy does not identify an original cleanup scope"
        );
        if let PendingRoute::Break { label: Some(label) }
        | PendingRoute::Continue { label: Some(label) } = &self.route
        {
            ensure!(!label.is_empty(), "Empty pending transfer label");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SiteAddress {
    pub path: IrPath,
    pub call_slot: usize,
    pub cleanup: Vec<CleanupCopy>,
}
impl SiteAddress {
    pub fn validate(&self) -> Result<()> {
        self.path.validate()?;
        ensure!(
            !matches!(self.path.0.last(), Some(IrPathComponent::Field { field }) if field.list()),
            "Site path must identify a concrete IR owner"
        );
        ensure!(
            !self.path.0.iter().any(|part| matches!(
                part,
                IrPathComponent::Field {
                    field: IrField::Calls
                }
            )),
            "Site path identifies its owner, not the calls array"
        );
        for copy in &self.cleanup {
            copy.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallChoice {
    pub site: SiteAddress,
    pub target: CallTarget,
}
impl CallChoice {
    pub(crate) fn validate(&self) -> Result<()> {
        self.site.validate()?;
        ensure!(
            !self.target.symbol.trim().is_empty(),
            "Call choice target identity is empty"
        );
        ensure!(
            matches!(self.target.kind.as_str(), "call" | "navigation" | "http"),
            "Unknown relationship kind"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OccurrencePath(pub Vec<CallChoice>);

pub struct LocatedCallSite<'a> {
    pub address: SiteAddress,
    pub site: &'a CallSite,
}
/// Structural listing in the existing validation walk's order; never execution order.
/// Cleanup contexts are empty here. Assembly supplies semantic replay routes later.
pub fn call_sites(function: &FunctionFlowIR) -> Vec<LocatedCallSite<'_>> {
    let mut parts = Parts::located(function);
    let mut sites = Vec::new();
    while let Some(located) = parts.next_located() {
        if let Part::Call { site, .. } = located.part {
            let mut path = parts.path(located.path);
            // Calls are pushed only as owner.calls[slot] by the shared typed child walk.
            let Some(IrPathComponent::Index { index: call_slot }) = path.0.pop() else {
                unreachable!("constructed call slot")
            };
            assert_eq!(
                path.0.pop(),
                Some(IrPathComponent::Field {
                    field: IrField::Calls
                })
            );
            sites.push(LocatedCallSite {
                address: SiteAddress {
                    path,
                    call_slot,
                    cleanup: Vec::new(),
                },
                site,
            });
        }
    }
    sites
}
