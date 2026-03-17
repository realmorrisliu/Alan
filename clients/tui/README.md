# Alan TUI

Terminal client for Alan (Bun + Ink). By default it auto-manages the backend via `alan daemon`.

## Features

- Auto mode: when `ALAN_AGENTD_URL` is not set, it auto-runs `alan daemon start/stop`
- First-run setup wizard: starts with service presets, then generates canonical
  `~/.config/alan/config.toml` (or path set by `ALAN_CONFIG_PATH`)
- Session management: create, connect, and switch sessions
- Live event stream: receives runtime `EventEnvelope` over WebSocket
- Protocol-first timeline: renders `alan_protocol` turn/tool/yield/error events
- Yield interactions: supports confirmation / structured input / dynamic-custom `resume`
- Native terminal scrolling: uses terminal scrollback and preserves `Ctrl+L` / `Ctrl+C`

## Install

```bash
# From repository root
just install
```

After install, this is generated:

- `~/.alan/bin/alan-tui` (standalone executable, does not require Bun runtime)

## Run

```bash
alan-tui
```

First run enters the setup wizard.

The wizard is service-first by default. It presents presets such as:

- OpenAI API Platform
- OpenRouter
- Kimi Coding
- DeepSeek
- Google Gemini via Vertex AI
- Anthropic API

If you need raw API-family control, choose `Advanced / custom setup` and then pick the
underlying API family manually.

## Development

```bash
# Build alan from repository root first
just build

# Then run in TUI directory
cd clients/tui
bun install
bun run dev
```

## Common Commands

| Command | Description |
| --- | --- |
| `/new` | Create a new session |
| `/new conservative` | Create a session with conservative governance profile |
| `/connect <id>` | Connect to an existing session |
| `/sessions` | List sessions |
| `/status` | Show daemon status |
| `/input <text>` | Append input to current turn (`Op::Input`) |
| `/interrupt` | Interrupt current execution (`Op::Interrupt`) |
| `/compact` | Trigger manual context compaction (`Op::CompactWithOptions`) |
| `/rollback <n>` | Roll back the most recent N turns in memory only (`Op::Rollback`) |
| `/approve` | Approve pending confirmation |
| `/reject` | Reject pending confirmation |
| `/modify <text>` | Modify and continue |
| `/answer <text>` | Reply to single-question structured input |
| `/answers <json>` | Reply to multi-question structured input |
| `/resume <json>` | Manually resume a pending yield |
| `/clear` | Clear the current timeline display |
| `/help` | Show help |
| `/exit` | Exit |

## Config File

Path: `~/.config/alan/config.toml` (overridable via `ALAN_CONFIG_PATH`)

Example:

```toml
llm_provider = "google_gemini_generate_content"
google_gemini_generate_content_project_id = "your-project"
google_gemini_generate_content_location = "us-central1"
google_gemini_generate_content_model = "gemini-2.0-flash"

llm_request_timeout_secs = 180
tool_timeout_secs = 30
max_tool_loops = 0
tool_repeat_limit = 4

[memory]
enabled = true
strict_workspace = true
```

The config file always uses Alan's canonical provider names. The service presets only affect
how the wizard guides setup and which defaults it prefills.

## Troubleshooting

- `alan` not found: run `just install` again
- Session creation failed: check `~/.config/alan/config.toml` (or `ALAN_CONFIG_PATH`) and API key setup
- Enable verbose logs: `ALAN_VERBOSE=1 alan`
