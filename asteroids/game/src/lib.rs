//! Asteroids — neon wireframe 2D arcade game built with Fyrox.

mod constants;
mod construction;
mod helpers;
mod hud;
mod logic;
mod types;

#[allow(unused_imports)]
use fyrox::graph::prelude::*;
use fyrox::{
    core::{
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
        dim2::rigidbody::RigidBody,
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
};
use rand::Rng;

use constants::*;
use helpers::*;
use types::*;

pub use fyrox;

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

impl Plugin for Game {
    fn register(&self, _context: PluginRegistrationContext) -> GameResult {
        Ok(())
    }

    fn init(&mut self, _scene_path: Option<&str>, context: PluginContext) -> GameResult {
        let mut scene = Scene::default();

        // Dark background — disable the default skybox so clear_color is used.
        scene.set_skybox(None);
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
            fyrox::scene::dim2::rectangle::RectangleBuilder::new(
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
            hud::draw_hud(
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
        hud::draw_hud(
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
