# Alan TUI

Terminal client for Alan (Bun + Ink). By default it auto-manages the backend via `alan daemon`.

## Features

- Auto mode: when `ALAN_AGENTD_URL` is not set, it auto-runs `alan daemon start/stop`
- First-run setup wizard: starts with service presets, then generates canonical
  `~/.alan/agent/agent.toml` plus `~/.alan/host.toml`
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

- ChatGPT / Codex managed login
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
| `/new profile=<id>` | Create a session bound to a specific connection profile |
| `/new conservative` | Create a session with conservative governance profile |
| `/connect <id>` | Connect to an existing session |
| `/sessions` | List sessions |
| `/status` | Show daemon status |
| `/connection list` | List configured connection profiles |
| `/connection current` | Show global pin, workspace pin, default profile, and effective profile |
| `/connection status [profile]` | Compatibility alias for `current` / `show` |
| `/connection login <profile> [browser\|device]` | Start managed login for a profile |
| `/connection default set <profile>` | Set the default connection profile for new sessions |
| `/connection pin <profile> [scope=global\|workspace]` | Pin a profile in agent config |
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

Agent config path: `~/.alan/agent/agent.toml` (overridable via `ALAN_CONFIG_PATH`)

Connections config path: `~/.alan/connections.toml`

Example:

```toml
llm_request_timeout_secs = 180
tool_timeout_secs = 30
max_tool_loops = 0
tool_repeat_limit = 4

[memory]
enabled = true
strict_workspace = true
```

```toml
version = 1
default_profile = "gemini"

[profiles.gemini]
provider = "google_gemini_generate_content"
label = "Google Gemini via Vertex AI"
source = "managed"

[profiles.gemini.settings]
project_id = "your-project"
location = "us-central1"
model = "gemini-2.0-flash"
```

`agent.toml` carries runtime settings and may optionally pin a profile via
`connection_profile = "..."`. Provider metadata and credentials live under the
connection-profile control plane in `connections.toml` plus the credential
backend. Onboarding now writes only `default_profile`; use `pin` when you
explicitly want an override.

Host-facing daemon/client settings live in `~/.alan/host.toml`.

## Troubleshooting

- `alan` not found: run `just install` again
- Session creation failed: check `~/.alan/agent/agent.toml` (or `ALAN_CONFIG_PATH`) and API key setup
- Enable verbose logs: `ALAN_VERBOSE=1 alan`
