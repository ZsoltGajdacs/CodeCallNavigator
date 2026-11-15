//! Exact analyzed-source binding shared by adapters and read-only flow queries.
use super::{FunctionFlowIR, SourceSpan};
use anyhow::{Result, ensure};

pub fn validate_source(function: &FunctionFlowIR, content: &str) -> Result<()> {
    ensure!(
        function.span.start < function.span.end,
        "Function has no meaningful analyzed source range"
    );
    ensure!(
        content
            .get(function.span.start..function.span.end)
            .is_some(),
        "Function range does not match its analyzed UTF-8 source"
    );
    let ranges = SourceRanges {
        function: function.span,
        content,
    };
    for part in super::parts::Parts::new(function) {
        ranges.check(part.span())?;
        super::metadata::validate_part(part)?;
    }
    Ok(())
}

struct SourceRanges<'a> {
    function: SourceSpan,
    content: &'a str,
}
impl SourceRanges<'_> {
    fn check(&self, span: SourceSpan) -> Result<()> {
        if span == SourceSpan::default() {
            return Ok(());
        } // Version-1 unlocated/synthetic sentinel.
        ensure!(
            span.start >= self.function.start && span.end <= self.function.end,
            "Flow range is outside its callable's analyzed source"
        );
        ensure!(
            self.content.get(span.start..span.end).is_some(),
            "Flow range does not match its analyzed UTF-8 source"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn function(body: serde_json::Value) -> FunctionFlowIR {
        serde_json::from_value(json!({"span":{"start":2,"end":9},"body":body})).unwrap()
    }
    #[test]
    fn malformed_nested_ranges_are_rejected_but_unlocated_and_deferred_spans_remain_valid() {
        let content = "é😀abc";
        let invalid = function(
            json!([{"kind":"evaluate","span":{"start":2,"end":6},"value":{"kind":"value","text":"bad","span":{"start":3,"end":6}}}]),
        );
        assert!(validate_source(&invalid, content).is_err());
        let outside =
            function(json!([{"kind":"action","text":"outside","span":{"start":0,"end":2}}]));
        assert!(validate_source(&outside, content).is_err());
        let synthetic = function(
            json!([{"kind":"region","label":"deferred","deferred":true,"span":{"start":6,"end":7},"body":[{"kind":"action","text":"unlocated"},{"kind":"action","text":"earlier source","span":{"start":2,"end":6}}]}]),
        );
        validate_source(&synthetic, content).unwrap();
    }
    #[test]
    fn publication_source_validation_does_not_apply_the_query_display_cap() {
        let content = "x".repeat(17 * 1024 * 1024);
        let function = FunctionFlowIR {
            span: SourceSpan {
                start: 0,
                end: content.len(),
            },
            ..Default::default()
        };
        validate_source(&function, &content).unwrap();
    }
}
