use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct ApiHealth {
    pub(crate) service: String,
    pub(crate) status: String,
    pub(crate) environment: String,
    pub(crate) storage_mode: String,
    pub(crate) semantic_index_mode: String,
    pub(crate) production_ready: bool,
    pub(crate) database_connected: bool,
    pub(crate) supabase_configured: bool,
    pub(crate) embedding_mode: String,
    pub(crate) embedding_model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiscoveryMode {
    Seed,
    RobotsSitemap,
    Sitemap,
    Rss,
    LinkCrawl,
    ExternalIndex,
}

impl DiscoveryMode {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Seed => "seed",
            Self::RobotsSitemap => "robots sitemap",
            Self::Sitemap => "sitemap",
            Self::Rss => "RSS / Atom",
            Self::LinkCrawl => "same-domain links",
            Self::ExternalIndex => "external candidates",
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct SourceDomain {
    pub(crate) id: Uuid,
    pub(crate) tenant_id: Uuid,
    pub(crate) name: String,
    pub(crate) base_url: String,
    pub(crate) host: String,
    pub(crate) include_subdomains: bool,
    pub(crate) seed_urls: Vec<String>,
    pub(crate) discovery_modes: Vec<DiscoveryMode>,
    pub(crate) max_pages_per_scan: usize,
    pub(crate) source_priority: f32,
    pub(crate) respect_robots: bool,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct PageIndexRecord {
    pub(crate) id: Uuid,
    pub(crate) source_id: Uuid,
    pub(crate) canonical_url: String,
    pub(crate) fetched_at: DateTime<Utc>,
    pub(crate) content_hash: String,
    pub(crate) title: Option<String>,
    pub(crate) summary: String,
    pub(crate) model: serde_json::Value,
    pub(crate) extractor_version: String,
    pub(crate) segment_count: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct MatchCandidate {
    pub(crate) id: Uuid,
    pub(crate) match_key: String,
    pub(crate) tenant_id: Uuid,
    pub(crate) alert_rule_id: Uuid,
    pub(crate) alert_rule_revision: u32,
    pub(crate) page_revision_id: Uuid,
    pub(crate) source_id: Uuid,
    pub(crate) canonical_url: String,
    pub(crate) content_hash: String,
    pub(crate) query_hash: String,
    pub(crate) model: serde_json::Value,
    pub(crate) score: f32,
    pub(crate) components: ScoreComponents,
    pub(crate) evidence: Vec<MatchEvidence>,
    pub(crate) state: String,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ScoreComponents {
    pub(crate) semantic: f32,
    pub(crate) lexical: f32,
    pub(crate) entity: f32,
    pub(crate) recency: f32,
    pub(crate) source_priority: f32,
    pub(crate) weights: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct MatchEvidence {
    pub(crate) page_segment_kind: String,
    pub(crate) page_text: String,
    pub(crate) query_segment_kind: String,
    pub(crate) similarity: f32,
    pub(crate) weighted_similarity: f32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct InboxQuery {
    pub(crate) min_score: Option<String>,
    pub(crate) source_id: Option<String>,
    pub(crate) state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboxFilter {
    pub(crate) min_score: f32,
    pub(crate) source_id: Option<Uuid>,
    pub(crate) state: Option<String>,
}

impl Default for InboxFilter {
    fn default() -> Self {
        Self {
            min_score: 0.0,
            source_id: None,
            state: None,
        }
    }
}

impl TryFrom<InboxQuery> for InboxFilter {
    type Error = String;

    fn try_from(query: InboxQuery) -> Result<Self, Self::Error> {
        let min_score = match query.min_score.as_deref().map(str::trim) {
            None | Some("") => 0.0,
            Some(value) => value
                .parse::<f32>()
                .map_err(|_| "Minimum score must be a number between 0 and 1.".to_owned())?,
        };
        if !min_score.is_finite() || !(0.0..=1.0).contains(&min_score) {
            return Err("Minimum score must be a finite number between 0 and 1.".into());
        }

        let source_id = match query.source_id.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => Some(
                Uuid::parse_str(value)
                    .map_err(|_| "Source filter must be a valid UUID.".to_owned())?,
            ),
        };

        let state = match query.state.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(value) => {
                let value = value.to_ascii_lowercase();
                const ALLOWED: [&str; 6] = [
                    "candidate",
                    "suppressed",
                    "approved",
                    "rejected",
                    "delivered",
                    "failed",
                ];
                if !ALLOWED.contains(&value.as_str()) {
                    return Err(format!(
                        "State filter must be one of: {}.",
                        ALLOWED.join(", ")
                    ));
                }
                Some(value)
            }
        };

        Ok(Self {
            min_score,
            source_id,
            state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_are_bounded_and_tenantless() {
        let filter = InboxFilter::try_from(InboxQuery {
            min_score: Some("0.72".into()),
            source_id: None,
            state: Some("candidate".into()),
        })
        .expect("valid filter");
        assert_eq!(filter.min_score, 0.72);
        assert_eq!(filter.state.as_deref(), Some("candidate"));
    }

    #[test]
    fn invalid_state_is_rejected() {
        let error = InboxFilter::try_from(InboxQuery {
            min_score: None,
            source_id: None,
            state: Some("send-now".into()),
        })
        .expect_err("mutation-like state must be rejected");
        assert!(error.contains("candidate"));
    }
}
