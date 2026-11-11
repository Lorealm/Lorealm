//! Downloads the official web UI for this server version into the frontend directory.

use std::{path::Path, time::Duration};

use anyhow::{Context, bail};
use sha2::{Digest, Sha256};

const RELEASES: &str = "https://github.com/Lorealm/Lorealm/releases/download";
const ASSET: &str = "lorealm-web.zip";
/// Written into directories the server installed.
const MARKER: &str = ".lorealm-version";

/// Makes sure `dir` holds a frontend: installs the official one if `dir` is missing,
/// and replaces one installed by an older server version.
pub async fn ensure_frontend(dir: &Path) -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    match std::fs::read_to_string(dir.join(MARKER)) {
        Ok(installed) if installed.trim() == version => return Ok(()),
        Err(_) if dir.exists() => return Ok(()),
        _ => {}
    }

    tracing::info!(
        "downloading the web UI for v{version} into {}",
        dir.display()
    );
    install(&format!("{RELEASES}/v{version}"), dir, version).await
}

async fn install(base: &str, dir: &Path, version: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let get = async |name: &str| -> anyhow::Result<Vec<u8>> {
        let url = format!("{base}/{name}");
        let response = client
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        Ok(response
            .with_context(|| format!("couldn't download {url}"))?
            .bytes()
            .await?
            .to_vec())
    };

    let zip = get(ASSET).await?;
    let sums = String::from_utf8(get("SHA256SUMS").await?)?;
    let expected = sums
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            (parts.next()? == ASSET).then_some(hash)
        })
        .with_context(|| format!("SHA256SUMS has no entry for {ASSET}"))?;
    if hex::encode(Sha256::digest(&zip)) != expected {
        bail!("{ASSET} does not match its checksum");
    }

    // Unpack next to `dir` and swap it in, so a failed install never leaves half a frontend.
    let parent = dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let tmp = tempfile::tempdir_in(parent)?;
    zip::ZipArchive::new(std::io::Cursor::new(zip))?.extract(tmp.path())?;
    std::fs::write(tmp.path().join(MARKER), version)?;

    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::rename(tmp.keep(), dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use axum::{Router, routing::get};

    use super::*;

    fn web_zip() -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("index.html", options).unwrap();
        zip.write_all(b"<title>Lorealm</title>").unwrap();
        zip.start_file("assets/app.js", options).unwrap();
        zip.write_all(b"console.log(1)").unwrap();
        zip.finish().unwrap().into_inner()
    }

    /// Serves `zip` and `sums` as a release, returning its base URL.
    async fn release(zip: Vec<u8>, sums: String) -> String {
        let app = Router::new()
            .route("/lorealm-web.zip", get(move || async move { zip }))
            .route("/SHA256SUMS", get(move || async move { sums }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn installs_and_replaces_its_own_install() {
        let zip = web_zip();
        let sums = format!("{}  lorealm-web.zip\n", hex::encode(Sha256::digest(&zip)));
        let base = release(zip, sums).await;
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("web");

        install(&base, &dir, "0.1.0").await.unwrap();
        assert!(dir.join("assets/app.js").is_file());
        assert_eq!(std::fs::read_to_string(dir.join(MARKER)).unwrap(), "0.1.0");

        std::fs::write(dir.join("stale.js"), "").unwrap();
        install(&base, &dir, "0.2.0").await.unwrap();
        assert!(!dir.join("stale.js").exists());
        assert_eq!(std::fs::read_to_string(dir.join(MARKER)).unwrap(), "0.2.0");
    }

    #[tokio::test]
    async fn rejects_a_checksum_mismatch() {
        let base = release(web_zip(), format!("{}  lorealm-web.zip\n", "0".repeat(64))).await;
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("web");

        assert!(install(&base, &dir, "0.1.0").await.is_err());
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn leaves_user_frontends_alone() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("index.html"), "mine").unwrap();

        ensure_frontend(root.path()).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(root.path().join("index.html")).unwrap(),
            "mine"
        );
        assert!(!root.path().join(MARKER).exists());
    }
}
