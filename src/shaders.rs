use raylib::prelude::*;
use crate::vertex::Vertex;
use crate::fragment::Fragment;
use crate::Uniforms;
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

  // Apply View transformation (camera)
  let view_position = multiply_matrix_vector4(&uniforms.view_matrix, &world_position);

  // Apply Projection transformation (perspective)
  let clip_position = multiply_matrix_vector4(&uniforms.projection_matrix, &view_position);

  // Perform perspective division to get NDC (Normalized Device Coordinates)
  let ndc = if clip_position.w != 0.0 {
      Vector3::new(
          clip_position.x / clip_position.w,
          clip_position.y / clip_position.w,
          clip_position.z / clip_position.w,
      )
  } else {
      Vector3::new(clip_position.x, clip_position.y, clip_position.z)
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
    position: vertex.position,
    normal: vertex.normal,
    tex_coords: vertex.tex_coords,
    color: vertex.color,
    transformed_position,
    transformed_normal: vertex.normal, // Note: Correct normal transformation is more complex
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

// === Main Fragment Shader ===
pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let time = uniforms.time;

    // Uncomment one of the shader examples below to see different effects!

    // === Animated Shaders ===
    // shader_random_flicker(fragment, time)
    // shader_moving_stripes(fragment, time)
    // shader_pulsing_waves(fragment, time)
    // shader_rotating_rainbow(fragment, time)
    // shader_expanding_rings(fragment, time)
    // shader_breathing(fragment, time)

    // === Noise-based Shaders ===
    // shader_animated_clouds(fragment, time)  // Clouds that move over time
    // shader_clouds(fragment, time)           // Static cloud pattern

    // === Multi-Pass Shaders (Combining Multiple Effects) ===
    shader_water_with_stripes(fragment, time)  // Water gradient + animated stripes
    // shader_sunset_waves(fragment, time)       // Sunset gradient + pulsing waves
    // shader_noise_colorshift(fragment, time)   // Noise texture + color shifting

    // shader_base_color(fragment, time) // Default: just show the lighting
}