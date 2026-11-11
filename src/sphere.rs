use crate::vertex::Vertex;
use raylib::prelude::{Vector2, Vector3};
use std::f32::consts::PI;

fn sphere_vertex(latitude: f32, longitude: f32, u: f32, v: f32) -> Vertex {
    let cos_lat = latitude.cos();
    let position = Vector3::new(
        cos_lat * longitude.cos(),
        latitude.sin(),
        cos_lat * longitude.sin(),
    );
    Vertex::new(position, position, Vector2::new(u, v))
}

/// Generates a unit UV sphere as an unindexed triangle list.
/// A small mesh keeps the software rasterizer responsive.
pub fn generate_uv_sphere(stacks: usize, slices: usize) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(stacks * slices * 6);

    for stack in 0..stacks {
        let v0 = stack as f32 / stacks as f32;
        let v1 = (stack + 1) as f32 / stacks as f32;
        let latitude0 = -PI * 0.5 + PI * v0;
        let latitude1 = -PI * 0.5 + PI * v1;

        for slice in 0..slices {
            let u0 = slice as f32 / slices as f32;
            let u1 = (slice + 1) as f32 / slices as f32;
            let longitude0 = 2.0 * PI * u0;
            let longitude1 = 2.0 * PI * u1;

            let a = sphere_vertex(latitude0, longitude0, u0, v0);
            let b = sphere_vertex(latitude1, longitude0, u0, v1);
            let c = sphere_vertex(latitude1, longitude1, u1, v1);
            let d = sphere_vertex(latitude0, longitude1, u1, v0);

            if stack != 0 {
                vertices.extend_from_slice(&[a.clone(), b.clone(), d.clone()]);
            }
            if stack + 1 != stacks {
                vertices.extend_from_slice(&[d, b, c]);
            }
        }
    }

    vertices
}
