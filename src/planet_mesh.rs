//! Generación procedural de planetas (etapa propia, antes de rasterizar):
//!   1. Se parte de un icosaedro y se SUBDIVIDE recursivamente (cada triángulo → 4).
//!   2. Los vértices se proyectan a la esfera unitaria.
//!   3. Se DEFORMA la geometría desplazando cada vértice con ruido fractal (OpenSimplex).
//!   4. Se calculan normales (planas para el estilo low-poly, o suaves).

use crate::vertex::Vertex;
use noise::{NoiseFn, OpenSimplex};
use raylib::prelude::{Vector2, Vector3};
use std::f32::consts::PI;

/// Generador de relieve: altura = fbm(dirección) · amplitud (fracción del radio).
pub struct TerrainGen {
    noise: OpenSimplex,
    pub amplitude: f32,
    pub frequency: f32,
}

impl TerrainGen {
    pub fn new(seed: u32, amplitude: f32, frequency: f32) -> Self {
        TerrainGen {
            noise: OpenSimplex::new(seed),
            amplitude,
            frequency,
        }
    }

    /// Desplazamiento radial (fracción del radio) en la dirección unitaria `dir`.
    pub fn height(&self, dir: Vector3) -> f32 {
        fbm(&self.noise, dir, self.frequency, 4) * self.amplitude
    }
}

/// Ruido fractal (suma de octavas), aproximadamente en [-1, 1].
pub fn fbm(noise: &OpenSimplex, p: Vector3, frequency: f32, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut norm = 0.0;
    let mut amp = 1.0;
    let mut freq = frequency;
    for _ in 0..octaves {
        sum += noise.get([
            (p.x * freq) as f64,
            (p.y * freq) as f64,
            (p.z * freq) as f64,
        ]) as f32
            * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

fn icosahedron() -> (Vec<Vector3>, Vec<[usize; 3]>) {
    let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let raw = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ];
    let vertices = raw
        .iter()
        .map(|&(x, y, z)| Vector3::new(x, y, z).normalized())
        .collect();
    let faces = vec![
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];
    (vertices, faces)
}

fn subdivide(a: Vector3, b: Vector3, c: Vector3, level: u32, out: &mut Vec<[Vector3; 3]>) {
    if level == 0 {
        out.push([a, b, c]);
        return;
    }
    let ab = ((a + b) * 0.5).normalized();
    let bc = ((b + c) * 0.5).normalized();
    let ca = ((c + a) * 0.5).normalized();
    subdivide(a, ab, ca, level - 1, out);
    subdivide(b, bc, ab, level - 1, out);
    subdivide(c, ca, bc, level - 1, out);
    subdivide(ab, bc, ca, level - 1, out);
}

/// Triángulos de una icosfera unitaria subdividida `subdivisions` veces (20 · 4ⁿ caras).
pub fn icosphere_triangles(subdivisions: u32) -> Vec<[Vector3; 3]> {
    let (vertices, faces) = icosahedron();
    let mut out = Vec::new();
    for f in faces {
        subdivide(vertices[f[0]], vertices[f[1]], vertices[f[2]], subdivisions, &mut out);
    }
    out
}

fn uv_of(dir: Vector3) -> Vector2 {
    Vector2::new(
        dir.z.atan2(dir.x) / (2.0 * PI) + 0.5,
        dir.y.clamp(-1.0, 1.0).asin() / PI + 0.5,
    )
}

/// Malla de planeta (lista de triángulos sin indexar) de radio ~1.
/// - `terrain`: si existe, deforma la geometría con ruido.
/// - `flat`: normales por cara (aspecto low-poly) o normales suaves radiales.
pub fn generate_planet_mesh(
    subdivisions: u32,
    terrain: Option<&TerrainGen>,
    flat: bool,
) -> Vec<Vertex> {
    let triangles = icosphere_triangles(subdivisions);
    let mut vertices = Vec::with_capacity(triangles.len() * 3);

    for tri in triangles {
        let mut positions = [Vector3::zero(); 3];
        for i in 0..3 {
            let h = terrain.map(|t| t.height(tri[i])).unwrap_or(0.0);
            positions[i] = tri[i] * (1.0 + h);
        }

        let face_normal = {
            let n = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
            let centroid = (positions[0] + positions[1] + positions[2]) * (1.0 / 3.0);
            let n = if n.length() > 1e-9 { n.normalized() } else { centroid.normalized() };
            // Orientar hacia afuera de la esfera.
            if n.dot(centroid) < 0.0 { n * -1.0 } else { n }
        };

        for i in 0..3 {
            let normal = if flat { face_normal } else { tri[i] };
            vertices.push(Vertex::new(positions[i], normal, uv_of(tri[i])));
        }
    }
    vertices
}
