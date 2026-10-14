//! Pure find/replace engine, ported from `src/lib/editor/search.ts`.
//!
//! Literal searches are `$`-safe; regex searches expand capture references
//! (`$0`, `$1`, `${name}`, ...) like the `regex` crate. Invalid regexes yield
//! no matches instead of throwing.

use std::ops::Range;

use regex::{Regex, RegexBuilder};

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

/// Expand `replacement` for the single match at `range`:
/// regex mode expands `$1`, `$&`, ... like `replace_all`; literal mode is
/// `$`-safe. Falls back to the literal replacement when the regex does not
/// re-match the exact range.
pub fn expand_replacement(
    content: &str,
    query: &str,
    replacement: &str,
    range: &Range<usize>,
    opts: SearchOptions,
) -> String {
    if !opts.use_regex {
        return replacement.to_string();
    }
    let Some(re) = build_regex(query, opts) else {
        return replacement.to_string();
    };
    expand_with_regex(&re, content, range, replacement)
}

fn expand_with_regex(re: &Regex, content: &str, range: &Range<usize>, replacement: &str) -> String {
    let Some(caps) = re.captures_at(content, range.start) else {
        return replacement.to_string();
    };
    if caps.get(0).map(|m| m.start()..m.end()) != Some(range.clone()) {
        return replacement.to_string();
    }
    let mut expanded = String::new();
    caps.expand(replacement, &mut expanded);
    expanded
}

/// Replace every non-empty match. Returns the new content and the replacement
/// count (always equal to the number of matches replaced).
pub fn replace_all(
    content: &str,
    query: &str,
    replacement: &str,
    opts: SearchOptions,
) -> (String, usize) {
    let matches = find_all(content, query, opts);
    if matches.is_empty() {
        return (content.to_string(), 0);
    }
    let re = if opts.use_regex {
        build_regex(query, opts)
    } else {
        None
    };
    let mut out = String::with_capacity(content.len());
    let mut last = 0;
    for range in &matches {
        out.push_str(&content[last..range.start]);
        match &re {
            Some(re) => out.push_str(&expand_with_regex(re, content, range, replacement)),
            None => out.push_str(replacement),
        }
        last = range.end;
    }
    out.push_str(&content[last..]);
    (out, matches.len())
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

    #[test]
    fn replace_all_skips_zero_length_matches() {
        let opts = SearchOptions {
            case_sensitive: false,
            use_regex: true,
        };
        // `a*` also matches empty strings at offsets 0 and 2; only the
        // non-empty match is replaced and counted.
        let (out, count) = replace_all("ba", "a*", "X", opts);
        assert_eq!(out, "bX");
        assert_eq!(count, 1);
    }

    #[test]
    fn replace_all_count_matches_replaced_spans() {
        let opts = SearchOptions {
            case_sensitive: false,
            use_regex: true,
        };
        let (out, count) = replace_all("ab ab ab", "b", "[$0]", opts);
        assert_eq!(out, "a[b] a[b] a[b]");
        assert_eq!(count, 3);
    }

    #[test]
    fn expand_replacement_expands_captures_for_single_match() {
        let opts = SearchOptions {
            case_sensitive: false,
            use_regex: true,
        };
        let content = "first last";
        let range = 0..content.len();
        assert_eq!(
            expand_replacement(content, r"(\w+) (\w+)", "$2 $1", &range, opts),
            "last first"
        );
    }

    #[test]
    fn expand_replacement_is_literal_outside_regex_mode() {
        let content = "price";
        let range = 0..content.len();
        assert_eq!(
            expand_replacement(content, "price", "$100", &range, SearchOptions::default()),
            "$100"
        );
    }
}
