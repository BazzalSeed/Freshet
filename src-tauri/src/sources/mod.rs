pub mod hackernews;
pub mod reddit;
pub mod github;
pub mod polymarket;

use std::collections::HashMap;
use std::sync::Arc;

use crate::model::SourceItem;

// ── HTTP seam ─────────────────────────────────────────────────────────────────

/// Minimal HTTP GET abstraction — one method, one string back.
/// The real implementation uses `reqwest::blocking`; tests swap in `FakeHttp`.
pub trait HttpClient: Send + Sync {
    fn get(&self, url: &str) -> anyhow::Result<String>;
}

/// Production HTTP client built on `reqwest::blocking`.
// UNVERIFIED: live path
pub struct ReqwestClient {
    inner: reqwest::blocking::Client,
}

impl ReqwestClient {
    pub fn new() -> anyhow::Result<Self> {
        // UNVERIFIED: live path
        let inner = reqwest::blocking::Client::builder()
            .user_agent("freshet/0.1")
            .build()?;
        Ok(Self { inner })
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        // UNVERIFIED: live path
        Self::new().expect("failed to build reqwest client")
    }
}

impl HttpClient for ReqwestClient {
    // UNVERIFIED: live path
    fn get(&self, url: &str) -> anyhow::Result<String> {
        let body = self.inner.get(url).send()?.text()?;
        Ok(body)
    }
}

/// Test-only fake HTTP client: returns pre-loaded bodies keyed by URL.
pub struct FakeHttp {
    pub responses: HashMap<String, String>,
}

impl FakeHttp {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    pub fn with(mut self, url: impl Into<String>, body: impl Into<String>) -> Self {
        self.responses.insert(url.into(), body.into());
        self
    }
}

impl HttpClient for FakeHttp {
    fn get(&self, url: &str) -> anyhow::Result<String> {
        self.responses
            .get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("FakeHttp: no response registered for URL: {url}"))
    }
}

// ── SourceProvider trait ──────────────────────────────────────────────────────

pub trait SourceProvider: Send + Sync {
    /// The channel name, e.g. "hackernews".
    fn channel(&self) -> &str;
    /// Fetch items for `topic`, returning at most `limit` entries.
    fn fetch(&self, topic: &str, limit: usize) -> anyhow::Result<Vec<SourceItem>>;
}

// ── Test-only fake provider ───────────────────────────────────────────────────

/// Used in engine tests to inject canned items without touching HTTP.
pub struct FakeSourceProvider {
    pub channel_name: String,
    pub items: Vec<SourceItem>,
    /// When true, `fetch` returns an error instead.
    pub fail: bool,
}

impl FakeSourceProvider {
    pub fn new(channel_name: impl Into<String>, items: Vec<SourceItem>) -> Self {
        Self {
            channel_name: channel_name.into(),
            items,
            fail: false,
        }
    }

    pub fn failing(channel_name: impl Into<String>) -> Self {
        Self {
            channel_name: channel_name.into(),
            items: vec![],
            fail: true,
        }
    }
}

impl SourceProvider for FakeSourceProvider {
    fn channel(&self) -> &str {
        &self.channel_name
    }

    fn fetch(&self, _topic: &str, _limit: usize) -> anyhow::Result<Vec<SourceItem>> {
        if self.fail {
            anyhow::bail!("FakeSourceProvider: simulated failure for channel '{}'", self.channel_name)
        }
        Ok(self.items.clone())
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

/// Build providers for each requested channel.  Unknown channel names are
/// silently skipped so the caller does not need to validate user input.
pub fn registry(
    channels: &[String],
    http: Arc<dyn HttpClient>,
) -> Vec<Box<dyn SourceProvider>> {
    let mut out: Vec<Box<dyn SourceProvider>> = Vec::new();
    for ch in channels {
        match ch.as_str() {
            "hackernews" => out.push(Box::new(hackernews::Provider::new(Arc::clone(&http)))),
            "reddit" => out.push(Box::new(reddit::Provider::new(Arc::clone(&http)))),
            "github" => out.push(Box::new(github::Provider::new(Arc::clone(&http)))),
            "polymarket" => out.push(Box::new(polymarket::Provider::new(Arc::clone(&http)))),
            _ => {} // unknown channel — skip gracefully
        }
    }
    out
}

// ── fetch_all ─────────────────────────────────────────────────────────────────

/// Run every provider sequentially, collect results, skip any that error.
///
/// Note: sequential is fine for v1 — parallelism (rayon / tokio) is a future
/// optimization if fetch latency becomes a bottleneck.
pub fn fetch_all(
    providers: &[Box<dyn SourceProvider>],
    topic: &str,
    limit: usize,
) -> Vec<SourceItem> {
    let mut items = Vec::new();
    for provider in providers {
        match provider.fetch(topic, limit) {
            Ok(mut fetched) => items.append(&mut fetched),
            Err(e) => {
                // Graceful degrade: log the error but keep going.
                eprintln!(
                    "[sources] provider '{}' failed (skipping): {e:#}",
                    provider.channel()
                );
            }
        }
    }
    items
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, source: &str) -> SourceItem {
        SourceItem {
            id: id.into(),
            source: source.into(),
            url: "https://example.com".into(),
            title: "Test item".into(),
            score: Some(1.0),
            snippet: "snippet".into(),
            created_at: None,
        }
    }

    // ── registry ──────────────────────────────────────────────────────────────

    #[test]
    fn registry_maps_all_known_channels() {
        let http = Arc::new(FakeHttp::new()) as Arc<dyn HttpClient>;
        let channels: Vec<String> = vec![
            "hackernews".into(),
            "reddit".into(),
            "github".into(),
            "polymarket".into(),
        ];
        let providers = registry(&channels, http);
        assert_eq!(providers.len(), 4);

        let channel_names: Vec<&str> = providers.iter().map(|p| p.channel()).collect();
        assert!(channel_names.contains(&"hackernews"));
        assert!(channel_names.contains(&"reddit"));
        assert!(channel_names.contains(&"github"));
        assert!(channel_names.contains(&"polymarket"));
    }

    #[test]
    fn registry_skips_unknown_channels() {
        let http = Arc::new(FakeHttp::new()) as Arc<dyn HttpClient>;
        let channels: Vec<String> = vec![
            "hackernews".into(),
            "twitter".into(),   // unknown
            "tiktok".into(),    // unknown
            "reddit".into(),
        ];
        let providers = registry(&channels, http);
        assert_eq!(providers.len(), 2, "only known channels should be registered");

        let names: Vec<&str> = providers.iter().map(|p| p.channel()).collect();
        assert!(names.contains(&"hackernews"));
        assert!(names.contains(&"reddit"));
    }

    #[test]
    fn registry_empty_channels_produces_empty_list() {
        let http = Arc::new(FakeHttp::new()) as Arc<dyn HttpClient>;
        let providers = registry(&[], http);
        assert!(providers.is_empty());
    }

    // ── fetch_all ─────────────────────────────────────────────────────────────

    #[test]
    fn fetch_all_merges_items_from_multiple_providers() {
        let providers: Vec<Box<dyn SourceProvider>> = vec![
            Box::new(FakeSourceProvider::new(
                "a",
                vec![make_item("a:1", "a"), make_item("a:2", "a")],
            )),
            Box::new(FakeSourceProvider::new(
                "b",
                vec![make_item("b:1", "b")],
            )),
        ];

        let items = fetch_all(&providers, "topic", 10);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn fetch_all_skips_failing_provider_returns_others() {
        let providers: Vec<Box<dyn SourceProvider>> = vec![
            Box::new(FakeSourceProvider::new(
                "ok",
                vec![make_item("ok:1", "ok"), make_item("ok:2", "ok")],
            )),
            Box::new(FakeSourceProvider::failing("bad")),
            Box::new(FakeSourceProvider::new(
                "ok2",
                vec![make_item("ok2:1", "ok2")],
            )),
        ];

        let items = fetch_all(&providers, "rust", 10);
        // Should get items from "ok" and "ok2" but not "bad".
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.source != "bad"));
    }

    #[test]
    fn fetch_all_with_all_failing_returns_empty() {
        let providers: Vec<Box<dyn SourceProvider>> = vec![
            Box::new(FakeSourceProvider::failing("x")),
            Box::new(FakeSourceProvider::failing("y")),
        ];
        let items = fetch_all(&providers, "topic", 10);
        assert!(items.is_empty());
    }
}
