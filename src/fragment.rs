#![allow(dead_code)]

use raylib::math::{Vector2, Vector3};

pub struct Fragment {
    pub position: Vector2,      // Screen-space position
    pub color: Vector3,          // Interpolated color
    pub depth: f32,              // Interpolated depth
    pub world_position: Vector3, // Interpolated world-space position
    pub normal: Vector3,         // Interpolated, normalized world-space normal
    pub light_dir: Vector3,      // Unit vector from the surface to the light
    pub intensity: f32,          // Lambert intensity already baked into `color`
}

impl Fragment {
    pub fn new(x: f32, y: f32, color: Vector3, depth: f32) -> Self {
        Fragment {
            position: Vector2::new(x, y),
            color,
            depth,
            world_position: Vector3::zero(),
            normal: Vector3::new(0.0, 1.0, 0.0),
            light_dir: Vector3::new(0.0, 1.0, 0.0),
            intensity: 1.0,
        }
    }

    /// Fragmento con los datos de iluminación, para que el fragment shader pueda
    /// recalcular la luz con una normal perturbada (mapeo de normales).
    #[allow(clippy::too_many_arguments)]
    pub fn new_lit(
        x: f32,
        y: f32,
        color: Vector3,
        depth: f32,
        world_pos: Vector3,
        normal: Vector3,
        light_dir: Vector3,
        intensity: f32,
    ) -> Self {
        Fragment {
            position: Vector2::new(x, y),
            color,
            depth,
            world_position: world_pos,
            normal,
            light_dir,
            intensity,
        }
    }
}