# Alan TUI

像 pi-mono 一样的完整终端 AI 助手。TUI 自动管理后端，开箱即用。

## 特性

- **自动模式**（默认）：TUI 自动启动并管理 `agentd` 进程
- **首次启动向导**：自动检测并引导配置 LLM
- **配置文件**：使用 TOML 配置 LLM 和其他设置
- **会话管理**：创建、切换、管理多个会话
- **实时事件流**：WebSocket 实时接收 agent 事件

## 安装

### 一键安装（推荐）

```bash
# 在项目根目录执行
just install

# 这将：
# 1. 编译 agentd (Rust)
# 2. 构建 TUI (Bun)
# 3. 安装到 ~/.alan/bin/
```

### 添加到 PATH

```bash
# fish
set -Ux fish_user_paths $HOME/.alan/bin $fish_user_paths

# bash/zsh
echo 'export PATH="$HOME/.alan/bin:$PATH"' >> ~/.bashrc  # 或 ~/.zshrc
```

### 运行（首次启动）

```bash
alan
```

第一次运行会自动启动**配置向导**，引导你选择 LLM 提供商并完成配置。

## 首次启动向导

当你第一次运行 `alan` 时，会看到一个交互式向导：

```
Welcome to Alan!

Alan is an AI assistant that runs in your terminal.

To get started, we need to configure your LLM provider.

Press Enter to continue...
```

然后选择你的 LLM 提供商：

```
Select your LLM provider:

> Google Gemini (Vertex AI)
  OpenAI
  Anthropic Claude

↑↓ to select, Enter to confirm
```

向导会自动创建 `~/.alan/config/agentd.toml` 配置文件。

## 开发模式

如果你想在开发时运行最新代码而不安装：

```bash
# 1. 确保 agentd 已编译
cargo build --release -p alan-agentd

# 2. 在 TUI 目录下运行
cd clients/tui
bun run src/index.tsx

# 或
./bin/alan
```

开发模式下，配置文件优先级：
1. 环境变量 `ALAN_CONFIG_PATH` 指定的路径
2. 项目根目录的 `agentd.toml`
3. `~/.alan/config/agentd.toml`

## 更新

当代码有更新时：

```bash
# 拉取最新代码
git pull

# 重新安装
just install
```

## 卸载

```bash
just uninstall

# 或手动删除
rm -rf ~/.alan/bin
rm -rf ~/.alan/config  # 如果也想删除配置
```

## 使用

### 命令

在 TUI 中输入 `/<command>` 使用以下命令：

| 命令 | 描述 |
|------|------|
| `/new` | 创建新会话 |
| `/connect <id>` | 连接到现有会话 |
| `/sessions` | 列出活跃会话 |
| `/status` | 显示 agentd 状态 |
| `/help` | 显示帮助 |
| `/exit` | 退出（或按 Ctrl+C） |

### 配置文件

配置文件位于：`~/.alan/config/agentd.toml`

首次启动向导会自动创建此文件。你也可以手动编辑：

```toml
[server]
bind_address = "127.0.0.1:8090"

[llm]
provider = "gemini"  # gemini | openai_compatible | anthropic_compatible

[llm.gemini]
project_id = "your-project-id"
location = "us-central1"
model = "gemini-2.0-flash"

[runtime]
llm_timeout_secs = 180
tool_timeout_secs = 30

[memory]
enabled = true
strict_workspace = true

[defaults]
approval_policy = "on_request"
sandbox_mode = "workspace_write"
```

## 项目结构

```
~/.alan/
├── bin/
│   ├── agentd          # Rust daemon
│   ├── alan            # TUI executable
│   └── alan.js         # TUI bundle (if using wrapper)
└── config/
    └── agentd.toml     # Auto-generated on first run
```

## 架构

```
┌─────────────────────────────────────┐
│           Alan TUI (Bun)             │
│  ┌─────────┐ ┌─────────┐ ┌────────┐ │
│  │  Chat   │ │  Tool   │ │ Session│ │
│  │   UI    │ │  View   │ │ Manager│ │
│  └────┬────┘ └─────────┘ └────────┘ │
│       │                              │
│  ┌────┴─────────────────────────┐    │
│  │      DaemonManager           │    │
│  │  (自动启动/停止 agentd)       │    │
│  └────┬─────────────────────────┘    │
└───────┼──────────────────────────────┘
        │ WebSocket / HTTP
┌───────┴──────────────────────────────┐
│            agentd (Rust)             │
│      ┌──────────┐ ┌────────┐         │
│      │  Runtime │ │ Rollout│         │
│      └──────────┘ └────────┘         │
└──────────────────────────────────────┘
```

## 故障排除

### "找不到 agentd 可执行文件"

```bash
# 重新安装
just install
```

### "Failed to create session"

检查 LLM 配置是否正确：

```bash
# 验证配置文件
cat ~/.alan/config/agentd.toml

# 手动编辑
vim ~/.alan/config/agentd.toml
```

### 详细日志

```bash
ALAN_VERBOSE=1 alan
```
