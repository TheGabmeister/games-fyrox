use fyrox::core::{
    algebra::{Matrix4, Vector2, Vector3},
    color::Color,
};

pub fn v3(x: f32, y: f32, z: f32) -> Vector3<f32> {
    Vector3::new(x, y, z)
}

/// Transform a 2D local-space point by a 4x4 global transform, returning a
/// world-space `Vector3` at the given z depth.
pub fn xform(m: &Matrix4<f32>, x: f32, y: f32, z: f32) -> Vector3<f32> {
    Vector3::new(
        m[(0, 0)] * x + m[(0, 1)] * y + m[(0, 3)],
        m[(1, 0)] * x + m[(1, 1)] * y + m[(1, 3)],
        z,
    )
}

/// Forward direction (local +Y in world space) from a global transform matrix.
pub fn forward_2d(m: &Matrix4<f32>) -> Vector2<f32> {
    Vector2::new(m[(0, 1)], m[(1, 1)])
}

pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color {
        r: (a.r as f32 * inv + b.r as f32 * t) as u8,
        g: (a.g as f32 * inv + b.g as f32 * t) as u8,
        b: (a.b as f32 * inv + b.b as f32 * t) as u8,
        a: (a.a as f32 * inv + b.a as f32 * t) as u8,
    }
}
