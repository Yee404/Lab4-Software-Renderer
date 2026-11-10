use crate::fragment::Fragment;
use crate::vertex::Vertex;
use crate::light::Light;
use raylib::prelude::Vector3;

/// Calculate barycentric coordinates for a point P with respect to triangle (A, B, C)
/// Returns (w1, w2, w3) where P = w1*A + w2*B + w3*C
/// If the point is outside the triangle, one or more weights will be negative
fn barycentric_coordinates(p_x: f32, p_y: f32, a: &Vertex, b: &Vertex, c: &Vertex) -> (f32, f32, f32) {
    let a_x = a.transformed_position.x;
    let a_y = a.transformed_position.y;
    let b_x = b.transformed_position.x;
    let b_y = b.transformed_position.y;
    let c_x = c.transformed_position.x;
    let c_y = c.transformed_position.y;

    // Calculate the area of the parallelogram formed by vectors AB and AC
    let denom = (b_y - c_y) * (a_x - c_x) + (c_x - b_x) * (a_y - c_y);

    // Avoid division by zero for degenerate triangles
    if denom.abs() < 1e-10 {
        return (-1.0, -1.0, -1.0);
    }

    // Calculate barycentric coordinates
    let w1 = ((b_y - c_y) * (p_x - c_x) + (c_x - b_x) * (p_y - c_y)) / denom;
    let w2 = ((c_y - a_y) * (p_x - c_x) + (a_x - c_x) * (p_y - c_y)) / denom;
    let w3 = 1.0 - w1 - w2;

    (w1, w2, w3)
}

pub fn triangle(v1: &Vertex, v2: &Vertex, v3: &Vertex, light: &Light) -> Vec<Fragment> {
    let mut fragments = Vec::new();

    // === DEMO: Uncomment to show RGB color interpolation ===
    // Assign RGB colors to the three vertices for interpolation demonstration
    // let color1 = Vector3::new(1.0, 0.0, 0.0); // Red
    // let color2 = Vector3::new(0.0, 0.0, 1.0); // Blue
    // let color3 = Vector3::new(0.0, 1.0, 0.0); // Green

    // Base gray color for all vertices
    let base_color = Vector3::new(0.15, 0.75, 0.95);

    // Get the bounding box of the triangle
    let min_x = v1.transformed_position.x.min(v2.transformed_position.x).min(v3.transformed_position.x).floor() as i32;
    let max_x = v1.transformed_position.x.max(v2.transformed_position.x).max(v3.transformed_position.x).ceil() as i32;
    let min_y = v1.transformed_position.y.min(v2.transformed_position.y).min(v3.transformed_position.y).floor() as i32;
    let max_y = v1.transformed_position.y.max(v2.transformed_position.y).max(v3.transformed_position.y).ceil() as i32;

    // Iterate over each pixel in the bounding box
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p_x = x as f32 + 0.5; // Sample at pixel center
            let p_y = y as f32 + 0.5;

            // Calculate barycentric coordinates
            let (w1, w2, w3) = barycentric_coordinates(p_x, p_y, v1, v2, v3);

            // Check if point is inside the triangle
            if w1 >= 0.0 && w2 >= 0.0 && w3 >= 0.0 {
                // === DEMO: Uncomment to show RGB color interpolation ===
                // Interpolate color using barycentric coordinates
                // let interpolated_color = Vector3::new(
                //     w1 * color1.x + w2 * color2.x + w3 * color3.x,
                //     w1 * color1.y + w2 * color2.y + w3 * color3.y,
                //     w1 * color1.z + w2 * color2.z + w3 * color3.z,
                // );

                // Interpolate normals using barycentric coordinates
                let interpolated_normal = Vector3::new(
                    w1 * v1.normal.x + w2 * v2.normal.x + w3 * v3.normal.x,
                    w1 * v1.normal.y + w2 * v2.normal.y + w3 * v3.normal.y,
                    w1 * v1.normal.z + w2 * v2.normal.z + w3 * v3.normal.z,
                );

                // Normalize the interpolated normal
                let normal_length = (interpolated_normal.x * interpolated_normal.x
                                   + interpolated_normal.y * interpolated_normal.y
                                   + interpolated_normal.z * interpolated_normal.z).sqrt();
                let mut normalized_normal = interpolated_normal;
                if normal_length > 0.0 {
                    normalized_normal.x /= normal_length;
                    normalized_normal.y /= normal_length;
                    normalized_normal.z /= normal_length;
                }

                // Interpolate position in world space for this fragment
                let world_pos = Vector3::new(
                    w1 * v1.position.x + w2 * v2.position.x + w3 * v3.position.x,
                    w1 * v1.position.y + w2 * v2.position.y + w3 * v3.position.y,
                    w1 * v1.position.z + w2 * v2.position.z + w3 * v3.position.z,
                );

                // Light direction (from surface to light) for this fragment
                let mut light_dir = Vector3::new(
                    light.position.x - world_pos.x,
                    light.position.y - world_pos.y,
                    light.position.z - world_pos.z,
                );

                // Normalize light direction
                let light_length = (light_dir.x * light_dir.x + light_dir.y * light_dir.y + light_dir.z * light_dir.z).sqrt();
                if light_length > 0.0 {
                    light_dir.x /= light_length;
                    light_dir.y /= light_length;
                    light_dir.z /= light_length;
                }

                // Calculate per-fragment lighting intensity using interpolated normal (Lambertian shading)
                let intensity = (normalized_normal.x * light_dir.x
                               + normalized_normal.y * light_dir.y
                               + normalized_normal.z * light_dir.z).clamp(0.2, 1.0);

                // Apply shading to base color
                let shaded_color = Vector3::new(
                    base_color.x * intensity,
                    base_color.y * intensity,
                    base_color.z * intensity,
                );

                // Interpolate depth using barycentric coordinates
                let depth = w1 * v1.transformed_position.z
                          + w2 * v2.transformed_position.z
                          + w3 * v3.transformed_position.z;

                fragments.push(Fragment::new_with_world_pos(p_x, p_y, shaded_color, depth, world_pos));
            }
        }
    }

    fragments
}
