use anyhow::{anyhow, Result};
use std::process::{Child, Command};

#[derive(Debug, Clone, PartialEq)]
pub enum Runtime {
    Node,
    Deno,
    Bun,
}

impl Runtime {
    pub fn from_command(cmd: &str) -> Result<Self> {
        match cmd.to_lowercase().as_str() {
            "node" => Ok(Runtime::Node),
            "deno" => Ok(Runtime::Deno),
            "bun" => Ok(Runtime::Bun),
            "npm" | "npx" | "pnpm" | "yarn" => Ok(Runtime::Node),
            _ => Err(anyhow!("Unknown runtime: {}", cmd)),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Runtime::Node => "node",
            Runtime::Deno => "deno",
            Runtime::Bun => "bun",
        }
    }

    pub fn as_str(&self) -> &str {
        self.name()
    }

    pub fn supports_memory_config(&self) -> bool {
        matches!(self, Runtime::Node | Runtime::Deno)
    }

    pub fn execute(&self, args: &[String], memory: Option<&str>) -> Result<Child> {
        if !self.supports_memory_config() && memory.is_some() {
            println!("⚠️  WARNING: Bun does not support manual memory configuration!");
            println!("   Bun uses JavaScriptCore and manages memory automatically.");
            println!("   Memory flag will be ignored.\n");
        }

        let mut cmd = Command::new(self.name());
        self.configure_memory(&mut cmd, memory);
        cmd.args(args);
        cmd.stderr(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::inherit());

        let child = cmd.spawn()?;
        Ok(child)
    }

    pub fn check_oom_from_output(&self, stderr: &str) -> bool {
        self.is_oom_error(stderr)
    }

    fn configure_memory(&self, cmd: &mut Command, memory: Option<&str>) {
        let Some(mem) = memory else { return };

        match self {
            Runtime::Node => {
                println!("Setting memory limit to {} MB for Node.js", mem);
                let flag = format!("--max-old-space-size={}", mem);
                let current = std::env::var("NODE_OPTIONS").unwrap_or_default();
                let value = if current.is_empty() {
                    flag
                } else {
                    format!("{} {}", current, flag)
                };
                cmd.env("NODE_OPTIONS", value);
            }
            Runtime::Deno => {
                println!("Setting memory limit to {} MB for Deno", mem);
                let flag = format!("--max-old-space-size={}", mem);
                cmd.arg("--v8-flags").arg(&flag);
            }
            Runtime::Bun => {}
        }
    }

    fn is_oom_error(&self, stderr: &str) -> bool {
        let patterns = [
            "JavaScript heap out of memory",
            "FATAL ERROR: Reached heap limit",
            "Allocation failed",
            "heap out of memory",
        ];

        let stderr_lower = stderr.to_lowercase();
        patterns
            .iter()
            .any(|p| stderr_lower.contains(&p.to_lowercase()))
    }

    pub fn recommend_memory(&self, system_gb: u64) -> String {
        if !self.supports_memory_config() {
            return "Bun manages memory automatically (GC at ~80% system memory)".to_string();
        }

        let recommendation = match system_gb {
            gb if gb >= 64 => "For 64GB+: 16384-24576 MB for large projects",
            gb if gb >= 32 => "For 32GB: 8192-12288 MB for large projects",
            gb if gb >= 16 => "For 16GB: 4096-6144 MB for large projects",
            _ => "For <16GB: 2048-4096 MB",
        };

        format!(
            "{}\nRule: Allocate 20-40% of system memory for development",
            recommendation
        )
    }

    pub fn validate_memory(&self, memory_mb: u64, system_gb: u64) -> Result<String> {
        if !self.supports_memory_config() {
            return Ok(String::new());
        }

        let system_mb = system_gb * 1024;
        let percentage = (memory_mb as f64 / system_mb as f64) * 100.0;

        if memory_mb > system_mb {
            return Err(anyhow!(
                "Memory limit ({} MB) exceeds system memory ({} GB)",
                memory_mb,
                system_gb
            ));
        }

        if percentage > 75.0 {
            Ok(format!(
                "⚠️  Warning: {}% of system memory (recommended: 20-40% dev, 50-75% prod)",
                percentage as u32
            ))
        } else if percentage < 10.0 {
            Ok(format!(
                "ℹ️  Info: Only {}% of system memory, can increase for better performance",
                percentage as u32
            ))
        } else {
            Ok(String::new())
        }
    }

    pub fn default_memory(system_gb: u64) -> u64 {
        match system_gb {
            gb if gb >= 64 => 16384,
            gb if gb >= 32 => 8192,
            gb if gb >= 16 => 4096,
            _ => 2048,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_from_command() {
        assert_eq!(Runtime::from_command("node").unwrap(), Runtime::Node);
        assert_eq!(Runtime::from_command("NODE").unwrap(), Runtime::Node);
        assert_eq!(Runtime::from_command("npm").unwrap(), Runtime::Node);
        assert_eq!(Runtime::from_command("npx").unwrap(), Runtime::Node);
        assert_eq!(Runtime::from_command("pnpm").unwrap(), Runtime::Node);
        assert_eq!(Runtime::from_command("yarn").unwrap(), Runtime::Node);
        assert_eq!(Runtime::from_command("deno").unwrap(), Runtime::Deno);
        assert_eq!(Runtime::from_command("DENO").unwrap(), Runtime::Deno);
        assert_eq!(Runtime::from_command("bun").unwrap(), Runtime::Bun);
        assert!(Runtime::from_command("unknown").is_err());
    }

    #[test]
    fn test_runtime_name() {
        assert_eq!(Runtime::Node.name(), "node");
        assert_eq!(Runtime::Deno.name(), "deno");
        assert_eq!(Runtime::Bun.name(), "bun");
    }

    #[test]
    fn test_runtime_as_str() {
        assert_eq!(Runtime::Node.as_str(), "node");
        assert_eq!(Runtime::Deno.as_str(), "deno");
        assert_eq!(Runtime::Bun.as_str(), "bun");
    }

    #[test]
    fn test_supports_memory_config() {
        assert!(Runtime::Node.supports_memory_config());
        assert!(Runtime::Deno.supports_memory_config());
        assert!(!Runtime::Bun.supports_memory_config());
    }

    #[test]
    fn test_validate_memory() {
        let runtime = Runtime::Node;

        // Valid memory
        assert!(runtime.validate_memory(4096, 16).is_ok());
        assert!(runtime.validate_memory(8192, 32).is_ok());

        // Exceeds system memory
        assert!(runtime.validate_memory(20480, 16).is_err());

        // Warning: too high percentage
        let result = runtime.validate_memory(14336, 16).unwrap();
        assert!(result.contains("Warning"));

        // Warning: too low percentage
        let result = runtime.validate_memory(512, 16).unwrap();
        assert!(result.contains("Info"));

        // Bun doesn't validate
        assert!(Runtime::Bun.validate_memory(4096, 16).unwrap().is_empty());
    }

    #[test]
    fn test_recommend_memory() {
        let node = Runtime::Node;
        assert!(node.recommend_memory(16).contains("For 16GB"));
        assert!(node.recommend_memory(32).contains("For 32GB"));
        assert!(node.recommend_memory(64).contains("For 64GB+"));

        let bun = Runtime::Bun;
        assert!(bun.recommend_memory(16).contains("automatically"));
    }

    #[test]
    fn test_default_memory() {
        assert_eq!(Runtime::default_memory(8), 2048);
        assert_eq!(Runtime::default_memory(16), 4096);
        assert_eq!(Runtime::default_memory(32), 8192);
        assert_eq!(Runtime::default_memory(64), 16384);
        assert_eq!(Runtime::default_memory(128), 16384);
    }

    #[test]
    fn test_is_oom_error() {
        let runtime = Runtime::Node;

        assert!(runtime.is_oom_error("FATAL ERROR: JavaScript heap out of memory"));
        assert!(runtime.is_oom_error("FATAL ERROR: Reached heap limit Allocation failed"));
        assert!(runtime.is_oom_error("some error heap out of memory details"));
        assert!(!runtime.is_oom_error("Some other error"));
        assert!(!runtime.is_oom_error("Success"));
    }
}
