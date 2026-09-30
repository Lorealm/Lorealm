use std::{
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use lorealm_plugins::{Level, MANIFEST_FILE, Manifest, Version};
use lorealm_storage::models::{RealmResourceKind, Snowflake, is_identifier};
use serde::Serialize;

use super::PluginService;
use crate::errors::PluginError;

/// How long one build command may run.
const BUILD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Marks an uninstalled version that's kept only while users are on it.
const UNINSTALLED: &str = ".uninstalled";

/// Marks a version installed for development; holds its source folder.
const DEV: &str = ".dev";

/// Build output kept in the plugin's log; the rest is only in the response.
const LOGGED_OUTPUT: usize = 4000;

/// The largest pre-built package accepted, packed or downloaded.
pub const MAX_PACKAGE: usize = 64 * 1024 * 1024;

/// The most a package may unpack to.
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

#[derive(Serialize)]
pub struct InstallReport {
    pub handle: String,
    pub version: String,
    /// What the build commands printed.
    pub log: String,
}

impl PluginService {
    /// Installs the plugin at `location`, a folder on the server or a link,
    /// working out how from what it is; see [`Location::parse`]. Installs
    /// that build the plugin are refused unless `may_build`.
    pub async fn install(
        &self,
        user_id: Snowflake,
        location: &str,
        may_build: bool,
    ) -> Result<InstallReport, PluginError> {
        let building = || {
            if may_build {
                Ok(())
            } else {
                Err(PluginError::BuildForbidden(location.to_owned()))
            }
        };
        match Location::parse(location) {
            Location::Folder(path) => {
                let manifest = Manifest::from_dir(&path)
                    .map_err(|e| PluginError::InvalidSource(e.to_string()))?;
                if manifest.build.backend.is_empty() && manifest.build.frontend.is_empty() {
                    tracing::info!(%location, "installing a pre-built folder");
                    self.install_prebuilt(user_id, path).await
                } else {
                    building()?;
                    tracing::info!(%location, "installing a folder from source");
                    self.install_from_source(user_id, path).await
                }
            }
            Location::Zip(url) => {
                tracing::info!(%location, "installing a zip from a link");
                let bytes = download(&download_client(), &url).await?;
                self.install_zip(user_id, bytes, may_build).await
            }
            Location::GitHub { repo, tag } => {
                tracing::info!(%location, "installing a GitHub release");
                if let Some(report) = self
                    .install_release(user_id, &repo, tag.as_deref(), may_build)
                    .await?
                {
                    return Ok(report);
                }
                if tag.is_some() {
                    return Err(PluginError::InvalidSource(format!(
                        "{location} has no such release"
                    )));
                }
                building()?;
                tracing::info!(%location, "no release; installing the repository from source");
                let url = format!("https://github.com/{repo}");
                self.install_git(user_id, &url, None).await
            }
            Location::Git { url, rev } => {
                building()?;
                tracing::info!(%location, "installing a git repository from source");
                self.install_git(user_id, &url, rev.as_deref()).await
            }
        }
    }

    /// Builds the plugin in `source` with its `[build]` commands and installs
    /// it for everyone. The commands run on the host, with the server's rights.
    pub async fn install_from_source(
        &self,
        user_id: Snowflake,
        source: PathBuf,
    ) -> Result<InstallReport, PluginError> {
        let manifest = self.check(&source, false)?;
        self.build_and_place(user_id, source, manifest, None).await
    }

    /// Installs the plugin in the folder at `location` from source.
    pub async fn install_dev(
        &self,
        user_id: Snowflake,
        location: &str,
    ) -> Result<InstallReport, PluginError> {
        let invalid = |reason: String| PluginError::InvalidSource(reason);
        let Location::Folder(source) = Location::parse(location) else {
            return Err(invalid(
                "only a folder on the server can be installed for development".into(),
            ));
        };
        let source = source.canonicalize().map_err(|e| invalid(e.to_string()))?;
        let manifest = self.check(&source, false)?;
        tracing::info!(%location, "installing a folder for development");
        self.build_and_place(user_id, source.clone(), manifest, Some(source))
            .await
    }

    /// Builds the development plugin `handle` from its folder again and
    /// replaces its installed version.
    pub async fn reload(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<InstallReport, PluginError> {
        let version = self
            .latest(handle)
            .ok_or_else(|| PluginError::NotFound(handle.to_owned()))?;
        let source = std::fs::read_to_string(self.version_dir(handle, &version).join(DEV))
            .map_err(|_| PluginError::NotInDevelopment(handle.to_owned()))?;
        let source = PathBuf::from(source);
        let manifest = self.check(&source, true)?;
        if manifest.plugin.handle != handle {
            return Err(PluginError::InvalidSource(format!(
                "its manifest now names the plugin `{}`",
                manifest.plugin.handle
            )));
        }
        tracing::info!(%handle, "reloading a plugin in development");
        let report = self
            .build_and_place(user_id, source.clone(), manifest, Some(source))
            .await?;
        self.unmount_handle(handle).await;
        Ok(report)
    }

    /// Whether `handle`'s `version` was installed for development.
    pub(super) fn is_dev(&self, handle: &str, version: &Version) -> bool {
        self.version_dir(handle, version).join(DEV).is_file()
    }

    /// Runs the `[build]` commands in `source`, then installs the result,
    /// remembering `dev` as its source folder if given.
    async fn build_and_place(
        &self,
        user_id: Snowflake,
        source: PathBuf,
        manifest: Manifest,
        dev: Option<PathBuf>,
    ) -> Result<InstallReport, PluginError> {
        let handle = manifest.plugin.handle.clone();

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
        let how = if dev.is_some() {
            "for development"
        } else {
            "from source"
        };
        self.place(user_id, source, manifest, log, how, dev).await
    }

    /// Installs the already built plugin in `source` for everyone, without
    /// running anything from it.
    pub async fn install_prebuilt(
        &self,
        user_id: Snowflake,
        source: PathBuf,
    ) -> Result<InstallReport, PluginError> {
        let manifest = self.check(&source, false)?;
        // Nothing outside the plugin may be copied in, such as a README
        // pointing at the server's files.
        let root = source
            .canonicalize()
            .map_err(|e| PluginError::InvalidSource(e.to_string()))?;
        for path in [
            &manifest.structure.backend,
            &manifest.structure.readme,
            &manifest.structure.settings,
            &manifest.frontend.path,
        ] {
            if let Ok(resolved) = root.join(path).canonicalize()
                && !resolved.starts_with(&root)
            {
                return Err(PluginError::InvalidSource(format!(
                    "`{}` is outside the plugin",
                    path.display()
                )));
            }
        }
        if !root.join(&manifest.structure.backend).is_file() {
            return Err(PluginError::InvalidSource(format!(
                "its backend `{}` is missing; build the plugin before packaging it",
                manifest.structure.backend.display()
            )));
        }
        self.place(user_id, source, manifest, String::new(), "pre-built", None)
            .await
    }

    /// Installs a zipped plugin. The manifest may be at the top of the
    /// archive or in its one folder. It's pre-built if its backend is there,
    /// and otherwise built, if it has `[build]` commands and `may_build`.
    pub async fn install_zip(
        &self,
        user_id: Snowflake,
        zip: Vec<u8>,
        may_build: bool,
    ) -> Result<InstallReport, PluginError> {
        let temp = self.temp_dir()?;
        let into = temp.path().to_owned();
        let root = tokio::task::spawn_blocking(move || unzip(&zip, &into))
            .await
            .expect("unzipping should not panic")
            .map_err(PluginError::InvalidSource)?;
        let manifest =
            Manifest::from_dir(&root).map_err(|e| PluginError::InvalidSource(e.to_string()))?;
        let has_build = !manifest.build.backend.is_empty() || !manifest.build.frontend.is_empty();
        if root.join(&manifest.structure.backend).is_file() || !has_build {
            tracing::info!("installing a pre-built zip");
            return self.install_prebuilt(user_id, root).await;
        }
        if !may_build {
            return Err(PluginError::BuildForbidden("a zip".into()));
        }
        tracing::info!("installing a zip from source");
        self.install_from_source(user_id, root).await
    }

    /// Installs a pre-built plugin from the GitHub release of `repo`
    /// (`owner/name`) tagged `tag`, or the latest. The release holds either
    /// the plugin's files, `manifest.toml` among them, or one `.zip` of them.
    /// `None` when there's no such release.
    pub async fn install_release(
        &self,
        user_id: Snowflake,
        repo: &str,
        tag: Option<&str>,
        may_build: bool,
    ) -> Result<Option<InstallReport>, PluginError> {
        let invalid = |reason: String| PluginError::InvalidSource(reason);
        let Some((owner, name)) = repo.split_once('/') else {
            return Err(invalid(format!("`{repo}` isn't written like `owner/name`")));
        };
        let mut url = reqwest::Url::parse("https://api.github.com/repos").expect("valid URL");
        {
            let mut path = url.path_segments_mut().expect("URL has a path");
            path.extend([owner, name, "releases"]);
            match tag {
                Some(tag) => path.extend(["tags", tag]),
                None => path.push("latest"),
            };
        }

        #[derive(serde::Deserialize)]
        struct Release {
            assets: Vec<Asset>,
        }
        #[derive(serde::Deserialize)]
        struct Asset {
            name: String,
            browser_download_url: String,
        }
        let client = download_client();
        let response = client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| invalid(format!("couldn't find the release: {e}")))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let release: Release = response
            .error_for_status()
            .map_err(|e| invalid(format!("couldn't find the release: {e}")))?
            .json()
            .await
            .map_err(|e| invalid(format!("couldn't read the release: {e}")))?;

        if release.assets.iter().any(|a| a.name == MANIFEST_FILE) {
            let temp = self.temp_dir()?;
            for asset in &release.assets {
                // Asset names are flat, but they come from the release author.
                if Path::new(&asset.name).file_name() != Some(asset.name.as_ref()) {
                    continue;
                }
                let bytes = download(&client, &asset.browser_download_url).await?;
                std::fs::write(temp.path().join(&asset.name), bytes)
                    .map_err(|e| invalid(e.to_string()))?;
            }
            return self
                .install_prebuilt(user_id, temp.path().to_owned())
                .await
                .map(Some);
        }
        let mut zips = release.assets.iter().filter(|a| a.name.ends_with(".zip"));
        match (zips.next(), zips.next()) {
            (Some(asset), None) => {
                let bytes = download(&client, &asset.browser_download_url).await?;
                self.install_zip(user_id, bytes, may_build).await.map(Some)
            }
            (None, _) => Err(invalid(format!(
                "the release has neither a {MANIFEST_FILE} nor a .zip"
            ))),
            (Some(_), Some(_)) => Err(invalid("the release has more than one .zip".into())),
        }
    }

    /// Clones the git repository at `url`, at the branch or tag `rev` or its
    /// default branch, and builds and installs it like
    /// [`Self::install_from_source`].
    pub async fn install_git(
        &self,
        user_id: Snowflake,
        url: &str,
        rev: Option<&str>,
    ) -> Result<InstallReport, PluginError> {
        let temp = self.temp_dir()?;
        let into = temp.path().to_string_lossy().into_owned();
        let mut command = vec!["git", "clone", "--depth", "1"];
        if let Some(rev) = rev {
            command.extend(["--branch", rev]);
        }
        command.extend(["--", url, &into]);
        let command: Vec<String> = command.into_iter().map(str::to_owned).collect();

        let mut log = String::new();
        if let Err(reason) = run(&command, &self.dir, &mut log).await {
            let tail = tail(&log, LOGGED_OUTPUT);
            return Err(PluginError::InvalidSource(format!("{reason}\n{tail}")));
        }
        let mut report = self
            .install_from_source(user_id, temp.path().to_owned())
            .await?;
        report.log.insert_str(0, &log);
        Ok(report)
    }

    /// Reads and validates the manifest in `source`, refusing a version
    /// that's already installed unless `replacing` it.
    fn check(&self, source: &Path, replacing: bool) -> Result<Manifest, PluginError> {
        let invalid = |reason: String| PluginError::InvalidSource(reason);
        let manifest = Manifest::from_dir(source).map_err(|e| invalid(e.to_string()))?;
        let handle = &manifest.plugin.handle;
        let version = &manifest.plugin.version;
        if !is_identifier(handle) {
            return Err(invalid(format!(
                "the handle `{handle}` isn't an identifier"
            )));
        }
        for kind in &manifest.definitions.api_keys {
            if !is_identifier(kind) {
                return Err(invalid(format!(
                    "the API key type `{kind}` isn't an identifier"
                )));
            }
            if RealmResourceKind::ALL
                .iter()
                .any(|r| r.ref_name().eq_ignore_ascii_case(kind))
            {
                return Err(invalid(format!(
                    "the API key type `{kind}` is taken by a built-in type"
                )));
            }
        }
        if let Ok(settings) = std::fs::read_to_string(source.join(&manifest.structure.settings)) {
            super::settings::declarations(&settings)
                .map_err(|e| invalid(format!("invalid settings: {e}")))?;
        }
        if !replacing && self.version_dir(handle, version).exists() {
            return Err(PluginError::AlreadyInstalled(format!("{handle}@{version}")));
        }
        Ok(manifest)
    }

    /// Copies the built plugin in `source` into place and loads it, marking
    /// it as developed in the folder `dev` if given.
    async fn place(
        &self,
        user_id: Snowflake,
        source: PathBuf,
        manifest: Manifest,
        log: String,
        how: &str,
        dev: Option<PathBuf>,
    ) -> Result<InstallReport, PluginError> {
        let invalid = |reason: String| PluginError::InvalidSource(reason);
        let handle = manifest.plugin.handle;
        let version = manifest.plugin.version;
        let target = self.version_dir(&handle, &version);

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
        if let Some(dev) = dev {
            std::fs::write(staging.join(DEV), dev.to_string_lossy().as_bytes())
                .map_err(|e| invalid(e.to_string()))?;
        }
        // Only a reload replaces an installed version.
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|e| invalid(e.to_string()))?;
        }
        std::fs::rename(&staging, &target).map_err(|e| invalid(e.to_string()))?;
        self.packages
            .insert(format!("{handle}@{version}"), Arc::new(package));

        let message = format!("Installed {handle}@{version} {how}.");
        self.logs.host(user_id, &handle, Level::Info, message).await;
        self.collect(&handle).await;
        Ok(InstallReport {
            handle,
            version: version.to_string(),
            log,
        })
    }

    /// A scratch directory in the plugins directory, removed when dropped.
    fn temp_dir(&self) -> Result<tempfile::TempDir, PluginError> {
        std::fs::create_dir_all(&self.dir)
            .and_then(|()| {
                tempfile::Builder::new()
                    .prefix(".download-")
                    .tempdir_in(&self.dir)
            })
            .map_err(|e| PluginError::InvalidSource(e.to_string()))
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
        // Fail instead of waiting on a password nobody can type.
        .env("GIT_TERMINAL_PROMPT", "0")
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

/// Where a plugin is installed from.
#[derive(Debug, PartialEq, Eq)]
enum Location {
    /// A folder on the server, built there if its manifest has `[build]`
    /// commands.
    Folder(PathBuf),
    /// A link to a zipped pre-built plugin.
    Zip(String),
    /// A GitHub repository's release tagged `tag`, or its latest. Without a
    /// tag and without releases, the repository is built instead.
    GitHub { repo: String, tag: Option<String> },
    /// A git repository to clone at the branch or tag `rev` and build.
    Git { url: String, rev: Option<String> },
}

impl Location {
    fn parse(location: &str) -> Self {
        let Ok(url) = reqwest::Url::parse(location) else {
            // `git@host:owner/name` isn't a URL, but git takes it.
            return if location.starts_with("git@") {
                Location::Git {
                    url: location.to_owned(),
                    rev: None,
                }
            } else {
                Location::Folder(PathBuf::from(location))
            };
        };
        if url.path().ends_with(".zip") {
            return Location::Zip(location.to_owned());
        }
        let segments: Vec<&str> = url
            .path_segments()
            .map(|s| s.filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();
        if url.host_str() == Some("github.com") {
            if let [owner, name, rest @ ..] = segments.as_slice() {
                let name = name.strip_suffix(".git").unwrap_or(name);
                let repo = format!("{owner}/{name}");
                match rest {
                    [] | ["releases"] | ["releases", "latest"] => {
                        return Location::GitHub { repo, tag: None };
                    }
                    ["releases", "tag", tag] => {
                        return Location::GitHub {
                            repo,
                            tag: Some((*tag).to_owned()),
                        };
                    }
                    ["tree", rev @ ..] if !rev.is_empty() => {
                        return Location::Git {
                            url: format!("https://github.com/{repo}"),
                            rev: Some(rev.join("/")),
                        };
                    }
                    _ => {}
                }
            }
        }
        Location::Git {
            url: location.to_owned(),
            rev: None,
        }
    }
}

/// Follows redirects, which GitHub's downloads go through.
fn download_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("Lorealm/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .build()
        .expect("download client should build")
}

/// Downloads `url`, up to [`MAX_PACKAGE`].
async fn download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, PluginError> {
    let failed =
        |e: reqwest::Error| PluginError::InvalidSource(format!("couldn't download {url}: {e}"));
    let mut response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(failed)?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        if body.len() + chunk.len() > MAX_PACKAGE {
            return Err(PluginError::InvalidSource(format!(
                "{url} is over {} MiB",
                MAX_PACKAGE >> 20
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Unpacks the regular files and folders in `zip` into `into`, returning
/// the folder holding the manifest: `into`, or its only folder.
fn unzip(zip: &[u8], into: &Path) -> Result<PathBuf, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).map_err(|e| e.to_string())?;
    let mut budget = MAX_UNPACKED;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        // Links are skipped, so nothing unpacked can point outside `into`.
        let Some(name) = file.enclosed_name() else {
            continue;
        };
        let path = into.join(name);
        if file.is_dir() {
            std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        } else if file.is_file() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = std::fs::File::create(&path).map_err(|e| e.to_string())?;
            let written = std::io::copy(&mut (&mut file).take(budget + 1), &mut out)
                .map_err(|e| e.to_string())?;
            if written > budget {
                return Err(format!("it unpacks to over {} MiB", MAX_UNPACKED >> 20));
            }
            budget -= written;
        }
    }

    if into.join(MANIFEST_FILE).is_file() {
        return Ok(into.to_owned());
    }
    let entries: Vec<_> = std::fs::read_dir(into)
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();
    match entries.as_slice() {
        [only] if only.path().join(MANIFEST_FILE).is_file() => Ok(only.path()),
        _ => Err(format!("it has no {MANIFEST_FILE}")),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn git(url: &str, rev: Option<&str>) -> Location {
        Location::Git {
            url: url.into(),
            rev: rev.map(Into::into),
        }
    }

    fn github(repo: &str, tag: Option<&str>) -> Location {
        Location::GitHub {
            repo: repo.into(),
            tag: tag.map(Into::into),
        }
    }

    #[test]
    fn infers_where_a_plugin_comes_from() {
        let cases = [
            (
                "/srv/plugins/tts",
                Location::Folder("/srv/plugins/tts".into()),
            ),
            ("plugins/tts", Location::Folder("plugins/tts".into())),
            (
                "https://example.com/tts.zip",
                Location::Zip("https://example.com/tts.zip".into()),
            ),
            (
                "https://github.com/lorealm/tts",
                github("lorealm/tts", None),
            ),
            (
                "https://github.com/lorealm/tts.git",
                github("lorealm/tts", None),
            ),
            (
                "https://github.com/lorealm/tts/releases/tag/v1.0",
                github("lorealm/tts", Some("v1.0")),
            ),
            (
                "https://github.com/lorealm/tts/tree/feat/voices",
                git("https://github.com/lorealm/tts", Some("feat/voices")),
            ),
            (
                "https://gitlab.com/lorealm/tts",
                git("https://gitlab.com/lorealm/tts", None),
            ),
            (
                "git@github.com:lorealm/tts.git",
                git("git@github.com:lorealm/tts.git", None),
            ),
        ];
        for (location, expected) in cases {
            assert_eq!(Location::parse(location), expected, "{location}");
        }
    }
}
