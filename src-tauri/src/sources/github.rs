use std::sync::Arc;

use anyhow::Context;
use serde::Deserialize;

use crate::model::SourceItem;
use super::HttpClient;

// ── API shapes ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GithubResponse {
    items: Vec<GithubRepo>,
}

#[derive(Debug, Deserialize)]
struct GithubRepo {
    full_name: String,
    html_url: String,
    description: Option<String>,
    stargazers_count: Option<i64>,
    pushed_at: Option<String>,
}

// ── Pure parser ───────────────────────────────────────────────────────────────

/// Parse GitHub repository search JSON into `SourceItem`s, sorted by stars descending.
pub fn parse(json: &str) -> anyhow::Result<Vec<SourceItem>> {
    let resp: GithubResponse =
        serde_json::from_str(json).context("github: failed to parse response JSON")?;

    let mut items: Vec<SourceItem> = resp
        .items
        .into_iter()
        .map(|repo| {
            let stars = repo.stargazers_count.unwrap_or(0);
            let snippet = repo.description.clone().unwrap_or_default();
            SourceItem {
                id: format!("github:{}", repo.full_name),
                source: "github".into(),
                url: repo.html_url,
                title: repo.full_name,
                score: Some(stars as f64),
                snippet,
                created_at: repo.pushed_at,
            }
        })
        .collect();

    // Sort by score (stars) descending.
    items.sort_by(|a, b| {
        b.score
            .unwrap_or(0.0)
            .partial_cmp(&a.score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(items)
}

// ── Provider ──────────────────────────────────────────────────────────────────

pub struct Provider {
    http: Arc<dyn HttpClient>,
}

impl Provider {
    pub fn new(http: Arc<dyn HttpClient>) -> Self {
        Self { http }
    }
}

impl super::SourceProvider for Provider {
    fn channel(&self) -> &str {
        "github"
    }

    fn fetch(&self, topic: &str, limit: usize) -> anyhow::Result<Vec<SourceItem>> {
        let encoded = urlencoded(topic);
        let url = format!(
            "https://api.github.com/search/repositories?q={encoded}&sort=stars&order=desc&per_page={limit}"
        );
        let body = self.http.get(&url)?;
        let mut items = parse(&body)?;
        items.truncate(limit);
        Ok(items)
    }
}

/// Minimal percent-encoding for query parameters.
fn urlencoded(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            ' ' => vec!['%', '2', '0'],
            '&' => vec!['%', '2', '6'],
            '+' => vec!['%', '2', 'B'],
            '#' => vec!['%', '2', '3'],
            _ => vec![c],
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{FakeHttp, SourceProvider};
    use std::sync::Arc;

    fn fixture() -> String {
        std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/github.json"),
        )
        .expect("github fixture missing")
    }

    // ── parse ──────────────────────────────────────────────────────────────

    #[test]
    fn parse_returns_correct_count() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn parse_source_qualified_id() {
        let items = parse(&fixture()).expect("parse");
        // Highest-starred in fixture is rust-lang/rust (102500).
        assert_eq!(items[0].id, "github:rust-lang/rust");
    }

    #[test]
    fn parse_url_is_html_url() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items[0].url, "https://github.com/rust-lang/rust");
    }

    #[test]
    fn parse_score_is_stargazers_count() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items[0].score, Some(102500.0));
    }

    #[test]
    fn parse_sorted_by_score_descending() {
        let items = parse(&fixture()).expect("parse");
        let scores: Vec<f64> = items.iter().map(|i| i.score.unwrap_or(0.0)).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert_eq!(scores, sorted);
    }

    #[test]
    fn parse_null_description_becomes_empty_snippet() {
        let items = parse(&fixture()).expect("parse");
        // BurntSushi/ripgrep has description: null in fixture.
        let ripgrep = items
            .iter()
            .find(|i| i.id == "github:BurntSushi/ripgrep")
            .expect("ripgrep item not found");
        assert_eq!(ripgrep.snippet, "");
    }

    #[test]
    fn parse_description_used_as_snippet() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(
            items[0].snippet,
            "Empowering everyone to build reliable and efficient software."
        );
    }

    #[test]
    fn parse_created_at_is_pushed_at() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items[0].created_at, Some("2026-06-11T22:14:03Z".into()));
    }

    // ── fetch via FakeHttp ─────────────────────────────────────────────────

    #[test]
    fn fetch_uses_correct_url_and_parses() {
        let body = fixture();
        let topic = "rust";
        let limit = 30_usize;
        let expected_url = format!(
            "https://api.github.com/search/repositories?q={topic}&sort=stars&order=desc&per_page={limit}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "github:rust-lang/rust");
    }

    #[test]
    fn fetch_truncates_to_limit() {
        let body = fixture();
        let topic = "rust";
        let limit = 2_usize;
        let expected_url = format!(
            "https://api.github.com/search/repositories?q={topic}&sort=stars&order=desc&per_page={limit}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 2);
    }
}
