//! Deliberately bounded matching: unsupported routes remain potential competitors.
use super::RouteFact;

/// Validate before normalization; URL parsers can otherwise hide traversal/backslashes.
pub(super) fn path(url: &str) -> Option<&str> {
    let path = UrlPath(url.split(['?', '#']).next()?);
    path.valid().then_some(path.0)
}
struct UrlPath<'a>(&'a str);
impl UrlPath<'_> {
    fn valid(&self) -> bool {
        [
            self.root_relative(),
            self.safe_bytes(),
            self.safe_segments(),
        ]
        .into_iter()
        .all(|safe| safe)
    }
    fn template_shape(&self) -> bool {
        [
            self.root_relative(),
            self.safe_bytes(),
            !self.0.contains("//"),
        ]
        .into_iter()
        .all(|safe| safe)
    }
    fn root_relative(&self) -> bool {
        self.0.starts_with('/') && !self.0.starts_with("//")
    }
    fn safe_bytes(&self) -> bool {
        self.0.is_ascii() && !self.0.bytes().any(ambiguous_byte)
    }
    fn safe_segments(&self) -> bool {
        if self.0 == "/" {
            return true;
        }
        !self.0.ends_with('/')
            && !self
                .0
                .split('/')
                .skip(1)
                .any(|part| matches!(part, "" | "." | ".."))
    }
}
fn ambiguous_byte(byte: u8) -> bool {
    byte.is_ascii_control() || byte.is_ascii_whitespace() || matches!(byte, b'%' | b'\\')
}

pub(super) fn potential(route: &RouteFact, method: Option<&str>, request: &str) -> bool {
    if !method_matches(route, method) {
        return false;
    }
    let Some(template) = &route.path else {
        return true;
    };
    if !UrlPath(template).template_shape() {
        return true;
    }
    let insensitive = route.framework == "aspnet";
    let template = if insensitive && template != "/" {
        template.trim_end_matches('/')
    } else {
        template.as_str()
    };
    let pattern: Vec<_> = template.split('/').collect();
    let actual: Vec<_> = request.split('/').collect();
    segments_match(&pattern, &actual, insensitive)
}
fn method_matches(route: &RouteFact, method: Option<&str>) -> bool {
    // Spring owns automatic OPTIONS handling; an unconstrained method is not its handler.
    if matches!(
        (route.framework.as_str(), route.methods.is_empty(), method),
        ("spring", true, Some("OPTIONS"))
    ) {
        return false;
    }

    method.is_none_or(|method| {
        route.methods.is_empty() || route.methods.iter().any(|candidate| candidate == method)
    })
}
fn segments_match(pattern: &[&str], actual: &[&str], insensitive: bool) -> bool {
    for (index, segment) in pattern.iter().enumerate() {
        if unsupported(segment) {
            return prefix_match(&pattern[..index], actual, insensitive);
        }
        if !actual
            .get(index)
            .is_some_and(|value| segment_match(segment, value, insensitive))
        {
            return false;
        }
    }
    pattern.len() == actual.len()
}

fn segment_match(pattern: &str, value: &str, insensitive: bool) -> bool {
    if parameter(pattern) {
        !value.is_empty()
    } else {
        Segment(pattern).matches(Segment(value), insensitive)
    }
}

fn prefix_match(prefix: &[&str], actual: &[&str], insensitive: bool) -> bool {
    prefix.len() <= actual.len()
        && prefix.iter().zip(actual).all(|(left, right)| {
            parameter(left) || Segment(left).matches(Segment(right), insensitive)
        })
}
struct Segment<'a>(&'a str);
impl Segment<'_> {
    fn matches(self, other: Self, insensitive: bool) -> bool {
        if insensitive {
            self.0.eq_ignore_ascii_case(other.0)
        } else {
            self.0 == other.0
        }
    }
}

fn parameter(segment: &str) -> bool {
    segment.starts_with('{')
        && segment.ends_with('}')
        && segment.len() > 2
        && segment[1..segment.len() - 1]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
fn unsupported(segment: &str) -> bool {
    segment.contains(['[', ']', '*', '?', ':', '%', '\\'])
        || (segment.contains(['{', '}']) && !parameter(segment))
}

pub(super) fn supported(route: &RouteFact) -> bool {
    route.symbol.is_some()
        && route.restrictions.is_empty()
        && route.order == Some(0)
        && route.path.as_ref().is_some_and(|template| {
            UrlPath(template).template_shape() && !template.split('/').any(unsupported)
        })
}
fn literal(route: &RouteFact) -> bool {
    route.path.as_ref().is_some_and(|path| !path.contains('{'))
}
/// Only a verified default static-over-single-parameter distinction is ranked.
/// Other precedence cases are conservatively ambiguous.
pub(super) fn dominates(winner: &RouteFact, other: &RouteFact) -> bool {
    supported(other) && winner.framework == other.framework && literal(winner) && !literal(other)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn url_subset_rejects_ambiguous_normalization_before_query_split() {
        for value in [
            "//host/a", "/a/%2f", "/a\\b", "/a/../b", "/./a", "/a//b", "/a/", "/a b",
        ] {
            assert_eq!(path(value), None, "{value}");
        }
        assert_eq!(path("/items/one?q=%2F#part"), Some("/items/one"));
    }
}
