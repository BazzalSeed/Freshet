use std::sync::Arc;

use anyhow::Context;
use serde::Deserialize;

use crate::model::SourceItem;
use super::HttpClient;

// ── API shapes ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HnResponse {
    hits: Vec<HnHit>,
}

#[derive(Debug, Deserialize)]
struct HnHit {
    #[serde(rename = "objectID")]
    object_id: String,
    title: String,
    /// May be null for Ask/Show HN posts that have no external link.
    url: Option<String>,
    points: Option<i64>,
    num_comments: Option<i64>,
    created_at: Option<String>,
}

// ── Pure parser ───────────────────────────────────────────────────────────────

/// Parse Algolia HN search JSON into `SourceItem`s, sorted by points descending.
pub fn parse(json: &str) -> anyhow::Result<Vec<SourceItem>> {
    let resp: HnResponse =
        serde_json::from_str(json).context("hackernews: failed to parse response JSON")?;

    let mut items: Vec<SourceItem> = resp
        .hits
        .into_iter()
        .map(|hit| {
            let points = hit.points.unwrap_or(0);
            let comments = hit.num_comments.unwrap_or(0);
            let url = hit.url.unwrap_or_else(|| {
                format!("https://news.ycombinator.com/item?id={}", hit.object_id)
            });
            SourceItem {
                id: format!("hackernews:{}", hit.object_id),
                source: "hackernews".into(),
                url,
                title: hit.title,
                score: Some(points as f64),
                snippet: format!("{points} points · {comments} comments"),
                created_at: hit.created_at,
            }
        })
        .collect();

    // Sort by score descending (highest-ranked first).
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
        "hackernews"
    }

    fn fetch(&self, topic: &str, limit: usize) -> anyhow::Result<Vec<SourceItem>> {
        let encoded = urlencoded(topic);
        let url = format!(
            "https://hn.algolia.com/api/v1/search?query={encoded}&tags=story&hitsPerPage={limit}"
        );
        let body = self.http.get(&url)?;
        let mut items = parse(&body)?;
        items.truncate(limit);
        Ok(items)
    }
}

/// Minimal percent-encoding for query parameters (encodes space as %20).
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
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/hackernews.json"),
        )
        .expect("hackernews fixture missing")
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
        // The highest-scoring item in the fixture has objectID "40123456" (412 pts).
        assert_eq!(items[0].id, "hackernews:40123456");
    }

    #[test]
    fn parse_url_present_used_directly() {
        let items = parse(&fixture()).expect("parse");
        // Top item has a real URL.
        assert_eq!(
            items[0].url,
            "https://blog.rust-lang.org/2024/rust-2024-edition.html"
        );
    }

    #[test]
    fn parse_null_url_falls_back_to_hn_permalink() {
        let items = parse(&fixture()).expect("parse");
        // Second item after sort-desc is objectID 40098765 (287 pts) which has url=null.
        let null_url_item = items
            .iter()
            .find(|i| i.id == "hackernews:40098765")
            .expect("item with null url not found");
        assert_eq!(
            null_url_item.url,
            "https://news.ycombinator.com/item?id=40098765"
        );
    }

    #[test]
    fn parse_score_is_points() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items[0].score, Some(412.0));
    }

    #[test]
    fn parse_sorted_by_score_descending() {
        let items = parse(&fixture()).expect("parse");
        let scores: Vec<f64> = items.iter().map(|i| i.score.unwrap_or(0.0)).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert_eq!(scores, sorted, "items must be sorted by score desc");
    }

    #[test]
    fn parse_snippet_format() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items[0].snippet, "412 points · 140 comments");
    }

    #[test]
    fn parse_created_at_present() {
        let items = parse(&fixture()).expect("parse");
        assert!(items[0].created_at.is_some());
    }

    // ── fetch via FakeHttp ─────────────────────────────────────────────────

    #[test]
    fn fetch_uses_correct_url_and_parses() {
        let body = fixture();
        let topic = "rust";
        let limit = 10_usize;
        let expected_url = format!(
            "https://hn.algolia.com/api/v1/search?query={topic}&tags=story&hitsPerPage={limit}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "hackernews:40123456");
    }

    #[test]
    fn fetch_truncates_to_limit() {
        let body = fixture();
        let topic = "rust";
        let limit = 2_usize;
        let expected_url = format!(
            "https://hn.algolia.com/api/v1/search?query={topic}&tags=story&hitsPerPage={limit}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 2);
    }
}
