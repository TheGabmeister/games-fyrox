use fyrox::{
    core::{color::Color, pool::Handle},
    scene::node::Node,
};

#[derive(Default, Clone, Debug)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub thrust: bool,
    pub shoot: bool,
    pub restart: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl AsteroidSize {
    pub fn radius(self) -> f32 {
        match self {
            Self::Large => 1.4,
            Self::Medium => 0.85,
            Self::Small => 0.45,
        }
    }
    pub fn score(self) -> u32 {
        match self {
            Self::Large => 20,
            Self::Medium => 50,
            Self::Small => 100,
        }
    }
    pub fn color(self) -> Color {
        match self {
            Self::Large => Color::opaque(255, 160, 50),
            Self::Medium => Color::opaque(255, 120, 50),
            Self::Small => Color::opaque(255, 80, 50),
        }
    }
    pub fn wire_color(self) -> Color {
        match self {
            Self::Large => Color::opaque(255, 200, 100),
            Self::Medium => Color::opaque(255, 160, 80),
            Self::Small => Color::opaque(255, 120, 70),
        }
    }
    pub fn particle_count(self) -> usize {
        match self {
            Self::Large => 25,
            Self::Medium => 15,
            Self::Small => 8,
        }
    }
    pub fn child(self) -> Option<Self> {
        match self {
            Self::Large => Some(Self::Medium),
            Self::Medium => Some(Self::Small),
            Self::Small => None,
        }
    }
    pub fn speed_range(self) -> (f32, f32) {
        match self {
            Self::Large => (1.0, 2.5),
            Self::Medium => (2.0, 4.0),
            Self::Small => (3.0, 5.5),
        }
    }
}

#[derive(Debug)]
pub struct ShipData {
    pub body: Handle<Node>,
    pub visuals: Vec<Handle<Node>>,
    pub thrust_vis: Vec<Handle<Node>>,
    pub shoot_cd: f32,
    pub invuln: f32,
}

#[derive(Debug)]
pub struct AsteroidData {
    pub body: Handle<Node>,
    pub size: AsteroidSize,
    /// Polygon vertices in local space for wireframe drawing.
    pub verts: Vec<[f32; 2]>,
}

#[derive(Debug)]
pub struct BulletData {
    pub body: Handle<Node>,
    pub life: f32,
}

#[derive(Debug)]
pub struct Particle {
    pub node: Handle<Node>,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
}
