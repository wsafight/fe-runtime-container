# FRC - Frontend Runtime Container

A command-line tool for managing memory configuration of JavaScript runtimes (Node.js, Deno, Bun).

[中文文档](./README.md) | English

## Core Features

- **Auto-save Configuration** - Set memory on first run, automatically applied afterwards
- **Smart Recommendations** - Provides reasonable configuration based on system memory
- **OOM Auto-recovery** - Automatically increases memory when out-of-memory is detected
- **Multi-project Management** - Independent configuration for each project
- **Zero Performance Overhead** - Only sets environment variables, no runtime cost

## Installation

```bash
# Clone repository
git clone https://github.com/yourusername/fe-run-container.git
cd fe-run-container

# Build
cargo build --release

# Install to system (optional)
cargo install --path .
```

## Quick Start

```bash
# First run: set 4GB memory and save
frc -m 4096 npm run build

# Subsequent runs: automatically use 4GB
frc npm run build

# View system recommendations
frc info node
```

## Usage

### Basic Commands

```bash
# Run command with memory limit
frc -m <MB> <command> [args...]

# View system recommendations
frc info <runtime>

# View current project configuration
frc project

# List all project configurations
frc list

# Remove current project configuration
frc forget

# Clean up old configurations (30 days unused)
frc cleanup --days 30
```

### Supported Runtimes

| Runtime | Commands | Memory Config | Implementation |
|---------|----------|---------------|----------------|
| Node.js | node, npm, npx, pnpm, yarn | ✅ | `NODE_OPTIONS` environment variable |
| Deno | deno | ✅ | `--v8-flags` command-line argument |
| Bun | bun | ❌ | JavaScriptCore auto-managed |

**Note**: Bun uses JavaScriptCore engine and does not support manual memory configuration.

## Workflow Examples

### Initial Configuration

```bash
cd my-project

# Set 8GB and save
frc -m 8192 npm run build
# Output: 💾 Saved config for 'my-project': node 8192 MB
```

### Auto-apply

```bash
# Subsequent runs automatically use 8GB
frc npm run dev
# Output: 📌 Using saved config for 'my-project': 8192 MB
```

### OOM Auto-recovery

```bash
frc npm run build
# If OOM occurs, output:
# 🔴 Out of Memory Detected!
# 📈 Auto-increased: 4096 MB → 6144 MB
# 💡 Run the same command again to use 6144 MB

# Re-run to use new configuration
frc npm run build
```

## Memory Configuration Recommendations

### By System Memory

| System Memory | Small Project | Medium Project | Large Project |
|---------------|---------------|----------------|---------------|
| 16GB | 2GB | 4GB | 6GB |
| 32GB | 4GB | 8GB | 12GB |
| 64GB+ | - | 16GB | 24GB |

### By Build Tool

```bash
# Webpack project
frc -m 8192 npm run build

# Vite project
frc -m 4096 npm run build

# Next.js project
frc -m 6144 npm run build

# Development server
frc -m 4096 npm run dev

# Run tests
frc -m 4096 npm test
```

**Configuration Principles**:
- Development environment: Allocate 20-40% of system memory
- Production build: Allocate 50-75% of container memory

## package.json Integration

```json
{
  "scripts": {
    "dev": "frc -m 4096 vite dev",
    "build": "frc -m 8192 vite build",
    "test": "frc -m 4096 vitest"
  }
}
```

## Technical Implementation

### Environment Variable Mechanism

**Node.js Implementation:**
```rust
// Set environment variable for child process
cmd.env("NODE_OPTIONS", "--max-old-space-size=4096")
```

**Deno Implementation:**
```rust
// Pass via command-line argument
cmd.arg("--v8-flags").arg("--max-old-space-size=4096")
```

**Execution Flow:**
```
System Env → frc Process → Child Process (set env) → node/deno Process
    ↓            ↓                  ↓                        ↓
  Clean        Clean       NODE_OPTIONS Active       Memory Limit Active
```

**Core Characteristics:**
- Environment variables **only affect child processes**, don't pollute system environment
- Child processes run independently after inheriting variables, unaffected by frc exit
- Each frc process is completely independent, can run multiple projects simultaneously
- Different projects don't interfere with each other

**Verification:**
```bash
# System environment remains clean
$ echo $NODE_OPTIONS
(empty)

# Environment variable works in child process
$ frc -m 4096 node -e "console.log(process.env.NODE_OPTIONS)"
--max-old-space-size=4096

# System environment still clean after frc exits
$ echo $NODE_OPTIONS
(empty)
```

### Project Detection

Automatically identifies project root directory by these marker files:
- `package.json` - Node.js project
- `.git` - Git repository
- `Cargo.toml` - Rust project
- `deno.json` - Deno project
- `pnpm-workspace.yaml` - pnpm monorepo
- `lerna.json` - Lerna monorepo
- `nx.json` - Nx workspace

### Configuration Storage

Configuration file location: `~/.config/frc/config.json`

Stored content:
- Runtime type for each project (node/deno)
- Memory configuration (MB)
- Last used time (for automatic cleanup)

### OOM Detection and Recovery

**Detection Mechanism**: Monitors process stderr for these error keywords:
- `JavaScript heap out of memory`
- `FATAL ERROR: Reached heap limit`
- `Allocation failed`

**Auto-recovery Strategy**:
- New memory = max(current × 1.5, current + 2048 MB)
- Automatically updates project configuration
- Prompts user to re-run command

### Architecture Design

```
src/
├── main.rs      - CLI entry and command parsing
├── manager.rs   - Core business logic coordination
├── config.rs    - Configuration management and business logic
├── storage.rs   - JSON configuration file I/O
├── project.rs   - Project root directory detection
└── runtime.rs   - Runtime abstraction and execution
```

## Command Reference

### Main Command

```bash
frc [OPTIONS] <COMMAND> [ARGS]...
```

**Options:**
- `-m, --memory <MB>` - Set memory limit (unit: MB)
- `-r, --runtime <RUNTIME>` - Explicitly specify runtime (node/deno/bun)
- `-h, --help` - Show help information
- `-V, --version` - Show version number

### Subcommands

| Command | Description | Example |
|---------|-------------|---------|
| `info <runtime>` | Show memory recommendations | `frc info node` |
| `project` | Show current project configuration | `frc project` |
| `list` | List all project configurations | `frc list` |
| `forget [path]` | Remove project configuration | `frc forget` |
| `cleanup --days <N>` | Clean up configs unused for N days | `frc cleanup --days 30` |

## FAQ

**Q: Will environment variables pollute the system?**
A: No. Environment variables only affect child processes launched by frc, not the system environment or other processes.

**Q: Will long-running processes (like dev server) lose effectiveness?**
A: No. Child processes run independently after inheriting environment variables, unaffected by frc exit.
```bash
# dev server continues running, unaffected by frc exit
frc -m 4096 npm run dev
```

**Q: Can multiple projects run simultaneously?**
A: Yes. Each frc process is completely independent, multiple projects can run in different terminals without interfering with each other.

**Q: Why doesn't Bun support memory configuration?**
A: Bun uses the JavaScriptCore engine, which doesn't provide manual memory configuration options and manages memory automatically.

**Q: Where is the configuration file saved?**
A: `~/.config/frc/config.json`

**Q: How do I determine how much memory to set?**
A: Run `frc info node` to see recommended configuration values based on system memory.

**Q: Will it affect program performance?**
A: No. FRC only sets environment variables at startup, with no runtime overhead.

**Q: Can it be used in CI/CD?**
A: Yes. Examples:

```yaml
# GitHub Actions
- name: Build
  run: frc -m 4096 npm run build

# GitLab CI
build:
  script:
    - frc -m 8192 npm run build
```

## Development

```bash
# Run tests
cargo test

# Run in development mode
cargo run -- -m 4096 node script.js

# Build release version
cargo build --release

# Run clippy checks
cargo clippy
```

## License

MIT

---

**Contributing**: Issues and Pull Requests are welcome!
