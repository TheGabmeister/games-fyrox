# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A **Fyrox game project template** (not the engine source). The Fyrox engine (v1.0.0) is consumed as Cargo workspace dependencies. Game-specific logic lives here; the engine is external.

## Build & Run Commands

All commands run from workspace root:

```bash
cargo check --workspace          # Smoke test after any code change
cargo run -p editor              # Launch Fyrox editor with game plugin
cargo run -p executor            # Run game standalone (desktop)
cargo run -p export-cli -- --target-platform pc  # Export for CI/CD
```

**Hot reloading** (uses game-dylib for live code reload in editor):
```bash
cargo run -p editor --features dylib --profile dev-hot-reload
cargo run -p executor --features dylib --profile dev-hot-reload
```

**WASM target** (from `executor-wasm/`):
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build --target web --release
cargo install basic-http-server && basic-http-server  # serve on :4000
```

**Android target:**
```bash
cargo-apk apk run --target=armv7-linux-androideabi
```

There is no test suite or linter configured in this template.

## Architecture

**Plugin-based design:** The game is a plugin loaded by different host executors. All hosts use the same `Plugin` trait interface, keeping game code platform-agnostic.

```
game/src/lib.rs  (Game plugin - all game logic)
    │
    ├── editor/       loads Game plugin into FyroxEd for visual editing
    ├── executor/     loads Game plugin for desktop runtime
    ├── executor-wasm/    WASM runtime (browser)
    ├── executor-android/ Android runtime
    ├── game-dylib/   dynamic library wrapper enabling hot-reload
    └── export-cli/   automated export for CI/CD
```

- **`game/`** — The only crate you normally edit. `Game` struct implements Fyrox's `Plugin` trait.
- **`editor/` and `executor/`** — Only change these when launch flow or integration needs modification.
- **`game-dylib/`** — Thin wrapper exporting `fyrox_plugin()` for dynamic linking. Feature-gated via `dylib`.
- **`data/`** — Scenes and assets. `data/scene.rgs` is the default scene loaded in `Game::init()`.

## Key Fyrox Patterns

- **Plugin lifecycle hooks** in `Game` (`game/src/lib.rs`):
  - `register()` — register custom scripts (required for editor visibility)
  - `init()` — load scenes, set up initial state
  - `update()` — per-frame global logic
  - `on_os_event()` — OS/window/input events
  - `on_ui_message()` — UI interactions

- **Scripts** are Rust structs attached to scene nodes with their own lifecycle (`on_init`, `on_start`, `on_update`, `on_message`, `on_os_event`, `on_deinit`). Scripts handle entity-specific logic; global concerns go in the `Game` plugin.

- **Scenes** are containers for game entities. Static content is built in the editor; dynamic content is instantiated at runtime. Scenes load asynchronously.

- **Handles** (not references) are used to access objects in Fyrox's pool-based memory system. Handles are index+generation pairs providing O(1) access.

- Scripts must derive `Visit`, `Reflect`, `Debug`, `Clone`, `Default`, `TypeUuidProvider`, and `ComponentProvider`.

## Conventions

- Prefer changing `game/` when adding gameplay behavior.
- Keep asset and scene paths stable unless intentionally reorganizing.
- Treat `data/scene.rgs` as the canonical starting scene.
- Do not assume Fyrox engine source is available — it's an external dependency.
- Use `cargo check --workspace` as the default verification after changes.
