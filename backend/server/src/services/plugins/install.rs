//! Installed plugins on disk: `{handle}@{version}` directories under the
//! plugins directory, installed from source and removed once unused.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use lorealm_plugins::{Level, Manifest, Version};
use lorealm_storage::models::{Snowflake, is_identifier};
use serde::Serialize;

use super::PluginService;
use crate::errors::PluginError;

/// How long one build command may run.
const BUILD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Marks an uninstalled version that's kept only while users are on it.
const UNINSTALLED: &str = ".uninstalled";

/// Build output kept in the plugin's log; the rest is only in the response.
const LOGGED_OUTPUT: usize = 4000;

#[derive(Serialize)]
pub struct InstallReport {
    pub handle: String,
    pub version: String,
    /// What the build commands printed.
    pub log: String,
}

impl PluginService {
    /// Builds the plugin in `source` with its `[build]` commands and installs
    /// it for everyone. The commands run on the host, with the server's rights.
    pub async fn install_from_source(
        &self,
        user_id: Snowflake,
        source: PathBuf,
    ) -> Result<InstallReport, PluginError> {
        let invalid = |reason: String| PluginError::InvalidSource(reason);
        let manifest = Manifest::from_dir(&source).map_err(|e| invalid(e.to_string()))?;
        let handle = manifest.plugin.handle.clone();
        let version = manifest.plugin.version.clone();
        if !is_identifier(&handle) {
            return Err(invalid(format!(
                "the handle `{handle}` isn't an identifier"
            )));
        }
        if let Ok(settings) = std::fs::read_to_string(source.join(&manifest.structure.settings)) {
            super::settings::declarations(&settings)
                .map_err(|e| invalid(format!("invalid settings: {e}")))?;
        }
        let target = self.version_dir(&handle, &version);
        if target.exists() {
            return Err(PluginError::AlreadyInstalled(format!("{handle}@{version}")));
        }

        let mut log = String::new();
        for command in [&manifest.build.backend, &manifest.build.frontend] {
            if command.is_empty() {
                continue;
            }
            if let Err(reason) = run(command, &source, &mut log).await {
                let tail = tail(&log, LOGGED_OUTPUT);
                let message = format!("Build failed: {reason}.\n{tail}");
                self.logs
                    .host(user_id, &handle, Level::Error, message)
                    .await;
                return Err(PluginError::BuildFailed(handle));
            }
        }

        // Staged next to the target, so moving it into place is one rename.
        let staging = self.dir.join(format!(".staging-{handle}@{version}"));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).map_err(|e| invalid(e.to_string()))?;
        let host = Arc::clone(&self.host);
        let staged = staging.clone();
        let package = tokio::task::spawn_blocking(move || {
            lorealm_plugins::stage(&source, &staged).map_err(|e| e.to_string())?;
            host.compile(&staged).map_err(|e| e.to_string())
        })
        .await
        .expect("staging should not panic");
        let package = match package {
            Ok(package) => package,
            Err(reason) => {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(invalid(reason));
            }
        };
        std::fs::rename(&staging, &target).map_err(|e| invalid(e.to_string()))?;
        self.packages
            .insert(format!("{handle}@{version}"), Arc::new(package));

        let message = format!("Installed {handle}@{version} from source.");
        self.logs.host(user_id, &handle, Level::Info, message).await;
        self.collect(&handle).await;
        Ok(InstallReport {
            handle,
            version: version.to_string(),
            log,
        })
    }

    /// Stops every version of `handle` from being enabled. Each is removed
    /// from disk once no user is on it.
    pub async fn uninstall(&self, handle: &str) -> Result<(), PluginError> {
        let versions = self.versions(handle);
        if versions.is_empty() {
            return Err(PluginError::NotFound(handle.to_owned()));
        }
        for (version, _) in versions {
            let marker = self.version_dir(handle, &version).join(UNINSTALLED);
            std::fs::write(marker, "").map_err(|e| PluginError::InvalidSource(e.to_string()))?;
        }
        self.collect(handle).await;
        Ok(())
    }

    /// Removes the versions of `handle` no user is on, except the latest
    /// installed one.
    pub(super) async fn collect(&self, handle: &str) {
        let latest = self.latest(handle);
        for (version, uninstalled) in self.versions(handle) {
            if !uninstalled && latest.as_ref() == Some(&version) {
                continue;
            }
            match self
                .storage
                .is_plugin_version_used(handle, &version.to_string())
                .await
            {
                Ok(false) => {
                    let dir = self.version_dir(handle, &version);
                    if let Err(err) = std::fs::remove_dir_all(&dir) {
                        tracing::error!(%handle, %version, error = %err, "failed to remove plugin");
                    }
                    self.packages.remove(&format!("{handle}@{version}"));
                }
                Ok(true) => {}
                Err(err) => {
                    tracing::error!(%handle, %version, error = %err, "failed to check plugin use")
                }
            }
        }
    }

    pub(super) fn version_dir(&self, handle: &str, version: &Version) -> PathBuf {
        self.dir.join(format!("{handle}@{version}"))
    }

    /// Every installed handle, sorted.
    pub(super) fn installed_handles(&self) -> Vec<String> {
        let mut handles: Vec<String> = self.entries().into_iter().map(|(h, _)| h).collect();
        handles.sort();
        handles.dedup();
        handles
    }

    /// The newest version of `handle` that can be enabled.
    pub(super) fn latest(&self, handle: &str) -> Option<Version> {
        self.versions(handle)
            .into_iter()
            .filter(|(_, uninstalled)| !uninstalled)
            .map(|(version, _)| version)
            .next_back()
    }

    pub(super) fn is_available(&self, handle: &str, version: &Version) -> bool {
        self.versions(handle)
            .iter()
            .any(|(v, uninstalled)| v == version && !uninstalled)
    }

    /// Installed versions of `handle`, oldest first, and whether each was
    /// uninstalled.
    fn versions(&self, handle: &str) -> Vec<(Version, bool)> {
        let mut versions: Vec<(Version, bool)> = self
            .entries()
            .into_iter()
            .filter(|(h, _)| h == handle)
            .map(|(_, version)| {
                let uninstalled = self
                    .version_dir(handle, &version)
                    .join(UNINSTALLED)
                    .exists();
                (version, uninstalled)
            })
            .collect();
        versions.sort();
        versions
    }

    /// `(handle, version)` for every `{handle}@{version}` directory.
    fn entries(&self) -> Vec<(String, Version)> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let (handle, version) = name.split_once('@')?;
                let version = version.parse().ok()?;
                is_identifier(handle).then(|| (handle.to_owned(), version))
            })
            .collect()
    }
}

/// Runs one build command in `dir`, appending what it prints to `log`.
async fn run(command: &[String], dir: &Path, log: &mut String) -> Result<(), String> {
    let shown = command.join(" ");
    log.push_str(&format!("$ {shown}\n"));
    let output = tokio::process::Command::new(&command[0])
        .args(&command[1..])
        .current_dir(dir)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = match tokio::time::timeout(BUILD_TIMEOUT, output).await {
        Err(_) => {
            return Err(format!(
                "`{shown}` ran past {} minutes",
                BUILD_TIMEOUT.as_secs() / 60
            ));
        }
        Ok(Err(err)) => return Err(format!("couldn't run `{shown}`: {err}")),
        Ok(Ok(output)) => output,
    };
    log.push_str(&String::from_utf8_lossy(&output.stdout));
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("`{shown}` exited with {}", output.status))
    }
}

/// The last `max` bytes of `text`, on a character boundary.
fn tail(text: &str, max: usize) -> &str {
    let mut start = text.len().saturating_sub(max);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}
