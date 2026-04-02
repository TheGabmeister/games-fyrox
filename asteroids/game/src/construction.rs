#[allow(unused_imports)]
use fyrox::graph::prelude::*;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
    },
    scene::{
        base::BaseBuilder,
        dim2::{
            rectangle::RectangleBuilder,
            rigidbody::RigidBodyBuilder,
        },
        graph::Graph,
        node::Node,
        rigidbody::RigidBodyType,
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
    pub(crate) fn build_ship(graph: &mut Graph, pos: Vector3<f32>) -> ShipData {
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

    pub(crate) fn build_asteroid(
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

    pub(crate) fn build_bullet(graph: &mut Graph, pos: Vector3<f32>, dir: Vector2<f32>) -> BulletData {
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

    pub(crate) fn make_rect(
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

    pub(crate) fn make_rect_rotated(
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
