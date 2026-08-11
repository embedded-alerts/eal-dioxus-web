use std::{env, net::IpAddr};

use anyhow::{Context, bail};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppEnvironment {
    Development,
    Test,
    Production,
}

impl AppEnvironment {
    pub(crate) fn parse(value: Option<&str>) -> anyhow::Result<Self> {
        let normalized = value.unwrap_or("development").trim().to_ascii_lowercase();
        match normalized.as_str() {
            "" | "dev" | "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "prod" | "production" => Ok(Self::Production),
            other => bail!("unsupported APP_ENV value: {other}"),
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Test => "test",
            Self::Production => "production",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InboxConfig {
    pub(crate) environment: AppEnvironment,
    pub(crate) api_base_url: Url,
    pub(crate) tenant_id: Uuid,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl InboxConfig {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        let environment = AppEnvironment::parse(env::var("APP_ENV").ok().as_deref())?;
        let api_base_url =
            env::var("EAL_API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/".into());
        let tenant_id = env::var("EAL_TENANT_ID")
            .context("EAL_TENANT_ID is required until Shared Auth tenant claims are wired")?;
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8083".into())
            .parse::<u16>()
            .context("PORT must be a valid TCP port")?;

        Self::from_values(environment, &api_base_url, &tenant_id, host, port)
    }

    pub(crate) fn from_values(
        environment: AppEnvironment,
        api_base_url: &str,
        tenant_id: &str,
        host: String,
        port: u16,
    ) -> anyhow::Result<Self> {
        if environment == AppEnvironment::Production {
            bail!(
                "production startup blocked: Shared Auth, durable candidate state, authenticated tenant-filtered events, and DEN-3460 delivery are not certified"
            );
        }

        let mut api_base_url =
            Url::parse(api_base_url.trim()).context("EAL_API_BASE_URL must be an absolute URL")?;
        if !matches!(api_base_url.scheme(), "http" | "https") {
            bail!("EAL_API_BASE_URL must use http or https");
        }
        if !api_base_url.username().is_empty() || api_base_url.password().is_some() {
            bail!("EAL_API_BASE_URL must not contain credentials");
        }
        if api_base_url.host_str().is_none() {
            bail!("EAL_API_BASE_URL must contain a host");
        }
        api_base_url.set_query(None);
        api_base_url.set_fragment(None);
        if !api_base_url.path().ends_with('/') {
            let path = format!("{}/", api_base_url.path());
            api_base_url.set_path(&path);
        }

        let tenant_id =
            Uuid::parse_str(tenant_id.trim()).context("EAL_TENANT_ID must be a UUID")?;
        if tenant_id.is_nil() {
            bail!("EAL_TENANT_ID must not be the nil UUID");
        }
        if host.trim().is_empty() {
            bail!("HOST must not be empty");
        }
        if let Ok(address) = host.parse::<IpAddr>() {
            if address.is_multicast() {
                bail!("HOST must not be a multicast address");
            }
        }
        if port == 0 {
            bail!("PORT must be greater than zero");
        }

        Ok(Self {
            environment,
            api_base_url,
            tenant_id,
            host,
            port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "11111111-1111-4111-8111-111111111111";

    #[test]
    fn production_is_fail_closed() {
        let error = InboxConfig::from_values(
            AppEnvironment::Production,
            "https://api.example.com",
            TENANT,
            "127.0.0.1".into(),
            8083,
        )
        .expect_err("production must remain blocked");
        assert!(error.to_string().contains("DEN-3460"));
    }

    #[test]
    fn api_url_cannot_embed_credentials() {
        let error = InboxConfig::from_values(
            AppEnvironment::Development,
            "https://operator:secret@api.example.com",
            TENANT,
            "127.0.0.1".into(),
            8083,
        )
        .expect_err("credentials must be rejected");
        assert!(error.to_string().contains("credentials"));
    }
}
