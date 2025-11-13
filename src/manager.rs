use crate::config::Config;
use crate::project::Project;
use crate::runtime::Runtime;
use anyhow::Result;
use std::process::Command;

pub struct Manager {
    config: Config,
}

impl Manager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Config::load()?,
        })
    }

    pub fn run(
        &mut self,
        runtime: &Runtime,
        args: &[String],
        memory: Option<String>,
        save: bool,
    ) -> Result<()> {
        let system_gb = Self::system_memory_gb();
        let final_memory = self.resolve_memory(runtime, &memory, system_gb)?;

        if save && memory.is_some() {
            self.save_project_config(runtime, memory.as_ref().unwrap())?;
        }

        println!("Running {} with args: {:?}", runtime.name(), args);

        // Start the child process and wait for completion
        let child = runtime.execute(args, final_memory.as_deref())?;
        let output = child.wait_with_output()?;

        // Print stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprint!("{}", stderr);

        // Check for OOM error
        if runtime.check_oom_from_output(&stderr) {
            self.handle_oom(runtime)?;
            return Err(anyhow::anyhow!(
                "Out of Memory - Config updated, please retry"
            ));
        }

        // Check if command succeeded
        if !output.status.success() {
            return Err(anyhow::anyhow!("Command failed: {}", output.status));
        }

        Ok(())
    }

    fn resolve_memory(
        &self,
        runtime: &Runtime,
        explicit_memory: &Option<String>,
        system_gb: u64,
    ) -> Result<Option<String>> {
        if let Some(mem) = explicit_memory.as_ref() {
            if let Ok(mem_mb) = mem.parse::<u64>() {
                match runtime.validate_memory(mem_mb, system_gb) {
                    Ok(warning) if !warning.is_empty() => println!("{}", warning),
                    Err(e) => {
                        eprintln!("❌ Error: {}", e);
                        eprintln!("\n{}", runtime.recommend_memory(system_gb));
                        return Err(e);
                    }
                    _ => {}
                }
            }
            return Ok(Some(mem.to_string()));
        }

        if let Ok(project_id) = Project::get_id() {
            if let Some(project_config) = self.config.get_project(&project_id) {
                if project_config.runtime == runtime.name() {
                    let name = Project::get_name(&project_id);
                    println!(
                        "📌 Using saved config for '{}': {} MB",
                        name, project_config.memory
                    );
                    return Ok(Some(project_config.memory.clone()));
                }
            } else if runtime.supports_memory_config() {
                let recommended = Runtime::default_memory(system_gb);
                println!("💡 No saved config. Recommended: {} MB", recommended);
                println!("   Run with -m {} to use and save this value", recommended);
            }
        }

        Ok(None)
    }

    fn save_project_config(&mut self, runtime: &Runtime, memory: &str) -> Result<()> {
        if let Ok(project_id) = Project::get_id() {
            let project_name = Project::get_name(&project_id);

            self.config
                .save_project(project_id, runtime.name().to_string(), memory.to_string());
            self.config.save()?;

            println!(
                "💾 Saved config for '{}': {} {} MB",
                project_name,
                runtime.name(),
                memory
            );
        }
        Ok(())
    }

    fn handle_oom(&mut self, _runtime: &Runtime) -> Result<()> {
        if let Ok(project_id) = Project::get_id()
            && let Some((old, new)) = self.config.increase_project_memory(&project_id)
        {
            self.config.save()?;

            let name = Project::get_name(&project_id);
            println!("\n🔴 Out of Memory Detected!");
            println!("📈 Auto-increased: {} MB → {} MB", old, new);
            println!("💾 Saved for project '{}'", name);
            println!("\n💡 Run the same command again to use {} MB", new);
        }
        Ok(())
    }

    pub fn show_project(&self) -> Result<()> {
        let project_id = Project::get_id()?;
        let project_name = Project::get_name(&project_id);

        println!("📂 Project: {}", project_name);
        println!("   Path: {}", project_id);

        if let Some(config) = self.config.get_project(&project_id) {
            let datetime = Self::format_timestamp(config.last_used);
            println!("\n⚙️  Saved Configuration:");
            println!("   Runtime: {}", config.runtime);
            println!("   Memory: {} MB", config.memory);
            println!("   Last used: {}", datetime);
        } else {
            println!("\n❌ No saved configuration");
            println!("   Run with -m <memory> to save a config");
        }

        Ok(())
    }

    pub fn list_projects(&self) -> Result<()> {
        let projects = self.config.list_projects();

        if projects.is_empty() {
            println!("No saved project configurations");
            return Ok(());
        }

        println!("📚 Saved Project Configurations:\n");

        for (path, config) in projects {
            let name = Project::get_name(path);
            let datetime = Self::format_timestamp(config.last_used);

            println!("  📂 {}", name);
            println!("     Path: {}", path);
            println!(
                "     Runtime: {} | Memory: {} MB | Last used: {}",
                config.runtime, config.memory, datetime
            );
            println!();
        }

        Ok(())
    }

    pub fn forget_project(&mut self, path: Option<String>) -> Result<()> {
        let project_id = path.unwrap_or_else(|| Project::get_id().unwrap());
        let project_name = Project::get_name(&project_id);

        if self.config.remove_project(&project_id) {
            self.config.save()?;
            println!("✅ Removed config for '{}'", project_name);
        } else {
            println!("❌ No config found for '{}'", project_name);
        }

        Ok(())
    }

    pub fn cleanup(&mut self, days: u64) -> Result<()> {
        let before = self.config.list_projects().len();
        self.config.cleanup_old_projects(days);
        let after = self.config.list_projects().len();

        self.config.save()?;

        println!(
            "🧹 Cleaned up {} config(s) older than {} days",
            before - after,
            days
        );
        Ok(())
    }

    pub fn show_recommendations(&self, runtime: &Runtime) -> Result<()> {
        let system_gb = Self::system_memory_gb();

        println!("\n📊 System: {} GB", system_gb);
        println!("\n💡 Recommendations for {}:", runtime.name());
        println!("   {}", runtime.recommend_memory(system_gb));

        if runtime.supports_memory_config() {
            let recommended = Runtime::default_memory(system_gb);
            println!("\n📝 Examples:");
            println!("   frc -m {} {} script.js", recommended, runtime.name());
        }

        Ok(())
    }

    fn system_memory_gb() -> u64 {
        if let Ok(output) = Command::new("sh")
            .arg("-c")
            .arg("sysctl -n hw.memsize 2>/dev/null || grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}'")
            .output()
            && let Ok(stdout) = String::from_utf8(output.stdout)
                && let Ok(bytes) = stdout.trim().parse::<u64>() {
                    return if bytes > 1_000_000_000 {
                        bytes / (1024 * 1024 * 1024)
                    } else {
                        bytes / (1024 * 1024)
                    };
                }
        16
    }

    fn format_timestamp(ts: u64) -> String {
        chrono::DateTime::from_timestamp(ts as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}
