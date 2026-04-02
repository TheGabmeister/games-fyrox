#[allow(unused_imports)]
use fyrox::graph::prelude::*;
use fyrox::{
    core::color::Color,
    scene::{
        debug::{Line, SceneDrawingContext},
        graph::Graph,
    },
};

use crate::constants::*;
use crate::helpers::*;
use crate::types::*;

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

#[allow(clippy::too_many_arguments)]
pub fn draw_hud(
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
