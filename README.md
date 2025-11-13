# FRC - Frontend Runtime Container

为 JavaScript 运行时（Node.js、Deno、Bun）管理内存配置的命令行工具。

中文 | [English](./README_EN.md)

## 核心特性

- **自动保存配置** - 首次运行指定内存，后续自动应用
- **智能推荐** - 根据系统内存给出合理的配置建议
- **OOM 自动恢复** - 检测内存溢出时自动增加配置
- **多项目管理** - 每个项目独立配置，互不干扰
- **零性能开销** - 仅设置环境变量，无运行时损耗

## 安装

```bash
# 克隆仓库
git clone https://github.com/yourusername/fe-run-container.git
cd fe-run-container

# 构建
cargo build --release

# 安装到系统（可选）
cargo install --path .
```

## 快速开始

```bash
# 首次运行：设置 4GB 内存并保存
frc -m 4096 npm run build

# 后续运行：自动使用 4GB
frc npm run build

# 查看系统推荐配置
frc info node
```

## 使用说明

### 基本命令

```bash
# 运行命令并设置内存限制
frc -m <MB> <command> [args...]

# 查看系统推荐配置
frc info <runtime>

# 查看当前项目配置
frc project

# 列出所有项目配置
frc list

# 删除当前项目配置
frc forget

# 清理旧配置（30 天未使用）
frc cleanup --days 30
```

### 支持的运行时

| 运行时 | 命令 | 内存配置 | 实现方式 |
|--------|------|---------|---------|
| Node.js | node, npm, npx, pnpm, yarn | ✅ | `NODE_OPTIONS` 环境变量 |
| Deno | deno | ✅ | `--v8-flags` 命令行参数 |
| Bun | bun | ❌ | JavaScriptCore 自动管理 |

**注意**：Bun 使用 JavaScriptCore 引擎，不支持手动内存配置。

## 工作流程示例

### 首次配置

```bash
cd my-project

# 设置 8GB 并保存
frc -m 8192 npm run build
# 输出：💾 Saved config for 'my-project': node 8192 MB
```

### 自动应用

```bash
# 后续运行自动使用 8GB
frc npm run dev
# 输出：📌 Using saved config for 'my-project': 8192 MB
```

### OOM 自动恢复

```bash
frc npm run build
# 如果发生 OOM，输出：
# 🔴 Out of Memory Detected!
# 📈 Auto-increased: 4096 MB → 6144 MB
# 💡 Run the same command again to use 6144 MB

# 重新运行即可使用新配置
frc npm run build
```

## 内存配置推荐

### 按系统内存

| 系统内存 | 小项目 | 中型项目 | 大项目 |
|---------|--------|---------|--------|
| 16GB | 2GB | 4GB | 6GB |
| 32GB | 4GB | 8GB | 12GB |
| 64GB+ | - | 16GB | 24GB |

### 按构建工具

```bash
# Webpack 项目
frc -m 8192 npm run build

# Vite 项目
frc -m 4096 npm run build

# Next.js 项目
frc -m 6144 npm run build

# 开发服务器
frc -m 4096 npm run dev

# 运行测试
frc -m 4096 npm test
```

**配置原则**：
- 开发环境：分配 20-40% 系统内存
- 生产构建：分配 50-75% 容器内存

## package.json 集成

```json
{
  "scripts": {
    "dev": "frc -m 4096 vite dev",
    "build": "frc -m 8192 vite build",
    "test": "frc -m 4096 vitest"
  }
}
```

## 技术实现

### 环境变量机制

**Node.js 实现：**
```rust
// 为子进程设置环境变量
cmd.env("NODE_OPTIONS", "--max-old-space-size=4096")
```

**Deno 实现：**
```rust
// 通过命令行参数传递
cmd.arg("--v8-flags").arg("--max-old-space-size=4096")
```

**执行流程：**
```
系统环境 → frc 进程 → 子进程（设置环境变量）→ node/deno 进程
   ↓           ↓              ↓                      ↓
 干净       干净      NODE_OPTIONS 生效        内存限制生效
```

**核心特点：**
- 环境变量**仅对子进程有效**，不污染系统环境
- 子进程继承后独立运行，frc 退出不影响子进程
- 每个 frc 进程完全独立，可同时运行多个项目
- 不同项目之间互不干扰

**验证方式：**
```bash
# 系统环境保持干净
$ echo $NODE_OPTIONS
(空)

# 子进程中环境变量生效
$ frc -m 4096 node -e "console.log(process.env.NODE_OPTIONS)"
--max-old-space-size=4096

# frc 退出后系统环境仍然干净
$ echo $NODE_OPTIONS
(空)
```

### 项目检测

自动识别项目根目录，通过以下文件标记：
- `package.json` - Node.js 项目
- `.git` - Git 仓库
- `Cargo.toml` - Rust 项目
- `deno.json` - Deno 项目
- `pnpm-workspace.yaml` - pnpm monorepo
- `lerna.json` - Lerna monorepo
- `nx.json` - Nx workspace

### 配置存储

配置文件位置：`~/.config/frc/config.json`

存储内容：
- 每个项目的运行时类型（node/deno）
- 内存配置（MB）
- 最后使用时间（用于自动清理）

### OOM 检测与恢复

**检测机制**：监控进程 stderr，识别以下错误关键字：
- `JavaScript heap out of memory`
- `FATAL ERROR: Reached heap limit`
- `Allocation failed`

**自动恢复策略**：
- 新内存 = max(当前 × 1.5, 当前 + 2048 MB)
- 自动更新项目配置
- 提示用户重新运行命令

### 架构设计

```
src/
├── main.rs      - CLI 入口和命令解析
├── manager.rs   - 核心业务逻辑协调
├── config.rs    - 配置管理和业务逻辑
├── storage.rs   - JSON 配置文件读写
├── project.rs   - 项目根目录检测
└── runtime.rs   - 运行时抽象和执行
```

## 命令参考

### 主命令

```bash
frc [OPTIONS] <COMMAND> [ARGS]...
```

**Options:**
- `-m, --memory <MB>` - 设置内存限制（单位：MB）
- `-r, --runtime <RUNTIME>` - 显式指定运行时（node/deno/bun）
- `-h, --help` - 显示帮助信息
- `-V, --version` - 显示版本号

### 子命令

| 命令 | 说明 | 示例 |
|------|------|------|
| `info <runtime>` | 显示内存推荐配置 | `frc info node` |
| `project` | 显示当前项目配置 | `frc project` |
| `list` | 列出所有项目配置 | `frc list` |
| `forget [path]` | 删除项目配置 | `frc forget` |
| `cleanup --days <N>` | 清理 N 天未使用的配置 | `frc cleanup --days 30` |

## FAQ

<details>
<summary><strong>Q: 环境变量会污染系统环境吗？</strong></summary>

不会。环境变量只对 frc 启动的子进程有效，不会影响系统环境或其他进程。
</details>

<details>
<summary><strong>Q: 长时间运行的进程（如 dev server）会失效吗？</strong></summary>

不会。子进程继承环境变量后独立运行，即使 frc 退出也不影响。
```bash
# dev server 持续运行，不受 frc 退出影响
frc -m 4096 npm run dev
```
</details>

<details>
<summary><strong>Q: 可以同时运行多个项目吗？</strong></summary>

可以。每个 frc 进程完全独立，可以在不同终端同时运行多个项目，互不干扰。
</details>

<details>
<summary><strong>Q: 为什么 Bun 不支持内存配置？</strong></summary>

Bun 使用 JavaScriptCore 引擎，该引擎不提供手动内存配置选项，而是自动管理内存。
</details>

<details>
<summary><strong>Q: 配置文件保存在哪里？</strong></summary>

`~/.config/frc/config.json`
</details>

<details>
<summary><strong>Q: 如何确定应该设置多少内存？</strong></summary>

运行 `frc info node` 查看根据系统内存推荐的配置值。
</details>

<details>
<summary><strong>Q: 会影响程序性能吗？</strong></summary>

不会。FRC 仅在启动时设置环境变量，没有运行时开销。
</details>

<details>
<summary><strong>Q: 可以在 CI/CD 中使用吗？</strong></summary>

可以。示例：

```yaml
# GitHub Actions
- name: Build
  run: frc -m 4096 npm run build

# GitLab CI
build:
  script:
    - frc -m 8192 npm run build
```
</details>

## 开发

```bash
# 运行测试
cargo test

# 开发模式运行
cargo run -- -m 4096 node script.js

# 构建 release 版本
cargo build --release

# 运行 clippy 检查
cargo clippy
```

## License

MIT

---

**贡献指南**：欢迎提交 Issue 和 Pull Request！
