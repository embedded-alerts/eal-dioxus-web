use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceView {
    pub id: String,
    pub domain: String,
    pub status: String,
    pub include_subdomains: bool,
    pub respect_robots: bool,
    pub page_budget: u32,
    pub indexed_pages: u64,
    pub last_scan_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuleView {
    pub id: String,
    pub name: String,
    pub revision: u32,
    pub query_text: String,
    pub threshold: f32,
    pub candidate_count: u64,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateView {
    pub id: String,
    pub rule_name: String,
    pub page_title: String,
    pub canonical_url: String,
    pub source_domain: String,
    pub state: String,
    pub overall_score: f32,
    pub semantic_score: f32,
    pub lexical_score: f32,
    pub entity_score: f32,
    pub recency_score: f32,
    pub source_priority_score: f32,
    pub best_sentence: String,
    pub entities: Vec<String>,
    pub keywords: Vec<String>,
    pub model_label: String,
    pub discovered_label: String,
}

#[component]
pub fn SemanticConsole(
    tenant_name: String,
    sources: Vec<SourceView>,
    rules: Vec<RuleView>,
    candidates: Vec<CandidateView>,
    csrf_token: String,
) -> Element {
    rsx! {
        main { class: "semantic-console", "data-component": "semantic-console",
            header { class: "console-header",
                div {
                    p { class: "eyebrow", "Embedded Alerts / {tenant_name}" }
                    h1 { "Semantic monitoring console" }
                    p { class: "lede",
                        "Index approved public domains, describe what matters in natural language, and review explainable matches before delivery."
                    }
                }
                nav { class: "section-nav", aria_label: "Semantic console sections",
                    a { href: "#sources", "Sources" }
                    a { href: "#rules", "Alert rules" }
                    a { href: "#matches", "Match candidates" }
                }
            }

            section { id: "sources", class: "console-section",
                div { class: "section-heading",
                    div { p { class: "section-kicker", "Discovery boundary" } h2 { "Approved domains" } }
                    p { "Only configured public hosts are eligible. External indexes may suggest URLs, but every page is fetched and checked against this policy." }
                }
                form { class: "source-form panel", action: "/ui/sources", method: "post",
                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                    div { class: "field-grid",
                        label { span { "Public domain" } input { name: "domain", r#type: "text", maxlength: 253, required: true } }
                        label { span { "Seed URLs" } textarea { name: "seed_urls", rows: 3, maxlength: 8000 } }
                        label { span { "Pages per scan" } input { name: "page_budget", r#type: "number", min: 1, max: 5000, value: 250, required: true } }
                        label { span { "Source priority" } input { name: "priority", r#type: "number", min: 0, max: 100, value: 50, required: true } }
                    }
                    fieldset { class: "choice-row",
                        legend { "Policy" }
                        label { input { r#type: "checkbox", name: "include_subdomains", value: "true" } "Include subdomains" }
                        label { input { r#type: "checkbox", name: "respect_robots", value: "true", checked: true } "Enforce robots.txt" }
                        label { input { r#type: "checkbox", name: "discover_sitemaps", value: "true", checked: true } "Discover sitemaps" }
                        label { input { r#type: "checkbox", name: "discover_links", value: "true", checked: true } "Follow bounded same-domain links" }
                    }
                    button { class: "primary-action", r#type: "submit", "Register source" }
                }
                div { class: "card-grid",
                    for source in sources {
                        SourceCard { source, csrf_token: csrf_token.clone() }
                    }
                }
            }

            section { id: "rules", class: "console-section",
                div { class: "section-heading",
                    div { p { class: "section-kicker", "Semantic intent" } h2 { "Natural-language alert rules" } }
                    p { "The complete sentence remains the strongest representation. Keywords and proper nouns are companion evidence, not replacements." }
                }
                form { class: "rule-form panel", action: "/ui/alert-rules", method: "post",
                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                    div { class: "field-grid",
                        label { span { "Rule name" } input { name: "name", r#type: "text", maxlength: 120, required: true } }
                        label { class: "wide-field", span { "What should Embedded Alerts find?" } textarea { name: "query_text", rows: 4, minlength: 3, maxlength: 700, required: true } }
                        label { span { "Candidate threshold" } input { name: "threshold", r#type: "number", min: 0, max: 1, step: 0.01, value: 0.72, required: true } }
                    }
                    div { class: "form-actions",
                        button { class: "primary-action", r#type: "submit", "Create immutable revision" }
                        button { r#type: "submit", formaction: "/ui/query-preview", "Preview semantic views" }
                    }
                }
                div { class: "card-grid",
                    for rule in rules {
                        RuleCard { rule, csrf_token: csrf_token.clone() }
                    }
                }
            }

            section { id: "matches", class: "console-section",
                div { class: "section-heading",
                    div { p { class: "section-kicker", "Explainable ranking" } h2 { "Match candidates" } }
                    p { "Review semantic, lexical, entity, recency, and source-priority evidence before a candidate enters delivery." }
                }
                div { class: "match-list", aria_live: "polite",
                    for candidate in candidates {
                        CandidateCard { candidate, csrf_token: csrf_token.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn SourceCard(source: SourceView, csrf_token: String) -> Element {
    let subdomain_label = if source.include_subdomains {
        "Exact host + subdomains"
    } else {
        "Exact host only"
    };
    let robots_label = if source.respect_robots {
        "Robots enforced"
    } else {
        "Robots disabled"
    };
    let scan_url = format!("/ui/sources/{}/scan", source.id);
    let detail_url = format!("/sources/{}", source.id);
    let status_class = format!("status-pill status-{}", source.status);

    rsx! {
        article { class: "source-card panel",
            div { class: "card-heading",
                div { p { class: "domain-label", "{source.domain}" } h3 { "{subdomain_label}" } }
                span { class: "{status_class}", "{source.status}" }
            }
            dl { class: "metric-list",
                div { dt { "Indexed pages" } dd { "{source.indexed_pages}" } }
                div { dt { "Page budget" } dd { "{source.page_budget}" } }
                div { dt { "Robots" } dd { "{robots_label}" } }
                div { dt { "Last scan" } dd { "{source.last_scan_label}" } }
            }
            div { class: "card-actions",
                form { action: "{scan_url}", method: "post",
                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                    button { r#type: "submit", "Scan now" }
                }
                a { href: "{detail_url}", "Inspect pages" }
            }
        }
    }
}

#[component]
fn RuleCard(rule: RuleView, csrf_token: String) -> Element {
    let evaluate_url = format!("/ui/alert-rules/{}/evaluate", rule.id);
    let detail_url = format!("/alert-rules/{}", rule.id);
    let status = if rule.enabled { "enabled" } else { "paused" };
    let status_class = format!("status-pill status-{status}");
    let threshold = format!("{:.0}%", clamp_score(rule.threshold) * 100.0);

    rsx! {
        article { class: "rule-card panel",
            div { class: "card-heading",
                div { p { class: "revision-label", "Revision {rule.revision}" } h3 { "{rule.name}" } }
                span { class: "{status_class}", "{status}" }
            }
            blockquote { "{rule.query_text}" }
            dl { class: "metric-list",
                div { dt { "Threshold" } dd { "{threshold}" } }
                div { dt { "Candidates" } dd { "{rule.candidate_count}" } }
            }
            div { class: "card-actions",
                form { action: "{evaluate_url}", method: "post",
                    input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
                    button { r#type: "submit", "Evaluate new pages" }
                }
                a { href: "{detail_url}", "Revision history" }
            }
        }
    }
}

#[component]
fn CandidateCard(candidate: CandidateView, csrf_token: String) -> Element {
    let approve_url = format!("/ui/matches/{}/approve", candidate.id);
    let suppress_url = format!("/ui/matches/{}/suppress", candidate.id);
    let dismiss_url = format!("/ui/matches/{}/dismiss", candidate.id);
    let detail_url = format!("/matches/{}", candidate.id);
    let status_class = format!("status-pill status-{}", candidate.state);
    let overall_score = clamp_score(candidate.overall_score);
    let score_percent = format!("{:.0}", overall_score * 100.0);

    rsx! {
        article { class: "match-card panel",
            div { class: "match-heading",
                div {
                    p { class: "match-context", "{candidate.rule_name} · {candidate.source_domain}" }
                    h3 { a { href: "{candidate.canonical_url}", target: "_blank", rel: "noopener noreferrer", "{candidate.page_title}" } }
                }
                div { class: "score-badge", aria_label: "Overall match score {score_percent} percent",
                    strong { "{score_percent}" } span { "/ 100" }
                }
            }
            meter { min: 0, max: 1, value: overall_score, aria_label: "Overall semantic match" }
            blockquote { class: "sentence-evidence",
                span { "Best complete-sentence evidence" }
                "{candidate.best_sentence}"
            }
            div { class: "evidence-grid",
                section { aria_label: "Score components",
                    h4 { "Why it matched" }
                    ScoreRow { label: "Semantic", score: candidate.semantic_score }
                    ScoreRow { label: "Lexical", score: candidate.lexical_score }
                    ScoreRow { label: "Entity", score: candidate.entity_score }
                    ScoreRow { label: "Recency", score: candidate.recency_score }
                    ScoreRow { label: "Source priority", score: candidate.source_priority_score }
                }
                section { aria_label: "Matched concepts",
                    h4 { "Concept evidence" }
                    div { class: "tag-group",
                        span { "Entities" }
                        ul { for entity in candidate.entities { li { class: "entity-tag", "{entity}" } } }
                    }
                    div { class: "tag-group",
                        span { "Keywords" }
                        ul { for keyword in candidate.keywords { li { class: "keyword-tag", "{keyword}" } } }
                    }
                }
            }
            footer { class: "match-footer",
                div {
                    span { class: "{status_class}", "{candidate.state}" }
                    span { "{candidate.discovered_label}" }
                    span { "{candidate.model_label}" }
                }
                div { class: "card-actions",
                    ReviewForm { action: approve_url, label: "Approve", csrf_token: csrf_token.clone(), primary: true }
                    ReviewForm { action: suppress_url, label: "Suppress", csrf_token: csrf_token.clone(), primary: false }
                    ReviewForm { action: dismiss_url, label: "Dismiss", csrf_token, primary: false }
                    a { href: "{detail_url}", "Full evidence" }
                }
            }
        }
    }
}

#[component]
fn ReviewForm(action: String, label: &'static str, csrf_token: String, primary: bool) -> Element {
    let class = if primary { "primary-action" } else { "" };
    rsx! {
        form { action: "{action}", method: "post",
            input { r#type: "hidden", name: "csrf_token", value: "{csrf_token}" }
            input { r#type: "hidden", name: "expected_state", value: "candidate" }
            button { class: "{class}", r#type: "submit", "{label}" }
        }
    }
}

#[component]
fn ScoreRow(label: &'static str, score: f32) -> Element {
    let score = clamp_score(score);
    let percent = format!("{:.0}%", score * 100.0);
    rsx! {
        div { class: "score-row",
            span { "{label}" }
            meter { min: 0, max: 1, value: score }
            strong { "{percent}" }
        }
    }
}

fn clamp_score(score: f32) -> f32 {
    if score.is_finite() {
        score.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub fn browser_contract_field_names() -> &'static [&'static str] {
    &[
        "domain",
        "seed_urls",
        "page_budget",
        "priority",
        "query_text",
        "threshold",
        "expected_state",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_contract_contains_no_embedding_values() {
        let fields = browser_contract_field_names();
        assert!(!fields.contains(&"vector"));
        assert!(!fields.contains(&"embedding"));
        assert!(!fields.contains(&"dimensions"));
    }

    #[test]
    fn non_finite_scores_fail_closed() {
        assert_eq!(clamp_score(f32::NAN), 0.0);
        assert_eq!(clamp_score(f32::INFINITY), 0.0);
        assert_eq!(clamp_score(-1.0), 0.0);
        assert_eq!(clamp_score(2.0), 1.0);
    }
}
