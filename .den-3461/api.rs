use std::{fmt, time::Duration};

use reqwest::{Method, StatusCode, redirect::Policy};
use serde::de::DeserializeOwned;
use url::Url;
use uuid::Uuid;

use crate::models::{ApiHealth, MatchCandidate, PageIndexRecord, SourceDomain};

const TENANT_HEADER: &str = "x-eal-tenant-id";
const MAX_API_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct ApiClient {
    http: reqwest::Client,
    base_url: Url,
    tenant_id: Uuid,
}

impl ApiClient {
    pub(crate) fn new(base_url: Url, tenant_id: Uuid) -> Result<Self, ApiClientError> {
        let http = reqwest::Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(20))
            .user_agent("eal-dioxus-web/0.2")
            .build()
            .map_err(|error| {
                ApiClientError::new("api_client", format!("build API client: {error}"))
            })?;
        Ok(Self {
            http,
            base_url,
            tenant_id,
        })
    }

    pub(crate) async fn health(&self) -> Result<ApiHealth, ApiClientError> {
        self.get("healthz").await
    }

    pub(crate) async fn list_sources(&self) -> Result<Vec<SourceDomain>, ApiClientError> {
        self.get("v1/sources").await
    }

    pub(crate) async fn list_pages(&self) -> Result<Vec<PageIndexRecord>, ApiClientError> {
        self.get("v1/pages").await
    }

    pub(crate) async fn list_matches(&self) -> Result<Vec<MatchCandidate>, ApiClientError> {
        self.get("v1/matches").await
    }

    async fn get<T>(&self, path: &str) -> Result<T, ApiClientError>
    where
        T: DeserializeOwned,
    {
        let endpoint = self.endpoint(path)?;
        let response = self
            .http
            .request(Method::GET, endpoint)
            .header(TENANT_HEADER, self.tenant_id.to_string())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| {
                ApiClientError::new(
                    "api_unreachable",
                    format!("Embedded Alerts API request failed: {error}"),
                )
            })?;
        decode_response(response).await
    }

    fn endpoint(&self, path: &str) -> Result<Url, ApiClientError> {
        self.base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| {
                ApiClientError::new("api_endpoint", format!("construct API endpoint: {error}"))
            })
    }
}

async fn decode_response<T>(response: reqwest::Response) -> Result<T, ApiClientError>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if status.is_redirection() {
        return Err(ApiClientError::new(
            "api_redirect_blocked",
            "The API returned a redirect; redirects are blocked to protect tenant context.",
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_API_RESPONSE_BYTES as u64)
    {
        return Err(ApiClientError::new(
            "api_response_too_large",
            "The API response exceeded the inbox response-size limit.",
        ));
    }
    let body = response.bytes().await.map_err(|error| {
        ApiClientError::new("api_response", format!("read API response: {error}"))
    })?;
    if body.len() > MAX_API_RESPONSE_BYTES {
        return Err(ApiClientError::new(
            "api_response_too_large",
            "The API response exceeded the inbox response-size limit.",
        ));
    }
    if !status.is_success() {
        let parsed = serde_json::from_slice::<ApiErrorEnvelope>(&body).ok();
        let (code, message) = parsed
            .map(|envelope| (envelope.error.code, envelope.error.message))
            .unwrap_or_else(|| {
                (
                    format!("api_http_{}", status.as_u16()),
                    safe_status_message(status).into(),
                )
            });
        return Err(ApiClientError::new(code, message));
    }
    serde_json::from_slice(&body).map_err(|error| {
        ApiClientError::new(
            "api_contract",
            format!("The API response did not match the expected contract: {error}"),
        )
    })
}

fn safe_status_message(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "The API rejected the request.",
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "The API rejected the current tenant or authentication context."
        }
        StatusCode::NOT_FOUND => "The requested API resource was not found.",
        StatusCode::CONFLICT => "The API reported a state conflict.",
        StatusCode::TOO_MANY_REQUESTS => "The API rate limit was reached.",
        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
            "The API or one of its providers is unavailable."
        }
        _ => "The API request failed.",
    }
}

#[derive(Debug, serde::Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorDetail,
}

#[derive(Debug, serde::Deserialize)]
struct ApiErrorDetail {
    code: String,
    message: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ApiClientError {
    pub(crate) code: String,
    pub(crate) message: String,
}

impl ApiClientError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ApiClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiClientError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_join_preserves_configured_prefix() {
        let client = ApiClient::new(
            Url::parse("https://api.example.com/embedded-alerts/").unwrap(),
            Uuid::new_v4(),
        )
        .expect("client");
        assert_eq!(
            client.endpoint("v1/matches").unwrap().as_str(),
            "https://api.example.com/embedded-alerts/v1/matches"
        );
    }
}
