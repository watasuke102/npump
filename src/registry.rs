use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::blocking::Client;
use semver::Version;
use serde_json::Value;

pub struct Registry {
    client: Client,
    cache: HashMap<String, Option<Version>>,
    warnings: Vec<String>,
}

impl Registry {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("npump")
                .timeout(Duration::from_secs(8))
                .build()
                .context("failed to build HTTP client")?,
            cache: HashMap::new(),
            warnings: Vec::new(),
        })
    }

    pub fn latest(&mut self, package_name: &str) -> Option<Version> {
        if let Some(cached) = self.cache.get(package_name) {
            return cached.clone();
        }

        let encoded = utf8_percent_encode(package_name, NON_ALPHANUMERIC).to_string();
        let url = format!("https://registry.npmjs.org/{encoded}/latest");
        let fetched = (|| -> Result<Option<Version>> {
            let response = self
                .client
                .get(&url)
                .send()
                .with_context(|| format!("request failed: {url}"))?;

            if response.status().as_u16() == 404 {
                return Ok(None);
            }

            let response = response
                .error_for_status()
                .with_context(|| format!("registry returned error for {package_name}"))?;
            let body: Value = response
                .json()
                .with_context(|| format!("invalid registry response for {package_name}"))?;
            let Some(latest) = body.get("version").and_then(Value::as_str) else {
                return Ok(None);
            };
            let parsed = Version::parse(latest)
                .with_context(|| format!("invalid latest version '{latest}' for {package_name}"))?;
            Ok(Some(parsed))
        })();

        let latest = match fetched {
            Ok(version) => version,
            Err(err) => {
                self.warnings.push(format!(
                    "{package_name}: failed to fetch latest version ({err})"
                ));
                None
            }
        };
        self.cache.insert(package_name.to_string(), latest.clone());
        latest
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}
