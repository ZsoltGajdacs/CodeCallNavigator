//! Declaration order controls deterministic BTreeMap grouping of completion routes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Completion {
    Normal,
    Return,
    Throw,
    Break(Option<String>),
    Continue(Option<String>),
    Yield,
}
#[derive(Clone)]
pub(super) struct Exit {
    pub(super) from: usize,
    pub(super) completion: Completion,
    pub(super) label: String,
    pub(super) kind: String,
}
impl Exit {
    pub(super) fn normal(from: usize) -> Self {
        Self {
            from,
            completion: Completion::Normal,
            label: String::new(),
            kind: "normal".into(),
        }
    }
    pub(super) fn with_completion(from: usize, completion: Completion) -> Self {
        Self {
            completion,
            ..Self::normal(from)
        }
    }
    pub(super) fn resume(mut self) -> Self {
        self.completion = Completion::Normal;
        self
    }
    pub(super) fn branch(mut self, label: &str, kind: &str) -> Self {
        self.label = label.into();
        self.kind = kind.into();
        self
    }
}

impl Completion {
    pub(super) fn route(&self) -> crate::flow::PendingRoute {
        use crate::flow::PendingRoute;
        match self {
            Self::Normal => PendingRoute::Normal,
            Self::Return => PendingRoute::Return,
            Self::Throw => PendingRoute::Throw,
            Self::Break(label) => PendingRoute::Break {
                label: label.clone(),
            },
            Self::Continue(label) => PendingRoute::Continue {
                label: label.clone(),
            },
            Self::Yield => PendingRoute::Yield,
        }
    }
}
