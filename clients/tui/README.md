# Alan TUI

Alan 的终端交互客户端（Bun + Ink），默认自动通过 `alan daemon` 管理后端。

## 特性

- 自动模式：无 `ALAN_AGENTD_URL` 时自动调用 `alan daemon start/stop`
- 首次启动向导：自动生成 `~/.alan/config.toml`
- 会话管理：创建、连接、切换 session
- 实时事件流：WebSocket 接收 runtime EventEnvelope
- 协议优先事件时间线：按 `alan_protocol` 事件渲染 turn/tool/yield/error
- Yield 交互：支持 confirmation / structured input / dynamic/custom 的 `resume`
- 键盘友好：`PgUp/PgDn`、`Shift+↑/↓`、`Ctrl+L`、`Ctrl+C`

## 安装

```bash
# 在仓库根目录
just install
```

安装后会生成：

- `~/.alan/bin/alan`
- `~/.alan/bin/alan-tui`（独立可执行文件，不依赖 bun 运行时）

## 运行

```bash
alan
```

首次运行会进入配置向导。

## 开发

```bash
# 在仓库根目录先构建 alan
cargo build --release -p alan

# 在 TUI 目录运行
cd clients/tui
bun run src/index.tsx
```

## 常用命令

| 命令 | 说明 |
| --- | --- |
| `/new` | 创建新会话 |
| `/new conservative` | 以 conservative 治理配置创建会话 |
| `/connect <id>` | 连接已有会话 |
| `/sessions` | 列出会话 |
| `/status` | 查看 daemon 状态 |
| `/input <text>` | 追加输入到当前 turn（Op::Input） |
| `/interrupt` | 中断当前执行（Op::Interrupt） |
| `/compact` | 手动触发上下文压缩（Op::Compact） |
| `/rollback <n>` | 回滚最近 N 个 turn（Op::Rollback） |
| `/approve` | 通过待确认请求 |
| `/reject` | 拒绝待确认请求 |
| `/modify <text>` | 修改后继续 |
| `/answer <text>` | 回复单题 structured input |
| `/answers <json>` | 回复多题 structured input |
| `/resume <json>` | 手动恢复 pending yield |
| `/clear` | 清空当前时间线显示 |
| `/help` | 显示帮助 |
| `/exit` | 退出 |

## 配置文件

路径：`~/.alan/config.toml`

示例：

```toml
bind_address = "127.0.0.1:8090"

llm_provider = "gemini"
gemini_project_id = "your-project-id"
gemini_location = "us-central1"
gemini_model = "gemini-2.0-flash"

llm_request_timeout_secs = 180
tool_timeout_secs = 30

[memory]
enabled = true
strict_workspace = true
```

## 故障排查

- 找不到 `alan`：重新执行 `just install`
- 创建 session 失败：检查 `~/.alan/config.toml` 与 API key
- 开启详细日志：`ALAN_VERBOSE=1 alan`
