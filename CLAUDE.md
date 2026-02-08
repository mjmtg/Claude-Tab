# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
npm install          # Install dependencies
npm run dev          # Development (Vite + Tauri)
npm run build        # Production build
cargo build          # Rust workspace only
```

## Architecture

Claude Tabs is a **Tauri 2 desktop application** — a tab-based terminal manager with an extension-based plugin system. Frontend is React/TypeScript with xterm.js; backend is Rust with Tokio.

### Frontend (`src/`)

- **Kernel** (`src/kernel/`): `ExtensionHost`, `ComponentRegistry`, `EventBus`, `KeybindingManager`
- **SDK** (`src/sdk/`): Hooks for extensions (`useSession`, `useEvent`, `createExtension`)
- **Extensions** (`src/extensions/`): `terminal-panel`, `tab-bar`, `command-palette`, `settings`, `profiles`
- **Services** (`src/services/`): `IBackendService`, `IEventListener` — typed IPC abstractions

### Backend (`crates/`)

- **`core`** — Event bus, SessionStore, state machine, config, traits
- **`pty`** — PTY management via `portable-pty`
- **`storage`** — SQLite persistence (`~/.claude-tabs/archive.db`)
- **`tauri-bridge`** — Tauri IPC commands and event forwarding
- **`extensions/`** — `claude-hooks`, `auto-switch`, `output-parser`, `file-watcher`, `system-notify`

### IPC

Commands in `crates/tauri-bridge/src/commands.rs`. Events via channels: `core-event`, `pty-output`, `pty-exit`.

## Design Philosophy

**Interface-driven architecture** — All components are replaceable without changing functionality.

1. **Program to interfaces** — Components define contracts via TypeScript interfaces (`IEventBus`, `IBackendService`) and Rust traits (`Extension`, `SessionProvider`, `StateDetector`).

2. **Dependency injection** — Extensions receive dependencies through `ExtensionContext` (frontend) or `ActivationContext` (backend), not imports.

3. **Event-driven communication** — Cross-component communication via EventBus with typed topics (see `src/types/events.ts`, `crates/core/src/events/topics.rs`).

4. **Slot-based UI** — Extensions register React components into named slots (`TAB_BAR_CENTER`, `MAIN_CONTENT`, `STATUS_BAR_LEFT`), not hardcoded layouts.

5. **Registry pattern** — Detectors, reactions, and providers are registered dynamically for pluggable behavior.

## Extension Pattern

Both frontend and backend extensions follow the same lifecycle:

```
Register → Resolve Dependencies → Activate → (running) → Deactivate
```

- **Frontend**: Implement `FrontendExtension`, register components/keybindings via `ExtensionContext`
- **Backend**: Implement `Extension` trait, register detectors/reactions via `ActivationContext`

Key files for creating extensions:
- Frontend: `src/sdk/createExtension.ts`, `src/types/extension.ts`
- Backend: `crates/core/src/traits/extension.rs`

## Key Data Flows

1. **Session creation**: `create_session` command → PTY spawns → `session.created` event → tab appears
2. **Terminal I/O**: PTY output → broadcast → `pty-output` event → xterm.js renders
3. **State detection**: Backend extensions monitor output → detect states → emit events → UI updates
4. **Archiving**: On close, session transcript stored in SQLite for resume/fork
