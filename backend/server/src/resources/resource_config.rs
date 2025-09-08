use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    fs::{self, File},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectoryScope {
    Global, // Accessible to all users
    Local,  // Specific to a user
    System, // Generally inaccessible to users
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceConfig {
    resource_dir: PathBuf,
    directory_manager: DirectoryManager,
}

impl ResourceConfig {
    pub fn new(resource_dir: impl Into<PathBuf>, directory_manager: Option<DirectoryManager>) -> Self {
        let resource_dir = resource_dir.into();
        let mut directory_manager = directory_manager.unwrap_or_default();
        directory_manager.set_root_directory(resource_dir.clone());

        Self {
            resource_dir,
            directory_manager,
        }
    }

    pub fn resource_dir(&self) -> &PathBuf {
        &self.resource_dir
    }

    pub fn directory_manager(&self) -> &DirectoryManager {
        &self.directory_manager
    }
}

pub trait FileLike: Read + Write + Seek + Debug {
    fn path(&self) -> &Path;
}

#[derive(Debug)]
pub struct ManagedFile {
    path: PathBuf,
    handle: File,
}

impl ManagedFile {
    pub fn new(path: PathBuf, file: File) -> Self {
        Self { path, handle: file }
    }
}

impl FileLike for ManagedFile {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Read for ManagedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.handle.read(buf)
    }
}

impl Write for ManagedFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.handle.flush()
    }
}

impl Seek for ManagedFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.handle.seek(pos)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryManager {
    base_directory: PathBuf,
    directories: Vec<DirectoryEntry>,
}

impl DirectoryManager {
    pub fn new(base_directory: impl Into<PathBuf>) -> Self {
        let mut manager = Self::default();
        manager.set_root_directory(base_directory);
        manager
    }

    pub(super) fn set_root_directory(&mut self, path: impl Into<PathBuf>) {
        self.base_directory = path.into();
    }

    /// Writes a serde-serializable value to a file at the specified path, respecting existing permission logic.
    pub(super) fn write_file<T: Serialize>(
        &self,
        path: impl AsRef<Path>,
        user_id: Uuid,
        value: &T,
    ) -> Result<(), ResourceError> {
        let normalized_path = self.normalize_path(path.as_ref())?;
        let file_entry = self.get_file_entry(&normalized_path)?;

        // Check permissions
        if user_id != Uuid::nil() {
            if file_entry.scope != DirectoryScope::Global && file_entry.owner != Some(user_id) {
                return Err(ResourceError::PermissionDenied);
            }
        }

        // Ensure parent directory exists
        let full_path = self.base_directory.join(&normalized_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ResourceError::IoError(e.to_string()))?;
        }

        // Serialize to JSON
        let serialized = serde_json::to_string(value)
            .map_err(|e| ResourceError::IoError(e.to_string()))?;

        // Write to file
        let mut file = File::create(&full_path)
            .map_err(|e| ResourceError::IoError(e.to_string()))?;
        file.write_all(serialized.as_bytes())
            .map_err(|e| ResourceError::IoError(e.to_string()))?;
        file.flush()
            .map_err(|e| ResourceError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Reads and deserializes a serde value from a file at the specified path, respecting existing permission logic.
    pub fn read_file<T: for<'de> Deserialize<'de>>(
        &self,
        path: impl AsRef<Path>,
        user_id: Uuid,
    ) -> Result<T, ResourceError> {
        let normalized_path = self.normalize_path(path.as_ref())?;
        let file_entry = self.get_file_entry(&normalized_path)?;

        // Check permissions
        if user_id != Uuid::nil() {
            if file_entry.scope != DirectoryScope::Global && file_entry.owner != Some(user_id) {
                return Err(ResourceError::PermissionDenied);
            }
        }

        let full_path = self.base_directory.join(&normalized_path);
        let mut file = File::open(&full_path)
            .map_err(|e| ResourceError::IoError(e.to_string()))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| ResourceError::IoError(e.to_string()))?;

        let deserialized = serde_json::from_str(&contents)
            .map_err(|e| ResourceError::IoError(e.to_string()))?;

        Ok(deserialized)
    }

    /// Deletes a file or directory at the specified path, with an option for soft delete.
    /// If `soft_delete` is true, moves the file/directory to `res://trash/{relative_path}`,
    /// overwriting any existing items. Respects existing permission logic.
    pub fn delete_file(
        &self,
        path: impl AsRef<Path>,
        user_id: Uuid,
        soft_delete: bool,
    ) -> Result<(), ResourceError> {
        let normalized_path = self.normalize_path(path.as_ref())?;
        let file_entry = self.get_file_entry(&normalized_path)?;

        // Check permissions
        if user_id != Uuid::nil() {
            if file_entry.scope != DirectoryScope::Global && file_entry.owner != Some(user_id) {
                return Err(ResourceError::PermissionDenied);
            }
        }

        let full_path = self.base_directory.join(&normalized_path);

        // Verify the path exists
        if !fs::exists(&full_path).map_err(|e| ResourceError::IoError(e.to_string()))? {
            return Err(ResourceError::InvalidPath(normalized_path));
        }

        if soft_delete {
            // Construct the trash path: res://trash/{relative_path}
            let trash_path = self.base_directory.join("trash").join(&normalized_path);
            
            // Ensure the parent directory in trash exists
            if let Some(parent) = trash_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ResourceError::IoError(e.to_string()))?;
            }

            // Remove existing item in trash if it exists
            if fs::exists(&trash_path).map_err(|e| ResourceError::IoError(e.to_string()))? {
                if trash_path.is_dir() {
                    fs::remove_dir_all(&trash_path)
                        .map_err(|e| ResourceError::IoError(e.to_string()))?;
                } else {
                    fs::remove_file(&trash_path)
                        .map_err(|e| ResourceError::IoError(e.to_string()))?;
                }
            }

            // Move the file or directory to the trash
            fs::rename(&full_path, &trash_path)
                .map_err(|e| ResourceError::IoError(e.to_string()))?;
        } else {
            // Hard delete: remove file or directory
            if full_path.is_dir() {
                fs::remove_dir_all(&full_path)
                    .map_err(|e| ResourceError::IoError(e.to_string()))?;
            } else {
                fs::remove_file(&full_path)
                    .map_err(|e| ResourceError::IoError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Retrieves a `DirectoryEntry` for the given path, inferring from the closest parent if necessary.
    pub(super) fn get_directory_entry(&self, path: impl AsRef<Path>) -> Result<DirectoryEntry, ResourceError> {
        let normalized_path = self.normalize_path(path.as_ref())?;

        // Check for an exact match
        if let Some(entry) = self.directories.iter().find(|d| PathBuf::from(d.path.as_str()) == normalized_path) {
            return Ok(entry.clone());
        }

        // Find the closest parent directory
        let parent_entry = self.find_closest_parent(&normalized_path)?;

        // Read immediate subdirectories from the filesystem
        let subdirs = self
            .read_directory_io(&self.base_directory.join(&normalized_path))
            .map(|dir_data| {
                dir_data
                    .subdirectories
                    .into_iter()
                    .filter_map(|subdir| {
                        subdir
                            .path
                            .file_name()
                            .and_then(|name| name.to_str().map(String::from))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(DirectoryEntry {
            path: normalized_path.to_string_lossy().to_string(),
            scope: parent_entry.scope,
            description: format!("Inferred directory from parent: {}", parent_entry.path),
            owner: parent_entry.owner,
            subdirectories: subdirs,
        })
    }

    /// Retrieves a `FileEntry` for the given path, inheriting permissions from the closest parent.
    pub(super) fn get_file_entry(&self, path: impl AsRef<Path>) -> Result<FileEntry, ResourceError> {
        let normalized_path = self.normalize_path(path.as_ref())?;

        // Find the closest parent directory
        let parent_entry = self.find_closest_parent(&normalized_path)?;

        Ok(FileEntry {
            path: self.anonymize_path(&PathBuf::from(&normalized_path)),
            scope: parent_entry.scope,
            owner: parent_entry.owner,
        })
    }

    pub(super) fn add_directory(&mut self, directory: DirectoryEntry) {
        self.directories.push(directory);
    }

    pub(super) fn add_user_directory(&mut self, user_id: Uuid) {
        self.add_directory(DirectoryEntry {
            path: format!("users/{}", user_id),
            scope: DirectoryScope::Local,
            description: "User directory for storing user-specific files.".to_string(),
            owner: Some(user_id),
            subdirectories: vec!["conversations".to_string(), "lorebooks".to_string()],
        });
    }

    pub(super) fn remove_user_directory(&mut self, user_id: Uuid) {
        self.directories.retain(|d| d.owner != Some(user_id));
    }

    pub(super) fn get_accessible_directories(&self, user_id: Uuid) -> Vec<&DirectoryEntry> {
        self.directories
            .iter()
            .filter(|d| d.scope == DirectoryScope::Global || d.owner == Some(user_id))
            .collect()
    }

    pub(super) fn request_directory_structure(
        &self,
        path: impl AsRef<Path>,
        user_id: Uuid,
    ) -> Result<DirectoryStructure, ResourceError> {
        let normalized_path = self.normalize_path(path.as_ref())?;
        let dir_entry = self.get_directory_entry(&normalized_path)?;

        // Check permissions
        if dir_entry.scope != DirectoryScope::Global && dir_entry.owner != Some(user_id) {
            return Err(ResourceError::PermissionDenied);
        }

        let full_path = self.base_directory.join(&normalized_path);
        let mut dir_data = self
            .read_directory_io(&full_path)
            .ok_or(ResourceError::DirectoryNotFound)?;

        // Update subdirectories with inferred entries
        let mut entry = dir_entry.clone();
        for dir in dir_data.subdirectories.iter_mut() {
            let key = self.normalize_path(&dir.path)?.to_string_lossy().to_string();
            if !entry.subdirectories.contains(&key) {
                entry.subdirectories.push(key);
            }
        }

        // Process subdirectories recursively
        let valid_subdirs = entry
            .subdirectories
            .iter()
            .filter_map(|subdir| {
                let subdir_path = self.deanonymize_path(&PathBuf::from(subdir));
                let subdir_normalized = subdir_path.to_string_lossy().to_string();
                let subdir_entry = self.get_directory_entry(&subdir_normalized).ok()?;
                if subdir_entry.scope == DirectoryScope::Global
                    || subdir_entry.owner == Some(user_id)
                {
                    self.request_directory_structure(&subdir_normalized, user_id)
                        .map(|mut subdir_data| {
                            subdir_data.path = self.anonymize_path(&subdir_data.path);
                            subdir_data
                        })
                        .ok()
                } else {
                    None
                }
            })
            .collect();

        dir_data.subdirectories = valid_subdirs;
        Ok(dir_data)
    }

    pub(super) fn request_file_contents(
        &self,
        path: impl AsRef<Path>,
        user_id: Uuid,
    ) -> Result<Box<dyn FileLike>, ResourceError> {
        let normalized_path = self.normalize_path(path.as_ref())?;
        let file_entry = self.get_file_entry(&normalized_path)?;

        if user_id != Uuid::nil() {
            if file_entry.scope != DirectoryScope::Global && file_entry.owner != Some(user_id) {
                return Err(ResourceError::PermissionDenied);
            }
        }
        

        let full_path = self.base_directory.join(&normalized_path);
        self.read_file_io(&full_path)
    }

    /// Gets the full, non-obfuscated path of a resource file. Should only ever be used locally.
    pub(super) fn request_qualifed_path(
        &self,
        path: impl AsRef<Path>
    ) -> Result<PathBuf, ResourceError> {
        let full_path = self.deanonymize_path(&path);
        match fs::exists(&full_path) {
            Ok(exists) => {
                if exists {
                    return Ok(full_path);
                };
                return Err(ResourceError::InvalidPath(path.as_ref().into()));
            },
            Err(e) => Err(ResourceError::IoError(e.to_string()))
        }
    }

    fn read_file_io(&self, path: &PathBuf) -> Result<Box<dyn FileLike>, ResourceError> {
        let file = File::open(path).map_err(|e| ResourceError::IoError(e.to_string()))?;
        Ok(Box::new(ManagedFile::new(path.to_path_buf(), file)))
    }

    fn read_directory_io(&self, path: &PathBuf) -> Option<DirectoryStructure> {
        let entries = fs::read_dir(path).ok()?;
        let mut files = Vec::new();
        let mut directories = Vec::new();

        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                directories.push(DirectoryStructure::new(self.anonymize_path(&entry.path())));
            } else {
                files.push(self.anonymize_path(&entry.path()));
            }
        }

        Some(DirectoryStructure {
            path: self.anonymize_path(path),
            files,
            subdirectories: directories,
        })
    }

    fn normalize_path(&self, path: &Path) -> Result<PathBuf, ResourceError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| ResourceError::InvalidPath(path.to_path_buf()))?;
        let prefix = format!("{}/", self.base_directory.to_string_lossy());
        let normalized = path_str
            .strip_prefix(&prefix)
            .unwrap_or(path_str)
            .trim_start_matches('/')
            .to_string();
        Ok(PathBuf::from(normalized))
    }

    fn find_closest_parent(&self, path: &Path) -> Result<&DirectoryEntry, ResourceError> {
        let path_components: Vec<&str> = path.to_str().ok_or_else(|| ResourceError::InvalidPath(path.to_path_buf()))?.split('/').collect();
        let mut best_match: Option<&DirectoryEntry> = None;
        let mut max_matched_components = 0;

        for entry in &self.directories {
            let entry_components: Vec<&str> = entry.path.split('/').collect();
            let min_len = path_components.len().min(entry_components.len());
            let is_prefix = path_components
                .iter()
                .take(min_len)
                .eq(entry_components.iter().take(min_len));

            if is_prefix && entry_components.len() >= max_matched_components {
                best_match = Some(entry);
                max_matched_components = entry_components.len();
            }
        }

        best_match.ok_or(ResourceError::DirectoryNotFound)
    }

    fn anonymize_path(&self, path: impl AsRef<Path>) -> PathBuf {
        PathBuf::from(self.sanitize(path))
    }

    fn deanonymize_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let stripped = path_str.strip_prefix("res://").unwrap_or(&path_str);
        self.base_directory.join(stripped)
    }

    pub(super) fn sanitize(&self, input: impl AsRef<Path>) -> String {
        let input_path = input.as_ref();
        let relative = input_path
            .strip_prefix(&self.base_directory)
            .unwrap_or(input_path);
        let clean = relative
            .components()
            .filter(|comp| !matches!(comp, std::path::Component::CurDir))
            .fold(PathBuf::new(), |mut acc, comp| {
                if comp != std::path::Component::ParentDir {
                    acc.push(comp);
                } else {
                    acc.pop();
                }
                acc
            });
        format!("res://{}", clean.to_string_lossy().replace('\\', "/").trim_start_matches('/'))
    }
}

impl Default for DirectoryManager {
    fn default() -> Self {
        Self {
            base_directory: PathBuf::from("."),
            directories: vec![
                DirectoryEntry {
                    path: "system".to_string(),
                    scope: DirectoryScope::System,
                    description: "System directory for core system files.".to_string(),
                    owner: None,
                    subdirectories: vec!["tokenizers".to_string()],
                },
                DirectoryEntry {
                    path: "trash".to_string(),
                    scope: DirectoryScope::System,
                    description: "System directory for soft-deleted files".to_string(),
                    owner: None,
                    subdirectories: vec![]
                },
                DirectoryEntry {
                    path: "users".to_string(),
                    scope: DirectoryScope::Global,
                    description: "Base directory for user-specific data.".to_string(),
                    owner: None,
                    subdirectories: vec!["conversations".to_string(), "lorebooks".to_string()],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub path: String,
    pub scope: DirectoryScope,
    pub description: String,
    pub owner: Option<Uuid>,
    pub subdirectories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub scope: DirectoryScope,
    pub owner: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryStructure {
    pub path: PathBuf,
    pub files: Vec<PathBuf>,
    pub subdirectories: Vec<DirectoryStructure>,
}

impl DirectoryStructure {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            files: vec![],
            subdirectories: vec![],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("Directory not found")]
    DirectoryNotFound,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("IO error: {0}")]
    IoError(String),
}