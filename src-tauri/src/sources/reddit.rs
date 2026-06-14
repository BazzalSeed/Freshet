use std::sync::Arc;

use anyhow::Context;
use serde::Deserialize;

use crate::model::SourceItem;
use super::HttpClient;

// ── API shapes ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RedditResponse {
    data: RedditListing,
}

#[derive(Debug, Deserialize)]
struct RedditListing {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    id: String,
    title: String,
    permalink: String,
    score: Option<i64>,
    #[allow(dead_code)]
    num_comments: Option<i64>,
    subreddit: Option<String>,
}

// ── Pure parser ───────────────────────────────────────────────────────────────

/// Parse Reddit search JSON into `SourceItem`s, sorted by score descending.
pub fn parse(json: &str) -> anyhow::Result<Vec<SourceItem>> {
    let resp: RedditResponse =
        serde_json::from_str(json).context("reddit: failed to parse response JSON")?;

    let mut items: Vec<SourceItem> = resp
        .data
        .children
        .into_iter()
        .map(|child| {
            let post = child.data;
            let score = post.score.unwrap_or(0);
            let subreddit = post.subreddit.as_deref().unwrap_or("unknown");
            SourceItem {
                id: format!("reddit:{}", post.id),
                source: "reddit".into(),
                url: format!("https://www.reddit.com{}", post.permalink),
                title: post.title,
                score: Some(score as f64),
                snippet: format!("r/{subreddit} · {score} ↑"),
                created_at: None,
            }
        })
        .collect();

    // Sort by score descending.
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
        "reddit"
    }

    fn fetch(&self, topic: &str, limit: usize) -> anyhow::Result<Vec<SourceItem>> {
        let encoded = urlencoded(topic);
        let url = format!(
            "https://www.reddit.com/search.json?q={encoded}&sort=top&t=month&limit={limit}"
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
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/reddit.json"),
        )
        .expect("reddit fixture missing")
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
        // Highest score in fixture is abc123 (4820).
        assert_eq!(items[0].id, "reddit:abc123");
    }

    #[test]
    fn parse_url_is_full_reddit_url() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(
            items[0].url,
            "https://www.reddit.com/r/rust/comments/abc123/rust_is_now_the_2_most_used_language/"
        );
    }

    #[test]
    fn parse_score_is_upvotes() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items[0].score, Some(4820.0));
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
    fn parse_snippet_format() {
        let items = parse(&fixture()).expect("parse");
        // Top item is in r/rust with 4820 upvotes.
        assert_eq!(items[0].snippet, "r/rust · 4820 ↑");
    }

    // ── fetch via FakeHttp ─────────────────────────────────────────────────

    #[test]
    fn fetch_uses_correct_url_and_parses() {
        let body = fixture();
        let topic = "rust";
        let limit = 25_usize;
        let expected_url = format!(
            "https://www.reddit.com/search.json?q={topic}&sort=top&t=month&limit={limit}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "reddit:abc123");
    }

    #[test]
    fn fetch_truncates_to_limit() {
        let body = fixture();
        let topic = "rust";
        let limit = 1_usize;
        let expected_url = format!(
            "https://www.reddit.com/search.json?q={topic}&sort=top&t=month&limit={limit}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 1);
    }
}
