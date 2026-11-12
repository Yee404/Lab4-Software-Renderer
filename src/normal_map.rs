//! Mapas horneados del planeta para el renderizador por software.
//! Se generan UNA vez al arrancar (equirrectangulares y sin costuras, porque el ruido se
//! evalúa sobre la esfera) y luego los shaders solo los consultan con interpolación
//! bilineal, lo que es muy barato por píxel. Contienen:
//!   - pendientes (du, dv) del relieve  → mapeo de normales,
//!   - altura fbm                       → biomas del shader (mares, montañas, estratos),
//!   - un patrón extra (`DetailKind`)   → nubes, dunas o celdas de Voronoi.

use crate::planet_mesh::fbm;
use noise::OpenSimplex;
use raylib::prelude::Vector3;
use std::f32::consts::PI;

const MAP_W: usize = 256;
const MAP_H: usize = 128;

/// Patrón extra que se hornea en el mapa, según el planeta.
#[derive(Clone, Copy)]
pub enum DetailKind {
    /// Ruido suave en [0, 1] (nubes que se desplazan).
    Clouds,
    /// Ruido "ridged" en [0, 1]: crestas afiladas (dunas).
    Ridged,
    /// Distancia al borde de las celdas de Voronoi (grietas, placas de hielo, corteza).
    Cells,
}

pub struct NormalMap {
    slopes: Vec<[f32; 2]>,
    heights: Vec<f32>,
    detail: Vec<f32>,
    /// Cuánto se inclina la normal por unidad de pendiente.
    pub strength: f32,
}

fn hash3(x: f32, y: f32, z: f32) -> [f32; 3] {
    let h = |a: f32, b: f32, c: f32| {
        ((a * 127.1 + b * 311.7 + c * 74.7).sin() * 43758.547).fract().abs()
    };
    [h(x, y, z), h(y + 17.0, z + 3.0, x + 41.0), h(z + 29.0, x + 7.0, y + 13.0)]
}

/// Distancia al borde de la celda de Voronoi 3D más cercana: F2 - F1 (≈0 sobre los bordes).
fn voronoi_edge(p: Vector3) -> f32 {
    let (cx, cy, cz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (nx, ny, nz) = (cx + dx as f32, cy + dy as f32, cz + dz as f32);
                let o = hash3(nx, ny, nz);
                let dxv = nx + o[0] - p.x;
                let dyv = ny + o[1] - p.y;
                let dzv = nz + o[2] - p.z;
                let d = (dxv * dxv + dyv * dyv + dzv * dzv).sqrt();
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
    }
    f2 - f1
}

impl NormalMap {
    pub fn generate(seed: u32, frequency: f32, strength: f32, kind: DetailKind) -> Self {
        let noise = OpenSimplex::new(seed);
        let noise2 = OpenSimplex::new(seed.wrapping_add(1000));
        let mut heights = vec![0.0_f32; MAP_W * MAP_H];
        let mut detail = vec![0.0_f32; MAP_W * MAP_H];

        for j in 0..MAP_H {
            let lat = ((j as f32 + 0.5) / MAP_H as f32 - 0.5) * PI;
            for i in 0..MAP_W {
                let lon = ((i as f32 + 0.5) / MAP_W as f32 - 0.5) * 2.0 * PI;
                let dir = Vector3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
                let idx = j * MAP_W + i;
                heights[idx] = fbm(&noise, dir, frequency, 3);
                detail[idx] = match kind {
                    DetailKind::Clouds => (fbm(&noise2, dir, frequency * 0.6, 4) + 1.0) * 0.5,
                    DetailKind::Ridged => 1.0 - fbm(&noise2, dir, frequency * 0.8, 4).abs() * 1.6,
                    DetailKind::Cells => voronoi_edge(dir * (frequency * 0.9)),
                };
            }
        }

        // Pendientes por diferencias centrales, en unidades de altura / longitud de arco.
        let mut slopes = vec![[0.0_f32; 2]; MAP_W * MAP_H];
        let ds_v = PI / MAP_H as f32;
        for j in 0..MAP_H {
            let lat = ((j as f32 + 0.5) / MAP_H as f32 - 0.5) * PI;
            let ds_u = (2.0 * PI / MAP_W as f32) * lat.cos().max(0.2);
            for i in 0..MAP_W {
                let left = heights[j * MAP_W + (i + MAP_W - 1) % MAP_W];
                let right = heights[j * MAP_W + (i + 1) % MAP_W];
                let down = heights[j.saturating_sub(1) * MAP_W + i];
                let up = heights[(j + 1).min(MAP_H - 1) * MAP_W + i];
                slopes[j * MAP_W + i] = [
                    (right - left) / (2.0 * ds_u),
                    (up - down) / (2.0 * ds_v),
                ];
            }
        }
        NormalMap { slopes, heights, detail, strength }
    }

    fn index(x: i32, y: i32) -> usize {
        let xi = x.rem_euclid(MAP_W as i32) as usize;
        let yi = y.clamp(0, MAP_H as i32 - 1) as usize;
        yi * MAP_W + xi
    }

    /// Interpolación bilineal de un campo escalar; u se envuelve, v se recorta.
    fn bilinear(&self, field: &[f32], u: f32, v: f32) -> f32 {
        let x = u * MAP_W as f32 - 0.5;
        let y = v * MAP_H as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let a = field[Self::index(x0, y0)];
        let b = field[Self::index(x0 + 1, y0)];
        let c = field[Self::index(x0, y0 + 1)];
        let d = field[Self::index(x0 + 1, y0 + 1)];
        (a * (1.0 - fx) + b * fx) * (1.0 - fy) + (c * (1.0 - fx) + d * fx) * fy
    }

    /// Pendientes (du, dv) del relieve.
    pub fn sample(&self, u: f32, v: f32) -> (f32, f32) {
        let x = u * MAP_W as f32 - 0.5;
        let y = v * MAP_H as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let a = self.slopes[Self::index(x0, y0)];
        let b = self.slopes[Self::index(x0 + 1, y0)];
        let c = self.slopes[Self::index(x0, y0 + 1)];
        let d = self.slopes[Self::index(x0 + 1, y0 + 1)];
        let mix = |k: usize| {
            (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy
        };
        (mix(0), mix(1))
    }

    /// Altura fbm en [-1, 1] aproximadamente.
    pub fn height(&self, u: f32, v: f32) -> f32 {
        self.bilinear(&self.heights, u, v)
    }

    /// Patrón extra horneado (ver `DetailKind`).
    pub fn detail(&self, u: f32, v: f32) -> f32 {
        self.bilinear(&self.detail, u, v)
    }
}
