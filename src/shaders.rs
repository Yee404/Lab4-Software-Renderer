use raylib::prelude::*;
use crate::vertex::Vertex;
use crate::fragment::Fragment;
use crate::{ShaderKind, Uniforms};
use noise::{NoiseFn, OpenSimplex};

// This function manually multiplies a 4x4 matrix with a 4D vector (in homogeneous coordinates)
fn multiply_matrix_vector4(matrix: &Matrix, vector: &Vector4) -> Vector4 {
    Vector4::new(
        matrix.m0 * vector.x + matrix.m4 * vector.y + matrix.m8 * vector.z + matrix.m12 * vector.w,
        matrix.m1 * vector.x + matrix.m5 * vector.y + matrix.m9 * vector.z + matrix.m13 * vector.w,
        matrix.m2 * vector.x + matrix.m6 * vector.y + matrix.m10 * vector.z + matrix.m14 * vector.w,
        matrix.m3 * vector.x + matrix.m7 * vector.y + matrix.m11 * vector.z + matrix.m15 * vector.w,
    )
}

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
  // Convert vertex position to homogeneous coordinates (Vec4) by adding a w-component of 1.0
  let position_vec4 = Vector4::new(
    vertex.position.x,
    vertex.position.y,
    vertex.position.z,
    1.0
  );

  // Apply Model transformation
  let world_position = multiply_matrix_vector4(&uniforms.model_matrix, &position_vec4);

  // Normals use w = 0 so translation has no effect. Transforming them with the
  // model rotation keeps the illuminated side facing the Sun while objects rotate.
  let normal_vec4 = Vector4::new(vertex.normal.x, vertex.normal.y, vertex.normal.z, 0.0);
  let transformed_normal4 = multiply_matrix_vector4(&uniforms.model_matrix, &normal_vec4);
  let normal_length = (
      transformed_normal4.x * transformed_normal4.x
      + transformed_normal4.y * transformed_normal4.y
      + transformed_normal4.z * transformed_normal4.z
  ).sqrt();
  let world_normal = if normal_length > 0.0 {
      Vector3::new(
          transformed_normal4.x / normal_length,
          transformed_normal4.y / normal_length,
          transformed_normal4.z / normal_length,
      )
  } else {
      vertex.normal
  };

  // Apply View transformation (camera)
  let view_position = multiply_matrix_vector4(&uniforms.view_matrix, &world_position);

  // Apply Projection transformation (perspective)
  let clip_position = multiply_matrix_vector4(&uniforms.projection_matrix, &view_position);

  // Perform perspective division to get NDC (Normalized Device Coordinates)
  let ndc = if clip_position.w > 0.05 {
      Vector3::new(
          clip_position.x / clip_position.w,
          clip_position.y / clip_position.w,
          clip_position.z / clip_position.w,
      )
  } else {
      Vector3::new(f32::NAN, f32::NAN, f32::NAN)
  };

  // Apply Viewport transformation to get screen coordinates
  let ndc_vec4 = Vector4::new(ndc.x, ndc.y, ndc.z, 1.0);
  let screen_position = multiply_matrix_vector4(&uniforms.viewport_matrix, &ndc_vec4);

  let transformed_position = Vector3::new(
      screen_position.x,
      screen_position.y,
      screen_position.z,
  );

  // Create a new Vertex with the transformed position
  Vertex {
    position: Vector3::new(world_position.x, world_position.y, world_position.z),
    normal: world_normal,
    tex_coords: vertex.tex_coords,
    color: vertex.color,
    transformed_position,
    transformed_normal: world_normal,
  }
}

// === Animated Fragment Shader Examples ===

/// Example 1: Random flickering colors per fragment
#[allow(dead_code)]
fn shader_random_flicker(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let base_color = fragment.color;

    // Create pseudo-random values based on position and time
    let seed = world_pos.x * 12.9898 + world_pos.y * 78.233 + world_pos.z * 45.164 + time * 3.0;
    let random = (seed.sin() * 43758.5453).fract();

    let flicker_color = Vector3::new(
        (random * 7.0).sin() * 0.5 + 0.5,
        (random * 11.0).cos() * 0.5 + 0.5,
        (random * 13.0).sin() * 0.5 + 0.5,
    );

    // Mix with base lighting
    Vector3::new(
        base_color.x * 0.5 + flicker_color.x * 0.5,
        base_color.y * 0.5 + flicker_color.y * 0.5,
        base_color.z * 0.5 + flicker_color.z * 0.5,
    )
}

/// Example 2: Horizontal stripes moving upward
#[allow(dead_code)]
fn shader_moving_stripes(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let base_color = fragment.color;

    // Add time to Y position to make stripes move upward
    let stripe_frequency = 1.0;
    let animated_y = world_pos.y + time * 0.5; // Speed of movement
    let stripe = ((animated_y * stripe_frequency).floor() % 2.0).abs();

    let stripe_color1 = Vector3::new(1.0, 0.3, 0.1); // Orange
    let stripe_color2 = Vector3::new(0.1, 0.3, 1.0); // Blue

    let stripe_color = Vector3::new(
        stripe_color1.x * stripe + stripe_color2.x * (1.0 - stripe),
        stripe_color1.y * stripe + stripe_color2.y * (1.0 - stripe),
        stripe_color1.z * stripe + stripe_color2.z * (1.0 - stripe),
    );

    Vector3::new(
        base_color.x * stripe_color.x,
        base_color.y * stripe_color.y,
        base_color.z * stripe_color.z,
    )
}

/// Example 3: Pulsing color waves
#[allow(dead_code)]
fn shader_pulsing_waves(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let base_color = fragment.color;

    // Animated sine waves that pulse over time
    let wave1 = (world_pos.x * 3.0 + time * 2.0).sin() * 0.5 + 0.5;
    let wave2 = (world_pos.y * 3.0 + time * 1.5).cos() * 0.5 + 0.5;
    let wave3 = (world_pos.z * 3.0 + time * 2.5).sin() * 0.5 + 0.5;

    let wave_color = Vector3::new(wave1, wave2, wave3);

    Vector3::new(
        base_color.x * 0.6 + wave_color.x * 0.4,
        base_color.y * 0.6 + wave_color.y * 0.4,
        base_color.z * 0.6 + wave_color.z * 0.4,
    )
}

/// Example 4: Rotating rainbow gradient
#[allow(dead_code)]
fn shader_rotating_rainbow(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let base_color = fragment.color;

    // Create rotating rainbow effect
    let angle = world_pos.x.atan2(world_pos.z) + time;
    let hue = (angle / (2.0 * 3.14159)) % 1.0;

    // Convert hue to RGB (simplified HSV to RGB)
    let rainbow_color = Vector3::new(
        ((hue * 6.0).sin()).abs(),
        ((hue * 6.0 + 2.0).sin()).abs(),
        ((hue * 6.0 + 4.0).sin()).abs(),
    );

    Vector3::new(
        base_color.x * 0.5 + rainbow_color.x * 0.5,
        base_color.y * 0.5 + rainbow_color.y * 0.5,
        base_color.z * 0.5 + rainbow_color.z * 0.5,
    )
}

/// Example 5: Expanding rings from origin
#[allow(dead_code)]
fn shader_expanding_rings(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let base_color = fragment.color;

    // Distance from origin
    let distance = (world_pos.x * world_pos.x + world_pos.y * world_pos.y + world_pos.z * world_pos.z).sqrt();

    // Animated rings expanding outward
    let ring = (distance * 2.0 - time * 2.0).sin() * 0.5 + 0.5;

    let ring_color = Vector3::new(ring, 1.0 - ring, ring * 0.5);

    Vector3::new(
        base_color.x * 0.5 + ring_color.x * 0.5,
        base_color.y * 0.5 + ring_color.y * 0.5,
        base_color.z * 0.5 + ring_color.z * 0.5,
    )
}

/// Example 6: Breathing/pulsing color intensity
#[allow(dead_code)]
fn shader_breathing(fragment: &Fragment, time: f32) -> Vector3 {
    let base_color = fragment.color;

    // Pulse intensity over time
    let pulse = (time * 2.0).sin() * 0.3 + 0.7; // Range: 0.4 to 1.0

    Vector3::new(
        base_color.x * pulse,
        base_color.y * pulse,
        base_color.z * pulse,
    )
}

/// Example 7: Just pass through the base color (standard lighting only)
#[allow(dead_code)]
fn shader_base_color(fragment: &Fragment, _time: f32) -> Vector3 {
    fragment.color
}

/// Example 8: Cloud-like noise pattern using OpenSimplex noise
#[allow(dead_code)]
fn shader_clouds(fragment: &Fragment, _time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let base_color = fragment.color;

    // Create a noise generator (OpenSimplex)
    let noise = OpenSimplex::new(42); // Seed value for consistent results

    // Sample 3D noise at the fragment's world position
    // Scale the position to control noise frequency
    let noise_scale = 2.0;
    let noise_value = noise.get([
        (world_pos.x * noise_scale) as f64,
        (world_pos.y * noise_scale) as f64,
        (world_pos.z * noise_scale) as f64,
    ]);

    // Noise returns values in range [-1, 1], normalize to [0, 1]
    let normalized_noise = (noise_value as f32 + 1.0) / 2.0;

    // Add multiple octaves of noise for more detail (fractal noise)
    let octave1 = noise.get([
        (world_pos.x * noise_scale * 2.0) as f64,
        (world_pos.y * noise_scale * 2.0) as f64,
        (world_pos.z * noise_scale * 2.0) as f64,
    ]) as f32;

    let octave2 = noise.get([
        (world_pos.x * noise_scale * 4.0) as f64,
        (world_pos.y * noise_scale * 4.0) as f64,
        (world_pos.z * noise_scale * 4.0) as f64,
    ]) as f32;

    // Combine octaves (fractal brownian motion)
    let fbm = normalized_noise + octave1 * 0.5 + octave2 * 0.25;
    let cloud_density = (fbm / 1.75).clamp(0.0, 1.0);

    // Create cloud colors (white to light blue/gray)
    let cloud_color = Vector3::new(
        0.8 + cloud_density * 0.2,  // R: White to brighter
        0.85 + cloud_density * 0.15, // G: Slightly blue-ish
        0.9 + cloud_density * 0.1,   // B: Light blue tint
    );

    // Blend with base lighting
    Vector3::new(
        base_color.x * cloud_color.x,
        base_color.y * cloud_color.y,
        base_color.z * cloud_color.z,
    )
}

// === Multi-Pass Shader Components (Base Layers) ===

/// Base Pass 1: Water gradient (vertical blue gradient)
#[allow(dead_code)]
fn pass_water_gradient(fragment: &Fragment, _time: f32) -> Vector3 {
    let world_pos = fragment.world_position;

    // Create a vertical gradient based on Y position
    let gradient = ((world_pos.y + 1.0) / 2.0).clamp(0.0, 1.0);

    // Water colors: deep blue to cyan
    let deep_water = Vector3::new(0.0, 0.1, 0.4);   // Deep blue
    let shallow_water = Vector3::new(0.2, 0.6, 0.8); // Cyan

    Vector3::new(
        deep_water.x + (shallow_water.x - deep_water.x) * gradient,
        deep_water.y + (shallow_water.y - deep_water.y) * gradient,
        deep_water.z + (shallow_water.z - deep_water.z) * gradient,
    )
}

/// Overlay Pass 1: Animated horizontal stripes
#[allow(dead_code)]
fn pass_animated_stripes(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;

    // Animated stripes moving upward
    let stripe_frequency = 3.0;
    let animated_y = world_pos.y + time * 0.5;
    let stripe = (animated_y * stripe_frequency).sin() * 0.5 + 0.5;

    // Return stripe intensity (0 = transparent, 1 = opaque)
    Vector3::new(stripe, stripe, stripe)
}

/// Base Pass 2: Sunset gradient
#[allow(dead_code)]
fn pass_sunset_gradient(fragment: &Fragment, _time: f32) -> Vector3 {
    let world_pos = fragment.world_position;

    // Vertical gradient for sunset
    let gradient = ((world_pos.y + 1.0) / 2.0).clamp(0.0, 1.0);

    // Sunset colors: orange to purple
    let bottom_color = Vector3::new(0.8, 0.3, 0.1); // Orange
    let top_color = Vector3::new(0.2, 0.1, 0.4);    // Purple

    Vector3::new(
        bottom_color.x + (top_color.x - bottom_color.x) * gradient,
        bottom_color.y + (top_color.y - bottom_color.y) * gradient,
        bottom_color.z + (top_color.z - bottom_color.z) * gradient,
    )
}

/// Overlay Pass 2: Pulsing wave pattern
#[allow(dead_code)]
fn pass_pulsing_waves(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;

    // Wavy pattern that pulses
    let wave = (world_pos.x * 4.0 + world_pos.z * 4.0 + time * 2.0).sin() * 0.5 + 0.5;

    Vector3::new(wave, wave, wave)
}

/// Base Pass 3: Simple noise texture
#[allow(dead_code)]
fn pass_noise_base(fragment: &Fragment, _time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let noise = OpenSimplex::new(42);

    let noise_value = noise.get([
        (world_pos.x * 2.0) as f64,
        (world_pos.y * 2.0) as f64,
        (world_pos.z * 2.0) as f64,
    ]);

    let normalized = (noise_value as f32 + 1.0) / 2.0;

    // Grayscale noise
    Vector3::new(normalized, normalized, normalized)
}

/// Overlay Pass 3: Animated color shift
#[allow(dead_code)]
fn pass_color_shift(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;

    // Shifting colors based on position and time
    let r = (world_pos.x + time).sin() * 0.5 + 0.5;
    let g = (world_pos.y + time * 1.3).cos() * 0.5 + 0.5;
    let b = (world_pos.z + time * 0.7).sin() * 0.5 + 0.5;

    Vector3::new(r, g, b)
}

// === Multi-Pass Shader Combinations ===

/// Example: Water with animated stripes
#[allow(dead_code)]
fn shader_water_with_stripes(fragment: &Fragment, time: f32) -> Vector3 {
    let base_color = fragment.color;

    // Pass 1: Get water gradient
    let water = pass_water_gradient(fragment, time);

    // Pass 2: Get stripe overlay
    let stripes = pass_animated_stripes(fragment, time);

    // Combine: Use stripes as a mask (additive blend)
    let combined = Vector3::new(
        water.x + stripes.x * 0.3,
        water.y + stripes.y * 0.3,
        water.z + stripes.z * 0.3,
    );

    // Apply base lighting
    Vector3::new(
        combined.x * base_color.x,
        combined.y * base_color.y,
        combined.z * base_color.z,
    )
}

/// Example: Sunset with pulsing waves
#[allow(dead_code)]
fn shader_sunset_waves(fragment: &Fragment, time: f32) -> Vector3 {
    let base_color = fragment.color;

    // Pass 1: Sunset gradient base
    let sunset = pass_sunset_gradient(fragment, time);

    // Pass 2: Pulsing waves overlay
    let waves = pass_pulsing_waves(fragment, time);

    // Combine: Multiply blend for waves
    let combined = Vector3::new(
        sunset.x * (0.7 + waves.x * 0.3),
        sunset.y * (0.7 + waves.y * 0.3),
        sunset.z * (0.7 + waves.z * 0.3),
    );

    // Apply base lighting
    Vector3::new(
        combined.x * base_color.x,
        combined.y * base_color.y,
        combined.z * base_color.z,
    )
}

/// Example: Noise with color shifting
#[allow(dead_code)]
fn shader_noise_colorshift(fragment: &Fragment, time: f32) -> Vector3 {
    let base_color = fragment.color;

    // Pass 1: Noise texture base
    let noise = pass_noise_base(fragment, time);

    // Pass 2: Color shift overlay
    let colors = pass_color_shift(fragment, time);

    // Combine: Multiply the noise with the colors
    let combined = Vector3::new(
        noise.x * colors.x,
        noise.y * colors.y,
        noise.z * colors.z,
    );

    // Apply base lighting
    Vector3::new(
        combined.x * base_color.x,
        combined.y * base_color.y,
        combined.z * base_color.z,
    )
}

/// Example 9: Animated cloud-like noise that moves over time
#[allow(dead_code)]
fn shader_animated_clouds(fragment: &Fragment, time: f32) -> Vector3 {
    let world_pos = fragment.world_position;
    let base_color = fragment.color;

    let noise = OpenSimplex::new(42);

    // Animate by offsetting the noise sampling position with time
    let noise_scale = 1.5;
    let time_offset = time * 0.3; // Speed of cloud movement

    let noise_value = noise.get([
        (world_pos.x * noise_scale + time_offset) as f64,
        (world_pos.y * noise_scale) as f64,
        (world_pos.z * noise_scale + time_offset) as f64,
    ]);

    let normalized_noise = (noise_value as f32 + 1.0) / 2.0;

    // Add octaves for detail
    let octave1 = noise.get([
        (world_pos.x * noise_scale * 2.0 + time_offset * 1.5) as f64,
        (world_pos.y * noise_scale * 2.0) as f64,
        (world_pos.z * noise_scale * 2.0 + time_offset * 1.5) as f64,
    ]) as f32;
    let normalized_octave1 = (octave1 + 1.0) / 2.0;

    let octave2 = noise.get([
        (world_pos.x * noise_scale * 4.0 + time_offset * 2.0) as f64,
        (world_pos.y * noise_scale * 4.0) as f64,
        (world_pos.z * noise_scale * 4.0 + time_offset * 2.0) as f64,
    ]) as f32;
    let normalized_octave2 = (octave2 + 1.0) / 2.0;

    // Combine octaves (fractal brownian motion)
    let fbm = normalized_noise * 1.0 + normalized_octave1 * 0.5 + normalized_octave2 * 0.25;
    let cloud_value = (fbm / 1.75).clamp(0.0, 1.0);

    // More pronounced cloud colors - darker to lighter
    let cloud_color = Vector3::new(
        0.3 + cloud_value * 0.7,  // R: Dark to bright
        0.4 + cloud_value * 0.6,  // G: Slightly more green
        0.6 + cloud_value * 0.4,  // B: Blue-ish base
    );

    // Mix cloud pattern with base lighting
    Vector3::new(
        base_color.x * 0.5 + cloud_color.x * 0.5,
        base_color.y * 0.5 + cloud_color.y * 0.5,
        base_color.z * 0.5 + cloud_color.z * 0.5,
    )
}

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

/// Posición del fragmento en el sistema del propio cuerpo (esfera de radio ~1).
/// Se deshace la rotación del model matrix (X·Y·Z), de modo que los patrones del
/// shader giran junto con el planeta y la rotación sobre su eje se nota.
fn body_coords(p: Vector3, uniforms: &Uniforms) -> Vector3 {
    let c = uniforms.object_center;
    let r = uniforms.object_radius;
    let rot = uniforms.object_rotation;
    let q = Vector3::new((p.x - c.x) / r, (p.y - c.y) / r, (p.z - c.z) / r);
    rotate_z(rotate_y(rotate_x(q, -rot.x), -rot.y), -rot.z)
}

// === Main Fragment Shader ===
//
// Cada planeta usa una técnica distinta (no solo otro color):
//   Terran  → biomas por altura (mar, playa, pasto, bosque, roca, nieve) + capa de nubes que se desplaza.
//   Desierto→ estratos de arena con ruido "ridged" y crestas de dunas.
//   Hielo   → placas de Voronoi con grietas y destellos animados.
//   Gas     → bandas turbulentas animadas con una tormenta en espiral.
//   Lava    → corteza de Voronoi con grietas incandescentes que laten (emisivas).
// Después se aplica el estilo del objeto: escalones de luz (cel shading), semitono y halo.

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rgb(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3::new(r, g, b)
}

fn mix(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    Vector3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

fn scale(v: Vector3, k: f32) -> Vector3 {
    Vector3::new(v.x * k, v.y * k, v.z * k)
}

fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn rotate_forward(v: Vector3, rot: Vector3) -> Vector3 {
    rotate_x(rotate_y(rotate_z(v, rot.z), rot.y), rot.x)
}

/// Coordenadas de mapa (u, v) de una dirección unitaria del cuerpo.
fn map_uv(dir: Vector3) -> (f32, f32) {
    (
        dir.z.atan2(dir.x) / (2.0 * std::f32::consts::PI) + 0.5,
        dir.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI + 0.5,
    )
}

/// Intensidad de luz del fragmento. Si el objeto tiene mapa de normales, la normal se
/// perturba con las pendientes del mapa (espacio tangente de la esfera: este = longitud
/// creciente, norte = latitud creciente, llevado a mundo con la rotación del planeta).
fn lit_intensity(fragment: &Fragment, uniforms: &Uniforms, dir: Vector3) -> f32 {
    let Some(map) = uniforms.normal_map else {
        return fragment.intensity;
    };
    let east_len = (dir.x * dir.x + dir.z * dir.z).sqrt();
    if east_len < 1e-3 {
        return fragment.intensity; // en los polos la base tangente no está definida
    }
    let east = Vector3::new(-dir.z / east_len, 0.0, dir.x / east_len);
    let north = east.cross(dir);
    let (u, v) = map_uv(dir);
    let (slope_u, slope_v) = map.sample(u, v);

    let rot = uniforms.object_rotation;
    let tangent = rotate_forward(east, rot);
    let bitangent = rotate_forward(north, rot);
    let n = fragment.normal;
    let k = map.strength;
    let perturbed = Vector3::new(
        n.x - (tangent.x * slope_u + bitangent.x * slope_v) * k,
        n.y - (tangent.y * slope_u + bitangent.y * slope_v) * k,
        n.z - (tangent.z * slope_u + bitangent.z * slope_v) * k,
    );
    let plen = perturbed.length();
    if plen < 1e-6 {
        return fragment.intensity;
    }
    let l = fragment.light_dir;
    ((perturbed.x * l.x + perturbed.y * l.y + perturbed.z * l.z) / plen).clamp(0.38, 1.0)
}

/// Cel shading: cuantiza la luz en escalones. En el escalón más oscuro puede añadir
/// puntos de semitono (estilo cómic / Spider-Verse) en una rejilla a 45° en pantalla.
fn stylize(intensity: f32, fragment: &Fragment, uniforms: &Uniforms) -> f32 {
    let s = uniforms.style;
    if s.levels < 1.0 {
        return intensity;
    }
    let t = ((intensity - 0.38) / 0.62).clamp(0.0, 1.0);
    let band = (t * s.levels).floor().min(s.levels - 1.0);
    let level = band / (s.levels - 1.0).max(1.0);
    let mut out = 0.40 + 0.60 * level;

    if band == 0.0 && s.halftone > 0.0 {
        let (px, py) = (fragment.position.x, fragment.position.y);
        let a = (px + py) * 0.7071;
        let b = (px - py) * 0.7071;
        let cell = 4.0;
        let du = a.rem_euclid(cell) - cell * 0.5;
        let dv = b.rem_euclid(cell) - cell * 0.5;
        if du * du + dv * dv < 1.7 * 1.7 {
            out *= 1.0 - s.halftone * 0.55;
        }
    }
    out
}

fn terran_albedo(map: &crate::normal_map::NormalMap, u: f32, v: f32, dir: Vector3, time: f32) -> Vector3 {
    let h = map.height(u, v) * 1.5;
    let sea = -0.05;
    let mut c = if h < sea {
        mix(rgb(0.03, 0.14, 0.45), rgb(0.12, 0.62, 0.80), smoothstep(-0.7, sea, h))
    } else if h < 0.04 {
        rgb(0.88, 0.82, 0.58) // playa
    } else if h < 0.30 {
        mix(rgb(0.32, 0.72, 0.30), rgb(0.20, 0.55, 0.26), smoothstep(0.04, 0.30, h))
    } else if h < 0.50 {
        rgb(0.13, 0.40, 0.22) // bosque
    } else if h < 0.65 {
        rgb(0.50, 0.46, 0.42) // roca
    } else {
        rgb(0.95, 0.97, 1.0) // nieve
    };
    // Casquetes polares
    c = mix(c, rgb(0.95, 0.98, 1.0), smoothstep(0.80, 0.92, dir.y.abs()));
    // Nubes: el patrón horneado se desplaza con el tiempo
    let cloud = smoothstep(0.55, 0.72, map.detail(u + time * 0.012, v));
    mix(c, rgb(0.98, 0.99, 1.0), cloud * 0.85)
}

fn desert_albedo(map: &crate::normal_map::NormalMap, u: f32, v: f32, dir: Vector3) -> Vector3 {
    let h = map.height(u, v) * 1.5;
    let ridge = map.detail(u, v).clamp(0.0, 1.0);
    let strata = (h * 10.0 + ridge * 2.5 + dir.y * 3.0).sin() * 0.5 + 0.5;
    let sand = rgb(0.93, 0.78, 0.50);
    let orange = rgb(0.84, 0.48, 0.20);
    let red = rgb(0.52, 0.22, 0.14);
    let mut c = mix(sand, orange, strata);
    c = mix(c, red, smoothstep(0.1, 0.6, -h));
    // Crestas de duna iluminadas
    mix(c, scale(sand, 1.08), smoothstep(0.70, 0.95, ridge) * 0.6)
}

/// Devuelve (albedo, destello). El destello se suma después de la luz.
fn ice_albedo(map: &crate::normal_map::NormalMap, u: f32, v: f32, time: f32) -> (Vector3, f32) {
    let edge = map.detail(u, v);
    let h = map.height(u, v);
    let crack = 1.0 - smoothstep(0.0, 0.07, edge);
    let mut base = mix(rgb(0.78, 0.92, 1.0), rgb(0.55, 0.78, 0.97), smoothstep(-0.4, 0.4, h));
    base = mix(base, rgb(1.0, 1.0, 1.0), smoothstep(0.25, 0.60, h) * 0.7);
    let c = mix(base, rgb(0.10, 0.28, 0.62), crack * 0.9);
    let sparkle = ((time * 2.0 + h * 25.0).sin() * 0.5 + 0.5).powi(12) * crack * 0.8;
    (c, sparkle)
}

fn gas_albedo(dir: Vector3, time: f32) -> Vector3 {
    let turb = (dir.x * 3.0 + time * 0.5).sin() * 0.07 + (dir.z * 2.0 - time * 0.3).sin() * 0.05;
    let lat = dir.y + turb;
    let band = (lat * 11.0).sin() * 0.5 + 0.5;
    let fine = (lat * 27.0).sin() * 0.06;
    let mut c = mix(rgb(0.70, 0.42, 0.25), rgb(0.95, 0.84, 0.62), (band + fine).clamp(0.0, 1.0));

    // Gran tormenta: óvalo con espiral, fija en el planeta (gira con él)
    let (lat0, lon0) = (-0.30_f32, 0.60_f32);
    let storm_dir = Vector3::new(lat0.cos() * lon0.cos(), lat0.sin(), lat0.cos() * lon0.sin());
    let ang = (dir.x * storm_dir.x + dir.y * storm_dir.y + dir.z * storm_dir.z)
        .clamp(-1.0, 1.0)
        .acos();
    let inside = smoothstep(0.30, 0.20, ang);
    if inside > 0.0 {
        let swirl = (ang * 26.0 - time * 1.2).sin() * 0.5 + 0.5;
        c = mix(c, mix(rgb(0.70, 0.18, 0.12), rgb(0.95, 0.55, 0.40), swirl), inside);
    }
    c
}

/// Devuelve (corteza, brillo emisivo).
fn lava_albedo(map: &crate::normal_map::NormalMap, u: f32, v: f32, time: f32) -> (Vector3, Vector3) {
    let edge = map.detail(u, v);
    let h = map.height(u, v);
    let crust = mix(rgb(0.10, 0.06, 0.06), rgb(0.26, 0.14, 0.10), smoothstep(-0.4, 0.5, h));
    let width = 0.10 + 0.03 * (time * 1.5 + h * 6.0).sin();
    let glow = 1.0 - smoothstep(0.0, width, edge);
    let pulse = 0.85 + 0.15 * (time * 2.0 + h * 8.0).sin();
    let emissive = scale(mix(rgb(1.0, 0.35, 0.05), rgb(1.0, 0.85, 0.25), glow * glow), glow * pulse);
    (crust, emissive)
}

pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let time = uniforms.time;
    let p = fragment.world_position;
    let q = body_coords(p, uniforms);

    // Dirección unitaria del punto en el sistema del cuerpo.
    let qlen = q.length().max(1e-6);
    let dir = Vector3::new(q.x / qlen, q.y / qlen, q.z / qlen);
    let (u, v) = map_uv(dir);

    // El Sol es emisivo: no recibe luz ni estilo.
    if let ShaderKind::Sun = uniforms.shader_kind {
        let pulse = 0.88 + (time * 2.4 + q.y * 3.5).sin() * 0.12;
        let granules = ((q.x * 9.0 + time * 0.8).sin() * (q.z * 8.0 - time * 0.6).sin()) * 0.06;
        let sun = rgb(1.0, 0.48 * pulse + 0.18 + granules, 0.05 * pulse);
        // Calor: hacia 1 se vuelve blanco (colapso y explosión); negativo = enfriándose (rojo).
        let heat = uniforms.heat;
        let hot = if heat >= 0.0 {
            mix(sun, rgb(1.0, 0.97, 0.90), heat.min(1.0))
        } else {
            mix(sun, rgb(0.45, 0.05, 0.02), (-heat).min(1.0))
        };
        return Vector3::new(hot.x.clamp(0.0, 1.0), hot.y.clamp(0.0, 1.0), hot.z.clamp(0.0, 1.0));
    }

    // Luz (con normal perturbada si hay mapa) y su cuantización por escalones.
    let lit = lit_intensity(fragment, uniforms, dir);
    let toon = stylize(lit, fragment, uniforms);
    // Color propio del objeto sin luz (el triángulo ya lo traía multiplicado por la luz).
    let base = scale(fragment.color, 1.0 / fragment.intensity.max(0.01));

    let mut color = match (uniforms.shader_kind, uniforms.normal_map) {
        (ShaderKind::Terran, Some(map)) => scale(terran_albedo(map, u, v, dir, time), toon),
        (ShaderKind::Desert, Some(map)) => scale(desert_albedo(map, u, v, dir), toon),
        (ShaderKind::Ice, Some(map)) => {
            let (albedo, sparkle) = ice_albedo(map, u, v, time);
            add(scale(albedo, toon), rgb(sparkle, sparkle, sparkle))
        }
        (ShaderKind::Lava, Some(map)) => {
            let (crust, emissive) = lava_albedo(map, u, v, time);
            add(scale(crust, toon), emissive)
        }
        (ShaderKind::Gas, _) => scale(gas_albedo(dir, time), toon),
        // Props de superficie: color plano con leve variación.
        (ShaderKind::Prop, _) => {
            let shade = 0.92 + 0.08 * (p.x * 3.0 + p.z * 2.0).sin();
            scale(base, toon * shade)
        }
        // Cristales y luces: brillo animado que se suma a la luz.
        (ShaderKind::Crystal, _) => {
            let shimmer = 0.15 + 0.15 * (time * 2.5 + (p.x + p.y + p.z) * 1.7).sin();
            add(scale(base, toon), rgb(shimmer, shimmer, shimmer))
        }
        // Nave y cualquier otro caso: color de la pieza con la luz estilizada.
        _ => scale(base, toon),
    };

    // Halo de atmósfera / borde: más fuerte en el lado iluminado y en el contorno.
    let style = uniforms.style;
    if style.rim_strength > 0.0 {
        let c = uniforms.object_center;
        let r = Vector3::new(p.x - c.x, p.y - c.y, p.z - c.z);
        let rl = r.length().max(1e-6);
        let radial = Vector3::new(r.x / rl, r.y / rl, r.z / rl);
        let e = uniforms.eye;
        let to_eye = Vector3::new(e.x - p.x, e.y - p.y, e.z - p.z);
        let el = to_eye.length().max(1e-6);
        let view = Vector3::new(to_eye.x / el, to_eye.y / el, to_eye.z / el);
        let fres = (1.0 - (radial.x * view.x + radial.y * view.y + radial.z * view.z).max(0.0)).powf(3.0);
        let l = fragment.light_dir;
        let lit_side = smoothstep(-0.2, 0.5, radial.x * l.x + radial.y * l.y + radial.z * l.z);
        color = add(color, scale(style.rim_color, fres * style.rim_strength * (0.2 + 0.8 * lit_side)));
    }

    Vector3::new(
        color.x.clamp(0.0, 1.0),
        color.y.clamp(0.0, 1.0),
        color.z.clamp(0.0, 1.0),
    )
}
