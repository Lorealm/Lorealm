use std::path::{Path, PathBuf};

use crate::manifest::{MANIFEST_FILE, Manifest, ManifestError};

/// Where an installed plugin keeps its backend and frontend.
const BACKEND: &str = "backend.wasm";
const FRONTEND: &str = "frontend";
const README: &str = "README.md";

#[derive(Debug, thiserror::Error)]
pub enum StageError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("failed to copy {path}: {source}")]
    Copy {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid manifest: {0}")]
    Rewrite(String),
}

/// Copies a built plugin from `source` into the empty directory `into`: the
/// manifest, the backend, the frontend and the README, nothing else. The paths in the
/// copied manifest are rewritten to fixed ones, so an installed plugin doesn't
/// depend on how its source was laid out.
pub fn stage(source: &Path, into: &Path) -> Result<Manifest, StageError> {
    let manifest = Manifest::from_dir(source)?;
    let text = read(&source.join(MANIFEST_FILE))?;
    let mut table: toml::Table =
        toml::from_str(&text).map_err(|e| StageError::Rewrite(e.to_string()))?;

    copy_file(
        &source.join(&manifest.structure.backend),
        &into.join(BACKEND),
    )?;
    set(&mut table, "structure", "backend", BACKEND);
    if !manifest.frontend.slots.is_empty() {
        copy_dir(&source.join(&manifest.frontend.path), &into.join(FRONTEND))?;
        set(&mut table, "frontend", "path", FRONTEND);
    }
    let readme = source.join(&manifest.structure.readme);
    if readme.is_file() {
        copy_file(&readme, &into.join(README))?;
        set(&mut table, "structure", "readme", README);
    }
    table.remove("build");

    let text = toml::to_string(&table).map_err(|e| StageError::Rewrite(e.to_string()))?;
    std::fs::write(into.join(MANIFEST_FILE), text).map_err(|source| StageError::Copy {
        path: into.join(MANIFEST_FILE),
        source,
    })?;
    Ok(Manifest::from_dir(into)?)
}

fn set(table: &mut toml::Table, section: &str, key: &str, value: &str) {
    if let toml::Value::Table(section) = table
        .entry(section)
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
    {
        section.insert(key.into(), value.into());
    }
}

fn read(path: &Path) -> Result<String, StageError> {
    std::fs::read_to_string(path).map_err(|source| StageError::Copy {
        path: path.to_owned(),
        source,
    })
}

fn copy_file(from: &Path, to: &Path) -> Result<(), StageError> {
    std::fs::copy(from, to)
        .map(drop)
        .map_err(|source| StageError::Copy {
            path: from.to_owned(),
            source,
        })
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), StageError> {
    let err = |path: &Path| {
        let path = path.to_owned();
        move |source| StageError::Copy { path, source }
    };
    std::fs::create_dir_all(to).map_err(err(to))?;
    for entry in std::fs::read_dir(from).map_err(err(from))? {
        let entry = entry.map_err(err(from))?;
        let path = entry.path();
        let target = to.join(entry.file_name());
        if entry.file_type().map_err(err(&path))?.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            copy_file(&path, &target)?;
        }
    }
    Ok(())
}
