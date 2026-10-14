//! Pure find/replace engine, ported from `src/lib/editor/search.ts`.
//!
//! Literal searches are `$`-safe; regex searches support `$1`, `$&`, ...
//! Invalid regexes yield no matches instead of throwing.

use std::ops::Range;

use regex::{NoExpand, Regex, RegexBuilder};

#[derive(Clone, Copy, Default)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub use_regex: bool,
}

fn build_regex(query: &str, opts: SearchOptions) -> Option<Regex> {
    if query.is_empty() {
        return None;
    }
    let pattern = if opts.use_regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!opts.case_sensitive)
        .build()
        .ok()
}

/// All non-overlapping, non-empty matches of `query` in `content`, in order.
pub fn find_all(content: &str, query: &str, opts: SearchOptions) -> Vec<Range<usize>> {
    let Some(re) = build_regex(query, opts) else {
        return Vec::new();
    };
    re.find_iter(content)
        .filter(|m| m.start() != m.end())
        .map(|m| m.start()..m.end())
        .collect()
}

/// First match at or after `from`; wraps around to the first match when none.
/// Kept for parity with `search.ts`; exercised by tests.
#[allow(dead_code)]
pub fn find_next_from(
    content: &str,
    query: &str,
    from: usize,
    opts: SearchOptions,
) -> Option<Range<usize>> {
    let matches = find_all(content, query, opts);
    if matches.is_empty() {
        return None;
    }
    matches
        .iter()
        .find(|m| m.start >= from)
        .or_else(|| matches.first())
        .cloned()
}

/// Replace every match. Returns the new content and the replacement count.
pub fn replace_all(
    content: &str,
    query: &str,
    replacement: &str,
    opts: SearchOptions,
) -> (String, usize) {
    let Some(re) = build_regex(query, opts) else {
        return (content.to_string(), 0);
    };
    let count = find_all(content, query, opts).len();
    if count == 0 {
        return (content.to_string(), 0);
    }
    let result = if opts.use_regex {
        re.replace_all(content, replacement).into_owned()
    } else {
        re.replace_all(content, NoExpand(replacement)).into_owned()
    };
    (result, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_search_is_case_insensitive_by_default() {
        let matches = find_all("Foo foo FOO", "foo", SearchOptions::default());
        assert_eq!(matches, vec![0..3, 4..7, 8..11]);
    }

    #[test]
    fn case_sensitive_search_respects_case() {
        let opts = SearchOptions {
            case_sensitive: true,
            use_regex: false,
        };
        let matches = find_all("Foo foo FOO", "foo", opts);
        assert_eq!(matches, vec![4..7]);
    }

    #[test]
    fn invalid_regex_returns_no_matches() {
        let opts = SearchOptions {
            case_sensitive: false,
            use_regex: true,
        };
        assert!(find_all("anything", "([", opts).is_empty());
    }

    #[test]
    fn literal_replacement_is_dollar_safe() {
        let (out, count) = replace_all("price", "price", "$100", SearchOptions::default());
        assert_eq!(out, "$100");
        assert_eq!(count, 1);
    }

    #[test]
    fn regex_replacement_supports_capture_groups() {
        let opts = SearchOptions {
            case_sensitive: false,
            use_regex: true,
        };
        let (out, count) = replace_all("first last", r"(\w+) (\w+)", "$2 $1", opts);
        assert_eq!(out, "last first");
        assert_eq!(count, 1);
    }

    #[test]
    fn find_next_from_wraps_around() {
        let content = "aa bb aa";
        let next = find_next_from(content, "aa", 3, SearchOptions::default());
        assert_eq!(next, Some(6..8));
        let wrapped = find_next_from(content, "aa", 7, SearchOptions::default());
        assert_eq!(wrapped, Some(0..2));
    }

    #[test]
    fn zero_length_matches_are_ignored() {
        let opts = SearchOptions {
            case_sensitive: false,
            use_regex: true,
        };
        assert!(find_all("abc", "x*", opts).is_empty());
    }
}
