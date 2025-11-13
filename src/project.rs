use anyhow::Result;
use std::env;
use std::path::{Path, PathBuf};

pub struct Project;

impl Project {
    const MARKERS: &'static [&'static str] = &[
        "package.json",
        "deno.json",
        "deno.jsonc",
        "Cargo.toml",
        ".git",
        "pnpm-workspace.yaml",
        "lerna.json",
        "nx.json",
    ];

    pub fn detect_root() -> Result<PathBuf> {
        let current_dir = env::current_dir()?;
        let mut dir = current_dir.as_path();

        loop {
            for marker in Self::MARKERS {
                if dir.join(marker).exists() {
                    return Ok(dir.to_path_buf());
                }
            }

            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }

        Ok(current_dir)
    }

    pub fn get_id() -> Result<String> {
        let root = Self::detect_root()?;
        Ok(root.to_string_lossy().to_string())
    }

    pub fn get_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_detect_root_with_cargo() {
        // Current project should have Cargo.toml
        let root = Project::detect_root().unwrap();
        assert!(root.join("Cargo.toml").exists());
    }

    #[test]
    fn test_get_id() {
        let id = Project::get_id().unwrap();
        assert!(!id.is_empty());
        assert!(Path::new(&id).is_absolute());
    }

    #[test]
    fn test_get_name() {
        assert_eq!(Project::get_name("/path/to/my-project"), "my-project");
        assert_eq!(Project::get_name("/Users/dev/project-a"), "project-a");
        assert_eq!(Project::get_name("/single"), "single");
        assert_eq!(Project::get_name("relative/path"), "path");
    }

    #[test]
    fn test_get_name_current_project() {
        let id = Project::get_id().unwrap();
        let name = Project::get_name(&id);
        assert_eq!(name, "fe-run-container");
    }

    #[test]
    fn test_markers() {
        // Verify that MARKERS contains expected files
        assert!(Project::MARKERS.contains(&"package.json"));
        assert!(Project::MARKERS.contains(&"Cargo.toml"));
        assert!(Project::MARKERS.contains(&".git"));
        assert!(Project::MARKERS.contains(&"deno.json"));
    }

    #[test]
    fn test_detect_root_in_temp_dir() {
        // Test when no markers are found
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("frc-test-{}", std::process::id()));
        fs::create_dir_all(&test_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&test_dir).unwrap();

        let root = Project::detect_root().unwrap();
        // Should return current dir when no markers found
        // Use canonicalize to resolve symlinks (macOS /var -> /private/var)
        let expected = test_dir.canonicalize().unwrap();
        let actual = root.canonicalize().unwrap();
        assert_eq!(actual, expected);

        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&test_dir).ok();
    }
}
