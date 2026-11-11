//! Conservative source attribution for display labels; bytecode identities stay untouched.
use anyhow::{Result, ensure};

pub(super) struct SourceLambda {
    start: u32,
    end: u32,
    column: u32,
    context: String,
    assigned: String,
}

impl SourceLambda {
    pub fn parse(fields: &[String]) -> Result<Self> {
        let start = fields[3].parse()?;
        let end = fields[4].parse()?;
        let column = fields[5].parse()?;
        let end_column: u32 = fields[6].parse()?;
        ensure!(
            start > 0 && end >= start && column > 0 && end_column > 0,
            "Invalid Java lambda source range"
        );
        ensure!(
            start != end || end_column >= column,
            "Invalid Java lambda columns"
        );
        ensure!(
            !fields[8].is_empty() || !fields[7].is_empty(),
            "Missing Java lambda context"
        );
        Ok(Self {
            start,
            end,
            column,
            context: fields[7].clone(),
            assigned: fields[8].clone(),
        })
    }

    pub fn contains(&self, start: u32, end: u32) -> bool {
        start > 0 && end >= start && self.start <= start && end <= self.end
    }

    fn label(&self, sources: &[Self]) -> String {
        if !self.assigned.is_empty() {
            return self.assigned.clone();
        }
        let column = if sources.iter().filter(|s| s.start == self.start).count() > 1 {
            format!(":C{}", self.column)
        } else {
            String::new()
        };
        format!("{} · lambda at L{}{column}", self.context, self.start)
    }
}

pub(super) fn resolve(sources: &[SourceLambda], start: u32, end: u32) -> Option<String> {
    let mut matches = sources.iter().filter(|source| source.contains(start, end));
    let source = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(source.label(sources))
}

pub(super) fn method_name(signature: &str) -> &str {
    let method = signature
        .split_once(':')
        .map(|(_, name)| name)
        .unwrap_or(signature);
    method.split('(').next().unwrap_or(method)
}

pub(super) fn fallback(name: &str) -> String {
    let suffix = name.strip_prefix("lambda$").unwrap_or(name);
    let (context, ordinal) = suffix.rsplit_once('$').unwrap_or((suffix, "?"));
    let context = match context {
        "static" => "Static initializer",
        "new" => "Constructor / instance initializer",
        "" => "Anonymous",
        other => other,
    };
    format!("{context} · lambda #{ordinal}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(start: u32, end: u32, assigned: &str) -> SourceLambda {
        SourceLambda {
            start,
            end,
            column: 12,
            context: "validate".into(),
            assigned: assigned.into(),
        }
    }

    #[test]
    fn overlapping_and_unlocated_lambdas_cannot_borrow_names() {
        let sources = [source(10, 20, "outer"), source(12, 15, "inner")];
        assert_eq!(resolve(&sources, 12, 14), None);
        assert_eq!(resolve(&sources, 0, 0), None);
        assert_eq!(resolve(&sources, 10, 20).as_deref(), Some("outer"));
        assert_eq!(resolve(&sources, 19, 21), None);
        let same_line = [source(30, 30, "first"), source(30, 30, "second")];
        assert_eq!(resolve(&same_line, 30, 30), None);
    }

    #[test]
    fn anonymous_labels_include_columns_only_when_needed() {
        let sources = [source(10, 12, ""), source(10, 10, "named")];
        assert_eq!(
            resolve(&sources, 11, 12).as_deref(),
            Some("validate · lambda at L10:C12")
        );
        assert_eq!(
            fallback("lambda$static$11"),
            "Static initializer · lambda #11"
        );
        assert_eq!(fallback("lambda$validate$2"), "validate · lambda #2");
    }
}
