mod camera;
mod fragment;
mod framebuffer;
mod light;
mod line;
mod matrix;
mod normal_map;
mod obj;
mod planet_mesh;
mod shaders;
mod surface;
mod triangle;
mod vertex;

use crate::camera::Camera;
use crate::light::Light;
use crate::matrix::{
    create_model_matrix, create_projection_matrix, create_viewport_matrix, new_matrix4,
};
use crate::normal_map::{DetailKind, NormalMap};
use crate::planet_mesh::{generate_planet_mesh, TerrainGen};
use crate::surface::{build_prop, place_props, rotate_forward, PropInstance, PropKind, PropMesh, PropSpec};
use framebuffer::Framebuffer;
use obj::load_centered_parts;
use raylib::prelude::*;
use shaders::{fragment_shader, vertex_shader};
use std::f32::consts::PI;
use triangle::triangle;
use vertex::Vertex;

// Tamaño de la ventana (y del framebuffer). Como el renderizado es por software, el costo
// crece con el número de píxeles: 960x720 tiene 2.25 veces los de 640x480.
const WINDOW_WIDTH: i32 = 960;
const WINDOW_HEIGHT: i32 = 720;

// --- Proyección ---
const FOV_Y: f32 = PI / 3.0;
const NEAR_PLANE: f32 = 0.25;
const FAR_PLANE: f32 = 1800.0;

// --- Nave (objeto del mundo) y cámara de tercera persona rígida ---
// Escala de la malla centrada (la nave Astroneer mide ~10 de largo y ~8 de ancho).
const SHIP_SCALE: f32 = 0.14;
// Altura de la cámara sobre ship_position.
const CAMERA_HEIGHT: f32 = 0.75;
// Punto mirado: adelante de la nave y a CAMERA_TARGET_HEIGHT sobre ella.
const CAMERA_LOOK_AHEAD: f32 = 5.0;
const CAMERA_TARGET_HEIGHT: f32 = 0.2;
// Distancia de la cámara detrás del CENTRO de la nave (zoom con Q/E/rueda) y sus límites.
const CAMERA_DISTANCE_DEFAULT: f32 = 2.9;
const CAMERA_DISTANCE_MIN: f32 = 1.9;
const CAMERA_DISTANCE_MAX: f32 = 7.0;
const ZOOM_SPEED: f32 = 2.0;
// Desfase fijo de orientación del OBJ respecto a forward. La nave Astroneer tiene la
// proa en -Z, así que no hace falta desfase.
const SHIP_YAW_OFFSET: f32 = 0.0;

// --- Física de la nave (todo depende de delta_time) ---
// Empuje en unidades/s², velocidad máxima en unidades/s; Shift multiplica ambos.
const THRUST_ACCEL: f32 = 60.0;
const MAX_SPEED: f32 = 50.0;
const TURBO_MULTIPLIER: f32 = 3.0;
// Rozamiento exponencial: suave con empuje, fuerte al soltar las teclas (frenado).
const DRAG_THRUST: f32 = 0.35;
const DRAG_BRAKE: f32 = 1.8;
// Giro progresivo: velocidad angular máxima (rad/s) y rapidez con que se alcanza.
const TURN_MAX_SPEED: f32 = 1.1;
const TURN_RESPONSE: f32 = 4.0;
// Límite de la inclinación vertical (rad), para no voltear la cámara en los polos.
const PITCH_LIMIT: f32 = 1.45;
// Margen de colisión sobre la superficie (incluye la cámara, que va ~2 unidades detrás).
const SHIP_COLLISION_MARGIN: f32 = 3.4;
// Zona de control local: dentro de LOCAL_RANGE_FACTOR × radio la nave viaja con el
// planeta, así se puede rodear sin que la órbita del planeta "se la lleve".
const LOCAL_RANGE_FACTOR: f32 = 5.0;
// Gravedad: aceleración en la superficie y alcance (en radios) donde se desvanece.
const PLANET_SURFACE_GRAVITY: f32 = 5.0;
const SUN_SURFACE_GRAVITY: f32 = 6.0;
const GRAVITY_RANGE_FACTOR: f32 = 5.0;

// --- Sistema solar ---
const SUN_RADIUS: f32 = 30.0;

// --- Escenario "colapso del universo" (tecla K) ---
// Duración de cada fase en segundos.
const COLLAPSE_SECONDS: f32 = 22.0; // los planetas caen en espiral hacia el Sol
const EXPLOSION_SECONDS: f32 = 5.0; // el Sol estalla y la onda lo cubre todo
const REBORN_SECONDS: f32 = 3.0; // el universo reaparece desde el blanco
// Radio máximo de la onda expansiva de la explosión.
const SHOCKWAVE_RADIUS: f32 = 700.0;
const EXPLOSION_PARTICLES: usize = 200;
const IMPACT_PARTICLES: usize = 24;
const MAX_PARTICLES: usize = 420;
// Semilla de la distribución procedural de modelos sobre los planetas (misma semilla →
// mismas posiciones). Cámbiala para obtener otra distribución.
const PROP_SEED: u32 = 2026;
// Los modelos de superficie solo se dibujan a menos de PROP_DRAW_RANGE × radio del planeta.
const PROP_DRAW_RANGE: f32 = 7.0;

#[derive(Clone, Copy)]
pub enum ShaderKind {
    Ship,
    Sun,
    Terran,
    Desert,
    Ice,
    Gas,
    Lava,
    Prop,
    Crystal,
}

/// Estilo de iluminación de un objeto (look "cómic" / Astroneer):
///  - `levels`: escalones de luz (cel shading). 0 = sombreado continuo.
///  - `halftone`: intensidad de los puntos de semitono (estilo Spider-Verse) en la sombra.
///  - `rim_color` / `rim_strength`: halo de atmósfera en el borde del cuerpo.
#[derive(Clone, Copy)]
pub struct StyleParams {
    pub levels: f32,
    pub halftone: f32,
    pub rim_color: Vector3,
    pub rim_strength: f32,
}

impl StyleParams {
    /// Solo escalones de luz, sin semitono ni halo.
    pub fn toon(levels: f32) -> Self {
        StyleParams {
            levels,
            halftone: 0.0,
            rim_color: Vector3::zero(),
            rim_strength: 0.0,
        }
    }
}

/// Versión de `StyleParams` que se puede escribir en una constante.
struct StyleSpec {
    levels: f32,
    halftone: f32,
    rim_color: [f32; 3],
    rim_strength: f32,
}

impl StyleSpec {
    fn params(&self) -> StyleParams {
        StyleParams {
            levels: self.levels,
            halftone: self.halftone,
            rim_color: Vector3::new(self.rim_color[0], self.rim_color[1], self.rim_color[2]),
            rim_strength: self.rim_strength,
        }
    }
}

pub struct Uniforms<'a> {
    pub model_matrix: Matrix,
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub viewport_matrix: Matrix,
    pub time: f32,
    pub shader_kind: ShaderKind,
    // Datos del objeto para que los shaders usen coordenadas del propio cuerpo.
    pub object_center: Vector3,
    pub object_radius: f32,
    pub object_rotation: Vector3,
    pub normal_map: Option<&'a NormalMap>,
    pub style: StyleParams,
    /// Posición de la cámara (para el halo de borde).
    pub eye: Vector3,
    /// Calor del Sol: 0 = normal, hacia 1 se pone blanco; negativo = enfriándose (rojo).
    pub heat: f32,
}

struct Planet {
    orbit_radius: f32,
    /// Velocidad angular de la órbita (rad/s).
    orbit_speed: f32,
    phase: f32,
    height: f32,
    radius: f32,
    /// Velocidad de rotación sobre su eje (rad/s).
    spin_speed: f32,
    /// Inclinación del eje (rad).
    tilt: f32,
    color: [f32; 3],
    shader: ShaderKind,
    /// Deformación de la geometría con ruido: amplitud (fracción del radio) y frecuencia.
    terrain_amp: f32,
    terrain_freq: f32,
    terrain_seed: u32,
    /// Normales por cara (aspecto low-poly) o suaves.
    flat_shading: bool,
    /// Intensidad del mapa de normales (0 = sin mapa) y frecuencia de su ruido.
    normal_strength: f32,
    normal_freq: f32,
    props: &'static [PropSpec],
    /// Tipo de patrón extra que se hornea en el mapa del planeta (nubes, dunas, celdas).
    detail: DetailKind,
    style: StyleSpec,
}

const TERRAN_PROPS: [PropSpec; 2] = [
    PropSpec { kind: PropKind::Tree, tint: [0.18, 0.62, 0.30], count: 70, cluster_freq: 2.0, cluster_threshold: -0.1, min_scale: 0.9, max_scale: 1.5 },
    PropSpec { kind: PropKind::Rock, tint: [0.50, 0.50, 0.54], count: 35, cluster_freq: 3.5, cluster_threshold: -0.3, min_scale: 0.5, max_scale: 1.0 },
];
const DESERT_PROPS: [PropSpec; 1] = [
    PropSpec { kind: PropKind::Rock, tint: [0.72, 0.38, 0.18], count: 70, cluster_freq: 2.5, cluster_threshold: -0.2, min_scale: 0.8, max_scale: 2.2 },
];
const ICE_PROPS: [PropSpec; 1] = [
    PropSpec { kind: PropKind::Crystal, tint: [0.45, 0.78, 1.0], count: 55, cluster_freq: 2.2, cluster_threshold: -0.1, min_scale: 0.8, max_scale: 1.9 },
];
const LAVA_PROPS: [PropSpec; 2] = [
    PropSpec { kind: PropKind::Spire, tint: [0.16, 0.08, 0.07], count: 40, cluster_freq: 2.4, cluster_threshold: -0.1, min_scale: 0.8, max_scale: 1.7 },
    PropSpec { kind: PropKind::Rock, tint: [0.22, 0.14, 0.12], count: 20, cluster_freq: 3.0, cluster_threshold: -0.3, min_scale: 0.4, max_scale: 0.9 },
];
const NO_PROPS: [PropSpec; 0] = [];

const PLANETS: [Planet; 5] = [
    // Terran: continentes verdes, relieve suave, árboles y rocas
    Planet {
        orbit_radius: 110.0, orbit_speed: 0.040, phase: 1.0, height: 0.0, radius: 12.0,
        spin_speed: 0.45, tilt: 0.10, color: [0.08, 0.75, 0.48], shader: ShaderKind::Terran,
        terrain_amp: 0.06, terrain_freq: 1.6, terrain_seed: 11, flat_shading: true,
        normal_strength: 0.35, normal_freq: 5.0, props: &TERRAN_PROPS,
        detail: DetailKind::Clouds,
        // Estilo Astroneer: escalones suaves y atmósfera azul.
        style: StyleSpec { levels: 4.0, halftone: 0.0, rim_color: [0.45, 0.75, 1.0], rim_strength: 0.60 },
    },
    // Desierto: bandas naranjas, dunas, rocas rojizas
    Planet {
        orbit_radius: 170.0, orbit_speed: 0.030, phase: 2.2, height: 4.0, radius: 15.0,
        spin_speed: 0.35, tilt: 0.15, color: [0.95, 0.43, 0.08], shader: ShaderKind::Desert,
        terrain_amp: 0.05, terrain_freq: 1.2, terrain_seed: 22, flat_shading: true,
        normal_strength: 0.30, normal_freq: 4.0, props: &DESERT_PROPS,
        detail: DetailKind::Ridged,
        // Estilo Astroneer: escalones suaves y bruma cálida.
        style: StyleSpec { levels: 4.0, halftone: 0.0, rim_color: [1.0, 0.70, 0.40], rim_strength: 0.45 },
    },
    // Hielo: grietas azules, relieve escarpado, cristales
    Planet {
        orbit_radius: 235.0, orbit_speed: 0.023, phase: 4.1, height: -5.0, radius: 10.0,
        spin_speed: 0.30, tilt: 0.20, color: [0.45, 0.82, 1.0], shader: ShaderKind::Ice,
        terrain_amp: 0.09, terrain_freq: 2.2, terrain_seed: 33, flat_shading: true,
        normal_strength: 0.40, normal_freq: 6.0, props: &ICE_PROPS,
        detail: DetailKind::Cells,
        // Estilo Spider-Verse: 3 escalones, puntos de semitono y halo neón cian.
        style: StyleSpec { levels: 3.0, halftone: 0.55, rim_color: [0.40, 1.0, 1.0], rim_strength: 0.85 },
    },
    // Gigante gaseoso: bandas animadas, esfera lisa
    Planet {
        orbit_radius: 310.0, orbit_speed: 0.017, phase: 5.8, height: 0.0, radius: 26.0,
        spin_speed: 0.60, tilt: 0.05, color: [0.85, 0.62, 0.40], shader: ShaderKind::Gas,
        terrain_amp: 0.0, terrain_freq: 1.0, terrain_seed: 44, flat_shading: false,
        normal_strength: 0.0, normal_freq: 1.0, props: &NO_PROPS,
        detail: DetailKind::Clouds,
        // Estilo Spider-Verse: 3 escalones, semitono y halo magenta.
        style: StyleSpec { levels: 3.0, halftone: 0.60, rim_color: [1.0, 0.40, 0.85], rim_strength: 0.80 },
    },
    // Lava: grietas incandescentes animadas, relieve rugoso, espigas volcánicas
    Planet {
        orbit_radius: 390.0, orbit_speed: 0.013, phase: 3.3, height: 6.0, radius: 9.0,
        spin_speed: 0.25, tilt: 0.12, color: [0.35, 0.12, 0.10], shader: ShaderKind::Lava,
        terrain_amp: 0.12, terrain_freq: 2.0, terrain_seed: 55, flat_shading: true,
        normal_strength: 0.50, normal_freq: 7.0, props: &LAVA_PROPS,
        detail: DetailKind::Cells,
        // Mezcla: 3 escalones, semitono suave y halo naranja.
        style: StyleSpec { levels: 3.0, halftone: 0.40, rim_color: [1.0, 0.50, 0.10], rim_strength: 0.75 },
    },
];

/// Posición de un planeta. `collapse` ∈ [0, 1] es el avance del colapso (0 = órbita normal):
/// el radio de la órbita se encoge y el planeta gira cada vez más rápido (como un patinador
/// que cierra los brazos), cayendo en espiral hacia el Sol.
fn planet_position(planet: &Planet, time: f32, collapse: f32) -> Vector3 {
    let shrink = (1.0 - collapse).max(0.0).powf(1.5);
    let angle = planet.phase + time * planet.orbit_speed + 6.0 * collapse * collapse;
    Vector3::new(
        angle.cos() * planet.orbit_radius * shrink,
        planet.height * (1.0 - collapse),
        angle.sin() * planet.orbit_radius * shrink,
    )
}

fn planet_rotation(planet: &Planet, time: f32, collapse: f32) -> Vector3 {
    Vector3::new(
        planet.tilt,
        time * planet.spin_speed + 25.0 * collapse * collapse,
        0.0,
    )
}

/// Chispa de la explosión o de un impacto contra el Sol.
struct Particle {
    position: Vector3,
    velocity: Vector3,
    life: f32,
    max_life: f32,
    size: f32,
    heat: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum ScenarioPhase {
    Normal,
    Collapse,
    Explosion,
    Reborn,
}

/// Generador pseudoaleatorio sencillo para las chispas (no necesita ser reproducible).
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / 16_777_216.0
    }
}

fn spawn_burst(
    particles: &mut Vec<Particle>,
    rng: &mut Lcg,
    origin: Vector3,
    count: usize,
    speed_min: f32,
    speed_max: f32,
    size: f32,
) {
    for _ in 0..count {
        if particles.len() >= MAX_PARTICLES {
            return;
        }
        let z = rng.next() * 2.0 - 1.0;
        let phi = rng.next() * 2.0 * PI;
        let r = (1.0 - z * z).sqrt();
        let dir = Vector3::new(r * phi.cos(), z, r * phi.sin());
        let speed = speed_min + (speed_max - speed_min) * rng.next();
        let life = 2.0 + 2.0 * rng.next();
        particles.push(Particle {
            position: origin,
            velocity: dir * speed,
            life,
            max_life: life,
            size: size * (0.6 + 0.8 * rng.next()),
            heat: 0.3 + 0.7 * rng.next(),
        });
    }
}

/// Mallas con distinto detalle (LOD): icosfera subdividida 2, 3 y 4 veces.
struct MeshLods {
    low: Vec<Vertex>,
    mid: Vec<Vertex>,
    high: Vec<Vertex>,
}

impl MeshLods {
    fn generate(terrain: Option<&TerrainGen>, flat: bool) -> Self {
        MeshLods {
            low: generate_planet_mesh(2, terrain, flat),
            mid: generate_planet_mesh(3, terrain, flat),
            high: generate_planet_mesh(4, terrain, flat),
        }
    }
}

/// Color y shader de cada pieza de la nave según el nombre del objeto en el OBJ
/// (paleta tipo Astroneer: blanco hueso, naranja, gris y cristal celeste).
fn ship_part_look(name: &str) -> (Vector3, ShaderKind) {
    if name.starts_with("casco") {
        (Vector3::new(0.90, 0.88, 0.82), ShaderKind::Ship)
    } else if name.starts_with("cabina") {
        (Vector3::new(0.25, 0.75, 0.95), ShaderKind::Crystal)
    } else if name.starts_with("modulo") || name.starts_with("aleta") {
        (Vector3::new(0.96, 0.52, 0.12), ShaderKind::Ship)
    } else if name.starts_with("tobera") {
        (Vector3::new(0.30, 0.30, 0.34), ShaderKind::Ship)
    } else if name.starts_with("luz") {
        (Vector3::new(1.0, 0.85, 0.30), ShaderKind::Crystal)
    } else {
        (Vector3::new(0.55, 0.57, 0.60), ShaderKind::Ship) // antena, patas
    }
}

/// Todo lo generado una sola vez al arrancar para un planeta.
struct PlanetRuntime {
    lods: MeshLods,
    normal_map: Option<NormalMap>,
    prop_meshes: Vec<PropMesh>,
    instances: Vec<PropInstance>,
    /// Radio máximo con el relieve (para colisión y descarte).
    max_radius: f32,
}

fn build_planet_runtime(planet: &Planet) -> PlanetRuntime {
    let terrain = if planet.terrain_amp > 0.0 {
        Some(TerrainGen::new(planet.terrain_seed, planet.terrain_amp, planet.terrain_freq))
    } else {
        None
    };
    let lods = MeshLods::generate(terrain.as_ref(), planet.flat_shading);
    let normal_map = if planet.normal_strength > 0.0 {
        Some(NormalMap::generate(
            planet.terrain_seed + 7,
            planet.normal_freq,
            planet.normal_strength,
            planet.detail,
        ))
    } else {
        None
    };

    let mut prop_meshes = Vec::new();
    let mut instances = Vec::new();
    for (i, spec) in planet.props.iter().enumerate() {
        prop_meshes.push(build_prop(spec.kind, spec.tint));
        place_props(
            i,
            spec,
            planet.radius,
            terrain.as_ref(),
            PROP_SEED.wrapping_add(planet.terrain_seed),
            &mut instances,
        );
    }

    PlanetRuntime {
        lods,
        normal_map,
        prop_meshes,
        instances,
        max_radius: planet.radius * (1.0 + planet.terrain_amp),
    }
}

/// Interruptores para comparar el trabajo del renderizador (descarte y LOD).
struct Toggles {
    culling: bool,
    lod: bool,
    props: bool,
    hud: bool,
}

#[derive(Default)]
struct RenderStats {
    drawn: u32,
    culled: u32,
    props_drawn: u32,
    props_culled: u32,
    triangles: u32,
}

/// Datos de la cámara de este frame, para descarte y LOD.
struct Scene {
    view_matrix: Matrix,
    projection_matrix: Matrix,
    viewport_matrix: Matrix,
    eye: Vector3,
    cam_forward: Vector3,
    cam_right: Vector3,
    cam_up: Vector3,
    time: f32,
}

struct ObjectParams<'a> {
    center: Vector3,
    radius: f32,
    rotation: Vector3,
    color: Vector3,
    shader: ShaderKind,
    normal_map: Option<&'a NormalMap>,
    style: StyleParams,
    heat: f32,
}

fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    light: &Light,
    base_color: Vector3,
) {
    let transformed_vertices: Vec<Vertex> = vertex_array
        .iter()
        .map(|vertex| vertex_shader(vertex, uniforms))
        .collect();

    for triangle_vertices in transformed_vertices.chunks_exact(3) {
        let fragments = triangle(
            &triangle_vertices[0],
            &triangle_vertices[1],
            &triangle_vertices[2],
            light,
            base_color,
            framebuffer.width,
            framebuffer.height,
        );

        for fragment in fragments {
            let final_color = fragment_shader(&fragment, uniforms);
            framebuffer.point(
                fragment.position.x as i32,
                fragment.position.y as i32,
                final_color,
                fragment.depth,
            );
        }
    }
}

fn render_with_model(
    framebuffer: &mut Framebuffer,
    stats: &mut RenderStats,
    vertices: &[Vertex],
    light: &Light,
    scene: &Scene,
    model_matrix: Matrix,
    params: &ObjectParams,
) {
    let uniforms = Uniforms {
        model_matrix,
        view_matrix: scene.view_matrix,
        projection_matrix: scene.projection_matrix,
        viewport_matrix: scene.viewport_matrix,
        time: scene.time,
        shader_kind: params.shader,
        object_center: params.center,
        object_radius: params.radius,
        object_rotation: params.rotation,
        normal_map: params.normal_map,
        style: params.style,
        eye: scene.eye,
        heat: params.heat,
    };
    stats.triangles += (vertices.len() / 3) as u32;
    render(framebuffer, &uniforms, vertices, light, params.color);
}

/// Descarte por frustum: ¿la esfera toca el volumen visible?
fn sphere_visible(scene: &Scene, center: Vector3, radius: f32) -> bool {
    let to = center - scene.eye;
    let z = to.dot(scene.cam_forward);
    if z + radius < NEAR_PLANE || z - radius > FAR_PLANE {
        return false;
    }
    let aspect = WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32;
    let tan_v = (FOV_Y * 0.5).tan();
    let tan_h = tan_v * aspect;
    let x = to.dot(scene.cam_right);
    let y = to.dot(scene.cam_up);
    let zc = z.max(0.0);
    x.abs() <= zc * tan_h + radius * (1.0 + tan_h * tan_h).sqrt()
        && y.abs() <= zc * tan_v + radius * (1.0 + tan_v * tan_v).sqrt()
}

/// Dibuja un cuerpo esférico con descarte y nivel de detalle según su tamaño aparente.
/// Devuelve true si se dibujó.
#[allow(clippy::too_many_arguments)]
fn draw_body(
    framebuffer: &mut Framebuffer,
    stats: &mut RenderStats,
    scene: &Scene,
    toggles: &Toggles,
    light: &Light,
    lods: &MeshLods,
    params: &ObjectParams,
    bound_radius: f32,
) -> bool {
    if toggles.culling && !sphere_visible(scene, params.center, bound_radius) {
        stats.culled += 1;
        return false;
    }
    let distance = (params.center - scene.eye).length().max(0.001);
    let angular_size = params.radius / distance;
    let mesh = if !toggles.lod || angular_size > 0.22 {
        &lods.high
    } else if angular_size > 0.07 {
        &lods.mid
    } else {
        &lods.low
    };
    stats.drawn += 1;
    let model = create_model_matrix(params.center, params.radius, params.rotation);
    render_with_model(framebuffer, stats, mesh, light, scene, model, params);
    true
}

/// Dibuja los modelos de superficie de un planeta cercano. Cada instancia se coloca sobre
/// el terreno, orientada con la normal de la superficie, y gira con el planeta.
#[allow(clippy::too_many_arguments)]
fn draw_props(
    framebuffer: &mut Framebuffer,
    stats: &mut RenderStats,
    scene: &Scene,
    toggles: &Toggles,
    light: &Light,
    planet: &Planet,
    runtime: &PlanetRuntime,
    center: Vector3,
    rotation: Vector3,
) {
    let to_eye_center = scene.eye - center;
    let distance = to_eye_center.length().max(0.001);

    for instance in runtime.instances.iter() {
        let up = rotate_forward(instance.up, rotation);
        let surface_radius = planet.radius * (1.0 + instance.height);
        // Hundido un poco para que no flote sobre las caras planas del terreno.
        let position = center + up * (surface_radius - 0.08 * instance.scale);

        if toggles.culling {
            // Descarte por horizonte: del otro lado del planeta no se ve.
            let cos_angle = up.dot(to_eye_center) / distance;
            if cos_angle < planet.radius / distance - 0.2 {
                stats.props_culled += 1;
                continue;
            }
            if !sphere_visible(scene, position, instance.scale * 1.8) {
                stats.props_culled += 1;
                continue;
            }
        }

        let tx = rotate_forward(instance.tangent_x, rotation);
        let tz = rotate_forward(instance.tangent_z, rotation);
        let s = instance.scale;
        let model = new_matrix4(
            tx.x * s, up.x * s, tz.x * s, position.x,
            tx.y * s, up.y * s, tz.y * s, position.y,
            tx.z * s, up.z * s, tz.z * s, position.z,
            0.0, 0.0, 0.0, 1.0,
        );

        stats.props_drawn += 1;
        for part in runtime.prop_meshes[instance.mesh].parts.iter() {
            let params = ObjectParams {
                center: position,
                radius: s,
                rotation: Vector3::zero(),
                color: part.color,
                shader: part.shader,
                normal_map: None,
                style: StyleParams::toon(4.0),
                heat: 0.0,
            };
            render_with_model(framebuffer, stats, &part.vertices, light, scene, model, &params);
        }
    }
}

/// Estrellas infinitamente lejanas: cada una es una dirección fija del cielo que se
/// proyecta con la orientación de la cámara (no con su posición), así el cielo gira
/// al mirar a los lados, arriba o abajo.
fn draw_stars(framebuffer: &mut Framebuffer, scene: &Scene) {
    let tan_v = (FOV_Y * 0.5).tan();
    let tan_h = tan_v * (WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32);
    for i in 0..400_u32 {
        let a = ((i as f32 * 12.9898).sin() * 43758.547).fract().abs();
        let b = ((i as f32 * 78.233).sin() * 12345.679).fract().abs();
        let theta = a * 2.0 * PI;
        let y = b * 2.0 - 1.0;
        let r = (1.0 - y * y).sqrt();
        let dir = Vector3::new(r * theta.cos(), y, r * theta.sin());

        let cz = dir.dot(scene.cam_forward);
        if cz < 0.05 {
            continue;
        }
        let nx = dir.dot(scene.cam_right) / cz / tan_h;
        let ny = dir.dot(scene.cam_up) / cz / tan_v;
        let sx = ((nx * 0.5 + 0.5) * WINDOW_WIDTH as f32) as i32;
        let sy = ((0.5 - ny * 0.5) * WINDOW_HEIGHT as f32) as i32;
        if sx < 0 || sy < 0 || sx >= WINDOW_WIDTH || sy >= WINDOW_HEIGHT {
            continue;
        }
        let brightness = 0.45 + (i % 7) as f32 * 0.075;
        framebuffer.point(
            sx,
            sy,
            Vector3::new(brightness * 0.8, brightness * 0.9, brightness),
            1.0,
        );
    }
}

/// Cuerpo con gravedad y colisión (el Sol y los planetas).
struct Body {
    center: Vector3,
    previous_center: Vector3,
    radius: f32,
    collision_radius: f32,
    surface_gravity: f32,
    /// Distancia hasta la que atrae.
    gravity_range: f32,
}

fn main() {
    let (mut window, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Proyecto Final")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();
    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32);
    framebuffer.set_background_color(Vector3::new(0.004, 0.006, 0.02));
    framebuffer.init_texture(&mut window, &thread);

    // Nave por piezas, con el centro real del modelo en el origen (ship_position es el
    // centro de la nave). Cada pieza se pinta con el color que le toca según su nombre.
    let ship_parts = load_centered_parts("assets/models/nave_astroneer.obj")
        .expect("No se pudo cargar assets/models/nave_astroneer.obj");

    // Generación procedural (una sola vez): icosferas deformadas, mapas de normales y props.
    let sun_lods = MeshLods::generate(None, false);
    let runtimes: Vec<PlanetRuntime> = PLANETS.iter().map(build_planet_runtime).collect();
    // Icosaedro sin subdividir (20 caras): sirve para las chispas, que son muchas y pequeñas.
    let spark_mesh = generate_planet_mesh(0, None, true);

    // Escenario del colapso (tecla K).
    let mut phase = ScenarioPhase::Normal;
    let mut phase_timer = 0.0_f32;
    let mut consumed = [false; PLANETS.len()];
    let mut particles: Vec<Particle> = Vec::new();
    let mut rng = Lcg(2026);
    let mut collapse = 0.0_f32; // avance del colapso, 0..1

    let mut ship_position = Vector3::new(30.0, 12.0, 190.0);
    let mut ship_velocity = Vector3::zero();
    let mut ship_yaw = 0.0_f32;
    let mut yaw_velocity = 0.0_f32;
    let mut ship_pitch = 0.0_f32;
    let mut pitch_velocity = 0.0_f32;
    let mut camera_distance = CAMERA_DISTANCE_DEFAULT;
    let world_up = Vector3::new(0.0, 1.0, 0.0);
    let mut camera = Camera::new(
        Vector3::new(0.0, 4.0, 21.0),
        Vector3::new(0.0, 0.5, 12.0),
        Vector3::new(0.0, 1.0, 0.0),
    );

    let projection_matrix = create_projection_matrix(
        FOV_Y,
        WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
        NEAR_PLANE,
        FAR_PLANE,
    );
    let viewport_matrix = create_viewport_matrix(
        0.0,
        0.0,
        WINDOW_WIDTH as f32,
        WINDOW_HEIGHT as f32,
    );
    let mut elapsed_time = 0.0_f32;
    let mut toggles = Toggles { culling: true, lod: true, props: true, hud: true };

    while !window.window_should_close() {
        let delta_time = window.get_frame_time().min(0.05);
        let previous_time = elapsed_time;
        elapsed_time += delta_time;

        // Interruptores de demostración (descarte, LOD, modelos de superficie, HUD).
        if window.is_key_pressed(KeyboardKey::KEY_C) {
            toggles.culling = !toggles.culling;
        }
        if window.is_key_pressed(KeyboardKey::KEY_L) {
            toggles.lod = !toggles.lod;
        }
        if window.is_key_pressed(KeyboardKey::KEY_P) {
            toggles.props = !toggles.props;
        }
        if window.is_key_pressed(KeyboardKey::KEY_H) {
            toggles.hud = !toggles.hud;
        }

        // Escenario del colapso: K lo inicia; pulsarla de nuevo lo cancela y restaura todo.
        if window.is_key_pressed(KeyboardKey::KEY_K) {
            if phase == ScenarioPhase::Normal {
                phase = ScenarioPhase::Collapse;
            } else {
                phase = ScenarioPhase::Normal;
                collapse = 0.0;
                particles.clear();
            }
            phase_timer = 0.0;
            consumed = [false; PLANETS.len()];
        }
        // Avance del colapso en el cuadro anterior (para el control local de la nave).
        let mut previous_collapse = collapse;
        if phase != ScenarioPhase::Normal {
            phase_timer += delta_time;
        }

        let mut sun_radius = SUN_RADIUS;
        let mut sun_heat = 0.0_f32;
        let mut shockwave_radius = 0.0_f32; // 0 = sin onda expansiva
        let mut flash = 0.0_f32; // opacidad del destello blanco de pantalla

        match phase {
            ScenarioPhase::Normal => {
                collapse = 0.0;
            }
            // Los planetas se acercan en espiral y se estrellan contra el Sol, que se
            // calienta y crece un poco con cada impacto.
            ScenarioPhase::Collapse => {
                collapse = (phase_timer / COLLAPSE_SECONDS).min(1.0);
                sun_radius = SUN_RADIUS * (1.0 + 0.25 * collapse);
                sun_heat = 0.6 * collapse;
                for (i, planet) in PLANETS.iter().enumerate() {
                    if consumed[i] {
                        continue;
                    }
                    let position = planet_position(planet, elapsed_time, collapse);
                    if position.length() <= sun_radius + planet.radius * 0.5 {
                        consumed[i] = true;
                        spawn_burst(&mut particles, &mut rng, position, IMPACT_PARTICLES, 15.0, 70.0, 1.2);
                    }
                }
                if phase_timer >= COLLAPSE_SECONDS {
                    phase = ScenarioPhase::Explosion;
                    phase_timer = 0.0;
                    consumed = [true; PLANETS.len()];
                    spawn_burst(&mut particles, &mut rng, Vector3::zero(), EXPLOSION_PARTICLES, 60.0, 260.0, 2.0);
                }
            }
            // El Sol estalla: una onda expansiva blanca crece hasta cubrirlo todo.
            ScenarioPhase::Explosion => {
                collapse = 1.0;
                sun_radius = SUN_RADIUS * 1.25;
                let x = (phase_timer / 2.8).min(1.0);
                shockwave_radius = sun_radius + SHOCKWAVE_RADIUS * (1.0 - (1.0 - x).powi(3));
                let t = ((phase_timer - 0.1) / 0.8).clamp(0.0, 1.0);
                flash = t * t * (3.0 - 2.0 * t);
                if phase_timer >= EXPLOSION_SECONDS {
                    // Respawn: todo vuelve a su sitio y la nave reaparece al inicio.
                    phase = ScenarioPhase::Reborn;
                    phase_timer = 0.0;
                    collapse = 0.0;
                    previous_collapse = 0.0;
                    consumed = [false; PLANETS.len()];
                    particles.clear();
                    ship_position = Vector3::new(30.0, 12.0, 190.0);
                    ship_velocity = Vector3::zero();
                    ship_yaw = 0.0;
                    yaw_velocity = 0.0;
                    ship_pitch = 0.0;
                    pitch_velocity = 0.0;
                }
            }
            // El universo reaparece desde el blanco.
            ScenarioPhase::Reborn => {
                collapse = 0.0;
                flash = (1.0 - phase_timer / REBORN_SECONDS).max(0.0);
                if phase_timer >= REBORN_SECONDS {
                    phase = ScenarioPhase::Normal;
                    phase_timer = 0.0;
                }
            }
        }

        // Chispas: se mueven y se apagan con el tiempo.
        for particle in particles.iter_mut() {
            particle.position += particle.velocity * delta_time;
            particle.velocity = particle.velocity * (-0.4 * delta_time).exp();
            particle.life -= delta_time;
        }
        particles.retain(|p| p.life > 0.0);

        // 1. Vista (flechas): ←/→ giran, ↑/↓ miran arriba/abajo. Progresivo: la velocidad
        // angular se acerca a la deseada.
        let mut turn_input = 0.0_f32;
        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            turn_input += 1.0;
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            turn_input -= 1.0;
        }
        let mut pitch_input = 0.0_f32;
        if window.is_key_down(KeyboardKey::KEY_UP) {
            pitch_input += 1.0;
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            pitch_input -= 1.0;
        }
        let response = (TURN_RESPONSE * delta_time).min(1.0);
        yaw_velocity += (turn_input * TURN_MAX_SPEED - yaw_velocity) * response;
        pitch_velocity += (pitch_input * TURN_MAX_SPEED - pitch_velocity) * response;
        ship_yaw += yaw_velocity * delta_time;
        ship_pitch = (ship_pitch + pitch_velocity * delta_time).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        // 2. Base de la nave con yaw y pitch ya actualizados:
        //    forward (hacia donde mira, con inclinación), right (horizontal) y up propio.
        let forward = Vector3::new(
            ship_pitch.cos() * ship_yaw.sin(),
            ship_pitch.sin(),
            -ship_pitch.cos() * ship_yaw.cos(),
        );
        let right = Vector3::new(ship_yaw.cos(), 0.0, ship_yaw.sin());
        let ship_up = right.cross(forward);

        // 3. Empuje (W/S/A/D/Space/Ctrl) → aceleración; sin empuje la nave frena.
        let mut thrust = Vector3::zero();
        if window.is_key_down(KeyboardKey::KEY_W) {
            thrust += forward;
        }
        if window.is_key_down(KeyboardKey::KEY_S) {
            thrust -= forward;
        }
        if window.is_key_down(KeyboardKey::KEY_D) {
            thrust += right;
        }
        if window.is_key_down(KeyboardKey::KEY_A) {
            thrust -= right;
        }
        if window.is_key_down(KeyboardKey::KEY_SPACE) {
            thrust += world_up;
        }
        if window.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
            thrust -= world_up;
        }
        let turbo = if window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            TURBO_MULTIPLIER
        } else {
            1.0
        };
        let has_thrust = thrust.length() > 0.001;
        if has_thrust {
            ship_velocity += thrust.normalized() * (THRUST_ACCEL * turbo * delta_time);
        }
        let drag = if has_thrust { DRAG_THRUST } else { DRAG_BRAKE };
        ship_velocity = ship_velocity * (-drag * delta_time).exp();

        // 4. Gravedad del Sol y de los planetas (se desvanece con la distancia).
        let mut bodies: Vec<Body> = Vec::with_capacity(PLANETS.len() + 1);
        // Durante el colapso la gravedad del Sol crece y alcanza más lejos: arrastra a la nave.
        bodies.push(Body {
            center: Vector3::zero(),
            previous_center: Vector3::zero(),
            radius: sun_radius,
            collision_radius: sun_radius,
            surface_gravity: SUN_SURFACE_GRAVITY * (1.0 + 8.0 * collapse),
            gravity_range: sun_radius * GRAVITY_RANGE_FACTOR * (1.0 + 3.0 * collapse),
        });
        for (i, (planet, runtime)) in PLANETS.iter().zip(runtimes.iter()).enumerate() {
            if consumed[i] {
                continue; // ya cayó en el Sol
            }
            bodies.push(Body {
                center: planet_position(planet, elapsed_time, collapse),
                previous_center: planet_position(planet, previous_time, previous_collapse),
                radius: planet.radius,
                collision_radius: runtime.max_radius,
                surface_gravity: PLANET_SURFACE_GRAVITY,
                gravity_range: planet.radius * GRAVITY_RANGE_FACTOR,
            });
        }
        for body in bodies.iter() {
            let to_center = body.center - ship_position;
            let dist = to_center.length().max(0.001);
            let range = body.gravity_range;
            if dist < range {
                let falloff = 1.0 - dist / range;
                let g = body.surface_gravity * (body.radius / dist).powi(2).min(4.0) * falloff;
                ship_velocity += to_center * (g * delta_time / dist);
            }
        }
        let max_speed = MAX_SPEED * turbo;
        let speed = ship_velocity.length();
        if speed > max_speed {
            ship_velocity = ship_velocity * (max_speed / speed);
        }
        ship_position += ship_velocity * delta_time;

        // 5. Control local: cerca de un planeta la nave viaja con él para poder rodearlo.
        for body in bodies.iter().skip(1) {
            if (ship_position - body.center).length() < body.radius * LOCAL_RANGE_FACTOR {
                ship_position += body.center - body.previous_center;
                break;
            }
        }

        // 6. Colisión: no se puede atravesar el Sol ni los planetas (incluido su relieve).
        for body in bodies.iter() {
            let offset = ship_position - body.center;
            let dist = offset.length();
            let min_dist = body.collision_radius + SHIP_COLLISION_MARGIN;
            if dist < min_dist {
                let normal = if dist > 0.0001 { offset / dist } else { world_up };
                ship_position = body.center + normal * min_dist;
                let inward = ship_velocity.dot(normal);
                if inward < 0.0 {
                    ship_velocity -= normal * inward;
                }
            }
        }

        // 7. Zoom (Q acerca, E aleja, o rueda del ratón): solo camera_distance.
        if window.is_key_down(KeyboardKey::KEY_Q) {
            camera_distance -= ZOOM_SPEED * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_E) {
            camera_distance += ZOOM_SPEED * delta_time;
        }
        camera_distance -= window.get_mouse_wheel_move() * 0.4;
        camera_distance = camera_distance.clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX);

        // 8. Cámara rígida, asignada de forma absoluta desde la ship_position final
        // de este frame y la base de la nave. Sin input propio, sin suavizado ni retraso.
        camera.eye = ship_position - forward * camera_distance + ship_up * CAMERA_HEIGHT;
        camera.target =
            ship_position + forward * CAMERA_LOOK_AHEAD + ship_up * CAMERA_TARGET_HEIGHT;
        camera.up = ship_up;

        let cam_forward = (camera.target - camera.eye).normalized();
        let cam_right = cam_forward.cross(ship_up).normalized();
        let cam_up = cam_right.cross(cam_forward);
        let scene = Scene {
            view_matrix: camera.get_view_matrix(),
            projection_matrix,
            viewport_matrix,
            eye: camera.eye,
            cam_forward,
            cam_right,
            cam_up,
            time: elapsed_time,
        };
        framebuffer.clear();
        draw_stars(&mut framebuffer, &scene);
        let light = Light::new(Vector3::zero());
        let mut stats = RenderStats::default();

        // Sol. En la explosión se dibuja como una onda expansiva blanca que crece.
        let (sun_draw_radius, sun_draw_heat) = if shockwave_radius > 0.0 {
            (shockwave_radius, 1.0)
        } else {
            (sun_radius, sun_heat)
        };
        draw_body(
            &mut framebuffer,
            &mut stats,
            &scene,
            &toggles,
            &light,
            &sun_lods,
            &ObjectParams {
                center: Vector3::zero(),
                radius: sun_draw_radius,
                rotation: Vector3::new(0.0, elapsed_time * 0.05, 0.0),
                color: Vector3::new(1.0, 0.55, 0.08),
                shader: ShaderKind::Sun,
                normal_map: None,
                style: StyleParams::toon(0.0),
                heat: sun_draw_heat,
            },
            sun_draw_radius,
        );

        // Planetas (malla deformada + mapa de normales) y sus modelos de superficie.
        for (i, (planet, runtime)) in PLANETS.iter().zip(runtimes.iter()).enumerate() {
            if consumed[i] {
                continue; // ya cayó en el Sol
            }
            let center = planet_position(planet, elapsed_time, collapse);
            let rotation = planet_rotation(planet, elapsed_time, collapse);
            let drawn = draw_body(
                &mut framebuffer,
                &mut stats,
                &scene,
                &toggles,
                &light,
                &runtime.lods,
                &ObjectParams {
                    center,
                    radius: planet.radius,
                    rotation,
                    color: Vector3::new(planet.color[0], planet.color[1], planet.color[2]),
                    shader: planet.shader,
                    normal_map: runtime.normal_map.as_ref(),
                    style: planet.style.params(),
                    heat: 0.0,
                },
                runtime.max_radius,
            );
            let near = (center - scene.eye).length() < planet.radius * PROP_DRAW_RANGE;
            if drawn && toggles.props && near {
                draw_props(
                    &mut framebuffer,
                    &mut stats,
                    &scene,
                    &toggles,
                    &light,
                    planet,
                    runtime,
                    center,
                    rotation,
                );
            }
        }

        // Chispas de los impactos y de la explosión (icosaedros pequeños emisivos).
        for particle in particles.iter() {
            let visible = !toggles.culling || sphere_visible(&scene, particle.position, particle.size);
            if !visible {
                continue;
            }
            let fade = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let size = particle.size * (0.3 + 0.7 * fade);
            let params = ObjectParams {
                center: particle.position,
                radius: size,
                rotation: Vector3::zero(),
                color: Vector3::new(1.0, 0.55, 0.08),
                shader: ShaderKind::Sun,
                normal_map: None,
                style: StyleParams::toon(0.0),
                heat: particle.heat * fade,
            };
            let model = create_model_matrix(particle.position, size, Vector3::zero());
            render_with_model(&mut framebuffer, &mut stats, &spark_mesh, &light, &scene, model, &params);
        }

        // Nave: el model matrix se arma con la base de la nave (yaw + pitch). Con
        // SHIP_YAW_OFFSET = 0 el eje -Z del modelo apunta a forward; el offset gira el
        // modelo sobre su propio eje Y.
        let (sin_o, cos_o) = SHIP_YAW_OFFSET.sin_cos();
        let axis_x = right * cos_o + forward * sin_o; // eje X del modelo
        let axis_y = ship_up; // eje Y del modelo
        let axis_z = right * sin_o - forward * cos_o; // eje Z del modelo
        let s = SHIP_SCALE;
        let ship_model = new_matrix4(
            axis_x.x * s, axis_y.x * s, axis_z.x * s, ship_position.x,
            axis_x.y * s, axis_y.y * s, axis_z.y * s, ship_position.y,
            axis_x.z * s, axis_y.z * s, axis_z.z * s, ship_position.z,
            0.0, 0.0, 0.0, 1.0,
        );
        for part in ship_parts.iter() {
            let (color, shader) = ship_part_look(&part.name);
            render_with_model(
                &mut framebuffer,
                &mut stats,
                &part.vertices,
                &light,
                &scene,
                ship_model,
                &ObjectParams {
                    center: ship_position,
                    radius: SHIP_SCALE,
                    rotation: Vector3::zero(),
                    color,
                    shader,
                    normal_map: None,
                    style: StyleParams::toon(4.0),
                    heat: 0.0,
                },
            );
        }

        // HUD con contadores: sirve para demostrar el efecto del descarte y del LOD.
        let overlay: Vec<String> = if toggles.hud {
            let on = |b: bool| if b { "ON " } else { "OFF" };
            let scenario = match phase {
                ScenarioPhase::Normal => "[K] iniciar el colapso del universo".to_string(),
                ScenarioPhase::Collapse => format!(
                    "COLAPSO: todo cae hacia el Sol ({:.0} s)   [K] cancelar",
                    (COLLAPSE_SECONDS - phase_timer).max(0.0)
                ),
                ScenarioPhase::Explosion => "EXPLOSION".to_string(),
                ScenarioPhase::Reborn => "El universo renace...".to_string(),
            };
            vec![
                format!("FPS {}   velocidad {:.1}", window.get_fps(), ship_velocity.length()),
                format!("Cuerpos: {} dibujados / {} descartados", stats.drawn, stats.culled),
                format!("Modelos de superficie: {} dibujados / {} descartados", stats.props_drawn, stats.props_culled),
                format!("Triangulos procesados: {}", stats.triangles),
                format!(
                    "[C] descarte {}  [L] LOD {}  [P] modelos {}  [H] ocultar",
                    on(toggles.culling),
                    on(toggles.lod),
                    on(toggles.props)
                ),
                scenario,
            ]
        } else {
            Vec::new()
        };

        framebuffer.swap_buffers(&mut window, &thread, &overlay, flash);
    }
}
