# Alan TUI

Terminal User Interface for Alan Agent Runtime

## Features

- 🖥️ Terminal-based chat interface
- 🔄 Real-time WebSocket event streaming
- 📊 Session management
- 🔧 Tool call visualization
- ⚡ Fast and lightweight (Bun + TypeScript)

## Installation

```bash
# Install dependencies
bun install

# Or use npm/pnpm
npm install
```

## Usage

```bash
# Start the TUI
bun run start

# Development mode with hot reload
bun run dev

# Type checking
bun run typecheck
```

## Environment Variables

```bash
# Agent daemon URL (default: ws://localhost:8090)
export AGENTD_URL=ws://localhost:8090

# Or for HTTP-only mode
export AGENTD_URL=http://localhost:8090
```

## Commands

Once running, type:

- `/new` - Create a new session
- `/connect <session-id>` - Connect to an existing session
- `/sessions` - List active sessions
- `/help` - Show help
- `Ctrl+C` or `q` - Quit
- `Ctrl+L` - Clear screen

Simply type your message to chat with the agent.

## Architecture

```
src/
├── index.ts      # Main TUI application
├── client.ts     # WebSocket/HTTP client for agentd
├── renderer.ts   # Event rendering to terminal
└── types.ts      # TypeScript type definitions
```

## Development

```bash
# Lint
bun run lint

# Format
bun run format

# Build for production
bun run build
```

## License

Apache-2.0
