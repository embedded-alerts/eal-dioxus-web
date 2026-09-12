mod api;
mod config;
mod models;

use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use api::{ApiClient, ApiClientError};
use axum::{
    Json, Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use config::{AppEnvironment, InboxConfig};
use dioxus::prelude::*;
use models::{
    ApiHealth, InboxFilter, InboxQuery, MatchCandidate, MatchEvidence, PageIndexRecord,
    ScoreComponents, SourceDomain,
};
use serde::Serialize;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    api: Arc<ApiClient>,
    environment: AppEnvironment,
    tenant_id: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
struct CandidateView {
    candidate: MatchCandidate,
    source_name: String,
    page: Option<PageIndexRecord>,
}

#[derive(Debug, Clone, PartialEq)]
struct InboxData {
    health: Option<ApiHealth>,
    sources: Vec<SourceDomain>,
    candidates: Vec<CandidateView>,
    total_candidates: usize,
    total_pages: usize,
    filter: InboxFilter,
    errors: Vec<String>,
    environment: String,
    tenant_label: String,
}

fn inbox_view(data: InboxData) -> Element {
    let InboxData {
        health,
        sources,
        candidates,
        total_candidates,
        total_pages,
        filter,
        errors,
        environment,
        tenant_label,
    } = data;
    let filtered_count = candidates.len();
    let min_score_value = format!("{:.2}", filter.min_score);
    let source_filter_value = filter.source_id.map(|id| id.to_string()).unwrap_or_default();
    let state_filter_value = filter.state.clone().unwrap_or_default();

    rsx! {
        main { class: "shell",
            header { class: "topbar",
                div {
                    p { class: "eyebrow", "EMBEDDED ALERTS / DIOXUS SSR" }
                    h1 { "Candidate evidence, without accidental delivery." }
                }
                div { class: "runtime",
                    span { "{environment}" }
                    code { "{tenant_label}" }
                }
            }

            section { class: "hero",
                p {
                    "This inbox reads tenant-scoped match evidence from the owned index. Filters change only this rendered view; they cannot select a tenant, mutate a candidate, crawl a URL, or send provider traffic."
                }
                div { class: "lock-note",
                    strong { "Delivery lock engaged." }
                    span {
                        "Approval state, cooldowns, grouping, durable outbox records, receipts, retries, and dead letters remain owned by DEN-3460."
                    }
                }
            }

            section { class: "stats", aria_label: "Inbox summary",
                article { strong { "{sources.len()}" } span { "source boundaries" } }
                article { strong { "{total_pages}" } span { "indexed revisions" } }
                article { strong { "{total_candidates}" } span { "total candidates" } }
                article { strong { "{filtered_count}" } span { "visible candidates" } }
                if let Some(health) = health {
                    {health_card(health)}
                }
            }

            for error in errors {
                div { class: "notice notice--error", role: "status",
                    strong { "Read-model boundary error" }
                    p { "{error}" }
                }
            }

            section { class: "panel filter-panel",
                div { class: "panel-head",
                    div {
                        p { class: "section-number", "01 / VIEW FILTERS" }
                        h2 { "Narrow evidence, never authority" }
                    }
                    span { class: "pill", "GET-only" }
                }
                form { method: "get", action: "/", class: "filter-form",
                    label {
                        span { "Minimum combined score" }
                        input {
                            r#type: "number",
                            name: "min_score",
                            min: "0",
                            max: "1",
                            step: "0.01",
                            value: "{min_score_value}"
                        }
                    }
                    label {
                        span { "Source UUID" }
                        input {
                            r#type: "text",
                            name: "source_id",
                            value: "{source_filter_value}",
                            placeholder: "optional"
                        }
                    }
                    label {
                        span { "Candidate state" }
                        input {
                            r#type: "text",
                            name: "state",
                            value: "{state_filter_value}",
                            placeholder: "candidate"
                        }
                    }
                    button { r#type: "submit", "Apply read-only filter" }
                    a { href: "/", "Clear" }
                }
            }

            section { class: "panel",
                div { class: "panel-head",
                    div {
                        p { class: "section-number", "02 / CANDIDATE INBOX" }
                        h2 { "Explainable matches" }
                    }
                    span { class: "pill pill--locked", "no mutation routes" }
                }
                if candidates.is_empty() {
                    {empty_state(
                        "No candidates match this view",
                        "Create candidates through an immutable alert-rule revision after indexing policy-approved pages, or broaden these read-only filters."
                    )}
                }
                div { class: "candidate-list",
                    for item in candidates {
                        {candidate_card(item)}
                    }
                }
            }

            section { class: "panel",
                div { class: "panel-head",
                    div {
                        p { class: "section-number", "03 / SOURCE BOUNDARY" }
                        h2 { "Where indexed pages may originate" }
                    }
                    a { class: "quiet-link", href: "/", "refresh" }
                }
                if sources.is_empty() {
                    {empty_state(
                        "No registered sources",
                        "An operator must register an exact public domain before a page can enter the owned index."
                    )}
                }
                div { class: "source-grid",
                    for source in sources {
                        {source_card(source)}
                    }
                }
            }
        }
    }
}

fn health_card(health: ApiHealth) -> Element {
    let detail = format!(
        "{} · {} · {} · db:{} · supabase:{} · production:{}",
        health.environment,
        health.storage_mode,
        health.semantic_index_mode,
        health.database_connected,
        health.supabase_configured,
        health.production_ready,
    );
    let embedding = format!("{} / {}", health.embedding_mode, health.embedding_model);
    rsx! {
        article { class: "health-card",
            strong { "{health.status}" }
            span { "{health.service}" }
            small { "{detail}" }
            code { "{embedding}" }
        }
    }
}

fn source_card(source: SourceDomain) -> Element {
    let class = if source.enabled {
        "source-card"
    } else {
        "source-card source-card--disabled"
    };
    let modes = source
        .discovery_modes
        .iter()
        .map(|mode| mode.label())
        .collect::<Vec<_>>()
        .join(" · ");
    let seeds = if source.seed_urls.is_empty() {
        "API default".to_owned()
    } else {
        source.seed_urls.join(" · ")
    };
    let source_id = short_uuid(source.id);
    let tenant_id = short_uuid(source.tenant_id);
    let priority = percent(source.source_priority);
    let subdomains = yes_no(source.include_subdomains);
    let enabled = yes_no(source.enabled);
    let robots = if source.respect_robots {
        "required"
    } else {
        "unsafe"
    };

    rsx! {
        article { class: "{class}", key: "{source.id}",
            div { class: "card-head",
                div {
                    h3 { "{source.name}" }
                    a {
                        href: "{source.base_url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "{source.host}"
                    }
                }
                strong { "{priority}" }
            }
            p { "{modes}" }
            dl {
                div { dt { "source" } dd { code { "{source_id}" } } }
                div { dt { "tenant" } dd { code { "{tenant_id}" } } }
                div { dt { "budget" } dd { "{source.max_pages_per_scan} pages" } }
                div { dt { "subdomains" } dd { "{subdomains}" } }
                div { dt { "robots" } dd { "{robots}" } }
                div { dt { "enabled" } dd { "{enabled}" } }
            }
            small { "Seeds: {seeds}" }
        }
    }
}

fn candidate_card(item: CandidateView) -> Element {
    let candidate = item.candidate;
    let candidate_id = short_uuid(candidate.id);
    let rule_id = short_uuid(candidate.alert_rule_id);
    let page_id = short_uuid(candidate.page_revision_id);
    let source_id = short_uuid(candidate.source_id);
    let score = percent(candidate.score);
    let created = timestamp(candidate.created_at);
    let model = compact_json(&candidate.model);
    let weights = compact_json(&candidate.components.weights);

    rsx! {
        article { class: "candidate-card", key: "{candidate.id}",
            div { class: "candidate-score",
                strong { "{score}" }
                span { "{candidate.state}" }
            }
            div { class: "candidate-body",
                p { class: "kicker",
                    "candidate {candidate_id} · {item.source_name} · rule {rule_id}@{candidate.alert_rule_revision}"
                }
                h3 {
                    a {
                        href: "{candidate.canonical_url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "Open matched page"
                    }
                }
                if let Some(page) = item.page {
                    {page_context(page)}
                }
                {score_breakdown(candidate.components, weights)}
                {evidence_list(candidate.evidence)}
                details {
                    summary { "Immutable identity and provenance" }
                    dl {
                        div { dt { "tenant" } dd { code { "{candidate.tenant_id}" } } }
                        div { dt { "page revision" } dd { code { "{page_id}" } } }
                        div { dt { "source" } dd { code { "{source_id}" } } }
                        div { dt { "match key" } dd { code { "{candidate.match_key}" } } }
                        div { dt { "content hash" } dd { code { "{candidate.content_hash}" } } }
                        div { dt { "query hash" } dd { code { "{candidate.query_hash}" } } }
                        div { dt { "model" } dd { code { "{model}" } } }
                        div { dt { "created" } dd { "{created}" } }
                    }
                }
            }
        }
    }
}

fn page_context(page: PageIndexRecord) -> Element {
    let title = page.title.unwrap_or_else(|| "Untitled page".into());
    let fetched = timestamp(page.fetched_at);
    let hash = short_hash(&page.content_hash);
    let model = compact_json(&page.model);
    let page_id = short_uuid(page.id);
    let source_id = short_uuid(page.source_id);

    rsx! {
        section { class: "page-context",
            p { class: "kicker", "page {page_id} · source {source_id}" }
            h4 {
                a {
                    href: "{page.canonical_url}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    "{title}"
                }
            }
            p { "{page.summary}" }
            small {
                "Fetched {fetched} · hash {hash} · {page.segment_count} segments · extractor {page.extractor_version} · model {model}"
            }
        }
    }
}

fn score_breakdown(score: ScoreComponents, weights: String) -> Element {
    rsx! {
        div { class: "score-grid", title: "weights: {weights}",
            {score_cell("semantic", score.semantic)}
            {score_cell("lexical", score.lexical)}
            {score_cell("entity", score.entity)}
            {score_cell("recency", score.recency)}
            {score_cell("source", score.source_priority)}
        }
    }
}

fn score_cell(label: &'static str, value: f32) -> Element {
    let rendered = percent(value);
    let meter_value = format!("{value:.4}");
    rsx! {
        div {
            span { "{label}" }
            strong { "{rendered}" }
            meter { min: "0", max: "1", value: "{meter_value}", "{rendered}" }
        }
    }
}

fn evidence_list(evidence: Vec<MatchEvidence>) -> Element {
    let count = evidence.len();
    rsx! {
        details { class: "evidence",
            summary { "{count} evidence segment(s)" }
            ol {
                for item in evidence {
                    li {
                        strong { "{item.page_segment_kind} ↔ {item.query_segment_kind}" }
                        span {
                            "{percent(item.weighted_similarity)} weighted / {percent(item.similarity)} raw"
                        }
                        p { "{item.page_text}" }
                    }
                }
            }
        }
    }
}

fn empty_state(title: &'static str, detail: &'static str) -> Element {
    rsx! {
        div { class: "empty",
            strong { "{title}" }
            p { "{detail}" }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = InboxConfig::from_env()?;
    let api = ApiClient::new(config.api_base_url.clone(), config.tenant_id)
        .context("configure Embedded Alerts API client")?;
    let state = AppState {
        api: Arc::new(api),
        environment: config.environment,
        tenant_id: config.tenant_id,
    };

    warn!(
        environment = state.environment.as_str(),
        tenant_context = "development_header",
        api_base_url = %config.api_base_url,
        "Dioxus candidate inbox is read-only and production startup is disabled"
    );

    let app = Router::new()
        .route("/", get(index))
        .route("/healthz", get(health))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind((config.host.as_str(), config.port))
        .await
        .with_context(|| format!("bind {}:{}", config.host, config.port))?;
    info!(address = %listener.local_addr()?, "Embedded Alerts Dioxus inbox listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn index(
    State(state): State<AppState>,
    Query(query): Query<InboxQuery>,
) -> Html<String> {
    let (filter, local_error) = match InboxFilter::try_from(query) {
        Ok(filter) => (filter, None),
        Err(error) => (InboxFilter::default(), Some(error)),
    };
    render_page(load_inbox(&state, filter, local_error).await)
}

#[derive(Debug, Serialize)]
struct InboxHealth {
    service: &'static str,
    status: &'static str,
    environment: &'static str,
    production_ready: bool,
    tenant_context: &'static str,
    api_reachable: bool,
    api_production_ready: Option<bool>,
}

async fn health(State(state): State<AppState>) -> Json<InboxHealth> {
    let api_health = state.api.health().await.ok();
    Json(InboxHealth {
        service: "eal-dioxus-web",
        status: if api_health.is_some() {
            "degraded"
        } else {
            "unavailable"
        },
        environment: state.environment.as_str(),
        production_ready: false,
        tenant_context: "development_header",
        api_reachable: api_health.is_some(),
        api_production_ready: api_health.map(|health| health.production_ready),
    })
}

async fn load_inbox(
    state: &AppState,
    filter: InboxFilter,
    local_error: Option<String>,
) -> InboxData {
    let (health, sources, pages, matches) = tokio::join!(
        state.api.health(),
        state.api.list_sources(),
        state.api.list_pages(),
        state.api.list_matches(),
    );

    let mut errors = Vec::new();
    if let Some(error) = local_error {
        errors.push(error);
    }
    let health = capture("health", health, &mut errors);
    let sources = capture("sources", sources, &mut errors).unwrap_or_default();
    let pages = capture("pages", pages, &mut errors).unwrap_or_default();
    let matches = capture("matches", matches, &mut errors).unwrap_or_default();

    let source_names: HashMap<Uuid, String> = sources
        .iter()
        .map(|source| (source.id, source.name.clone()))
        .collect();
    let page_by_id: HashMap<Uuid, PageIndexRecord> =
        pages.iter().cloned().map(|page| (page.id, page)).collect();
    let total_candidates = matches.len();
    let total_pages = pages.len();

    let mut candidates: Vec<CandidateView> = matches
        .into_iter()
        .filter(|candidate| candidate.score >= filter.min_score)
        .filter(|candidate| {
            filter
                .source_id
                .is_none_or(|source_id| candidate.source_id == source_id)
        })
        .filter(|candidate| {
            filter
                .state
                .as_deref()
                .is_none_or(|state| candidate.state == state)
        })
        .map(|candidate| CandidateView {
            source_name: source_names
                .get(&candidate.source_id)
                .cloned()
                .unwrap_or_else(|| "Unknown source".into()),
            page: page_by_id.get(&candidate.page_revision_id).cloned(),
            candidate,
        })
        .collect();
    candidates.sort_by(|left, right| {
        right
            .candidate
            .score
            .total_cmp(&left.candidate.score)
            .then_with(|| right.candidate.created_at.cmp(&left.candidate.created_at))
    });

    InboxData {
        health,
        sources,
        candidates,
        total_candidates,
        total_pages,
        filter,
        errors,
        environment: state.environment.as_str().into(),
        tenant_label: short_uuid(state.tenant_id),
    }
}

fn capture<T>(
    label: &str,
    result: Result<T, ApiClientError>,
    errors: &mut Vec<String>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("{label}: {} · {}", error.code, error.message));
            None
        }
    }
}

fn render_page(data: InboxData) -> Html<String> {
    let body = dioxus_ssr::render_element(inbox_view(data));
    Html(format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"color-scheme\" content=\"dark\"><title>Embedded Alerts · Candidate Inbox</title><style>{STYLES}</style></head><body>{body}</body></html>"
    ))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn compact_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "unknown".into())
}

fn short_uuid(value: Uuid) -> String {
    value.to_string().chars().take(8).collect()
}

fn short_hash(value: &str) -> String {
    if value.len() <= 16 {
        value.to_owned()
    } else {
        format!("{}…{}", &value[..8], &value[value.len() - 8..])
    }
}

fn timestamp(value: chrono::DateTime<chrono::Utc>) -> String {
    value.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn percent(value: f32) -> String {
    format!("{:.1}%", value.clamp(0.0, 1.0) * 100.0)
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

const STYLES: &str = r#"
:root{--bg:#0b1016;--panel:#111923;--panel2:#17212d;--ink:#e7eef6;--muted:#8291a2;--line:#293748;--blue:#66b8ff;--cyan:#7de0df;--red:#ff8279;--amber:#ffcc66;font-family:Inter,ui-sans-serif,system-ui,sans-serif;color:var(--ink);background:var(--bg)}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 85% 0,#14273b 0,transparent 35%),var(--bg)}a{color:inherit;text-decoration-color:var(--blue);text-underline-offset:.2em}code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;overflow-wrap:anywhere}.shell{width:min(1320px,calc(100% - 2rem));margin:auto;padding:3rem 0 7rem}.topbar{display:flex;justify-content:space-between;gap:2rem;align-items:end;border-bottom:1px solid var(--line);padding:2rem 0}.eyebrow,.section-number,.kicker{margin:0 0 .55rem;color:var(--cyan);font:700 .7rem/1.4 ui-monospace,monospace;letter-spacing:.14em;text-transform:uppercase}.topbar h1{max-width:900px;margin:0;font-size:clamp(3rem,7vw,6.6rem);line-height:.9;letter-spacing:-.06em;font-weight:620}.runtime{text-align:right;color:var(--muted);text-transform:uppercase;font-size:.72rem}.runtime span,.runtime code{display:block}.hero{display:grid;grid-template-columns:1.25fr .75fr;gap:4rem;padding:2rem 0}.hero>p{font-size:1.2rem;line-height:1.6}.lock-note{border-left:4px solid var(--red);background:#21171b;padding:1rem 1.2rem}.lock-note span{display:block;color:#d6a6a3;margin-top:.4rem}.stats{display:grid;grid-template-columns:repeat(5,1fr);border:1px solid var(--line);background:rgba(17,25,35,.76)}.stats article{min-height:125px;padding:1rem;border-right:1px solid var(--line);display:flex;flex-direction:column;justify-content:space-between}.stats article:last-child{border-right:0}.stats strong{font-size:1.8rem}.stats span,.stats small{color:var(--muted)}.health-card code{font-size:.64rem}.panel{border-top:1px solid var(--line);padding:2.5rem 0}.panel-head,.card-head{display:flex;justify-content:space-between;gap:1rem;align-items:start}.panel-head{margin-bottom:1.25rem}.panel h2{margin:0;font-size:clamp(1.8rem,4vw,3.4rem);letter-spacing:-.04em}.pill{border:1px solid var(--cyan);padding:.25rem .55rem;color:var(--cyan);font:.68rem ui-monospace,monospace;text-transform:uppercase}.pill--locked{border-color:var(--red);color:var(--red)}.filter-panel{background:linear-gradient(90deg,rgba(102,184,255,.06),transparent)}.filter-form{display:grid;grid-template-columns:1fr 1.4fr 1fr auto auto;gap:.75rem;align-items:end}.filter-form label>span{display:block;margin-bottom:.35rem;color:var(--muted);font-size:.68rem;text-transform:uppercase}.filter-form input{width:100%;border:1px solid var(--line);border-radius:0;background:var(--panel);color:var(--ink);padding:.75rem;font:inherit}.filter-form button,.filter-form a{border:1px solid var(--blue);background:var(--blue);color:#06111b;padding:.75rem 1rem;font:700 .8rem inherit;text-decoration:none}.filter-form a{background:transparent;color:var(--blue)}.candidate-list{display:grid;gap:1rem}.candidate-card{display:grid;grid-template-columns:125px minmax(0,1fr);gap:1.25rem;border:1px solid var(--line);background:var(--panel);padding:1rem}.candidate-score{border-right:1px solid var(--line)}.candidate-score strong,.candidate-score span{display:block}.candidate-score strong{font-size:1.75rem;color:var(--amber)}.candidate-score span{color:var(--muted);font-size:.7rem;text-transform:uppercase}.candidate-body h3,.source-card h3,.page-context h4{margin:.15rem 0 .5rem}.page-context{border-left:3px solid var(--blue);background:var(--panel2);padding:.8rem 1rem}.page-context p{line-height:1.5}.page-context small{color:var(--muted)}.score-grid{display:grid;grid-template-columns:repeat(5,1fr);gap:.5rem;margin:1rem 0}.score-grid>div{border-top:1px solid var(--line);padding-top:.35rem}.score-grid span,.score-grid strong{display:block}.score-grid span{color:var(--muted);font-size:.65rem;text-transform:uppercase}meter{width:100%;height:.35rem;accent-color:var(--amber)}.evidence li{margin-bottom:.75rem}.evidence li span{display:block;color:var(--muted);font-size:.72rem}.evidence li p{margin:.25rem 0;line-height:1.45}details{margin-top:.85rem}summary{cursor:pointer;color:var(--muted)}dl{display:grid;gap:.4rem;margin:1rem 0}dl>div{display:grid;grid-template-columns:140px minmax(0,1fr);gap:.75rem;border-top:1px solid var(--line);padding-top:.35rem}dt{color:var(--muted);font-size:.68rem;text-transform:uppercase}dd{margin:0}.source-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:1rem}.source-card{border:1px solid var(--line);background:var(--panel);padding:1rem}.source-card--disabled{opacity:.58}.source-card p,.source-card small{color:var(--muted)}.notice{border-left:4px solid var(--red);background:#21171b;padding:.9rem 1rem;margin:1rem 0}.notice p{margin:.35rem 0 0}.empty{border:1px dashed var(--line);padding:1rem;color:var(--muted)}.empty strong{color:var(--ink)}.quiet-link{font-size:.75rem;text-transform:uppercase}@media(max-width:1050px){.stats{grid-template-columns:repeat(3,1fr)}.filter-form{grid-template-columns:repeat(2,1fr)}.score-grid{grid-template-columns:repeat(2,1fr)}}@media(max-width:760px){.hero,.source-grid{grid-template-columns:1fr}.candidate-card{grid-template-columns:1fr}.candidate-score{border-right:0;border-bottom:1px solid var(--line);padding-bottom:.6rem}.topbar,.panel-head,.card-head{flex-direction:column}.stats,.filter-form{grid-template-columns:1fr}.stats article{border-right:0;border-bottom:1px solid var(--line)}}
"#;
