use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CadenceMode {
    Manual,
    OnLaunch,
    Interval,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cadence {
    pub mode: CadenceMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_minutes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamStatus {
    Active,
    Paused,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamDescription {
    pub id: String,
    pub title: String,
    pub topic: String,
    pub sources: Vec<String>,
    pub cadence: Cadence,
    pub status: StreamStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamSummary {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
    pub changed_since_seen: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceItem {
    pub id: String,
    pub source: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamState {
    pub seen_item_ids: Vec<String>,
    pub last_checked_at: Option<String>,
    pub last_changed_at: Option<String>,
    pub doc_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub changed: bool,
    pub n_new: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    ClaudeCode,
    Codex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub kind: AgentKind,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// StreamDescription with Interval cadence round-trips and uses camelCase field names.
    #[test]
    fn stream_description_interval_round_trips_and_camel_case() {
        let desc = StreamDescription {
            id: "s1".into(),
            title: "Rust news".into(),
            topic: "Rust programming language".into(),
            sources: vec!["hackernews".into()],
            cadence: Cadence {
                mode: CadenceMode::Interval,
                interval_minutes: Some(1440),
            },
            status: StreamStatus::Active,
            created_at: "2026-06-14T00:00:00Z".into(),
        };

        let json = serde_json::to_string(&desc).expect("serialize");

        // Field names must be camelCase on the wire.
        assert!(json.contains("\"createdAt\""), "expected 'createdAt' in: {json}");
        assert!(!json.contains("\"created_at\""), "must not contain 'created_at' in: {json}");
        assert!(json.contains("\"intervalMinutes\""), "expected 'intervalMinutes' in: {json}");
        assert!(!json.contains("\"interval_minutes\""), "must not contain 'interval_minutes' in: {json}");

        // Cadence mode value must be snake_case.
        assert!(json.contains("\"interval\""), "expected cadence mode value 'interval' in: {json}");

        // Round-trip fidelity.
        let back: StreamDescription = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(desc, back);
    }

    /// StreamSummary with changed_since_seen=true and no last_checked_at omits the None field.
    #[test]
    fn stream_summary_omits_none_last_checked_at() {
        let summary = StreamSummary {
            id: "s2".into(),
            title: "AI weekly".into(),
            last_checked_at: None,
            changed_since_seen: true,
        };

        let json = serde_json::to_string(&summary).expect("serialize");

        assert!(json.contains("\"changedSinceSeen\""), "expected 'changedSinceSeen' in: {json}");
        assert!(!json.contains("\"lastCheckedAt\""), "None field must be omitted, got: {json}");
        assert!(!json.contains("\"changed_since_seen\""), "must not contain snake_case in: {json}");
    }

    /// Summary.n_new serializes as "nNew".
    #[test]
    fn summary_n_new_camel_case() {
        let s = Summary { changed: true, n_new: 3 };
        let json = serde_json::to_string(&s).expect("serialize");

        assert!(json.contains("\"nNew\""), "expected 'nNew' in: {json}");
        assert!(!json.contains("\"n_new\""), "must not contain 'n_new' in: {json}");
        assert!(json.contains("3"), "value 3 must appear in: {json}");
    }

    /// AgentKind::ClaudeCode → "claude_code"; CadenceMode::OnLaunch → "on_launch".
    #[test]
    fn enum_variants_serialize_snake_case() {
        let kind_json = serde_json::to_string(&AgentKind::ClaudeCode).expect("serialize AgentKind");
        assert_eq!(kind_json, "\"claude_code\"");

        let mode_json = serde_json::to_string(&CadenceMode::OnLaunch).expect("serialize CadenceMode");
        assert_eq!(mode_json, "\"on_launch\"");
    }

    /// StreamState::default() produces an object (empty vecs / None fields present as null or omitted).
    #[test]
    fn stream_state_default_is_well_formed() {
        let state = StreamState::default();
        // Fields with None should either be null or absent; seen_item_ids is an empty vec.
        assert_eq!(state.seen_item_ids, Vec::<String>::new());
        assert!(state.last_checked_at.is_none());
        assert!(state.last_changed_at.is_none());
        assert!(state.doc_digest.is_none());

        // Must at minimum round-trip without error.
        let json = serde_json::to_string(&state).expect("serialize default StreamState");
        let back: StreamState = serde_json::from_str(&json).expect("deserialize default StreamState");
        assert_eq!(state, back);
    }
}
