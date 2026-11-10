use serde::{Deserialize, Serialize};

/// Half-open UTF-8 byte offsets into the immutable analyzed source snapshot.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowInventory {
    pub version: u32,
    pub functions: Vec<FunctionFlowIR>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FunctionFlowIR {
    pub symbol: String,
    pub file: String,
    pub build: String,
    pub project: String,
    pub language: String,
    pub span: SourceSpan,
    pub body: Vec<Statement>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CallTarget {
    #[serde(
        default = "call_kind",
        skip_serializing_if = "is_call",
        deserialize_with = "target_kind"
    )]
    pub kind: String,
    pub symbol: String,
    pub certainty: String,
    pub provenance: String,
}

fn call_kind() -> String {
    "call".into()
}
fn is_call(kind: &String) -> bool {
    kind == "call"
}
fn target_kind<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let kind = String::deserialize(deserializer)?;
    if matches!(kind.as_str(), "call" | "navigation" | "http") {
        Ok(kind)
    } else {
        Err(serde::de::Error::custom("Unsupported flow target kind"))
    }
}
impl Default for CallTarget {
    fn default() -> Self {
        Self {
            kind: call_kind(),
            symbol: String::new(),
            certainty: String::new(),
            provenance: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CallSite {
    pub span: SourceSpan,
    pub label: String,
    pub targets: Vec<CallTarget>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statement {
    #[serde(default)]
    pub span: SourceSpan,
    #[serde(flatten)]
    pub kind: StatementKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatementKind {
    Evaluate {
        value: Expression,
    },
    Action {
        text: String,
        #[serde(default)]
        calls: Vec<CallSite>,
    },
    If {
        condition: Expression,
        #[serde(default)]
        then_branch: Vec<Statement>,
        #[serde(default)]
        else_branch: Vec<Statement>,
    },
    Loop {
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        condition: Option<Expression>,
        #[serde(default)]
        body: Vec<Statement>,
        #[serde(default)]
        update: Vec<Statement>,
        #[serde(default = "yes")]
        test_first: bool,
    },
    Switch {
        #[serde(default)]
        label: Option<String>,
        subject: Expression,
        #[serde(default)]
        cases: Vec<SwitchCase>,
    },
    Try {
        #[serde(default)]
        resources: Vec<Statement>,
        #[serde(default)]
        body: Vec<Statement>,
        #[serde(default)]
        catches: Vec<CatchClause>,
        #[serde(default)]
        finally: Vec<Statement>,
        #[serde(default)]
        cleanup: Vec<Statement>,
    },
    Return {
        #[serde(default)]
        value: Option<Expression>,
    },
    Throw {
        value: Expression,
    },
    Yield {
        #[serde(default)]
        value: Option<Expression>,
    },
    Break {
        #[serde(default)]
        label: Option<String>,
    },
    Continue {
        #[serde(default)]
        label: Option<String>,
    },
    Block {
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        body: Vec<Statement>,
    },
    Region {
        label: String,
        #[serde(default)]
        deferred: bool,
        #[serde(default)]
        body: Vec<Statement>,
    },
    Unsupported {
        text: String,
        reason: String,
    },
}
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SwitchCase {
    pub labels: Vec<String>,
    pub test: Option<Expression>,
    pub span: SourceSpan,
    pub is_default: bool,
    pub falls_through: bool,
    pub body: Vec<Statement>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatchClause {
    pub parameter: String,
    pub catch_all: bool,
    pub body: Vec<Statement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expression {
    Sequence {
        items: Vec<Expression>,
        #[serde(default)]
        span: SourceSpan,
    },
    Switch {
        subject: Box<Expression>,
        #[serde(default)]
        cases: Vec<SwitchCase>,
        #[serde(default)]
        span: SourceSpan,
    },
    Value {
        text: String,
        #[serde(default)]
        span: SourceSpan,
        #[serde(default)]
        calls: Vec<CallSite>,
        #[serde(default)]
        regions: Vec<Statement>,
    },
    And {
        left: Box<Expression>,
        right: Box<Expression>,
        #[serde(default)]
        span: SourceSpan,
    },
    Or {
        left: Box<Expression>,
        right: Box<Expression>,
        #[serde(default)]
        span: SourceSpan,
    },
    Conditional {
        condition: Box<Expression>,
        when_true: Box<Expression>,
        when_false: Box<Expression>,
        #[serde(default)]
        span: SourceSpan,
    },
}
impl Expression {
    pub fn span(&self) -> SourceSpan {
        match self {
            Self::Sequence { span, .. }
            | Self::Switch { span, .. }
            | Self::Value { span, .. }
            | Self::And { span, .. }
            | Self::Or { span, .. }
            | Self::Conditional { span, .. } => *span,
        }
    }
    pub fn text(&self) -> String {
        match self {
            Self::Sequence { items, .. } => {
                items.iter().map(Self::text).collect::<Vec<_>>().join(", ")
            }
            Self::Switch { subject, .. } => format!("switch ({})", subject.text()),
            Self::Value { text, .. } => text.clone(),
            Self::And { left, right, .. } => format!("{} && {}", left.text(), right.text()),
            Self::Or { left, right, .. } => format!("{} || {}", left.text(), right.text()),
            Self::Conditional {
                condition,
                when_true,
                when_false,
                ..
            } => format!(
                "{} ? {} : {}",
                condition.text(),
                when_true.text(),
                when_false.text()
            ),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FlowStep {
    pub id: usize,
    pub kind: String,
    pub label: String,
    pub span: SourceSpan,
    pub calls: Vec<CallSite>,
    pub position: [f32; 2],
    pub size: [f32; 2],
}
#[derive(Clone, Debug, Default)]
pub struct FlowLink {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub kind: String,
    pub points: Vec<[f32; 2]>,
}
#[derive(Clone, Debug, Default)]
pub struct FlowRegion {
    pub label: String,
    pub deferred: bool,
    pub entry: usize,
    pub steps: Vec<usize>,
}
#[derive(Clone, Debug, Default)]
pub struct FlowChart {
    pub symbol: String,
    pub file: String,
    pub span: SourceSpan,
    pub entry: usize,
    pub steps: Vec<FlowStep>,
    pub links: Vec<FlowLink>,
    pub regions: Vec<FlowRegion>,
    pub size: [f32; 2],
    pub diagnostics: Vec<String>,
    pub partial: bool,
}
