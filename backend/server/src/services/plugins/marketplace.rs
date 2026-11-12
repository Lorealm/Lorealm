use std::sync::Arc;
use std::time::{Duration, Instant};

use lorealm_plugins::{API_VERSION, Version};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::install::{InstallReport, download, download_client};
use super::{PluginService, fetch};
use crate::errors::PluginError;

/// The index format this server reads.
const SCHEMA: u32 = 1;

/// How long the indexes are kept before they're fetched again.
const INDEX_TTL: Duration = Duration::from_secs(10 * 60);

const MAX_INDEX: usize = 8 * 1024 * 1024;
const MAX_README: usize = 1024 * 1024;

#[derive(Deserialize)]
struct Index {
    schema: u32,
    #[serde(default)]
    plugins: Vec<Listed>,
}

/// A plugin as an index lists it.
#[derive(Clone, Deserialize)]
struct Listed {
    handle: String,
    repository: String,
    readme: Option<String>,
    versions: Vec<ListedVersion>,
}

/// One release of a listed plugin: where it is, and what its manifest says.
#[derive(Clone, Deserialize, Serialize)]
pub struct ListedVersion {
    pub version: Version,
    /// The plugin API it was built for.
    pub api: Version,
    pub url: String,
    /// What the zip at `url` has to hash to.
    pub sha256: String,
    pub name: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub hosts: Vec<String>,
    #[serde(default)]
    pub slots: serde_json::Value,
    #[serde(default)]
    pub requires: serde_json::Value,
    #[serde(default)]
    pub recommends: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Serialize)]
pub struct MarketplaceVersion {
    #[serde(flatten)]
    pub listed: ListedVersion,
    /// Whether it was built for this server's plugin API
    pub compatible: bool,
}

#[derive(Serialize)]
pub struct MarketplacePlugin {
    pub handle: String,
    pub repository: String,
    pub has_readme: bool,
    pub versions: Vec<MarketplaceVersion>,
    /// The newest version this server can run, if any.
    pub offered: Option<Version>,
    pub installed: Option<Version>,
}

#[derive(Serialize)]
pub struct Marketplace {
    pub plugins: Vec<MarketplacePlugin>,
    /// The indexes that couldn't be read just now.
    pub unavailable: Vec<String>,
}

/// What the indexes listed when they were last fetched.
struct Catalog {
    fetched: Instant,
    plugins: Vec<(String, Listed)>,
    unavailable: Vec<String>,
}

impl Catalog {
    fn find(&self, handle: &str) -> Result<&(String, Listed), PluginError> {
        self.plugins
            .iter()
            .find(|(_, listed)| listed.handle == handle)
            .ok_or_else(|| PluginError::NotListed(handle.to_owned()))
    }
}

/// The plugin indexes the server reads, in order.
pub(super) struct Indexes {
    urls: Vec<String>,
    client: reqwest::Client,
    catalog: Mutex<Option<Arc<Catalog>>>,
}

impl Indexes {
    pub(super) fn new(urls: Vec<String>) -> Self {
        Self {
            urls,
            client: download_client(),
            catalog: Mutex::new(None),
        }
    }

    async fn fetch_catalog(&self) -> Arc<Catalog> {
        let mut kept = self.catalog.lock().await;
        if let Some(catalog) = kept.as_ref().filter(|c| c.fetched.elapsed() < INDEX_TTL) {
            return Arc::clone(catalog);
        }
        let mut plugins: Vec<(String, Listed)> = Vec::new();
        let mut unavailable = Vec::new();
        for url in &self.urls {
            match self.index(url).await {
                Ok(index) => {
                    for listed in index.plugins {
                        if !plugins.iter().any(|(_, p)| p.handle == listed.handle) {
                            plugins.push((url.clone(), listed));
                        }
                    }
                }
                Err(reason) => {
                    tracing::warn!(index = %url, %reason, "couldn't read a plugin index");
                    unavailable.push(url.clone());
                }
            }
        }
        let catalog = Arc::new(Catalog {
            fetched: Instant::now(),
            plugins,
            unavailable,
        });
        *kept = Some(Arc::clone(&catalog));
        catalog
    }

    async fn index(&self, url: &str) -> Result<Index, String> {
        let bytes = get(&self.client, url, MAX_INDEX).await?;
        let index: Index = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if index.schema != SCHEMA {
            return Err(format!(
                "its format is {}, and this server reads {SCHEMA}",
                index.schema
            ));
        }
        Ok(index)
    }
}

/// Downloads `url`, up to `max` bytes.
async fn get(client: &reqwest::Client, url: &str, max: usize) -> Result<Vec<u8>, String> {
    let failed = |e: reqwest::Error| e.to_string();
    let mut response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(failed)?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        if body.len() + chunk.len() > max {
            return Err(format!("it's over {} MiB", max >> 20));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

impl PluginService {
    pub async fn marketplace(&self) -> Marketplace {
        let catalog = self.marketplace.fetch_catalog().await;
        let plugins = catalog
            .plugins
            .iter()
            .map(|(_, listed)| {
                let versions: Vec<MarketplaceVersion> = listed
                    .versions
                    .iter()
                    .map(|version| MarketplaceVersion {
                        compatible: version.api == API_VERSION,
                        listed: version.clone(),
                    })
                    .collect();
                MarketplacePlugin {
                    handle: listed.handle.clone(),
                    repository: listed.repository.clone(),
                    has_readme: listed.readme.is_some(),
                    offered: offered(listed).map(|v| v.version.clone()),
                    installed: self.latest(&listed.handle),
                    versions,
                }
            })
            .collect();
        Marketplace {
            plugins,
            unavailable: catalog.unavailable.clone(),
        }
    }

    pub async fn marketplace_readme(&self, handle: &str) -> Result<String, PluginError> {
        let catalog = self.marketplace.fetch_catalog().await;
        let (index, listed) = catalog.find(handle)?;
        let missing = || PluginError::NotListed(format!("{handle}'s README"));
        // An index only names READMEs published next to it.
        let index = reqwest::Url::parse(index).map_err(|_| missing())?;
        let url = listed
            .readme
            .as_deref()
            .and_then(|readme| index.join(readme).ok())
            .filter(|url| url.origin() == index.origin())
            .ok_or_else(missing)?;
        let bytes = get(&self.marketplace.client, url.as_str(), MAX_README)
            .await
            .map_err(|_| missing())?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub async fn install_listed(
        &self,
        user_id: lorealm_storage::models::Snowflake,
        handle: &str,
        version: Option<&Version>,
    ) -> Result<InstallReport, PluginError> {
        let invalid = PluginError::InvalidSource;
        let catalog = self.marketplace.fetch_catalog().await;
        let (_, listed) = catalog.find(handle)?;
        let pick = match version {
            Some(version) => listed
                .versions
                .iter()
                .find(|v| v.version == *version)
                .ok_or_else(|| PluginError::NotListed(format!("{handle}@{version}")))?,
            None => offered(listed).ok_or_else(|| {
                invalid(format!(
                    "no version of it is built for plugin API {API_VERSION}, which this server runs"
                ))
            })?,
        };
        if pick.api != API_VERSION {
            return Err(invalid(format!(
                "version {} is built for plugin API {}, and this server runs {API_VERSION}",
                pick.version, pick.api
            )));
        }
        if !fetch::may_carry_keys(&pick.url) {
            return Err(invalid("its download isn't over https".into()));
        }

        tracing::info!(%handle, version = %pick.version, "installing from the marketplace");
        let bytes = download(&self.marketplace.client, &pick.url).await?;
        let hash = hex::encode(Sha256::digest(&bytes));
        if !hash.eq_ignore_ascii_case(&pick.sha256) {
            return Err(invalid(format!(
                "the download isn't the release the marketplace lists: its sha256 is {hash}, not {}",
                pick.sha256
            )));
        }
        let expected = (handle.to_owned(), pick.version.clone());
        self.install_zip_as(user_id, bytes, false, Some(expected))
            .await
    }
}

/// The newest version built for this server's plugin API.
fn offered(listed: &Listed) -> Option<&ListedVersion> {
    listed
        .versions
        .iter()
        .filter(|v| v.api == API_VERSION)
        .max_by(|a, b| a.version.cmp(&b.version))
}
