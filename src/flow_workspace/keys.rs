//! Explicit source widget identity; single-flow keys remain compatible with existing navigation.
#[derive(Clone)]
pub(crate) enum SourceKeys {
    Single,
    Comparison(String),
}
impl SourceKeys {
    pub fn line(&self, index: usize) -> String {
        match self {
            Self::Single => format!("flow-source-line-{index}"),
            Self::Comparison(namespace) => format!("{namespace}-line-{index}"),
        }
    }
    pub fn text(&self, index: usize) -> String {
        match self {
            Self::Single => format!("flow-source-text-{index}"),
            Self::Comparison(namespace) => format!("{namespace}-text-{index}"),
        }
    }
}
