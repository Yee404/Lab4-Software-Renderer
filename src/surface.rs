//! Modelos de superficie (creados por nosotros, proceduralmente) y su distribución
//! sobre los planetas con ruido y semilla fija:
//!   - 4 tipos de modelo: árbol, roca, cristal y espiga volcánica.
//!   - Distribución reproducible: misma semilla → mismas posiciones.
//!   - Cada instancia se orienta con la normal de la superficie y se mueve con el
//!     planeta cuando este rota (se transforma con la misma rotación que el planeta).

use crate::planet_mesh::{icosphere_triangles, TerrainGen};
use crate::vertex::Vertex;
use crate::ShaderKind;
use noise::{NoiseFn, OpenSimplex};
use raylib::prelude::{Vector2, Vector3};
use std::f32::consts::PI;

#[derive(Clone, Copy)]
pub enum PropKind {
    Tree,
    Rock,
    Crystal,
    Spire,
}

/// Cuántas instancias de un tipo van en un planeta y cómo se agrupan.
pub struct PropSpec {
    pub kind: PropKind,
    pub tint: [f32; 3],
    pub count: u32,
    /// Frecuencia del ruido que agrupa las instancias (bosques, campos de cristales…).
    pub cluster_freq: f32,
    /// Umbral del ruido: más alto = grupos más escasos y compactos.
    pub cluster_threshold: f32,
    pub min_scale: f32,
    pub max_scale: f32,
}

pub struct PropPart {
    pub vertices: Vec<Vertex>,
    pub color: Vector3,
    pub shader: ShaderKind,
}

pub struct PropMesh {
    pub parts: Vec<PropPart>,
}

pub struct PropInstance {
    /// Índice en la lista de `PropMesh` del planeta.
    pub mesh: usize,
    /// Normal de la superficie (eje "arriba" del modelo), en coordenadas del planeta.
    pub up: Vector3,
    pub tangent_x: Vector3,
    pub tangent_z: Vector3,
    /// Desplazamiento radial del terreno en ese punto (fracción del radio).
    pub height: f32,
    pub scale: f32,
}

// ---------- Rotaciones (mismo orden que el model matrix de los planetas: X·Y·Z) ----------

fn rotate_x(v: Vector3, a: f32) -> Vector3 {
    let (s, c) = a.sin_cos();
    Vector3::new(v.x, c * v.y - s * v.z, s * v.y + c * v.z)
}

fn rotate_y(v: Vector3, a: f32) -> Vector3 {
    let (s, c) = a.sin_cos();
    Vector3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
}

fn rotate_z(v: Vector3, a: f32) -> Vector3 {
    let (s, c) = a.sin_cos();
    Vector3::new(c * v.x - s * v.y, s * v.x + c * v.y, v.z)
}

/// Aplica la rotación de un cuerpo (inclinación X, giro Y, Z) a un vector de su sistema local.
pub fn rotate_forward(v: Vector3, rot: Vector3) -> Vector3 {
    rotate_x(rotate_y(rotate_z(v, rot.z), rot.y), rot.x)
}

// ---------- Constructor de mallas con normales planas ----------

struct MeshBuilder {
    vertices: Vec<Vertex>,
}

impl MeshBuilder {
    fn new() -> Self {
        MeshBuilder { vertices: Vec::new() }
    }

    /// Añade un triángulo con normal plana orientada hacia afuera de `inside`.
    fn tri(&mut self, a: Vector3, b: Vector3, c: Vector3, inside: Vector3) {
        let n = (b - a).cross(c - a);
        let centroid = (a + b + c) * (1.0 / 3.0);
        let n = if n.length() > 1e-9 { n.normalized() } else { Vector3::new(0.0, 1.0, 0.0) };
        let n = if n.dot(centroid - inside) < 0.0 { n * -1.0 } else { n };
        for p in [a, b, c] {
            self.vertices.push(Vertex::new(p, n, Vector2::zero()));
        }
    }

    /// Tronco de cono (cilindro si r0 == r1; cono si r1 == 0) a lo largo de +Y.
    fn frustum(&mut self, y0: f32, r0: f32, y1: f32, r1: f32, segments: usize, twist: f32) {
        let inside = Vector3::new(0.0, (y0 + y1) * 0.5, 0.0);
        let ring = |y: f32, r: f32, i: usize| {
            let a = twist + 2.0 * PI * i as f32 / segments as f32;
            Vector3::new(r * a.cos(), y, r * a.sin())
        };
        for i in 0..segments {
            let j = (i + 1) % segments;
            let (a0, a1) = (ring(y0, r0, i), ring(y0, r0, j));
            let (b0, b1) = (ring(y1, r1, i), ring(y1, r1, j));
            self.tri(a0, a1, b0, inside);
            if r1 > 0.0 {
                self.tri(a1, b1, b0, inside);
            }
            // Tapas
            self.tri(Vector3::new(0.0, y0, 0.0), a1, a0, inside);
            if r1 > 0.0 {
                self.tri(Vector3::new(0.0, y1, 0.0), b0, b1, inside);
            }
        }
    }

    /// Mueve todos los vértices con `f` (inclinaciones, desplazamientos) y recalcula normales.
    fn transformed(self, f: impl Fn(Vector3) -> Vector3, inside: Vector3) -> Vec<Vertex> {
        let mut out = MeshBuilder::new();
        for tri in self.vertices.chunks_exact(3) {
            out.tri(f(tri[0].position), f(tri[1].position), f(tri[2].position), f(inside));
        }
        out.vertices
    }
}

fn part(vertices: Vec<Vertex>, color: Vector3, shader: ShaderKind) -> PropPart {
    PropPart { vertices, color, shader }
}

fn tinted(tint: [f32; 3]) -> Vector3 {
    Vector3::new(tint[0], tint[1], tint[2])
}

/// Construye el modelo de un tipo (base en y = 0, altura ≈ 1, escala 1).
pub fn build_prop(kind: PropKind, tint: [f32; 3]) -> PropMesh {
    let color = tinted(tint);
    match kind {
        // Árbol: tronco + dos conos de follaje apilados.
        PropKind::Tree => {
            let mut trunk = MeshBuilder::new();
            trunk.frustum(0.0, 0.13, 0.55, 0.10, 6, 0.0);
            let mut crown = MeshBuilder::new();
            crown.frustum(0.40, 0.58, 1.10, 0.0, 8, 0.0);
            crown.frustum(0.85, 0.42, 1.45, 0.0, 8, 0.2);
            PropMesh {
                parts: vec![
                    part(trunk.vertices, Vector3::new(0.35, 0.22, 0.12), ShaderKind::Prop),
                    part(crown.vertices, color, ShaderKind::Prop),
                ],
            }
        }
        // Roca: icosfera deformada y aplastada.
        PropKind::Rock => {
            let noise = OpenSimplex::new(77);
            let mut b = MeshBuilder::new();
            let center = Vector3::new(0.0, 0.25, 0.0);
            for tri in icosphere_triangles(1) {
                let p = |d: Vector3| {
                    let n = noise.get([d.x as f64 * 2.3, d.y as f64 * 2.3, d.z as f64 * 2.3]) as f32;
                    let r = 0.55 * (1.0 + 0.35 * n);
                    Vector3::new(d.x * r * 1.1, d.y * r * 0.65 + 0.25, d.z * r)
                };
                b.tri(p(tri[0]), p(tri[1]), p(tri[2]), center);
            }
            PropMesh { parts: vec![part(b.vertices, color, ShaderKind::Prop)] }
        }
        // Cristal: racimo de tres bipirámides alargadas e inclinadas.
        PropKind::Crystal => {
            let mut verts = Vec::new();
            for &(tilt, azimuth, height, radius) in
                &[(0.0_f32, 0.0_f32, 1.4_f32, 0.28_f32), (0.45, 2.1, 1.0, 0.22), (-0.5, 4.2, 0.8, 0.2)]
            {
                let mut b = MeshBuilder::new();
                let mid = height * 0.35;
                let ring = |i: usize| {
                    let a = 2.0 * PI * i as f32 / 5.0;
                    Vector3::new(radius * a.cos(), mid, radius * a.sin())
                };
                let top = Vector3::new(0.0, height, 0.0);
                let bottom = Vector3::new(0.0, -0.05, 0.0);
                let inside = Vector3::new(0.0, mid, 0.0);
                for i in 0..5 {
                    let j = (i + 1) % 5;
                    b.tri(ring(i), ring(j), top, inside);
                    b.tri(ring(j), ring(i), bottom, inside);
                }
                let (st, ct) = tilt.sin_cos();
                let (sa, ca) = azimuth.sin_cos();
                let place = move |p: Vector3| {
                    // inclinar sobre Z, girar sobre Y y desplazar hacia afuera
                    let q = Vector3::new(ct * p.x - st * p.y, st * p.x + ct * p.y, p.z);
                    Vector3::new(ca * q.x + sa * q.z + 0.15 * ca, q.y, -sa * q.x + ca * q.z + 0.15 * sa)
                };
                verts.extend(b.transformed(place, Vector3::new(0.0, mid, 0.0)));
            }
            PropMesh { parts: vec![part(verts, color, ShaderKind::Crystal)] }
        }
        // Espiga volcánica: obelisco de cuatro caras con la punta incandescente.
        PropKind::Spire => {
            let mut body = MeshBuilder::new();
            body.frustum(0.0, 0.42, 1.2, 0.14, 4, PI / 4.0);
            let mut tip = MeshBuilder::new();
            tip.frustum(1.2, 0.14, 1.65, 0.0, 4, PI / 4.0);
            PropMesh {
                parts: vec![
                    part(body.vertices, color, ShaderKind::Prop),
                    part(tip.vertices, Vector3::new(1.0, 0.45, 0.10), ShaderKind::Crystal),
                ],
            }
        }
    }
}

// ---------- Distribución procedural ----------

struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / 16_777_216.0
    }
}

/// Coloca instancias de `spec` sobre un planeta de radio `planet_radius`.
/// Determinista: misma `seed` → mismas posiciones. Las instancias se agrupan donde
/// el ruido supera el umbral y no se solapan entre sí.
pub fn place_props(
    mesh_index: usize,
    spec: &PropSpec,
    planet_radius: f32,
    terrain: Option<&TerrainGen>,
    seed: u32,
    instances: &mut Vec<PropInstance>,
) {
    let mut rng = Lcg(seed.wrapping_mul(2654435761).wrapping_add(mesh_index as u32 * 7919 + 1));
    let cluster_noise = OpenSimplex::new(seed ^ 0x5bd1);
    let mut placed = 0;
    let mut attempts = 0;

    while placed < spec.count && attempts < spec.count * 40 {
        attempts += 1;

        // Dirección uniforme sobre la esfera.
        let z = rng.next() * 2.0 - 1.0;
        let phi = rng.next() * 2.0 * PI;
        let r = (1.0 - z * z).sqrt();
        let dir = Vector3::new(r * phi.cos(), z, r * phi.sin());

        // Agrupación por ruido.
        let n = cluster_noise.get([
            (dir.x * spec.cluster_freq) as f64,
            (dir.y * spec.cluster_freq) as f64,
            (dir.z * spec.cluster_freq) as f64,
        ]) as f32;
        if n < spec.cluster_threshold {
            continue;
        }

        let scale = spec.min_scale + (spec.max_scale - spec.min_scale) * rng.next();

        // Separación mínima respecto a las ya colocadas.
        let too_close = instances.iter().any(|other| {
            let min_sep = (scale.max(other.scale) * 1.1) / planet_radius;
            other.up.dot(dir) > (1.0 - min_sep * min_sep * 0.5).max(-1.0)
        });
        if too_close {
            continue;
        }

        let reference = if dir.y.abs() < 0.9 { Vector3::new(0.0, 1.0, 0.0) } else { Vector3::new(1.0, 0.0, 0.0) };
        let tx = reference.cross(dir).normalized();
        let tz = tx.cross(dir);
        let yaw = rng.next() * 2.0 * PI;
        let (s, c) = yaw.sin_cos();

        instances.push(PropInstance {
            mesh: mesh_index,
            up: dir,
            tangent_x: tx * c + tz * s,
            tangent_z: tx * -s + tz * c,
            height: terrain.map(|t| t.height(dir)).unwrap_or(0.0),
            scale,
        });
        placed += 1;
    }
}
