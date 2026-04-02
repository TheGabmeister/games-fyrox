#[allow(unused_imports)]
use fyrox::graph::prelude::*;
use fyrox::{
    core::{
        algebra::Vector2,
        color::Color,
        pool::Handle,
    },
    scene::{
        base::BaseBuilder,
        dim2::{
            rectangle::{Rectangle, RectangleBuilder},
        },
        graph::Graph,
        node::Node,
        transform::TransformBuilder,
    },
};
use rand::Rng;
use std::f32::consts::PI;

use crate::constants::*;
use crate::helpers::*;
use crate::types::*;
use crate::Game;

impl Game {
    pub(crate) fn spawn_wave(
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

    pub(crate) fn wrap_all(&self, graph: &mut Graph) {
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

    pub(crate) fn do_collisions(&mut self, graph: &mut Graph) {
        // Gather positions
        let ship_pos = self.ship.as_ref().and_then(|s| {
            graph.try_get(s.body).ok().map(|n| n.global_position())
        });
        let ship_invuln = self.ship.as_ref().map(|s| s.invuln).unwrap_or(1.0);

        let asteroid_info: Vec<(fyrox::core::algebra::Vector3<f32>, f32)> = self
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

        let bullet_pos: Vec<fyrox::core::algebra::Vector3<f32>> = self
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
            pos: fyrox::core::algebra::Vector3<f32>,
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

    pub(crate) fn spawn_explosion(
        &mut self,
        graph: &mut Graph,
        pos: fyrox::core::algebra::Vector3<f32>,
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

    pub(crate) fn do_particles(&mut self, graph: &mut Graph, dt: f32) {
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

    pub(crate) fn restart(&mut self, graph: &mut Graph) {
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
}
