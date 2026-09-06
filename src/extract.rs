//! URL extraction: pane text in, URLs out. Pure; no herdr or terminal-browser knowledge.
//!
//! Designed for coding-agent output rather than ported from another tool: dev-server addresses
//! (`localhost:5173`) are first-class, and Markdown `[text](url)` and Wikipedia `Foo_(bar)` both
//! come out right.
//!
//! Algorithm: every rule lists its candidate spans per line; overlaps are resolved by start
//! position then rule priority (an earlier / higher-priority match consumes the span, so
//! `https://www.example.com` is one URL, not also a `www.` match); the surviving candidates are
//! normalised, ordered newest-first, and de-duplicated keeping the newest occurrence.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

/// Extraction rules in priority order (lower = wins on equal start).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    /// `https://`, `http://`, `ftp://`, `file://`.
    Scheme,
    /// `localhost:PORT`, `127.0.0.1:PORT`, `0.0.0.0:PORT`, `[::1]:PORT` (+ optional path).
    LocalDev,
    /// IPv4 with a port and/or path.
    Ipv4,
    /// `www.` domains.
    Www,
    /// `git@host:owner/repo.git` / `ssh://git@host/owner/repo.git`.
    GitSsh,
}

/// Characters that end a URL in terminal output: whitespace, quotes, backtick, angle brackets,
/// and the pipe (tables, shell). Closing brackets are handled afterwards by balance.
const TAIL: &str = r#"[^\s"'`<>|]"#;

static SCHEME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"(?i)\b(?:https?|ftp|file)://{TAIL}+")).expect("valid regex")
});
static LOCAL_DEV: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:\blocalhost|\b127\.0\.0\.1|\b0\.0\.0\.0|\[::1\]):(?P<port>\d{{1,5}})(?P<path>/{TAIL}*)?"
    ))
    .expect("valid regex")
});
static IPV4: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"\b(?P<host>\d{{1,3}}(?:\.\d{{1,3}}){{3}})(?::(?P<port>\d{{1,5}}))?(?P<path>/{TAIL}*)?"
    ))
    .expect("valid regex")
});
static WWW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)\bwww\.[a-z0-9-]+(?:\.[a-z0-9-]+)+(?::\d{{1,5}})?(?:/{TAIL}*)?"
    ))
    .expect("valid regex")
});
static GIT_SSH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"\b(?:ssh://)?git@(?P<host>[A-Za-z0-9.-]+)[:/](?P<path>{TAIL}+)"
    ))
    .expect("valid regex")
});
static ANSI_CSI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").expect("valid regex"));
static ANSI_OSC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?").expect("valid regex"));

/// Removes CSI (colours, cursor movement) and OSC (hyperlinks, titles) escape sequences.
#[must_use]
pub fn strip_ansi(line: &str) -> String {
    let no_osc = ANSI_OSC.replace_all(line, "");
    ANSI_CSI.replace_all(&no_osc, "").into_owned()
}

/// Drops trailing punctuation that belongs to the surrounding prose, and closing brackets that
/// have no matching opener inside the URL (`[x](https://a/b)` yields `https://a/b`, while
/// `https://en.wikipedia.org/wiki/Foo_(bar)` keeps its `)`).
#[must_use]
pub fn trim_trailing(mut s: &str) -> &str {
    loop {
        let Some(last) = s.chars().next_back() else {
            return s;
        };
        let cut = match last {
            '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' | '…' => true,
            ')' => s.matches('(').count() < s.matches(')').count(),
            ']' => s.matches('[').count() < s.matches(']').count(),
            '}' => s.matches('{').count() < s.matches('}').count(),
            _ => false,
        };
        if !cut {
            return s;
        }
        s = &s[..s.len() - last.len_utf8()];
    }
}

#[derive(Debug)]
struct Candidate {
    start: usize,
    end: usize,
    rule: Rule,
    url: String,
}

fn valid_ipv4(host: &str) -> bool {
    host.split('.')
        .all(|octet| octet.parse::<u16>().is_ok_and(|n| n <= 255))
}

fn candidates(line: &str) -> Vec<Candidate> {
    let mut found = Vec::new();

    for m in SCHEME.find_iter(line) {
        let url = trim_trailing(m.as_str());
        found.push(Candidate {
            start: m.start(),
            end: m.start() + url.len(),
            rule: Rule::Scheme,
            url: url.to_string(),
        });
    }

    for m in LOCAL_DEV.find_iter(line) {
        let matched = trim_trailing(m.as_str());
        let colon = matched.find(':').unwrap_or(matched.len());
        let host = match &matched[..colon] {
            "0.0.0.0" => "localhost",
            other => other,
        };
        found.push(Candidate {
            start: m.start(),
            end: m.start() + matched.len(),
            rule: Rule::LocalDev,
            url: format!("http://{host}{}", &matched[colon..]),
        });
    }

    for caps in IPV4.captures_iter(line) {
        let whole = caps.get(0).expect("whole match");
        let has_port = caps.name("port").is_some();
        let has_path = caps.name("path").is_some_and(|p| !p.as_str().is_empty());
        if !(has_port || has_path) || !valid_ipv4(&caps["host"]) {
            continue;
        }
        let matched = trim_trailing(whole.as_str());
        found.push(Candidate {
            start: whole.start(),
            end: whole.start() + matched.len(),
            rule: Rule::Ipv4,
            url: format!("http://{matched}"),
        });
    }

    for m in WWW.find_iter(line) {
        let matched = trim_trailing(m.as_str());
        found.push(Candidate {
            start: m.start(),
            end: m.start() + matched.len(),
            rule: Rule::Www,
            url: format!("https://{matched}"),
        });
    }

    for caps in GIT_SSH.captures_iter(line) {
        let whole = caps.get(0).expect("whole match");
        let path = trim_trailing(&caps["path"]);
        let path = path.strip_suffix(".git").unwrap_or(path);
        if path.is_empty() {
            continue;
        }
        found.push(Candidate {
            start: whole.start(),
            end: whole.end(),
            rule: Rule::GitSsh,
            url: format!("https://{}/{path}", &caps["host"]),
        });
    }

    found
}

/// Keeps the earliest-starting candidate at each position (rule priority breaks ties) and drops
/// anything that starts inside an accepted span.
fn resolve_overlaps(mut found: Vec<Candidate>) -> Vec<Candidate> {
    found.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.rule.cmp(&b.rule))
            .then(b.end.cmp(&a.end))
    });
    let mut accepted: Vec<Candidate> = Vec::new();
    let mut last_end = 0;
    for c in found {
        if c.start < last_end {
            continue;
        }
        last_end = c.end;
        accepted.push(c);
    }
    accepted
}

/// Extracts URLs from terminal text, newest (bottom of the pane) first, de-duplicated.
#[must_use]
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut in_order: Vec<String> = text
        .lines()
        .flat_map(|raw| resolve_overlaps(candidates(&strip_ansi(raw))))
        .map(|c| c.url)
        .collect();
    in_order.reverse();
    let mut seen = HashSet::new();
    in_order.retain(|u| seen.insert(u.clone()));
    in_order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_prose_punctuation_but_keeps_balanced_parens() {
        assert_eq!(trim_trailing("https://a/b."), "https://a/b");
        assert_eq!(trim_trailing("https://a/b),"), "https://a/b");
        assert_eq!(
            trim_trailing("https://en.wikipedia.org/wiki/Foo_(bar))."),
            "https://en.wikipedia.org/wiki/Foo_(bar)"
        );
        assert_eq!(trim_trailing("https://a/b…"), "https://a/b");
    }

    #[test]
    fn markdown_json_quotes_and_brackets_end_a_url() {
        assert_eq!(
            extract_urls(
                r#"see [docs](https://example.com/docs) and {"url":"https://x.io/a","n":1} <https://a/b> `https://e/f`|x"#
            ),
            vec![
                "https://e/f",
                "https://a/b",
                "https://x.io/a",
                "https://example.com/docs"
            ]
        );
    }

    #[test]
    fn scheme_wins_over_www_and_local_dev_inside_it() {
        assert_eq!(
            extract_urls("https://www.example.com/x and http://localhost:3000/api"),
            vec!["http://localhost:3000/api", "https://www.example.com/x"]
        );
    }

    #[test]
    fn dev_servers_and_ipv4_are_normalised_but_bare_hosts_are_not_urls() {
        assert_eq!(
            extract_urls(
                "listening on localhost:5173\n0.0.0.0:8080/health\n[::1]:4000 ok\napi at 10.0.0.5:8080/api."
            ),
            vec![
                "http://10.0.0.5:8080/api",
                "http://[::1]:4000",
                "http://localhost:8080/health",
                "http://localhost:5173",
            ]
        );
        assert!(
            extract_urls("localhost is fine; server 192.168.1.1 up; tool v1.2.3.4; 256.1.1.1:80")
                .is_empty()
        );
    }

    #[test]
    fn www_and_git_ssh() {
        assert_eq!(
            extract_urls(
                "visit www.example.com/path, clone git@github.com:a/b.git or ssh://git@gitlab.com/x/y.git"
            ),
            vec![
                "https://gitlab.com/x/y",
                "https://github.com/a/b",
                "https://www.example.com/path",
            ]
        );
    }

    #[test]
    fn newest_first_deduped_and_ansi_stripped() {
        assert_eq!(
            extract_urls("https://a/1\n\x1b[32mhttps://a/2\x1b[0m\nhttps://a/1\n"),
            vec!["https://a/1", "https://a/2"]
        );
    }
}
