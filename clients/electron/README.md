# Alan Electron Client

Desktop client for Alan Agent Runtime built with Electron.

## Features

- 🖥️ Cross-platform desktop application (macOS, Windows, Linux)
- 💬 Real-time chat interface with WebSocket support
- 📊 Session management sidebar
- 🔧 Tool call visualization
- 🎨 Dark mode UI
- ⌨️ Keyboard shortcuts

## Installation

```bash
# Install dependencies
npm install

# Or use pnpm/yarn
pnpm install
```

## Development

```bash
# Start in development mode
npm run dev

# Type checking
npm run typecheck

# Linting
npm run lint

# Formatting
npm run format
```

## Building

```bash
# Build for current platform
npm run package

# Build for specific platforms
npm run package:mac
npm run package:win
npm run package:linux

# Build unpacked (for testing)
npm run package:dir
```

## Configuration

The client connects to the agent daemon at `ws://localhost:8090` by default.

You can change this by setting the `AGENTD_URL` environment variable:

```bash
# macOS/Linux
export AGENTD_URL=ws://localhost:8090

# Windows
set AGENTD_URL=ws://localhost:8090
```

## Architecture

```
src/
├── main.ts       # Main Electron process
├── preload.ts    # Preload script (bridge between main and renderer)
├── renderer.ts   # Renderer process (UI logic)
├── client.ts     # WebSocket/HTTP client for agentd
└── types.ts      # TypeScript type definitions
```

### Process Model

- **Main Process** (`main.ts`): Controls the application lifecycle, creates windows, handles system menus
- **Renderer Process** (`renderer.ts`): Runs the web UI, handles user interactions
- **Preload Script** (`preload.ts`): Securely exposes main process APIs to the renderer

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + N` | New session |
| `Cmd/Ctrl + W` | Close current session |
| `Cmd/Ctrl + L` | Clear chat |
| `Enter` | Send message |
| `Shift + Enter` | New line in message |
| `Cmd/Ctrl + +/-` | Zoom in/out |
| `Cmd/Ctrl + 0` | Reset zoom |
| `F11` | Toggle fullscreen |
| `F12` | Toggle dev tools |

## License

Apache-2.0
