# c-mux

A native terminal multiplexer purpose-built for supervising multiple Claude Code (and Codex) sessions across projects. macOS and Linux only.

## Prerequisites

- **Rust** (1.70+) with `cargo`
- **Node.js** (18+) with `npm`
- **System libraries** (Linux): `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev`
- **Tauri CLI**: `cargo install tauri-cli --version "^2"`
- `git` installed and available
- `claude` CLI (Claude Code) and/or `codex` CLI installed and authenticated

## Quick Start

```bash
# Install dependencies
npm install

# Development mode (opens the app with hot reload)
cargo tauri dev

# Production build
cargo tauri build
```

## Project Structure

```
├── CMUX-SPEC.md              # Full specification
├── src-tauri/                 # Rust backend
│   ├── src/
│   │   ├── main.rs            # Entry point
│   │   ├── lib.rs             # Module declarations + app setup
│   │   ├── session.rs         # Session struct, SessionManager
│   │   ├── pty.rs             # PTY spawning, read/write, pool
│   │   ├── status.rs          # StatusDetector state machine
│   │   ├── naming.rs          # Auto-naming logic
│   │   ├── git.rs             # Branch creation, detection
│   │   ├── projects.rs        # ProjectRegistry, config
│   │   ├── notifications.rs   # OS notification logic
│   │   └── commands.rs        # Tauri IPC command handlers
│   ├── Cargo.toml
│   └── build.rs
├── src/                       # React + TypeScript frontend
│   ├── main.tsx               # React entry point
│   ├── App.tsx                # Top-level: Overview vs Focus
│   ├── store.ts               # Zustand store
│   ├── types.ts               # TypeScript types
│   ├── styles.css             # Global styles + Tailwind
│   ├── hooks/
│   │   ├── useSession.ts      # Session state + event subscription
│   │   ├── usePty.ts          # PTY output + input hook
│   │   └── useHotkeys.ts      # Global hotkey handler
│   ├── components/
│   │   ├── Overview.tsx        # Security camera grid view
│   │   ├── SessionCell.tsx     # Single cell in overview
│   │   ├── FocusMode.tsx       # Full terminal + side panel
│   │   ├── Terminal.tsx        # xterm.js wrapper
│   │   ├── SidePanel.tsx       # Focus mode side panel
│   │   ├── TitleBar.tsx        # Top bar (both modes)
│   │   ├── StatusBar.tsx       # Bottom hotkey bar
│   │   ├── NewSessionModal.tsx # Project picker + launch
│   │   ├── SessionPill.tsx     # Small session indicator
│   │   ├── HelpOverlay.tsx     # ? help screen
│   │   └── KillConfirmDialog.tsx # Kill confirmation
│   └── lib/
│       ├── tauri.ts            # Tauri IPC wrappers
│       └── colors.ts           # Status color mapping
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── index.html
```

## Architecture

- **Backend**: Rust (Tauri v2) — manages PTY sessions, status detection, git operations
- **Frontend**: React + TypeScript + xterm.js — renders terminal grid and focus mode
- **IPC**: Tauri events for PTY output streaming, commands for session management
- **State**: Zustand store on frontend, `SessionManager` + `PtyPool` on backend

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `1-9` | Focus session N |
| `Tab` | Jump to next NeedsInput session |
| `Ctrl+N` | New session |

### Overview Mode
| Key | Action |
|-----|--------|
| `n` | New session |
| `q` | Kill session (with confirmation) |
| `?` | Help overlay |

### Focus Mode
| Key | Action |
|-----|--------|
| `Esc` | Return to overview |
| `Ctrl+P` | Open PR URL in browser |
| `Ctrl+B` | Toggle side panel |

## Demo Walkthrough

1. **Start the app**: `cargo tauri dev`
2. **Add a project**: Press `n` to open the new session modal, press `+` to add a project directory
3. **Create sessions**: Select a project, choose agent type (Claude/Codex/Shell), press Enter to launch
4. **Switch between sessions**: Press number keys `1-9` to focus different sessions
5. **Overview mode**: Press `Esc` to see all sessions in the grid view
6. **Status detection**: Sessions auto-detect when Claude Code needs input (amber), is working (blue), has a PR (green), or is stuck (red)
7. **Tab navigation**: Press `Tab` to jump between sessions that need your attention

## Running Tests

```bash
# Rust tests (17 unit tests across session, status, naming, notifications modules)
cd src-tauri && cargo test

# TypeScript type check
npx tsc --noEmit

# Frontend build
npx vite build
```

## Configuration

Config stored at `~/.cmux/config.json`:

```json
{
  "default_agent": "Claude",
  "claude_command": "claude",
  "codex_command": "codex",
  "shell": "/bin/bash",
  "notifications_enabled": true,
  "auto_branch": true,
  "branch_prefix": "cmux/",
  "max_sessions": 9
}
```

Projects registered at `~/.cmux/projects.json`.
