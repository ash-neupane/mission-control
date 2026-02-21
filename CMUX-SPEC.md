# c-mux — Build Specification

> A native terminal multiplexer purpose-built for supervising multiple Claude Code (and Codex) sessions across projects. macOS and Linux only.

---

## 1. What This Is

c-mux is a desktop application that replaces your terminal when working with AI coding agents. It lets one developer run 5-10 Claude Code sessions across different repositories simultaneously, with a cockpit view that eliminates idle time and makes context-switching instant.

**c-mux is NOT:**
- An agent orchestrator (it doesn't coordinate agents or assign tasks)
- A task manager (no backlog, no kanban)
- An IDE (no code editing — the human reviews, not writes)
- An agent wrapper (Claude Code runs completely unmodified inside real PTYs)

**c-mux IS:**
- A PTY multiplexer with agent-aware UI
- A session manager that auto-names sessions from agent context
- A notification router that tells you which session needs your attention
- A project launcher that makes "new repo + new branch + Claude Code" one action

---

## 2. Tech Stack

### Core
- **Language:** Rust
- **UI Framework:** Tauri v2 (Rust backend + WebView frontend)
- **Terminal Emulator:** xterm.js in the WebView (one instance per session)
- **PTY Management:** `portable-pty` crate (cross-platform PTY spawning)
- **Frontend:** React + TypeScript (bundled via Vite into Tauri WebView)
- **Styling:** Tailwind CSS (utility classes only, dark theme)
- **State Management:** Zustand (lightweight, minimal boilerplate)

### Why Tauri over Electron
- Rust backend gives native PTY performance — critical when streaming 6-10 terminal outputs simultaneously
- ~10MB binary vs ~150MB for Electron
- Direct access to Rust PTY crates without Node native module pain
- Tauri v2 IPC is well-suited for streaming PTY data to the frontend

### Platform Support
- **macOS:** Primary target. x86_64 and aarch64 (Apple Silicon).
- **Linux:** x86_64. Tested on Ubuntu 22.04+ and Fedora 38+.
- **Windows:** Not supported.

### Prerequisites on User's Machine
- `git` (for branch/worktree management)
- `claude` CLI (Claude Code) and/or `codex` CLI installed and authenticated
- Standard shell (bash/zsh/fish)

---

## 3. Architecture

```
┌──────────────────────────────────────────────┐
│                   Tauri App                   │
│                                               │
│  ┌─────────────────────────────────────────┐  │
│  │           WebView (Frontend)            │  │
│  │                                         │  │
│  │  ┌─────────┐  ┌─────────┐  ┌────────┐  │  │
│  │  │ xterm.js│  │ xterm.js│  │xterm.js│  │  │
│  │  │ Session1│  │ Session2│  │Session3│  │  │
│  │  └────┬────┘  └────┬────┘  └───┬────┘  │  │
│  │       │            │            │       │  │
│  │  React UI: Overview Grid / Focus Mode   │  │
│  │  Zustand Store: sessions, activeView    │  │
│  └──────────────┬──────────────────────────┘  │
│                 │ Tauri IPC (events)           │
│  ┌──────────────┴──────────────────────────┐  │
│  │         Rust Backend (Core)             │  │
│  │                                         │  │
│  │  SessionManager                         │  │
│  │    ├─ spawn_session(project, agent)     │  │
│  │    ├─ kill_session(id)                  │  │
│  │    ├─ resize_pty(id, cols, rows)        │  │
│  │    └─ write_to_pty(id, bytes)           │  │
│  │                                         │  │
│  │  PTY Pool                               │  │
│  │    ├─ One portable-pty MasterPty per    │  │
│  │    │  session                           │  │
│  │    └─ Output reader threads → IPC       │  │
│  │                                         │  │
│  │  StatusDetector                         │  │
│  │    ├─ Watches PTY output stream         │  │
│  │    ├─ Detects idle prompt / question    │  │
│  │    ├─ Extracts branch name, PR URL      │  │
│  │    └─ Emits status change events        │  │
│  │                                         │  │
│  │  ProjectRegistry                        │  │
│  │    ├─ ~/.cmux/projects.json             │  │
│  │    └─ Scan/add/remove project dirs      │  │
│  │                                         │  │
│  │  GitManager                             │  │
│  │    ├─ create_branch(project, name)      │  │
│  │    ├─ list_branches(project)            │  │
│  │    └─ detect_branch(cwd)               │  │
│  │                                         │  │
│  │  Notifications                          │  │
│  │    └─ OS-level notifications via Tauri  │  │
│  └─────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

### Data Flow: PTY → Screen

1. Rust spawns a PTY via `portable-pty`, running the user's shell
2. A dedicated reader thread per session reads PTY output bytes
3. Output bytes are sent to the frontend via Tauri event: `pty-output-{session_id}`
4. Frontend feeds bytes into the corresponding xterm.js instance
5. The same output bytes are also passed to `StatusDetector` on the Rust side
6. `StatusDetector` pattern-matches for status signals and emits `session-status-changed` events
7. Frontend updates session metadata in Zustand store

### Data Flow: Keyboard → PTY

1. xterm.js captures keystrokes in the active session
2. Frontend sends bytes to Rust via Tauri command: `write_to_pty(session_id, bytes)`
3. Rust writes bytes to the PTY master fd
4. Claude Code (running in the PTY) receives the input normally

---

## 4. Data Model

### Session

```rust
pub struct Session {
    pub id: String,               // UUID
    pub name: String,             // Auto-generated, evolves over time
    pub project_path: String,     // Absolute path to repo root
    pub project_name: String,     // Directory name of the repo
    pub working_dir: String,      // Actual cwd (may be worktree path)
    pub agent: AgentType,         // Claude, Codex, Shell
    pub status: SessionStatus,
    pub branch: Option<String>,
    pub pr_url: Option<String>,
    pub started_at: u64,          // Unix timestamp
    pub tokens_used: Option<u64>, // Parsed from agent output if available
    pub last_output_preview: String, // Last ~200 chars of PTY output
    pub needs_attention_since: Option<u64>, // When it started needing input
}

pub enum AgentType {
    Claude,
    Codex,
    Shell, // plain shell, no agent
}

pub enum SessionStatus {
    Empty,       // Just spawned, no agent command yet
    Working,     // Agent is actively processing
    NeedsInput,  // Agent is waiting for human input
    PrReady,     // Agent created a PR (URL detected)
    Stuck,       // Error detected or no progress for extended period
    Done,        // Agent returned to idle after completing work
}
```

### Project Registry

```rust
// Stored at ~/.cmux/projects.json
pub struct ProjectRegistry {
    pub projects: Vec<RegisteredProject>,
}

pub struct RegisteredProject {
    pub path: String,        // Absolute path
    pub name: String,        // Display name (defaults to dir name)
    pub last_used: u64,      // For sorting in project picker
}
```

### App Config

```rust
// Stored at ~/.cmux/config.json
pub struct Config {
    pub default_agent: AgentType,      // Default: Claude
    pub claude_command: String,        // Default: "claude"
    pub codex_command: String,         // Default: "codex"
    pub shell: String,                 // Default: from $SHELL
    pub notifications_enabled: bool,   // Default: true
    pub auto_branch: bool,             // Default: true
    pub branch_prefix: String,         // Default: "cmux/"
    pub max_sessions: u8,              // Default: 9
}
```

---

## 5. UI Specification

The UI has exactly **two modes**: Overview and Focus. Plus a modal for creating new sessions.

### 5.1 Overview Mode (Default)

This is the "security camera grid." All active sessions are visible simultaneously.

**Layout:**
```
┌─ c-mux ──────────────────────────── 6 sessions │ 1 needs input │ 1 PR ready ─┐
│                                                                                │
│  ┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────────────┐  │
│  │ 2  fix-auth-expiry   │  │ 6  fix-ci-flaky      │  │ 4  rate-limiting     │  │
│  │ user-service    ●ASK │  │ payments-api   ●STUCK│  │ api-gateway   ●PR   │  │
│  │                      │  │                      │  │                      │  │
│  │ [live terminal       │  │ [live terminal       │  │ [live terminal       │  │
│  │  output preview]     │  │  output preview]     │  │  output preview]     │  │
│  │                      │  │                      │  │                      │  │
│  │ 8m · 28.1k tokens    │  │ 15m · 52k tokens     │  │ 6m · 18.7k tokens   │  │
│  └──────────────────────┘  └──────────────────────┘  └──────────────────────┘  │
│  ┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────────────┐  │
│  │ 1  stripe-webhook    │  │ 3  pg-to-sqlite      │  │ 5  chart-refactor    │  │
│  │ payments-api ●WORK   │  │ analytics-db  ●WORK  │  │ web-frontend  ●WORK  │  │
│  │                      │  │                      │  │                      │  │
│  │ [live terminal       │  │ [live terminal       │  │ [live terminal       │  │
│  │  output preview]     │  │  output preview]     │  │  output preview]     │  │
│  │                      │  │                      │  │                      │  │
│  │ 4m · 12.3k tokens    │  │ 12m · 45.2k tokens   │  │ 3m · 8.4k tokens    │  │
│  └──────────────────────┘  └──────────────────────┘  └──────────────────────┘  │
│                                                                                │
│  [1-9] focus  [n] new session  [tab] next needs-input  [q] kill  [?] help     │
└────────────────────────────────────────────────────────────────────────────────┘
```

**Behavior:**

- **Grid auto-sizes:** 1 session = full width. 2 = two columns. 3-4 = 2×2. 5-6 = 3×2. 7-9 = 3×3.
- **Sort order:** Sessions sort by attention priority:
  1. `NeedsInput` (oldest first — longest-waiting gets priority)
  2. `Stuck`
  3. `PrReady`
  4. `Working`
  5. `Done`
  6. `Empty`
- **Session numbers are stable:** Session 2 is always session 2 regardless of sort position. The number is assigned at creation and never changes. This is critical — the user builds muscle memory for "2 is the auth task."
- **Each cell contains a real xterm.js instance** running in a compressed viewport. It's not a text summary — it's live terminal output, just small. The user can see if the agent is making progress or spinning.
- **The left border of each cell is color-coded by status:**
  - Blue: Working
  - Amber/yellow: NeedsInput (with subtle pulse animation)
  - Green: PrReady or Done
  - Red: Stuck
- **Title bar** shows aggregate stats: total sessions, how many need input, how many have PRs ready.
- **Bottom bar** shows hotkeys and total token spend across all sessions.

### 5.2 Focus Mode

Press a number key (1-9) in Overview to enter Focus Mode for that session.

**Layout:**
```
┌─ 2 fix-auth-expiry │ user-service │ fix/token-expiry ── [1][3][4][5][6] ──────┐
│                                                                                │
│  ┌─────────────────────────────────────────────────┐  ┌──────────────────────┐ │
│  │                                                 │  │ SESSION INFO         │ │
│  │                                                 │  │                      │ │
│  │                                                 │  │ Project: user-svc    │ │
│  │            Full xterm.js terminal               │  │ Branch: fix/token..  │ │
│  │                                                 │  │ Time: 8m 23s         │ │
│  │            (this IS your Claude Code session)   │  │ Tokens: 28.1k        │ │
│  │                                                 │  │ Model: opus-4.6      │ │
│  │            All keystrokes pass through          │  │ Status: NEEDS INPUT  │ │
│  │            to the PTY directly.                 │  │                      │ │
│  │                                                 │  │ FILES MODIFIED       │ │
│  │                                                 │  │ ~ auth.ts            │ │
│  │                                                 │  │ ~ token-service.ts   │ │
│  │                                                 │  │ + token-rotation.ts  │ │
│  │                                                 │  │                      │ │
│  │                                                 │  │ OTHER SESSIONS       │ │
│  │                                                 │  │ 1 ● stripe-webhook   │ │
│  │                                                 │  │ 3 ● pg-to-sqlite     │ │
│  │                                                 │  │ 4 ● rate-limiting    │ │
│  │                                                 │  │ 5 ● chart-refactor   │ │
│  │                                                 │  │ 6 ● fix-ci-flaky     │ │
│  │                                                 │  │                      │ │
│  └─────────────────────────────────────────────────┘  └──────────────────────┘ │
│                                                                                │
│  [esc] overview  [1-9] switch  [tab] next needs-input  [ctrl+p] open PR       │
└────────────────────────────────────────────────────────────────────────────────┘
```

**Behavior:**

- The **main area is a full xterm.js terminal**. All keyboard input goes to the PTY. The user interacts with Claude Code exactly as they would in any terminal.
- The **side panel** (right, ~220px wide) shows parsed session metadata. This panel is read-only and can be toggled hidden with `Ctrl+B` for more terminal space.
- The **title bar** shows: session number, auto-name, project, branch. On the right side, small numbered pills for other sessions — color-coded by status. Click or press the number to switch.
- **Bottom bar** shows context-sensitive hotkeys.
- **Hotkeys in Focus Mode:**
  - `Esc` → return to Overview
  - `1-9` → switch to another session (stays in Focus Mode)
  - `Tab` → jump to next session with `NeedsInput` status
  - `Ctrl+P` → open PR URL in default browser (if `pr_url` is set)
  - `Ctrl+B` → toggle side panel
  - `Ctrl+N` → new session (opens modal)

**Important:** Focus Mode hotkeys must not conflict with Claude Code's own keybindings. `Esc`, number keys, and `Tab` are safe because Claude Code uses its own input prompt. `Ctrl+P`, `Ctrl+B`, `Ctrl+N` use Ctrl modifier which terminals don't normally use for agent interaction. If any conflict arises, make hotkeys configurable in config.json.

**Hotkey passthrough rule:** When the xterm.js terminal has focus (which it does by default in Focus Mode), ALL keystrokes pass through to the PTY EXCEPT for the registered c-mux hotkeys listed above. The hotkey interception layer must be minimal and precise.

### 5.3 New Session Modal

Triggered by `n` (Overview) or `Ctrl+N` (Focus).

**Flow:**

```
Step 1: Select Project
┌────────────────────────────────────┐
│  New Session                       │
│                                    │
│  Search: [payments-a_________]     │
│                                    │
│  > payments-api    ~/src/pay...    │
│    user-service    ~/src/usr...    │
│    web-frontend    ~/src/web...    │
│    analytics-db    ~/src/ana...    │
│                                    │
│  [enter] select  [esc] cancel      │
│  [+] add new project directory     │
└────────────────────────────────────┘

Step 2: Confirm (auto-branch)
┌────────────────────────────────────┐
│  New Session: payments-api         │
│                                    │
│  Branch: cmux/session-7            │
│  Agent:  claude                    │
│  Dir:    ~/src/payments-api        │
│                                    │
│  [enter] launch  [esc] cancel      │
│  [e] edit branch name              │
│  [a] change agent (codex/shell)    │
└────────────────────────────────────┘
```

**Behavior:**

- **Project list** is fuzzy-searchable. Sorted by last used. The user types a few characters and hits Enter.
- **Auto-branch:** c-mux automatically creates a new git branch named `cmux/session-{n}` (where n is the session number) from the current HEAD of the default branch (main/master). The user can edit the name before confirming if they want.
- **Branch creation uses standard git:** `git checkout -b cmux/session-{n}` from within the project directory. If the project has uncommitted changes on the current branch, warn the user.
- After confirmation, c-mux:
  1. `cd` into the project directory
  2. Creates the branch
  3. Spawns a new PTY running the agent command (e.g., `claude`)
  4. Assigns the next available session number (1-9)
  5. Switches to Focus Mode for the new session
- If the user presses `+` to add a new project, open a native directory picker dialog (Tauri `dialog::FileDialogBuilder`).

### 5.4 Color Palette

```
Background:         #0a0a0f
Surface:            #0f0f1a
Border:             #1a1a2e
Text primary:       #e0e0e8
Text secondary:     #888888
Text muted:         #555555

Status Working:     #3b82f6  (blue)
Status NeedsInput:  #f59e0b  (amber)
Status PrReady:     #22c55e  (green)
Status Done:        #22c55e  (green, dimmer)
Status Stuck:       #ef4444  (red)
Status Empty:       #555555  (gray)
```

### 5.5 Typography

- Monospace everywhere: system monospace stack or `JetBrains Mono` if bundled.
- Session names: 13px semibold
- Project names: 11px regular, muted color
- Status labels: 9px bold uppercase, status color
- Hotkey hints: 10px, muted, with key badges

---

## 6. Status Detection

c-mux needs to know what each agent is doing **without modifying the agent**. This is done by observing the PTY output stream.

### Detection Strategy

The Rust-side `StatusDetector` maintains a small state machine per session. It reads the raw byte stream coming from the PTY and applies heuristics.

**Important principle:** Be conservative. False negatives (missing a status change) are acceptable. False positives (showing "needs input" when the agent is still working) are not. When in doubt, keep status as `Working`.

### Detection Rules

```rust
// These are heuristics, not exact parsers. They should be tuned over time.

// 1. NeedsInput detection
// Claude Code shows a colored "❯" prompt when waiting for user input.
// The PTY output contains ANSI escape codes around this character.
// Additionally, Claude Code's terminal bell fires when idle for 60s.
//
// Signal: PTY output ends with the prompt character pattern and no new
// output arrives for >2 seconds.
//
// Also detect: "Allow" / "Deny" permission prompts (Claude Code tool approval)

// 2. Working detection
// Any new PTY output after a NeedsInput state → back to Working.
// Continuous output with tool use indicators (file reads, writes, bash commands).

// 3. PR URL detection
// Match URLs in PTY output: https://github.com/{owner}/{repo}/pull/{number}
// When detected, store in session.pr_url and change status to PrReady.

// 4. Branch detection
// On session start: run `git branch --show-current` in the session's cwd.
// Also watch for branch change indicators in PTY output.

// 5. Stuck detection
// If the agent has been in Working status for >5 minutes with the same
// repeated output pattern (e.g., same error printed multiple times),
// transition to Stuck.
// This is a soft heuristic. Start with a simple "no new unique output
// for 3+ minutes" rule. Refine later.

// 6. Done detection
// Agent returns to idle prompt after extended work without asking a question.
// Distinguish from NeedsInput: Done means the task appears complete;
// NeedsInput means the agent explicitly asked something.
// Heuristic: idle prompt + last output contained completion signals
// (e.g., "Created PR", "All tests passing", or similar).
```

### Session Auto-Naming

The session name should be derived automatically. Priority order:

1. **Git branch name:** If the branch is `cmux/session-3`, that's not useful. But if the user edited it to `fix/token-expiry`, use `fix-token-expiry`. Strip common prefixes (`feature/`, `fix/`, `cmux/`).
2. **Agent's first substantive output:** Watch for Claude Code's plan mode output or the first task description. Extract a short slug. For example, if the agent outputs "I'll help you add a Stripe webhook handler", extract `stripe-webhook-handler`.
3. **Fallback:** `{project-name}-{session-number}` (e.g., `payments-api-3`)

**Implementation:** Use a simple regex/keyword extraction on the first ~500 chars of non-boilerplate agent output. Don't try to be clever — a mediocre auto-name that updates once is better than a sophisticated system that flickers. Once a name is set from a signal at priority 1 or 2, stop updating it.

---

## 7. Notification System

### OS Notifications

When a session transitions to `NeedsInput` or `Stuck`:
- Fire an OS notification via Tauri's notification API
- Title: `c-mux — Session {n}`
- Body: `{session-name}: {last_output_preview_truncated_to_80_chars}`
- Clicking the notification should bring c-mux to front and Focus the relevant session

### In-App Indicators

- Overview grid sorts attention-needing sessions to top
- NeedsInput cells have a pulsing left border (CSS animation, subtle)
- Title bar shows count: "2 need input"
- In Focus Mode, other session pills in title bar flash amber/red

### Sound (Optional, Off by Default)

A short, subtle alert sound on `NeedsInput` transition. Configurable in `config.json`. Default off.

---

## 8. Key Interactions Reference

### Global Hotkeys (work in both modes)

| Key | Action |
|---|---|
| `1-9` | Switch to / focus session N |
| `Tab` | Jump to next `NeedsInput` session (round-robin among them) |
| `Ctrl+N` | Open new session modal |

### Overview Mode

| Key | Action |
|---|---|
| `n` | Open new session modal (alias for Ctrl+N) |
| `q` | Kill focused/selected session (with confirmation) |
| `?` | Show help overlay |
| `Enter` on selected cell | Enter Focus Mode for that session |

### Focus Mode

| Key | Action |
|---|---|
| `Esc` | Return to Overview Mode |
| `Ctrl+P` | Open session's PR URL in default browser |
| `Ctrl+B` | Toggle side panel visibility |
| All other keys | Pass through to PTY |

### New Session Modal

| Key | Action |
|---|---|
| Type | Fuzzy filter project list |
| `Up/Down` | Navigate project list |
| `Enter` | Select project / confirm launch |
| `Esc` | Cancel |
| `+` | Add new project directory |
| `e` | Edit branch name (in confirmation step) |
| `a` | Cycle agent type: claude → codex → shell |

---

## 9. File Structure

```
cmux/
├── Cargo.toml
├── tauri.conf.json
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri app entry point
│   │   ├── lib.rs               # Module declarations
│   │   ├── session.rs           # Session struct, SessionManager
│   │   ├── pty.rs               # PTY spawning, read/write, pool
│   │   ├── status.rs            # StatusDetector state machine
│   │   ├── naming.rs            # Auto-naming logic
│   │   ├── git.rs               # Branch creation, detection
│   │   ├── projects.rs          # ProjectRegistry, config loading
│   │   ├── notifications.rs     # OS notification dispatch
│   │   └── commands.rs          # Tauri IPC command handlers
│   ├── Cargo.toml
│   └── build.rs
├── src/                         # Frontend (React + TypeScript)
│   ├── main.tsx                 # React entry point
│   ├── App.tsx                  # Top-level: routes Overview vs Focus
│   ├── store.ts                 # Zustand store
│   ├── types.ts                 # TypeScript types matching Rust models
│   ├── hooks/
│   │   ├── useSession.ts        # Session state hook
│   │   ├── usePty.ts            # PTY output subscription hook
│   │   └── useHotkeys.ts        # Hotkey registration hook
│   ├── components/
│   │   ├── Overview.tsx         # Overview grid layout
│   │   ├── SessionCell.tsx      # Single cell in overview grid
│   │   ├── FocusMode.tsx        # Focus mode layout
│   │   ├── Terminal.tsx         # xterm.js wrapper component
│   │   ├── SidePanel.tsx        # Focus mode side panel
│   │   ├── TitleBar.tsx         # Top bar (both modes)
│   │   ├── StatusBar.tsx        # Bottom hotkey bar
│   │   ├── NewSessionModal.tsx  # Project picker + confirmation
│   │   ├── SessionPill.tsx      # Small session indicator (title bar)
│   │   └── HelpOverlay.tsx      # ? help screen
│   └── lib/
│       ├── tauri.ts             # Tauri IPC wrappers
│       └── colors.ts            # Status → color mapping
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── index.html
```

---

## 10. Tauri IPC Commands

These are the commands exposed from Rust to the frontend.

```rust
// Session management
#[tauri::command]
fn create_session(project_path: String, agent: String, branch_name: Option<String>) -> Result<Session, String>;

#[tauri::command]
fn kill_session(session_id: String) -> Result<(), String>;

#[tauri::command]
fn list_sessions() -> Vec<Session>;

#[tauri::command]
fn get_session(session_id: String) -> Option<Session>;

// PTY interaction
#[tauri::command]
fn write_to_pty(session_id: String, data: Vec<u8>) -> Result<(), String>;

#[tauri::command]
fn resize_pty(session_id: String, cols: u16, rows: u16) -> Result<(), String>;

// Project management
#[tauri::command]
fn list_projects() -> Vec<RegisteredProject>;

#[tauri::command]
fn add_project(path: String) -> Result<RegisteredProject, String>;

#[tauri::command]
fn remove_project(path: String) -> Result<(), String>;

// Git
#[tauri::command]
fn create_branch(project_path: String, branch_name: String) -> Result<String, String>;

#[tauri::command]
fn get_current_branch(project_path: String) -> Result<String, String>;

// Config
#[tauri::command]
fn get_config() -> Config;

#[tauri::command]
fn update_config(config: Config) -> Result<(), String>;

// PR
#[tauri::command]
fn open_url(url: String) -> Result<(), String>;
```

### Tauri Events (Rust → Frontend)

```rust
// Emitted continuously as PTY produces output
// Frontend subscribes per session
"pty-output-{session_id}" → Vec<u8>

// Emitted when StatusDetector determines a status change
"session-status-changed" → { session_id: String, new_status: SessionStatus, name: Option<String> }

// Emitted when a PR URL is detected
"pr-detected" → { session_id: String, url: String }
```

---

## 11. Implementation Plan

Build in this order. Each phase should be a working, testable increment.

### Phase 1: Shell Multiplexer (no agent awareness)

**Goal:** A working terminal multiplexer — spawn multiple PTYs, switch between them.

1. Set up Tauri v2 project with React + TypeScript frontend
2. Implement `pty.rs`: spawn PTY, read output in thread, write input
3. Implement basic `session.rs`: create/kill sessions, assign numbers 1-9
4. Create `Terminal.tsx` xterm.js wrapper that subscribes to `pty-output-{id}` events
5. Build `Overview.tsx`: grid of xterm.js instances, clickable to focus
6. Build `FocusMode.tsx`: single full terminal with keypress passthrough
7. Wire up hotkeys: number keys to switch, Esc to go back, `n` for new session
8. Test: can open 4 sessions, switch between them, type commands, see output

**Acceptance test:** Open c-mux, press `n` four times (selecting different directories each time), type `ls` in each, switch between them with number keys and Esc. All four terminals work independently.

### Phase 2: Project Management + Git

**Goal:** The new session flow works end-to-end with project selection and auto-branching.

1. Implement `projects.rs`: load/save `~/.cmux/projects.json`, add/remove
2. Build `NewSessionModal.tsx` with fuzzy search project picker
3. Implement `git.rs`: create branch, detect current branch
4. Wire up: select project → create branch → spawn PTY with `cd {project} && claude`
5. Show project name and branch in session cell and title bar

**Acceptance test:** Press `n`, search for a project, confirm. A new branch `cmux/session-{n}` is created, Claude Code launches in that directory, and the session cell shows the project name and branch.

### Phase 3: Status Detection

**Goal:** Sessions show whether the agent is working, waiting for input, or done.

1. Implement `status.rs`: state machine per session, consume PTY output bytes
2. Add `NeedsInput` detection: watch for Claude Code's prompt pattern + idle timeout
3. Add basic PR URL regex matching
4. Emit `session-status-changed` events
5. Frontend: color-code session cells by status
6. Frontend: sort overview grid by attention priority

**Acceptance test:** Start a Claude Code session, give it a task. Cell shows blue "WORKING". When Claude asks a question, cell turns amber "NEEDS INPUT" within 3 seconds. Answer it, cell goes back to blue.

### Phase 4: Notifications + Session Naming

**Goal:** The user gets notified when a session needs attention. Sessions have meaningful names.

1. Implement `naming.rs`: extract name from branch, agent output, or fallback
2. Implement `notifications.rs`: OS notifications on NeedsInput/Stuck transitions
3. Add `Tab` hotkey to jump between NeedsInput sessions
4. Add pulsing animation on NeedsInput cells
5. Title bar: show aggregate status counts

**Acceptance test:** Start 3 sessions with different tasks. Switch to session 1. Session 2's agent asks a question. OS notification appears. Press Tab — c-mux jumps to session 2. Session 2 is auto-named based on what the agent is working on.

### Phase 5: Side Panel + Polish

**Goal:** Focus mode is fully featured. The app feels polished.

1. Build `SidePanel.tsx`: session info, files modified (parsed from output), other sessions list
2. Add `Ctrl+P` to open PR URL
3. Add `Ctrl+B` to toggle side panel
4. Add `?` help overlay
5. Add session kill confirmation (`q` in overview)
6. Add token counting (parse from Claude Code's output if it shows token usage)
7. Polish: smooth transitions between Overview/Focus, proper xterm.js resizing, handle window resize
8. Add `config.json` management: agent commands, notification preferences

### Phase 6: Codex Support

**Goal:** Codex CLI sessions work alongside Claude Code sessions.

1. Agent type selector in new session modal (press `a` to cycle)
2. Adjust status detection heuristics for Codex's output patterns (may differ from Claude Code)
3. Test: run 2 Claude Code + 1 Codex session simultaneously

---

## 12. Critical Implementation Notes

### xterm.js Performance with Many Instances

Running 6-9 xterm.js instances simultaneously in a WebView is the biggest performance risk.

Mitigations:
- In Overview mode, xterm.js instances should be **throttled**: update at most 5 fps (every 200ms) instead of real-time. Buffer PTY output and flush in batches.
- Only the **focused session** in Focus Mode gets real-time (60fps) rendering.
- Consider using `xterm.js` `renderer: 'canvas'` for overview cells and `renderer: 'webgl'` for the focused terminal.
- If performance is still an issue, Overview cells can fall back to a plain `<pre>` tag showing the last N lines of output, with xterm.js only instantiated for Focus Mode. This is the nuclear option — try the throttled approach first.

### PTY Output Buffering

The Rust PTY reader thread should:
- Read in a loop with a small buffer (4096 bytes)
- Batch events: don't emit a Tauri event for every read. Accumulate for up to 16ms (one frame) then emit.
- This prevents flooding the IPC channel with thousands of tiny events.

### Hotkey Conflicts

Claude Code uses these keybindings inside its own TUI:
- `Escape` — exits certain modes in Claude Code. **This is a conflict.** Solution: Use `Ctrl+[` or double-`Esc` for c-mux's "return to overview" in Focus Mode. OR: only intercept Esc when the Claude Code prompt is idle (not mid-input). Start with single Esc and see if it causes problems in practice.
- `Shift+Tab` — used by Claude Code for mode switching. c-mux uses plain `Tab`. No conflict.

Test hotkey behavior thoroughly with real Claude Code sessions.

### Git Branch Safety

Before creating a branch:
1. Check if the working directory is clean (`git status --porcelain`)
2. If dirty, show a warning: "Uncommitted changes on current branch. Continue anyway?"
3. Create branch from HEAD of the default branch, not the current branch (to avoid stacking branches). Detect default branch: `git symbolic-ref refs/remotes/origin/HEAD | sed 's@^refs/remotes/origin/@@'`

### Window Management

- Save window position and size to `~/.cmux/window.json` on close, restore on open.
- Save which sessions are open (project + branch + session number). On restart, offer to restore sessions (but PTY state is lost — the user will need to re-run the agent).

### Graceful Shutdown

When the user closes c-mux:
1. Send SIGHUP to all PTY child processes
2. Wait up to 5 seconds for graceful exit
3. SIGKILL any remaining processes
4. Clean up temporary files
5. Do NOT delete git branches — the user's work should persist

---

## 13. What Success Looks Like

c-mux is successful when a developer can:

1. Open c-mux in the morning
2. Press `n` five times, selecting different projects each time
3. Give each Claude Code session a task in Plan Mode
4. Press `Esc` to return to Overview
5. See all 5 sessions working simultaneously, each auto-named
6. Get notified when session 3 needs a decision
7. Press `Tab`, land on session 3, answer the question, press `Esc`
8. See session 2 has a PR ready, press `2` then `Ctrl+P` to review it
9. Come back, give session 2 a new task
10. Repeat all day with zero idle time

The entire experience should feel like **switching channels on a TV**, not like managing infrastructure.

---

## 14. Out of Scope (Explicitly)

These are things c-mux does NOT do in any version of this spec:

- **Manage tasks or backlogs** — the user decides what to work on
- **Orchestrate agents** — agents don't talk to each other through c-mux
- **Edit code** — there is no editor, no LSP, no syntax highlighting of source files
- **Manage CLAUDE.md or skills** — that's the user's responsibility
- **Provide a web/mobile interface** — this is a native desktop app only
- **Support Windows** — macOS and Linux only
- **Run in the cloud** — all sessions are local PTYs
- **Handle billing or API keys** — the user manages their own Claude/Codex authentication
- **Replace git workflows** — it creates branches for convenience, nothing more
