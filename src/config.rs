use crate::storage::{ProjectSettings, Storage, StorageData};
use anyhow::Result;

pub struct Config {
    data: StorageData,
}

impl Config {
    pub fn load() -> Result<Self> {
        let data = Storage::load()?;
        Ok(Self { data })
    }

    pub fn save(&self) -> Result<()> {
        Storage::save(&self.data)
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn get_project(&self, path: &str) -> Option<&ProjectSettings> {
        self.data.projects.get(path)
    }

    pub fn save_project(&mut self, path: String, runtime: String, memory: String) {
        self.data.projects.insert(
            path,
            ProjectSettings {
                runtime,
                memory,
                last_used: Self::current_timestamp(),
            },
        );
    }

    pub fn remove_project(&mut self, path: &str) -> bool {
        self.data.projects.remove(path).is_some()
    }

    pub fn list_projects(&self) -> Vec<(&String, &ProjectSettings)> {
        let mut projects: Vec<_> = self.data.projects.iter().collect();
        projects.sort_by(|a, b| b.1.last_used.cmp(&a.1.last_used));
        projects
    }

    pub fn cleanup_old_projects(&mut self, days: u64) {
        let cutoff = Self::current_timestamp() - (days * 24 * 60 * 60);
        self.data.projects.retain(|_, proj| proj.last_used > cutoff);
    }

    pub fn increase_project_memory(&mut self, path: &str) -> Option<(String, String)> {
        let project = self.data.projects.get_mut(path)?;
        let old_memory = project.memory.clone();
        let current_mb = old_memory.parse::<u64>().ok()?;

        let increase_50 = (current_mb as f64 * 1.5) as u64;
        let increase_2gb = current_mb + 2048;
        let new_memory = increase_50.max(increase_2gb);

        project.memory = new_memory.to_string();
        Some((old_memory, new_memory.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            data: StorageData {
                projects: std::collections::HashMap::new(),
            },
        }
    }

    #[test]
    fn test_save_and_get_project() {
        let mut config = create_test_config();

        config.save_project(
            "/path/to/project".to_string(),
            "node".to_string(),
            "8192".to_string(),
        );

        let project = config.get_project("/path/to/project").unwrap();
        assert_eq!(project.runtime, "node");
        assert_eq!(project.memory, "8192");
    }

    #[test]
    fn test_remove_project() {
        let mut config = create_test_config();

        config.save_project(
            "/path/to/project".to_string(),
            "node".to_string(),
            "4096".to_string(),
        );

        assert!(config.remove_project("/path/to/project"));
        assert!(!config.remove_project("/path/to/project")); // Already removed
        assert!(config.get_project("/path/to/project").is_none());
    }

    #[test]
    fn test_list_projects() {
        let mut config = create_test_config();

        config.save_project("/project-a".to_string(), "node".to_string(), "4096".to_string());
        std::thread::sleep(std::time::Duration::from_secs(1));
        config.save_project("/project-b".to_string(), "deno".to_string(), "8192".to_string());

        let projects = config.list_projects();
        assert_eq!(projects.len(), 2);

        // Should be sorted by last_used (newest first)
        assert_eq!(projects[0].0, "/project-b");
        assert_eq!(projects[1].0, "/project-a");
    }

    #[test]
    fn test_cleanup_old_projects() {
        let mut config = create_test_config();

        // Add project with old timestamp
        config.data.projects.insert(
            "/old-project".to_string(),
            ProjectSettings {
                runtime: "node".to_string(),
                memory: "4096".to_string(),
                last_used: 1000, // Very old timestamp
            },
        );

        // Add recent project
        config.save_project("/new-project".to_string(), "node".to_string(), "4096".to_string());

        config.cleanup_old_projects(1); // Remove projects older than 1 day

        assert!(config.get_project("/old-project").is_none());
        assert!(config.get_project("/new-project").is_some());
    }

    #[test]
    fn test_increase_project_memory() {
        let mut config = create_test_config();

        config.save_project("/project".to_string(), "node".to_string(), "4096".to_string());

        let (old, new) = config.increase_project_memory("/project").unwrap();
        assert_eq!(old, "4096");
        assert_eq!(new, "6144"); // max(4096 * 1.5, 4096 + 2048) = 6144

        let project = config.get_project("/project").unwrap();
        assert_eq!(project.memory, "6144");
    }

    #[test]
    fn test_increase_project_memory_small_value() {
        let mut config = create_test_config();

        config.save_project("/project".to_string(), "node".to_string(), "1024".to_string());

        let (old, new) = config.increase_project_memory("/project").unwrap();
        assert_eq!(old, "1024");
        assert_eq!(new, "3072"); // max(1024 * 1.5, 1024 + 2048) = 3072
    }

    #[test]
    fn test_increase_project_memory_nonexistent() {
        let mut config = create_test_config();
        assert!(config.increase_project_memory("/nonexistent").is_none());
    }
}
