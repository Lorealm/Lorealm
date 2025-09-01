use std:: path::PathBuf;
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum DirectoryScope {
    Global,    // For all users
    Local,     // For a specific user
    System,    // For the system; Generally inaccessible to users
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceConfig {
    resource_dir: PathBuf,
    directory_config: DirectoryConfig
}

impl ResourceConfig {
    pub fn new(resource_dir: PathBuf, directory_config: Option<DirectoryConfig>) -> Self {
        let mut directory_config = directory_config.unwrap_or_default();
        directory_config.set_root_directory(resource_dir.clone());

        Self {
            resource_dir,
            directory_config
        }
    }

    pub fn resource_dir(&self) -> &PathBuf {
        &self.resource_dir
    }

    pub fn directory_config(&self) -> &DirectoryConfig {
        &self.directory_config
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryConfig {
    base_directory: PathBuf,
    directories: Vec<DirectoryEntry>
}

impl DirectoryConfig {

    pub fn set_root_directory(&mut self, path: PathBuf) {
        self.base_directory = path;
    }

    /// Gets or infers a DirectoryEntry for the given path, creating a temporary entry if the exact path doesn't exist.
    pub fn get_directory(&self, path: &str) -> Option<DirectoryEntry> {
        let prefix = format!("{}/", self.base_directory.to_string_lossy());
        let normalized_path = path
            .strip_prefix(&prefix)
            .unwrap_or(path);
        
        // Check for an exact match first
        if let Some(entry) = self.directories.iter().find(|d| d.path == normalized_path) {
            return Some(entry.clone());
        }

        // Find the closest matching parent directory
        let path_components: Vec<&str> = normalized_path.split('/').collect();
        let mut best_match: Option<&DirectoryEntry> = None;
        let mut max_matched_components = 0;

        for entry in &self.directories {
            let entry_components: Vec<&str> = entry.path.split('/').collect();
            let min_len = path_components.len().min(entry_components.len());
            
            // Check if the entry path is a prefix of the requested path
            let is_prefix = path_components.iter().take(min_len).eq(entry_components.iter().take(min_len));
            
            if is_prefix && entry_components.len() >= max_matched_components {
                // Prefer the entry with the longest matching path
                if entry_components.len() > max_matched_components || best_match.is_none() {
                    best_match = Some(entry);
                    max_matched_components = entry_components.len();
                }
            }
        }

        // If no parent is found, return None
        let parent_entry = best_match?;

        // Create a temporary DirectoryEntry for the requested path
        let immediate_subdirs = self.read_directory_io(&PathBuf::from(normalized_path))
            .map(|dir_data| {
                dir_data.subdirectories.into_iter()
                    .filter_map(|subdir| subdir.path.file_name().and_then(|name| name.to_str().map(String::from)))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        // Return a new temporary DirectoryEntry with inherited permissions
        Some(DirectoryEntry {
            path: normalized_path.to_string(),
            scope: parent_entry.scope.clone(),
            description: format!("Inferred directory from parent: {}", parent_entry.path),
            owner: parent_entry.owner,
            subdirectiores: immediate_subdirs,
        })
    }

    pub fn get_directories(&self) -> &Vec<DirectoryEntry> {
        &self.directories
    }

    pub fn add_directory(&mut self, directory: DirectoryEntry) {
        self.directories.push(directory);
    }

    pub fn add_user_directory(&mut self, user_id: Uuid) {
        self.add_directory(DirectoryEntry {
            path: format!("users/{}", user_id),
            scope: DirectoryScope::Local,
            description: String::from("User directory. Stores files required for the user to run."),
            owner: Some(user_id),
            subdirectiores: vec!["conversations", "lorebooks"].iter().map(|s| s.to_string()).collect()
        });
    }

    pub fn remove_user_directory(&mut self, user_id: Uuid) {
        self.directories.retain(|d| d.owner != Some(user_id));
    }

    pub fn get_accessible_directories(&self, user_id: Uuid) -> Vec<&DirectoryEntry> {
        self.directories.iter().filter(|d| d.scope == DirectoryScope::Global || d.owner == Some(user_id)).collect()
    }

    pub fn request_directory_structure(&self, path: &str, user_id: Uuid) -> Result<DirectoryData, Box<dyn std::error::Error>> {
        let prefix = format!("{}/", self.base_directory.to_string_lossy());
        let normalized_path = path
            .strip_prefix(&prefix)
            .unwrap_or(path);
        println!("Requesting directory structure for path: {}", normalized_path);

        // Verify access to the initial directory
        let dir_entry = self.get_directory(normalized_path);
        if let Some(entry) = &dir_entry {
            if entry.scope != DirectoryScope::Global && entry.owner != Some(user_id) {
                return Err("Permission denied".into());
            }
        } else {
            return Err("Directory not found".into());
        }

        // Get the initial directory entry
        let mut entry = dir_entry.clone().unwrap();
        let initial_path = self.base_directory.join(&normalized_path);
        
        // Read the directory from the file system
        let mut dir_data = match self.read_directory_io(&initial_path) {
            Some(data) => data,
            None => return Err("Failed to read directory".into()),
        };

        // Add non-tracked subdirectories
        for dir in dir_data.subdirectories.iter_mut() {
            let prefix = "res:/".to_string();
            let key = &dir.path.strip_prefix(&prefix).unwrap().to_string_lossy().to_string();
            if !entry.subdirectiores.contains(key) {
                entry.subdirectiores.push(key.clone());
            }
        }

        // Recursively process subdirectories
        let mut valid_subdirectories = Vec::new();
        for subdir in &entry.subdirectiores {
            let subdir_path = self.deanonymize_path(&PathBuf::from(&subdir));
            println!("Requesting directory structure for subdirectory: {}", subdir_path.display());
            let subdir_normalized = subdir_path.to_string_lossy().to_string();
            
            // Check permissions for subdirectory
            let subdir_entry = self.get_directory(&subdir_normalized);
            if let Some(sub_entry) = subdir_entry {
                if sub_entry.scope == DirectoryScope::Global || sub_entry.owner == Some(user_id) {
                    // Recursively request subdirectory structure
                    
                    if let Ok(mut subdir_data) = self.request_directory_structure(&subdir_normalized, user_id) {
                        subdir_data.path = self.anonymize_path(&subdir_data.path);
                        valid_subdirectories.push(subdir_data);
                    }
                }
            }
        }
        
        // Update the directory data with valid subdirectories
        dir_data.subdirectories = valid_subdirectories;
        
        Ok(dir_data)
    }

    /// Reads a directory from the file system
    fn read_directory_io(&self, path: &PathBuf) -> Option<DirectoryData> {
        let entries = std::fs::read_dir(path);
        if let Ok(entries) = entries {
            let mut files = Vec::new();
            let mut directories = Vec::new();
            for entry in entries {
                let entry = entry.unwrap();
                if entry.path().is_dir() {
                    directories.push(DirectoryData::new(entry.path()));
                } else {
                    files.push(entry.path());
                }
            }
            Some(DirectoryData {
                path: path.to_path_buf(),
                files: files.iter().map(|f| self.anonymize_path(f)).collect(),
                subdirectories: directories.iter().map(|d| DirectoryData::new(self.anonymize_path(&d.path))).collect(),
            })
        } else {
            None
        }
    }

    fn anonymize_path(&self, path: &PathBuf) -> PathBuf {
        let path_str = path.to_string_lossy().to_string();
        let prefix = self.base_directory.to_string_lossy().to_string();
        let stripped_path = path_str.strip_prefix(&prefix).unwrap_or(&path_str).to_string();
        PathBuf::from(format!("res:/{}", stripped_path))
    }

    fn deanonymize_path(&self, path: &PathBuf) -> PathBuf {
        let path_str = path.to_string_lossy().to_string();
        let prefix = self.base_directory.to_string_lossy().to_string();
        let stripped_path = path_str.strip_prefix("res:/").unwrap_or(&path_str).to_string();
        PathBuf::from(format!("{}/{}", prefix, stripped_path))
    }
}

impl Default for DirectoryConfig {
    fn default() -> Self {
        Self {
            base_directory: PathBuf::from("."),
            directories: vec![
                DirectoryEntry {
                    path: String::from("system"),
                    scope: DirectoryScope::Global,
                    description: String::from("System directory. Stores files required for the system to run."),
                    owner: None,
                    subdirectiores: vec!["tokenizers".to_string()]
                },
                DirectoryEntry {
                    path: String::from("users"),
                    scope: DirectoryScope::Global, // The base user directory can be accessed by all users. Limitations apply further down the line
                    description: String::from("User directory. Stores files required for the user to run."),
                    owner: None,
                    subdirectiores: vec!["conversations", "lorebooks"].iter().map(|s| s.to_string()).collect()
                }
            ]
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    /// The path to the directory relative to the resource directory.
    pub path: String,
    pub scope: DirectoryScope,
    pub description: String,
    pub owner: Option<Uuid>,
    /// A list of immediate subdirectores. All derived subdirectories retain the same scope and owner as its parent.
    pub subdirectiores: Vec<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryData {
    pub path: PathBuf,
    pub files: Vec<PathBuf>,
    pub subdirectories: Vec<DirectoryData>
}

impl DirectoryData {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            files: vec![],
            subdirectories: vec![]
        }
    }
}