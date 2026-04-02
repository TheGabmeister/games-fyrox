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

## Fyrox Script & API Patterns

### Script Definition

Every script requires these derives and attributes:

```rust
#[derive(Visit, Reflect, Default, Debug, Clone, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "unique-uuid-here")]  // generate a unique UUID per script
#[visit(optional)]
struct MyScript {
    // Editor-visible, prefab-inheritable:
    some_node: InheritableVariable<Handle<Node>>,
    speed: InheritableVariable<f32>,

    // Runtime-only (not serialized, hidden from editor):
    #[reflect(hidden)]
    #[visit(skip)]
    is_moving: bool,
}
```

Register every script in `Plugin::register` or it won't appear in the editor:
```rust
context.serialization_context.script_constructors.add::<MyScript>("My Script");
```

### Script Lifecycle

```
on_init → on_start → [on_os_event, on_update, on_message]... → on_deinit
```

- `on_os_event` — capture input, set flags (no physics here)
- `on_update` — per-frame logic at stable ~60 Hz (use `ctx.dt` for delta time)
- `on_start` — runs after all scripts have called `on_init` (subscribe to messages here)

### Scene Graph Access

```rust
ctx.scene.graph.try_get(handle)?                          // read node
ctx.scene.graph.try_get_mut(handle)?                      // mutate node
ctx.scene.graph.try_get_mut_of_type::<RigidBody>(handle)? // typed mutable access
ctx.scene.graph.try_get_script_of::<MyScript>(handle)?    // access another script
node.has_script::<Player>()                                // check script type
ctx.scene.graph.pair_iter()                                // iterate all (handle, node) pairs
ctx.scene.graph.link_nodes(child, parent)                  // attach child to parent
ctx.scene.graph.remove_node(handle)                        // remove node + children
```

Useful node methods: `global_position()`, `look_vector()`, `side_vector()`, `local_transform_mut()`.

### Message Passing (inter-script communication)

```rust
// 1. Define message
#[derive(Debug)]
struct DamageMessage { amount: f32 }
impl ScriptMessagePayload for DamageMessage {}

// 2. Subscribe in on_start
ctx.message_dispatcher.subscribe_to::<DamageMessage>(ctx.handle);

// 3. Send from another script
ctx.message_sender.send_to_target(target_handle, DamageMessage { amount: 10.0 });

// 4. Receive in on_message
if let Some(msg) = message.downcast_ref::<DamageMessage>() { /* handle */ }
```

### Physics Patterns

Movement — always preserve Y velocity for gravity:
```rust
let y_vel = rigid_body.lin_vel().y;
rigid_body.set_lin_vel(Vector3::new(vel.x, y_vel, vel.z));
```

Character rigid bodies: set rotation locked on all axes, disable Can Sleep.

Raycasting:
```rust
let mut intersections = Vec::new();
ctx.scene.graph.physics.cast_ray(
    RayCastOptions {
        ray_origin: pos.into(),
        ray_direction: dir,
        max_len: 1000.0,
        groups: Default::default(),
        sort_results: true,
    },
    &mut intersections,
);
```

### Prefab Instantiation

```rust
prefab_resource.instantiate_at(ctx.scene, position, rotation);
```

### Animation State Machines (ABSM)

Set up states and transitions in the editor, drive from code:
```rust
let sm = ctx.scene.graph.try_get_mut(*self.state_machine)?;
sm.machine_mut()
    .get_value_mut_silent()
    .set_parameter("Running", Parameter::Rule(is_moving));
```

Root motion extraction:
```rust
if let Some(root_motion) = sm.machine().pose().root_motion() {
    let velocity = transform.transform_vector(&root_motion.delta_position).scale(1.0 / ctx.dt);
}
```

### Global State via Plugin

```rust
// Store in Game plugin:
ctx.plugins.get_mut::<Game>().player = ctx.handle;

// Read from any script:
let player = ctx.plugins.get::<Game>().player;
```

## Conventions

- Prefer changing `game/` when adding gameplay behavior.
- Keep asset and scene paths stable unless intentionally reorganizing.
- Treat `data/scene.rgs` as the canonical starting scene.
- Do not assume Fyrox engine source is available — it's an external dependency.
- Use `cargo check --workspace` as the default verification after changes.
