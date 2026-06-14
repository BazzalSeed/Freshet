use std::sync::Arc;

use anyhow::Context;
use serde::Deserialize;

use crate::model::SourceItem;
use super::HttpClient;

// ── API shapes ────────────────────────────────────────────────────────────────

/// The Gamma API returns a top-level JSON array.
///
/// Real response shape (as of 2026-06): numeric fields (`volume`, `liquidity`)
/// arrive as **JSON strings**, not numbers. We parse them defensively with
/// `parse_f64_str`. Unknown fields are ignored via `#[serde(default)]` +
/// `deny_unknown_fields` is NOT set so new server fields don't break parsing.
#[derive(Debug, Deserialize)]
struct PolymarketMarket {
    id: String,
    question: String,
    slug: String,
    /// Trading volume (string on the wire, e.g. `"98234.5"`).
    #[serde(default)]
    volume: Option<serde_json::Value>,
    /// Liquidity (string on the wire). Used as score fallback when volume absent.
    #[serde(default)]
    liquidity: Option<serde_json::Value>,
    /// Market open date (ISO-8601 string). Mapped to `created_at`.
    #[serde(rename = "startDate", default)]
    start_date: Option<String>,
}

// ── Numeric-string helper ─────────────────────────────────────────────────────

/// Parse a `serde_json::Value` that may be a JSON string or a JSON number into
/// `f64`. Returns `None` on null, absent, or unparseable value.
fn parse_f64_val(val: &Option<serde_json::Value>) -> Option<f64> {
    match val.as_ref()? {
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

// ── Pure parser ───────────────────────────────────────────────────────────────

/// Parse Polymarket Gamma API JSON into `SourceItem`s, sorted by volume descending.
pub fn parse(json: &str) -> anyhow::Result<Vec<SourceItem>> {
    let markets: Vec<PolymarketMarket> =
        serde_json::from_str(json).context("polymarket: failed to parse response JSON")?;

    let mut items: Vec<SourceItem> = markets
        .into_iter()
        .map(|m| {
            // Volume is the primary score; fall back to liquidity if absent.
            let volume = parse_f64_val(&m.volume);
            let liquidity = parse_f64_val(&m.liquidity);
            let score = volume.or(liquidity).unwrap_or(0.0);

            let snippet = if let Some(v) = volume {
                format!("Vol ${v:.0}")
            } else if let Some(liq) = liquidity {
                format!("Liquidity ${liq:.0}")
            } else {
                "No volume data".to_string()
            };

            SourceItem {
                id: format!("polymarket:{}", m.id),
                source: "polymarket".into(),
                url: format!("https://polymarket.com/event/{}", m.slug),
                title: m.question,
                score: Some(score),
                snippet,
                created_at: m.start_date,
            }
        })
        .collect();

    // Sort by volume (score) descending.
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
        "polymarket"
    }

    fn fetch(&self, topic: &str, limit: usize) -> anyhow::Result<Vec<SourceItem>> {
        let encoded = urlencoded(topic);
        let url = format!(
            "https://gamma-api.polymarket.com/markets?closed=false&limit={limit}&order=volume&ascending=false&search={encoded}"
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
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/polymarket.json"),
        )
        .expect("polymarket fixture missing")
    }

    // ── parse ──────────────────────────────────────────────────────────────

    #[test]
    fn parse_returns_correct_count() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(items.len(), 3);
    }

    /// id is `polymarket:<id>` using the numeric id from the fixture.
    #[test]
    fn parse_source_qualified_id() {
        let items = parse(&fixture()).expect("parse");
        // Highest volume in fixture is id "2322685" (volume "98234.5").
        assert_eq!(items[0].id, "polymarket:2322685");
    }

    /// Second item by volume has id "42001" and volume "45612.0" (string on wire).
    #[test]
    fn parse_source_qualified_id_numeric() {
        let items = parse(&fixture()).expect("parse");
        let item = items
            .iter()
            .find(|i| i.id == "polymarket:42001")
            .expect("numeric id item not found");
        // Score is parsed from the string "45612.0".
        assert_eq!(item.score, Some(45612.0));
    }

    /// URL is built from the slug field.
    #[test]
    fn parse_url_uses_slug() {
        let items = parse(&fixture()).expect("parse");
        // Highest-volume item has slug "fifwc-eng-hrv-2026-06-17-exact-score-0-1".
        assert_eq!(
            items[0].url,
            "https://polymarket.com/event/fifwc-eng-hrv-2026-06-17-exact-score-0-1"
        );
    }

    /// Score is parsed from the volume string field.
    #[test]
    fn parse_score_from_string_volume() {
        let items = parse(&fixture()).expect("parse");
        // "98234.5" (string) → f64 98234.5
        assert_eq!(items[0].score, Some(98234.5));
    }

    #[test]
    fn parse_sorted_by_volume_descending() {
        let items = parse(&fixture()).expect("parse");
        let scores: Vec<f64> = items.iter().map(|i| i.score.unwrap_or(0.0)).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert_eq!(scores, sorted);
    }

    /// Snippet shows volume from the string-encoded field.
    #[test]
    fn parse_snippet_shows_volume() {
        let items = parse(&fixture()).expect("parse");
        assert!(
            items[0].snippet.contains("Vol $"),
            "expected 'Vol $' in snippet: {}",
            items[0].snippet
        );
    }

    /// created_at is populated from startDate.
    #[test]
    fn parse_created_at_from_start_date() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(
            items[0].created_at.as_deref(),
            Some("2026-06-01T00:00:00Z"),
            "created_at must come from startDate"
        );
    }

    /// When volume is absent, liquidity is used as score (string-encoded).
    #[test]
    fn parse_falls_back_to_liquidity_when_volume_absent() {
        let json = r#"[{"id":"99","question":"Q","slug":"q-slug","liquidity":"500.0"}]"#;
        let items = parse(json).expect("parse liquidity fallback");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].score, Some(500.0));
    }

    /// Numeric (non-string) volume values are also accepted (defensive).
    #[test]
    fn parse_numeric_volume_also_accepted() {
        let json = r#"[{"id":"99","question":"Q","slug":"q-slug","volume":1000.0}]"#;
        let items = parse(json).expect("parse numeric volume");
        assert_eq!(items[0].score, Some(1000.0));
    }

    // ── fetch via FakeHttp ─────────────────────────────────────────────────

    #[test]
    fn fetch_uses_correct_url_and_parses() {
        let body = fixture();
        let topic = "rust";
        let limit = 20_usize;
        let expected_url = format!(
            "https://gamma-api.polymarket.com/markets?closed=false&limit={limit}&order=volume&ascending=false&search={topic}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 3);
        // Highest-volume item (2322685) is first.
        assert_eq!(items[0].id, "polymarket:2322685");
    }

    #[test]
    fn fetch_truncates_to_limit() {
        let body = fixture();
        let topic = "rust";
        let limit = 1_usize;
        let expected_url = format!(
            "https://gamma-api.polymarket.com/markets?closed=false&limit={limit}&order=volume&ascending=false&search={topic}"
        );
        let http = Arc::new(FakeHttp::new().with(expected_url, body)) as Arc<dyn crate::sources::HttpClient>;
        let provider = Provider::new(http);
        let items = provider.fetch(topic, limit).expect("fetch");
        assert_eq!(items.len(), 1);
    }
}
