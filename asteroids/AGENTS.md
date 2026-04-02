# AGENTS.md

## Project Overview

This repository is a **2D Asteroids game** built on the Fyrox engine (v1.0.0).

- The engine is consumed as Cargo dependencies from the workspace root `Cargo.toml`.
- All game logic lives in `game/src/lib.rs` (~1300 lines, centralized Plugin pattern).
- The scene is created programmatically in `Game::init()` — `data/scene.rgs` is not used.

## Workspace Layout

- `game/`: main game crate. `game/src/lib.rs` defines the `Game` plugin with all game state and logic.
- `executor/`: native desktop runner for the game.
- `editor/`: Fyrox editor launcher with the game plugin attached.
- `game-dylib/`: dynamic library target used for hot reloading workflows.
- `executor-wasm/`: web target and browser host files.
- `executor-android/`: Android target.
- `export-cli/`: export/build tooling entry point.
- `data/`: legacy scenes and project assets (scene.rgs is not loaded by the game).

## Common Commands

Run these from the workspace root:

- `cargo check --workspace` — smoke test after any change
- `cargo run -p executor` — run the game
- `cargo run -p editor` — launch editor

## Editing Guidance

- All gameplay code is in `game/src/lib.rs`. This is the only file you normally edit.
- Tunable gameplay constants (speeds, sizes, cooldowns) are at the top of the file.
- Entity data structs (`ShipData`, `AsteroidData`, `BulletData`, `Particle`) hold `Handle<Node>` references to scene graph nodes.
- Fyrox typed handles (`Handle<RigidBody>`, `Handle<Rectangle>`) must be converted to `Handle<Node>` via `.transmute()`.
- Graph methods (`try_get`, `try_get_mut`) return `Result`, not `Option`.

## Verification Guidance

- Use `cargo check --workspace` as the default smoke test after code changes.
- Run `cargo run -p executor` to verify the game plays correctly.
