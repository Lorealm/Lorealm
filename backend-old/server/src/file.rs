use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Determines the root directory of the project.
/// 
/// In development, this is `src-tauri/data`.
/// 
/// In release, this is the directory containing the executable.
fn get_root_dir() -> PathBuf {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        // In development, CARGO_MANIFEST_DIR points to src-tauri/data
        let mut path = PathBuf::from(manifest_dir).to_path_buf();
        path.push(PathBuf::from("data"));
        path
    } else {
        // In release, use the directory where the executable resides
        env::current_exe().unwrap().parent().unwrap().to_path_buf()
    }
}

// Resolves a relative path to a full path within the project's root directory.
pub fn resolve_path(relative_path: &str) -> PathBuf {
    get_root_dir().join(relative_path)
}

// Retrieves or creates a file at the specified relative path within the root directory.
// Opens the file for reading and writing, creating it if it doesn't exist.
pub fn get_path(relative_path: &str) -> io::Result<PathBuf> {
    let full_path = resolve_path(relative_path);
    fs::create_dir_all(&full_path)?;
    Ok(full_path)
}