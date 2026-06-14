use std::sync::Arc;

use anyhow::Context;
use serde::Deserialize;

use crate::model::SourceItem;
use super::HttpClient;

// ── API shapes ────────────────────────────────────────────────────────────────

/// The Gamma API returns a top-level JSON array.
#[derive(Debug, Deserialize)]
struct PolymarketMarket {
    id: String,
    question: String,
    slug: String,
    /// Trading volume; may be absent in some API responses.
    volume: Option<f64>,
    /// Either a JSON-encoded string array `"[\"0.61\",\"0.39\"]"` or a real
    /// JSON array, or null.  We handle all three defensively.
    #[serde(rename = "outcomePrices")]
    outcome_prices: Option<serde_json::Value>,
}

// ── Outcome-prices helper ─────────────────────────────────────────────────────

/// Extract the YES probability from `outcomePrices`.
///
/// The field is returned as either:
/// - A JSON-encoded string: `"[\"0.61\",\"0.39\"]"`
/// - A real JSON array: `["0.61","0.39"]`
/// - `null` / absent
///
/// Returns `None` if the value is absent, null, or cannot be parsed.
fn parse_yes_prob(outcome_prices: &Option<serde_json::Value>) -> Option<f64> {
    let val = outcome_prices.as_ref()?;

    // Normalise: if it's a JSON string, parse that string as JSON.
    let arr: serde_json::Value = match val {
        serde_json::Value::String(s) => serde_json::from_str(s).ok()?,
        other => other.clone(),
    };

    // Now we expect an array whose first element is a numeric string.
    let arr = arr.as_array()?;
    let first = arr.first()?;
    match first {
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
            let volume = m.volume.unwrap_or(0.0);
            let yes_prob = parse_yes_prob(&m.outcome_prices);

            let snippet = match yes_prob {
                Some(p) => format!(
                    "Yes {:.0}% · Vol ${:.0}",
                    p * 100.0,
                    volume,
                ),
                None => format!("Vol ${volume:.0}"),
            };

            SourceItem {
                id: format!("polymarket:{}", m.id),
                source: "polymarket".into(),
                url: format!("https://polymarket.com/event/{}", m.slug),
                title: m.question,
                score: Some(volume),
                snippet,
                created_at: None,
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

    #[test]
    fn parse_source_qualified_id_hex() {
        let items = parse(&fixture()).expect("parse");
        // Highest volume in fixture is 0xabc123def456789 (98234.5).
        assert_eq!(items[0].id, "polymarket:0xabc123def456789");
    }

    #[test]
    fn parse_source_qualified_id_numeric() {
        let items = parse(&fixture()).expect("parse");
        // Second item has id "42001".
        let item = items
            .iter()
            .find(|i| i.id == "polymarket:42001")
            .expect("numeric id item not found");
        assert_eq!(item.score, Some(45612.0));
    }

    #[test]
    fn parse_url_uses_slug() {
        let items = parse(&fixture()).expect("parse");
        assert_eq!(
            items[0].url,
            "https://polymarket.com/event/rust-top-3-language-2027"
        );
    }

    #[test]
    fn parse_score_is_volume() {
        let items = parse(&fixture()).expect("parse");
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

    #[test]
    fn parse_outcome_prices_string_encoded() {
        // First fixture item has outcomePrices as a JSON-encoded string.
        let items = parse(&fixture()).expect("parse");
        // YES probability is 72% → snippet contains "Yes 72%".
        assert!(
            items[0].snippet.contains("Yes 72%"),
            "expected 'Yes 72%' in snippet: {}",
            items[0].snippet
        );
    }

    #[test]
    fn parse_null_outcome_prices_produces_vol_only_snippet() {
        let items = parse(&fixture()).expect("parse");
        // Third item (42002) has outcomePrices: null.
        let item = items
            .iter()
            .find(|i| i.id == "polymarket:42002")
            .expect("null-prices item not found");
        assert!(
            item.snippet.starts_with("Vol $"),
            "expected Vol-only snippet: {}",
            item.snippet
        );
        assert!(
            !item.snippet.contains("Yes"),
            "should not contain Yes: {}",
            item.snippet
        );
    }

    #[test]
    fn parse_outcome_prices_real_array() {
        // Build a synthetic fixture with a real JSON array (not string-encoded).
        let json = r#"[{"id":"99","question":"Q","slug":"q-slug","volume":1000.0,"outcomePrices":["0.55","0.45"]}]"#;
        let items = parse(json).expect("parse real array");
        assert_eq!(items.len(), 1);
        assert!(
            items[0].snippet.contains("Yes 55%"),
            "expected 'Yes 55%' in snippet: {}",
            items[0].snippet
        );
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
        assert_eq!(items[0].id, "polymarket:0xabc123def456789");
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
