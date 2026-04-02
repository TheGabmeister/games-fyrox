//! Asteroids — neon wireframe 2D arcade game built with Fyrox.

#[allow(unused_imports)]
use fyrox::graph::prelude::*;
use fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
        reflect::prelude::*,
        visitor::prelude::*,
    },
    event::{ElementState, Event, WindowEvent},
    gui::{message::UiMessage, UserInterface},
    keyboard::{KeyCode, PhysicalKey},
    plugin::{error::GameResult, Plugin, PluginContext, PluginRegistrationContext},
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, OrthographicProjection, Projection},
        debug::{Line, SceneDrawingContext},
        dim2::{
            rectangle::{Rectangle, RectangleBuilder},
            rigidbody::{RigidBody, RigidBodyBuilder},
        },
        graph::Graph,
        node::Node,
        rigidbody::RigidBodyType,
        transform::TransformBuilder,
        Scene,
    },
};
use rand::Rng;
use std::f32::consts::PI;

pub use fyrox;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Constants
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

const WORLD_HALF_W: f32 = 14.0;
const WORLD_HALF_H: f32 = 9.0;
const WRAP_MARGIN: f32 = 2.0;

const SHIP_ROTATION_SPEED: f32 = 5.0;
const SHIP_THRUST: f32 = 8.0;
const SHIP_LIN_DAMPING: f32 = 1.2;
const SHIP_RADIUS: f32 = 0.45;

const BULLET_SPEED: f32 = 16.0;
const BULLET_LIFETIME: f32 = 1.8;
const BULLET_COOLDOWN: f32 = 0.12;
const BULLET_RADIUS: f32 = 0.15;

const RESPAWN_DELAY: f32 = 2.0;
const INVULN_DURATION: f32 = 3.0;
const WAVE_DELAY: f32 = 2.0;
const START_LIVES: u32 = 3;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn v3(x: f32, y: f32, z: f32) -> Vector3<f32> {
    Vector3::new(x, y, z)
}

/// Transform a 2D local-space point by a 4×4 global transform, returning a
/// world-space `Vector3` at the given z depth.
fn xform(m: &Matrix4<f32>, x: f32, y: f32, z: f32) -> Vector3<f32> {
    Vector3::new(
        m[(0, 0)] * x + m[(0, 1)] * y + m[(0, 3)],
        m[(1, 0)] * x + m[(1, 1)] * y + m[(1, 3)],
        z,
    )
}

/// Forward direction (local +Y in world space) from a global transform matrix.
fn forward_2d(m: &Matrix4<f32>) -> Vector2<f32> {
    Vector2::new(m[(0, 1)], m[(1, 1)])
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color {
        r: (a.r as f32 * inv + b.r as f32 * t) as u8,
        g: (a.g as f32 * inv + b.g as f32 * t) as u8,
        b: (a.b as f32 * inv + b.b as f32 * t) as u8,
        a: (a.a as f32 * inv + b.a as f32 * t) as u8,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Data structures
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Default, Clone, Debug)]
struct InputState {
    left: bool,
    right: bool,
    thrust: bool,
    shoot: bool,
    restart: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl AsteroidSize {
    fn radius(self) -> f32 {
        match self {
            Self::Large => 1.4,
            Self::Medium => 0.85,
            Self::Small => 0.45,
        }
    }
    fn score(self) -> u32 {
        match self {
            Self::Large => 20,
            Self::Medium => 50,
            Self::Small => 100,
        }
    }
    fn color(self) -> Color {
        match self {
            Self::Large => Color::opaque(255, 160, 50),
            Self::Medium => Color::opaque(255, 120, 50),
            Self::Small => Color::opaque(255, 80, 50),
        }
    }
    fn wire_color(self) -> Color {
        match self {
            Self::Large => Color::opaque(255, 200, 100),
            Self::Medium => Color::opaque(255, 160, 80),
            Self::Small => Color::opaque(255, 120, 70),
        }
    }
    fn particle_count(self) -> usize {
        match self {
            Self::Large => 25,
            Self::Medium => 15,
            Self::Small => 8,
        }
    }
    fn child(self) -> Option<Self> {
        match self {
            Self::Large => Some(Self::Medium),
            Self::Medium => Some(Self::Small),
            Self::Small => None,
        }
    }
    fn speed_range(self) -> (f32, f32) {
        match self {
            Self::Large => (1.0, 2.5),
            Self::Medium => (2.0, 4.0),
            Self::Small => (3.0, 5.5),
        }
    }
}

#[derive(Debug)]
struct ShipData {
    body: Handle<Node>,
    visuals: Vec<Handle<Node>>,
    thrust_vis: Vec<Handle<Node>>,
    shoot_cd: f32,
    invuln: f32,
}

#[derive(Debug)]
struct AsteroidData {
    body: Handle<Node>,
    size: AsteroidSize,
    /// Polygon vertices in local space for wireframe drawing.
    verts: Vec<[f32; 2]>,
}

#[derive(Debug)]
struct BulletData {
    body: Handle<Node>,
    life: f32,
}

#[derive(Debug)]
struct Particle {
    node: Handle<Node>,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 7-segment HUD
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

//  bit 6  top
//  bit 5  top-left
//  bit 4  top-right
//  bit 3  middle
//  bit 2  bottom-left
//  bit 1  bottom-right
//  bit 0  bottom
const SEG: [u8; 10] = [
    0b1110111, // 0
    0b0010010, // 1
    0b1011101, // 2
    0b1011011, // 3
    0b0111010, // 4
    0b1101011, // 5
    0b1101111, // 6
    0b1010010, // 7
    0b1111111, // 8
    0b1111011, // 9
];

fn draw_digit(dc: &mut SceneDrawingContext, x: f32, y: f32, d: u8, color: Color) {
    let w: f32 = 0.35;
    let h: f32 = 0.6;
    let hh = h * 0.5;
    let z: f32 = 0.5;
    let s = SEG[d as usize];
    if s & (1 << 6) != 0 {
        dc.add_line(Line { begin: v3(x, y + h, z), end: v3(x + w, y + h, z), color });
    }
    if s & (1 << 5) != 0 {
        dc.add_line(Line { begin: v3(x, y + hh, z), end: v3(x, y + h, z), color });
    }
    if s & (1 << 4) != 0 {
        dc.add_line(Line { begin: v3(x + w, y + hh, z), end: v3(x + w, y + h, z), color });
    }
    if s & (1 << 3) != 0 {
        dc.add_line(Line { begin: v3(x, y + hh, z), end: v3(x + w, y + hh, z), color });
    }
    if s & (1 << 2) != 0 {
        dc.add_line(Line { begin: v3(x, y, z), end: v3(x, y + hh, z), color });
    }
    if s & (1 << 1) != 0 {
        dc.add_line(Line { begin: v3(x + w, y, z), end: v3(x + w, y + hh, z), color });
    }
    if s & 1 != 0 {
        dc.add_line(Line { begin: v3(x, y, z), end: v3(x + w, y, z), color });
    }
}

fn draw_number(dc: &mut SceneDrawingContext, x: f32, y: f32, val: u32, color: Color) {
    let digits = if val == 0 {
        vec![0u8]
    } else {
        let mut v = val;
        let mut d = Vec::new();
        while v > 0 {
            d.push((v % 10) as u8);
            v /= 10;
        }
        d.reverse();
        d
    };
    for (i, &d) in digits.iter().enumerate() {
        draw_digit(dc, x + i as f32 * 0.5, y, d, color);
    }
}

/// Draw a tiny ship icon (for lives display).
fn draw_ship_icon(dc: &mut SceneDrawingContext, cx: f32, cy: f32, color: Color) {
    let s = 0.25;
    let z = 0.5;
    let top = v3(cx, cy + s, z);
    let bl = v3(cx - s * 0.7, cy - s * 0.7, z);
    let br = v3(cx + s * 0.7, cy - s * 0.7, z);
    let nl = v3(cx - s * 0.25, cy - s * 0.35, z);
    let nr = v3(cx + s * 0.25, cy - s * 0.35, z);
    dc.add_line(Line { begin: top, end: bl, color });
    dc.add_line(Line { begin: bl, end: nl, color });
    dc.add_line(Line { begin: nl, end: nr, color });
    dc.add_line(Line { begin: nr, end: br, color });
    dc.add_line(Line { begin: br, end: top, color });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Game Plugin
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Default, Visit, Reflect, Debug)]
#[reflect(non_cloneable)]
pub struct Game {
    #[visit(skip)]
    #[reflect(hidden)]
    scene: Handle<Scene>,
    #[visit(skip)]
    #[reflect(hidden)]
    camera: Handle<Node>,

    #[visit(skip)]
    #[reflect(hidden)]
    ship: Option<ShipData>,
    #[visit(skip)]
    #[reflect(hidden)]
    asteroids: Vec<AsteroidData>,
    #[visit(skip)]
    #[reflect(hidden)]
    bullets: Vec<BulletData>,
    #[visit(skip)]
    #[reflect(hidden)]
    particles: Vec<Particle>,

    #[visit(skip)]
    #[reflect(hidden)]
    input: InputState,

    #[visit(skip)]
    #[reflect(hidden)]
    score: u32,
    #[visit(skip)]
    #[reflect(hidden)]
    lives: u32,
    #[visit(skip)]
    #[reflect(hidden)]
    wave: u32,
    #[visit(skip)]
    #[reflect(hidden)]
    wave_timer: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    respawn_timer: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    game_over: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    initialized: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    time: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    shake: f32,
}

// ─── Plugin trait ────────────────────────────────────────────────────────────

impl Plugin for Game {
    fn register(&self, _context: PluginRegistrationContext) -> GameResult {
        Ok(())
    }

    fn init(&mut self, _scene_path: Option<&str>, context: PluginContext) -> GameResult {
        let mut scene = Scene::default();

        // Dark background
        scene
            .rendering_options
            .get_value_mut_and_mark_modified()
            .clear_color = Some(Color::opaque(4, 4, 18));

        // Orthographic camera looking down −Z
        self.camera = CameraBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(v3(0.0, 0.0, 20.0))
                    .build(),
            ),
        )
        .with_projection(Projection::Orthographic(OrthographicProjection {
            z_near: 0.0,
            z_far: 100.0,
            vertical_size: 20.0,
        }))
        .build(&mut scene.graph)
        .transmute();

        // Starfield
        let mut rng = rand::thread_rng();
        for _ in 0..120 {
            let x = rng.gen_range(-WORLD_HALF_W..WORLD_HALF_W);
            let y = rng.gen_range(-WORLD_HALF_H..WORLD_HALF_H);
            let b = rng.gen_range(30..120) as u8;
            let s = rng.gen_range(0.015..0.05);
            RectangleBuilder::new(
                BaseBuilder::new().with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(v3(x, y, -0.5))
                        .with_local_scale(v3(s, s, 1.0))
                        .build(),
                ),
            )
            .with_color(Color::opaque(b, b, (b as u16 + 40).min(255) as u8))
            .build(&mut scene.graph);
        }

        self.scene = context.scenes.add(scene);
        self.lives = START_LIVES;
        self.score = 0;
        self.wave = 0;
        self.game_over = false;
        self.initialized = false;
        self.wave_timer = 0.0;
        self.respawn_timer = 0.0;
        self.time = 0.0;
        self.shake = 0.0;
        Ok(())
    }

    fn on_deinit(&mut self, _context: PluginContext) -> GameResult {
        Ok(())
    }

    fn update(&mut self, context: &mut PluginContext) -> GameResult {
        let dt = context.dt;
        self.time += dt;

        let scene = &mut context.scenes[self.scene];

        // ── First-frame spawn ──
        if !self.initialized {
            self.ship = Some(Self::build_ship(&mut scene.graph, v3(0.0, 0.0, 0.0)));
            self.wave = 1;
            Self::spawn_wave(&mut scene.graph, &mut self.asteroids, 1, 1.0);
            self.wave_timer = WAVE_DELAY;
            self.initialized = true;
            return Ok(());
        }

        // ── Game-over state ──
        if self.game_over {
            if self.input.restart {
                self.restart(&mut scene.graph);
            }
            self.do_particles(&mut scene.graph, dt);
            scene.drawing_context.clear_lines();
            Self::draw_hud(
                &scene.graph,
                &mut scene.drawing_context,
                &self.ship,
                &self.asteroids,
                self.score,
                self.lives,
                self.wave,
                self.time,
                true,
            );
            return Ok(());
        }

        // ── Ship control ──
        if let Some(ref mut ship) = self.ship {
            ship.shoot_cd = (ship.shoot_cd - dt).max(0.0);
            ship.invuln = (ship.invuln - dt).max(0.0);

            if let Ok(body) = scene.graph.try_get_mut_of_type::<RigidBody>(ship.body) {
                // Rotation
                let ang = if self.input.left {
                    SHIP_ROTATION_SPEED
                } else if self.input.right {
                    -SHIP_ROTATION_SPEED
                } else {
                    0.0
                };
                body.set_ang_vel(ang);
            }

            // Thrust (need transform from node, then apply force)
            let fwd = scene
                .graph
                .try_get(ship.body)
                .ok()
                .map(|n| forward_2d(&n.global_transform()));

            if self.input.thrust {
                if let Some(dir) = fwd {
                    if let Ok(body) = scene.graph.try_get_mut_of_type::<RigidBody>(ship.body) {
                        body.apply_force(dir.normalize() * SHIP_THRUST);
                    }
                }
            }

            // Thrust visual
            for &h in &ship.thrust_vis {
                if let Ok(node) = scene.graph.try_get_mut(h) {
                    node.set_visibility(self.input.thrust);
                    if self.input.thrust {
                        let mut rng = rand::thread_rng();
                        let sy = rng.gen_range(0.15..0.4);
                        node.local_transform_mut()
                            .set_scale(v3(0.08, sy, 1.0));
                    }
                }
            }

            // Invulnerability blink
            if ship.invuln > 0.0 {
                let visible = (self.time * 12.0).sin() > 0.0;
                for &h in &ship.visuals {
                    if let Ok(n) = scene.graph.try_get_mut(h) {
                        n.set_visibility(visible);
                    }
                }
            } else {
                for &h in &ship.visuals {
                    if let Ok(n) = scene.graph.try_get_mut(h) {
                        n.set_visibility(true);
                    }
                }
            }
        }

        // ── Shooting ──
        if self.input.shoot {
            if let Some(ref mut ship) = self.ship {
                if ship.shoot_cd <= 0.0 {
                    ship.shoot_cd = BULLET_COOLDOWN;
                    if let Ok(node) = scene.graph.try_get(ship.body) {
                        let m = node.global_transform();
                        let pos = node.global_position();
                        let dir = forward_2d(&m).normalize();
                        let spawn = pos + v3(dir.x, dir.y, 0.0) * 0.55;
                        let bullet = Self::build_bullet(&mut scene.graph, spawn, dir);
                        self.bullets.push(bullet);
                    }
                }
            }
        }

        // ── Update bullets ──
        {
            let mut dead = Vec::new();
            for (i, b) in self.bullets.iter_mut().enumerate() {
                b.life -= dt;
                if b.life <= 0.0 {
                    dead.push(i);
                }
            }
            for &i in dead.iter().rev() {
                scene.graph.remove_node(self.bullets[i].body);
                self.bullets.remove(i);
            }
        }

        // ── Screen wrapping ──
        self.wrap_all(&mut scene.graph);

        // ── Collision detection ──
        self.do_collisions(&mut scene.graph);

        // ── Respawn ──
        if self.ship.is_none() && !self.game_over {
            self.respawn_timer -= dt;
            if self.respawn_timer <= 0.0 {
                if self.lives > 0 {
                    let mut s = Self::build_ship(&mut scene.graph, v3(0.0, 0.0, 0.0));
                    s.invuln = INVULN_DURATION;
                    self.ship = Some(s);
                } else {
                    self.game_over = true;
                }
            }
        }

        // ── Wave management ──
        if self.asteroids.is_empty() && self.ship.is_some() {
            self.wave_timer -= dt;
            if self.wave_timer <= 0.0 {
                self.wave += 1;
                let speed_mult = 1.0 + (self.wave as f32 - 1.0) * 0.12;
                Self::spawn_wave(&mut scene.graph, &mut self.asteroids, self.wave, speed_mult);
                self.wave_timer = WAVE_DELAY;
            }
        }

        // ── Particles ──
        self.do_particles(&mut scene.graph, dt);

        // ── Screen shake ──
        if self.shake > 0.01 {
            let mut rng = rand::thread_rng();
            let ox = rng.gen_range(-self.shake..self.shake);
            let oy = rng.gen_range(-self.shake..self.shake);
            if let Ok(cam) = scene.graph.try_get_mut(self.camera) {
                cam.local_transform_mut().set_position(v3(ox, oy, 20.0));
            }
            self.shake *= 0.88;
        } else {
            self.shake = 0.0;
            if let Ok(cam) = scene.graph.try_get_mut(self.camera) {
                cam.local_transform_mut().set_position(v3(0.0, 0.0, 20.0));
            }
        }

        // ── Drawing overlays ──
        scene.drawing_context.clear_lines();
        Self::draw_hud(
            &scene.graph,
            &mut scene.drawing_context,
            &self.ship,
            &self.asteroids,
            self.score,
            self.lives,
            self.wave,
            self.time,
            false,
        );

        Ok(())
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: PluginContext) -> GameResult {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event: key, .. },
            ..
        } = event
        {
            let pressed = key.state == ElementState::Pressed;
            match key.physical_key {
                PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyA) => {
                    self.input.left = pressed;
                }
                PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyD) => {
                    self.input.right = pressed;
                }
                PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::KeyW) => {
                    self.input.thrust = pressed;
                }
                PhysicalKey::Code(KeyCode::Space) => {
                    self.input.shoot = pressed;
                }
                PhysicalKey::Code(KeyCode::Enter) => {
                    self.input.restart = pressed;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn on_ui_message(
        &mut self,
        _context: &mut PluginContext,
        _message: &UiMessage,
        _ui_handle: Handle<UserInterface>,
    ) -> GameResult {
        Ok(())
    }
}

// ─── Construction helpers ────────────────────────────────────────────────────

impl Game {
    fn build_ship(graph: &mut Graph, pos: Vector3<f32>) -> ShipData {
        let body: Handle<Node> = RigidBodyBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(pos)
                    .build(),
            ),
        )
        .with_body_type(RigidBodyType::Dynamic)
        .with_gravity_scale(0.0)
        .with_lin_damping(SHIP_LIN_DAMPING)
        .with_ang_damping(8.0)
        .with_can_sleep(false)
        .build(graph)
        .transmute();

        let cyan = Color::opaque(0, 255, 255);
        let dim_cyan = Color::opaque(0, 140, 160);

        // Glow layer (behind, slightly larger, dimmer)
        let glow = Self::make_rect(graph, 0.0, 0.05, 0.35, 0.55, -0.05, dim_cyan);
        graph.link_nodes(glow, body);

        // Main body
        let main_body = Self::make_rect(graph, 0.0, 0.05, 0.18, 0.45, 0.0, cyan);
        graph.link_nodes(main_body, body);

        // Left wing
        let lw = Self::make_rect_rotated(graph, -0.18, -0.08, 0.12, 0.28, 0.0, cyan, 0.5);
        graph.link_nodes(lw, body);

        // Right wing
        let rw = Self::make_rect_rotated(graph, 0.18, -0.08, 0.12, 0.28, 0.0, cyan, -0.5);
        graph.link_nodes(rw, body);

        let visuals = vec![glow, main_body, lw, rw];

        // Thrust flames
        let flame_col = Color::opaque(255, 180, 50);
        let f1 = Self::make_rect(graph, -0.04, -0.35, 0.08, 0.25, 0.01, flame_col);
        graph.link_nodes(f1, body);
        if let Ok(n) = graph.try_get_mut(f1) {
            n.set_visibility(false);
        }
        let f2 = Self::make_rect(graph, 0.04, -0.32, 0.06, 0.2, 0.01, Color::opaque(255, 255, 200));
        graph.link_nodes(f2, body);
        if let Ok(n) = graph.try_get_mut(f2) {
            n.set_visibility(false);
        }
        let thrust_vis = vec![f1, f2];

        ShipData {
            body,
            visuals,
            thrust_vis,
            shoot_cd: 0.0,
            invuln: 0.0,
        }
    }

    fn build_asteroid(
        graph: &mut Graph,
        size: AsteroidSize,
        pos: Vector3<f32>,
        vel: Vector2<f32>,
    ) -> AsteroidData {
        let mut rng = rand::thread_rng();

        let body: Handle<Node> = RigidBodyBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(pos)
                    .build(),
            ),
        )
        .with_body_type(RigidBodyType::Dynamic)
        .with_gravity_scale(0.0)
        .with_lin_vel(vel)
        .with_ang_vel(rng.gen_range(-2.5..2.5))
        .with_can_sleep(false)
        .build(graph)
        .transmute();

        // Generate irregular polygon
        let n_verts = match size {
            AsteroidSize::Large => 10,
            AsteroidSize::Medium => 8,
            AsteroidSize::Small => 6,
        };
        let r = size.radius();
        let mut verts = Vec::with_capacity(n_verts);
        for i in 0..n_verts {
            let angle = (i as f32 / n_verts as f32) * 2.0 * PI;
            let variation = rng.gen_range(0.65..1.3);
            let vr = r * variation;
            verts.push([angle.cos() * vr, angle.sin() * vr]);
        }

        let color = size.color();
        let rect_size = r * 0.28;

        // Place rectangles at vertices
        for v in &verts {
            let vis = Self::make_rect(graph, v[0], v[1], rect_size, rect_size, 0.0, color);
            graph.link_nodes(vis, body);
        }

        // A few interior fill rectangles
        for _ in 0..3 {
            let angle = rng.gen_range(0.0..2.0 * PI);
            let dist = rng.gen_range(0.0..r * 0.45);
            let s = rng.gen_range(rect_size * 0.8..rect_size * 1.8);
            let inner = Self::make_rect_rotated(
                graph,
                angle.cos() * dist,
                angle.sin() * dist,
                s,
                s,
                -0.02,
                Color {
                    r: color.r,
                    g: color.g,
                    b: color.b,
                    a: 120,
                },
                rng.gen_range(0.0..PI),
            );
            graph.link_nodes(inner, body);
        }

        AsteroidData { body, size, verts }
    }

    fn build_bullet(graph: &mut Graph, pos: Vector3<f32>, dir: Vector2<f32>) -> BulletData {
        let vel = dir * BULLET_SPEED;

        let body: Handle<Node> = RigidBodyBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(pos)
                    .build(),
            ),
        )
        .with_body_type(RigidBodyType::Dynamic)
        .with_gravity_scale(0.0)
        .with_lin_vel(vel)
        .with_ccd_enabled(true)
        .with_can_sleep(false)
        .build(graph)
        .transmute();

        // Bullet core
        let core = Self::make_rect(graph, 0.0, 0.0, 0.08, 0.14, 0.0, Color::opaque(255, 255, 200));
        graph.link_nodes(core, body);

        // Glow
        let glow = Self::make_rect(graph, 0.0, 0.0, 0.16, 0.22, -0.01, Color::opaque(255, 255, 100));
        graph.link_nodes(glow, body);

        BulletData {
            body,
            life: BULLET_LIFETIME,
        }
    }

    // ── Rectangle helpers ──

    fn make_rect(
        graph: &mut Graph,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        z: f32,
        color: Color,
    ) -> Handle<Node> {
        RectangleBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(v3(x, y, z))
                    .with_local_scale(v3(w, h, 1.0))
                    .build(),
            ),
        )
        .with_color(color)
        .build(graph)
        .transmute()
    }

    fn make_rect_rotated(
        graph: &mut Graph,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        z: f32,
        color: Color,
        angle: f32,
    ) -> Handle<Node> {
        RectangleBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(v3(x, y, z))
                    .with_local_scale(v3(w, h, 1.0))
                    .with_local_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::z_axis(),
                        angle,
                    ))
                    .build(),
            ),
        )
        .with_color(color)
        .build(graph)
        .transmute()
    }
}

// ─── Game logic ──────────────────────────────────────────────────────────────

impl Game {
    fn spawn_wave(
        graph: &mut Graph,
        asteroids: &mut Vec<AsteroidData>,
        wave: u32,
        speed_mult: f32,
    ) {
        let count = 3 + wave as usize;
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            // Spawn on random edge
            let edge = rng.gen_range(0..4);
            let pos = match edge {
                0 => v3(-WORLD_HALF_W, rng.gen_range(-WORLD_HALF_H..WORLD_HALF_H), 0.0),
                1 => v3(WORLD_HALF_W, rng.gen_range(-WORLD_HALF_H..WORLD_HALF_H), 0.0),
                2 => v3(rng.gen_range(-WORLD_HALF_W..WORLD_HALF_W), WORLD_HALF_H, 0.0),
                _ => v3(rng.gen_range(-WORLD_HALF_W..WORLD_HALF_W), -WORLD_HALF_H, 0.0),
            };
            // Aim roughly toward center with some spread
            let to_center = -Vector2::new(pos.x, pos.y).normalize();
            let spread = rng.gen_range(-0.6..0.6);
            let angle = to_center.y.atan2(to_center.x) + spread;
            let sr = AsteroidSize::Large.speed_range();
            let speed = rng.gen_range(sr.0..sr.1) * speed_mult;
            let vel = Vector2::new(angle.cos() * speed, angle.sin() * speed);
            let a = Self::build_asteroid(graph, AsteroidSize::Large, pos, vel);
            asteroids.push(a);
        }
    }

    fn wrap_all(&self, graph: &mut Graph) {
        // Collect handles first to avoid borrow issues
        let mut handles: Vec<Handle<Node>> = Vec::new();
        if let Some(ref ship) = self.ship {
            handles.push(ship.body);
        }
        for a in &self.asteroids {
            handles.push(a.body);
        }
        for b in &self.bullets {
            handles.push(b.body);
        }

        for h in handles {
            let pos = match graph.try_get(h) {
                Ok(n) => n.global_position(),
                Err(_) => continue,
            };
            let mut np = pos;
            let mut wrapped = false;
            let mx = WORLD_HALF_W + WRAP_MARGIN;
            let my = WORLD_HALF_H + WRAP_MARGIN;
            if pos.x > mx {
                np.x = -mx + 0.1;
                wrapped = true;
            } else if pos.x < -mx {
                np.x = mx - 0.1;
                wrapped = true;
            }
            if pos.y > my {
                np.y = -my + 0.1;
                wrapped = true;
            } else if pos.y < -my {
                np.y = my - 0.1;
                wrapped = true;
            }
            if wrapped {
                if let Ok(node) = graph.try_get_mut(h) {
                    node.local_transform_mut().set_position(v3(np.x, np.y, 0.0));
                }
            }
        }
    }

    fn do_collisions(&mut self, graph: &mut Graph) {
        // Gather positions
        let ship_pos = self.ship.as_ref().and_then(|s| {
            graph.try_get(s.body).ok().map(|n| n.global_position())
        });
        let ship_invuln = self.ship.as_ref().map(|s| s.invuln).unwrap_or(1.0);

        let asteroid_info: Vec<(Vector3<f32>, f32)> = self
            .asteroids
            .iter()
            .map(|a| {
                let pos = graph
                    .try_get(a.body)
                    .map(|n| n.global_position())
                    .unwrap_or_default();
                (pos, a.size.radius())
            })
            .collect();

        let bullet_pos: Vec<Vector3<f32>> = self
            .bullets
            .iter()
            .map(|b| {
                graph
                    .try_get(b.body)
                    .map(|n| n.global_position())
                    .unwrap_or_default()
            })
            .collect();

        // Bullet vs asteroid
        let mut hit_bullets: Vec<usize> = Vec::new();
        let mut hit_asteroids: Vec<usize> = Vec::new();

        for (bi, bp) in bullet_pos.iter().enumerate() {
            for (ai, (ap, ar)) in asteroid_info.iter().enumerate() {
                if (bp - ap).magnitude() < BULLET_RADIUS + ar {
                    hit_bullets.push(bi);
                    hit_asteroids.push(ai);
                    break;
                }
            }
        }

        // Ship vs asteroid
        let mut ship_hit = false;
        if let Some(sp) = ship_pos {
            if ship_invuln <= 0.0 {
                for (ap, ar) in &asteroid_info {
                    if (sp - ap).magnitude() < SHIP_RADIUS + ar {
                        ship_hit = true;
                        break;
                    }
                }
            }
        }

        // Collect split/explosion data before mutating
        hit_asteroids.sort_unstable();
        hit_asteroids.dedup();
        hit_bullets.sort_unstable();
        hit_bullets.dedup();

        struct SplitInfo {
            size: AsteroidSize,
            pos: Vector3<f32>,
            color: Color,
            particles: usize,
            score: u32,
            shake: f32,
        }
        let mut splits: Vec<SplitInfo> = Vec::new();
        for &ai in &hit_asteroids {
            let a = &self.asteroids[ai];
            splits.push(SplitInfo {
                size: a.size,
                pos: asteroid_info[ai].0,
                color: a.size.color(),
                particles: a.size.particle_count(),
                score: a.size.score(),
                shake: match a.size {
                    AsteroidSize::Large => 0.3,
                    AsteroidSize::Medium => 0.15,
                    AsteroidSize::Small => 0.05,
                },
            });
        }

        // Remove hit asteroids (reverse order)
        for &ai in hit_asteroids.iter().rev() {
            graph.remove_node(self.asteroids[ai].body);
            self.asteroids.remove(ai);
        }
        // Remove hit bullets
        for &bi in hit_bullets.iter().rev() {
            graph.remove_node(self.bullets[bi].body);
            self.bullets.remove(bi);
        }

        // Process splits
        let mut rng = rand::thread_rng();
        for info in splits {
            self.score += info.score;
            self.shake += info.shake;
            self.spawn_explosion(graph, info.pos, info.particles, info.color);

            if let Some(child_size) = info.size.child() {
                for _ in 0..2 {
                    let angle = rng.gen_range(0.0..2.0 * PI);
                    let sr = child_size.speed_range();
                    let speed = rng.gen_range(sr.0..sr.1);
                    let vel = Vector2::new(angle.cos() * speed, angle.sin() * speed);
                    let a = Self::build_asteroid(graph, child_size, info.pos, vel);
                    self.asteroids.push(a);
                }
            }
        }

        // Ship hit
        if ship_hit {
            if let Some(ship) = self.ship.take() {
                let pos = graph
                    .try_get(ship.body)
                    .map(|n| n.global_position())
                    .unwrap_or_default();
                self.spawn_explosion(graph, pos, 35, Color::opaque(0, 255, 255));
                graph.remove_node(ship.body);
                self.lives = self.lives.saturating_sub(1);
                self.respawn_timer = RESPAWN_DELAY;
                self.shake += 0.6;
            }
        }
    }

    fn spawn_explosion(
        &mut self,
        graph: &mut Graph,
        pos: Vector3<f32>,
        count: usize,
        color: Color,
    ) {
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            let angle = rng.gen_range(0.0..2.0 * PI);
            let speed = rng.gen_range(2.0..9.0);
            let life = rng.gen_range(0.3..1.2);
            let s = rng.gen_range(0.04..0.12);

            // Start bright white, will fade toward the entity color
            let start_color = lerp_color(color, Color::WHITE, 0.7);

            let node: Handle<Node> = RectangleBuilder::new(
                BaseBuilder::new().with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(v3(pos.x, pos.y, 0.1))
                        .with_local_scale(v3(s, s, 1.0))
                        .build(),
                ),
            )
            .with_color(start_color)
            .build(graph)
            .transmute();

            self.particles.push(Particle {
                node,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                life,
                max_life: life,
            });
        }
    }

    fn do_particles(&mut self, graph: &mut Graph, dt: f32) {
        for p in self.particles.iter_mut() {
            p.life -= dt;
            if p.life <= 0.0 {
                continue;
            }
            // Move
            let pos = graph
                .try_get(p.node)
                .map(|n| n.global_position())
                .unwrap_or_default();
            if let Ok(node) = graph.try_get_mut(p.node) {
                node.local_transform_mut()
                    .set_position(v3(pos.x + p.vx * dt, pos.y + p.vy * dt, 0.1));
            }
            // Fade
            let t = (p.life / p.max_life).max(0.0);
            let c = lerp_color(
                Color { r: 80, g: 20, b: 0, a: 0 },
                Color::WHITE,
                t,
            );
            if let Ok(rect) = graph.try_get_mut_of_type::<Rectangle>(p.node) {
                rect.set_color(c);
            }
            // Slow down
            p.vx *= 0.98;
            p.vy *= 0.98;
        }

        // Remove dead
        let mut dead: Vec<Handle<Node>> = Vec::new();
        self.particles.retain(|p| {
            if p.life <= 0.0 {
                dead.push(p.node);
                false
            } else {
                true
            }
        });
        for h in dead {
            graph.remove_node(h);
        }
    }

    fn restart(&mut self, graph: &mut Graph) {
        // Remove all entities
        if let Some(ship) = self.ship.take() {
            graph.remove_node(ship.body);
        }
        for a in self.asteroids.drain(..) {
            graph.remove_node(a.body);
        }
        for b in self.bullets.drain(..) {
            graph.remove_node(b.body);
        }
        for p in self.particles.drain(..) {
            graph.remove_node(p.node);
        }

        self.score = 0;
        self.lives = START_LIVES;
        self.wave = 1;
        self.game_over = false;
        self.shake = 0.0;
        self.wave_timer = WAVE_DELAY;
        self.respawn_timer = 0.0;

        self.ship = Some(Self::build_ship(graph, v3(0.0, 0.0, 0.0)));
        Self::spawn_wave(graph, &mut self.asteroids, 1, 1.0);
    }

    // ── Drawing ──

    #[allow(clippy::too_many_arguments)]
    fn draw_hud(
        graph: &Graph,
        dc: &mut SceneDrawingContext,
        ship: &Option<ShipData>,
        asteroids: &[AsteroidData],
        score: u32,
        lives: u32,
        wave: u32,
        time: f32,
        game_over: bool,
    ) {
        let hud_color = Color::opaque(0, 255, 200);
        let z = 0.5;

        // ── World border ──
        let bc = Color::opaque(20, 25, 60);
        let w = WORLD_HALF_W;
        let h = WORLD_HALF_H;
        dc.add_line(Line { begin: v3(-w, -h, z), end: v3(w, -h, z), color: bc });
        dc.add_line(Line { begin: v3(w, -h, z), end: v3(w, h, z), color: bc });
        dc.add_line(Line { begin: v3(w, h, z), end: v3(-w, h, z), color: bc });
        dc.add_line(Line { begin: v3(-w, h, z), end: v3(-w, -h, z), color: bc });

        // ── Score (top-left) ──
        draw_number(dc, -WORLD_HALF_W + 0.5, WORLD_HALF_H - 1.2, score, hud_color);

        // ── Lives (ship icons below score) ──
        for i in 0..lives {
            draw_ship_icon(
                dc,
                -WORLD_HALF_W + 0.7 + i as f32 * 0.7,
                WORLD_HALF_H - 2.0,
                hud_color,
            );
        }

        // ── Wave (top-right) ──
        draw_number(
            dc,
            WORLD_HALF_W - 2.0,
            WORLD_HALF_H - 1.2,
            wave,
            Color::opaque(100, 100, 180),
        );

        // ── Ship wireframe ──
        if let Some(ref ship_data) = ship {
            if let Ok(node) = graph.try_get(ship_data.body) {
                let m = node.global_transform();
                let sc = Color::opaque(0, 255, 255);

                let nose = xform(&m, 0.0, 0.5, z);
                let bl = xform(&m, -0.35, -0.35, z);
                let br = xform(&m, 0.35, -0.35, z);
                let nl = xform(&m, -0.12, -0.18, z);
                let nr = xform(&m, 0.12, -0.18, z);

                dc.add_line(Line { begin: nose, end: bl, color: sc });
                dc.add_line(Line { begin: bl, end: nl, color: sc });
                dc.add_line(Line { begin: nl, end: nr, color: sc });
                dc.add_line(Line { begin: nr, end: br, color: sc });
                dc.add_line(Line { begin: br, end: nose, color: sc });
            }
        }

        // ── Asteroid wireframes ──
        for a in asteroids {
            if let Ok(node) = graph.try_get(a.body) {
                let m = node.global_transform();
                let wc = a.size.wire_color();
                let n = a.verts.len();
                for i in 0..n {
                    let v0 = a.verts[i];
                    let v1 = a.verts[(i + 1) % n];
                    let p0 = xform(&m, v0[0], v0[1], z);
                    let p1 = xform(&m, v1[0], v1[1], z);
                    dc.add_line(Line { begin: p0, end: p1, color: wc });
                }
            }
        }

        // ── Bullet trails ──
        // (Small cross at each bullet position for extra flair)
        // Bullets are small and fast, so we skip wireframe for them.

        // ── Game over display ──
        if game_over {
            let pulse = ((time * 2.5).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let go_color = lerp_color(
                Color::opaque(180, 0, 0),
                Color::opaque(255, 80, 30),
                pulse,
            );

            // Large X in center
            let s = 2.5;
            dc.add_line(Line { begin: v3(-s, -s, z), end: v3(s, s, z), color: go_color });
            dc.add_line(Line { begin: v3(-s, s, z), end: v3(s, -s, z), color: go_color });

            // Box around X
            dc.add_line(Line { begin: v3(-s, -s, z), end: v3(s, -s, z), color: go_color });
            dc.add_line(Line { begin: v3(s, -s, z), end: v3(s, s, z), color: go_color });
            dc.add_line(Line { begin: v3(s, s, z), end: v3(-s, s, z), color: go_color });
            dc.add_line(Line { begin: v3(-s, s, z), end: v3(-s, -s, z), color: go_color });

            // Score display centered
            let score_str_len = if score == 0 { 1 } else { (score as f32).log10().floor() as u32 + 1 };
            let offset = score_str_len as f32 * 0.25;
            draw_number(dc, -offset, -3.5, score, Color::opaque(255, 255, 255));
        }
    }
}
