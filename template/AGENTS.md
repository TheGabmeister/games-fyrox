# AGENTS.md

## Project Overview

This repository is a **Fyrox game project template**, not the Fyrox engine source tree.

- The engine is consumed as Cargo dependencies from the workspace root `Cargo.toml`.
- Game-specific logic lives in this repository.
- The default scene is `data/scene.rgs`.

## Workspace Layout

- `game/`: main game crate. `game/src/lib.rs` defines the `Game` plugin.
- `executor/`: native desktop runner for the game.
- `editor/`: Fyrox editor launcher with the game plugin attached.
- `game-dylib/`: dynamic library target used for hot reloading workflows.
- `executor-wasm/`: web target and browser host files.
- `executor-android/`: Android target.
- `export-cli/`: export/build tooling entry point.
- `data/`: scenes and project assets.

## Common Commands

Run these from the workspace root:

- `cargo check --workspace`
- `cargo run -p editor`
- `cargo run -p executor`

Web target notes live in `executor-wasm/README.md`.
Android target notes live in `executor-android/README.md`.

## Fyrox Conventions In This Repo

- The `Game` type in `game/src/lib.rs` implements Fyrox's `Plugin` trait.
- Scene loading currently happens in `Game::init()` via `data/scene.rgs`.
- Register custom scripts in `Game::register()`.
- Put global per-frame logic in `Game::update()`.
- Handle UI messages in `Game::on_ui_message()`.
- Handle OS/window/input events in `Game::on_os_event()`.

## Editing Guidance

- Prefer changing `game/` first when adding gameplay behavior.
- Only change `editor/` or `executor/` when the launch flow or integration needs to change.
- Keep asset and scene paths stable unless the change intentionally reorganizes project data.
- Treat `data/scene.rgs` as the canonical starting scene unless the project is being restructured.
- Do not assume the Fyrox engine source is available in this repository.

## Verification Guidance

- Use `cargo check --workspace` as the default smoke test after code changes.
- If platform-specific code changes, verify the affected target separately.
