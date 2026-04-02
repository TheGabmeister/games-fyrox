# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A **2D Asteroids game** built on the Fyrox engine (v1.0.0). The engine is consumed as Cargo workspace dependencies. All game logic lives in a single file: `game/src/lib.rs`.

## Build & Run Commands

All commands run from workspace root:

```bash
cargo check --workspace          # Smoke test after any code change
cargo run -p executor            # Run game standalone (desktop)
cargo run -p editor              # Launch Fyrox editor with game plugin
cargo run -p export-cli -- --target-platform pc  # Export for CI/CD
```

**Hot reloading** (uses game-dylib for live code reload in editor):
```bash
cargo run -p editor --features dylib --profile dev-hot-reload
cargo run -p executor --features dylib --profile dev-hot-reload
```

There is no test suite or linter configured.

## Architecture

### Workspace Layout

```
game/src/lib.rs  (Game plugin — ALL game logic, ~1300 lines)
    │
    ├── editor/           loads Game plugin into FyroxEd
    ├── executor/         desktop runtime
    ├── executor-wasm/    WASM runtime (browser)
    ├── executor-android/ Android runtime
    ├── game-dylib/       hot-reload dynamic library wrapper
    └── export-cli/       CI/CD export tool
```

- **`game/`** — The only crate you normally edit. Dependencies: `fyrox` (workspace) + `rand 0.8`.
- **`editor/` and `executor/`** — Only change these when launch flow needs modification.
- **`data/scene.rgs`** — Legacy template scene file. **Not used by the game** — the scene is created programmatically in `Game::init()`.

### Game Architecture (game/src/lib.rs)

**Centralized Plugin pattern** — the `Game` struct implements Fyrox's `Plugin` trait and holds all game state. No scripts are used; all entity types are plain Rust structs stored in `Vec`s on `Game`:

```
Game (Plugin)
├── ship: Option<ShipData>       — player ship (RigidBody2D + rectangle visuals)
├── asteroids: Vec<AsteroidData> — active asteroids with procedural polygon shapes
├── bullets: Vec<BulletData>     — projectiles with lifetime
├── particles: Vec<Particle>     — explosion/visual effects (no physics)
├── input: InputState            — keyboard state captured in on_os_event()
└── score, lives, wave, ...      — game state
```

**All fields** on `Game` use `#[visit(skip)] #[reflect(hidden)]` since they are runtime-only state.

### Per-Frame Update Flow

`Game::update()` runs this sequence each frame:
1. **Ship control** — rotation via angular velocity, thrust via `apply_force` on RigidBody2D
2. **Shooting** — spawn bullet at ship nose with cooldown
3. **Bullet lifetime** — decrement and despawn expired bullets
4. **Screen wrapping** — teleport all entities crossing world bounds
5. **Collision detection** — manual circle-circle distance checks (bullet↔asteroid, ship↔asteroid)
6. **Asteroid splitting** — Large→2 Medium, Medium→2 Small, Small→destroyed + particles
7. **Respawn/game-over** — timer-based ship respawn with invulnerability, or game over at 0 lives
8. **Wave management** — spawn next wave when all asteroids destroyed
9. **Particles** — move, fade color, shrink, remove expired
10. **Screen shake** — offset camera by decaying random amount
11. **Wireframe overlays** — redraw HUD and entity outlines via `SceneDrawingContext`

### Key Fyrox API Patterns Used

**Handle conversion**: Builders return typed handles (`Handle<RigidBody>`, `Handle<Rectangle>`, etc.) that must be converted to `Handle<Node>` via `.transmute()` for storage and graph methods.

**Graph access**: `try_get` / `try_get_mut` / `try_get_mut_of_type` return `Result`, not `Option`. Use `if let Ok(node) = graph.try_get(handle)` pattern.

**2D rigid bodies**: Created via `dim2::rigidbody::RigidBodyBuilder` with `gravity_scale(0.0)` (space!). Ship uses `lin_damping` for drag. Access typed: `graph.try_get_mut_of_type::<RigidBody>(handle)`.

**Rectangles**: Created via `dim2::rectangle::RectangleBuilder`. Default size is 1×1, controlled via transform scale. Color set at creation or runtime via `rect.set_color(color)`.

**Scene creation**: The game does NOT load `data/scene.rgs`. Instead, `Scene::default()` is created, configured (clear color, camera), and added via `context.scenes.add(scene)`.

**Debug drawing**: `scene.drawing_context` provides per-frame line drawing (cleared each frame). Used for ship/asteroid wireframe outlines, world border, and 7-segment HUD digits.

**Node hierarchy**: Visual rectangles are children of rigid body nodes via `graph.link_nodes(child, parent)`. Removing a body via `graph.remove_node(body)` removes all children.

### Tunable Constants

Gameplay balance parameters are constants at the top of `lib.rs`: world bounds, ship speed/thrust/damping, bullet speed/lifetime/cooldown, asteroid size radii/speeds/scores, respawn delay, invulnerability duration, wave delay.

### Visual Design

Neon wireframe aesthetic on dark background (RGB 4,4,18):
- **Ship**: Cyan rectangles (body + wings + glow layer) + wireframe triangle via drawing context
- **Asteroids**: Orange/red rectangles at procedural polygon vertices + wireframe outline
- **Bullets**: Yellow-white rectangles with glow
- **Particles**: White→orange fading rectangles with velocity decay
- **HUD**: 7-segment display (score top-left, lives as ship icons, wave top-right), world border outline

## Fyrox Reference Patterns

These patterns are useful if extending the game with scripts or new features:

- **Plugin lifecycle hooks** in `Game` (`game/src/lib.rs`):
  - `register()` — register custom scripts (required for editor visibility)
  - `init()` — create scene, set up initial state
  - `update()` — per-frame global logic
  - `on_os_event()` — OS/window/input events
  - `on_ui_message()` — UI interactions

- **Scripts** are Rust structs deriving `Visit, Reflect, Default, Debug, Clone, TypeUuidProvider, ComponentProvider` with `#[type_uuid(id = "...")]`. They attach to scene nodes with their own lifecycle (`on_init`, `on_start`, `on_update`, `on_message`, `on_os_event`, `on_deinit`). Currently unused in this game.

- **Handles** (not references) are used to access objects in Fyrox's pool-based memory system. Handles are index+generation pairs providing O(1) access.

- **Global state via Plugin**: Scripts can access plugin state via `ctx.plugins.get::<Game>()`.

## Conventions

- All gameplay code goes in `game/src/lib.rs`.
- Do not assume Fyrox engine source is available — it's an external dependency.
- Use `cargo check --workspace` as the default verification after changes.
- `data/scene.rgs` is a legacy template file; the game creates its scene programmatically.
