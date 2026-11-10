// main.rs

mod camera;
mod fragment;
mod framebuffer;
mod light;
mod line;
mod matrix;
mod obj;
mod shaders;
mod triangle;
mod vertex;

use crate::camera::Camera;
use crate::light::Light;
use crate::matrix::{create_model_matrix, create_projection_matrix, create_viewport_matrix};
use framebuffer::Framebuffer;
use obj::Obj;
use raylib::prelude::*;
use shaders::{fragment_shader, vertex_shader};
use std::f32::consts::PI;
use std::thread;
use std::time::Duration;
use triangle::triangle;
use vertex::Vertex;

pub struct Uniforms {
    pub model_matrix: Matrix,
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub viewport_matrix: Matrix,
    pub time: f32,
}

fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    light: &Light,
) {
    // Vertex Shader Stage
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    // Log the first 3 transformed vertices for debugging
    // println!("--- Transformed Vertices (first 3) ---");
    // for i in 0..3.min(transformed_vertices.len()) {
    //     println!("Vertex {}: {:?}", i, transformed_vertices[i].transformed_position);
    // }

    // Primitive Assembly Stage
    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            triangles.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    // Rasterization Stage
    let mut fragments = Vec::new();
    for tri in &triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2], light));
    }

    // Fragment Processing Stage
    for fragment in fragments {
        // Run fragment shader to compute final color
        let final_color = fragment_shader(&fragment, uniforms);

        framebuffer.point(
            fragment.position.x as i32,
            fragment.position.y as i32,
            final_color,
            fragment.depth,
        );
    }
}

fn main() {
    let window_width = 800;
    let window_height = 600;

    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Proyecto Final")
        .log_level(TraceLogLevel::LOG_WARNING) // Suppress INFO messages
        .build();

    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);
    framebuffer.set_background_color(Vector3::new(0.2, 0.2, 0.4)); // Dark blue-ish

    // Initialize the texture inside the framebuffer
    framebuffer.init_texture(&mut window, &thread);

    // Camera setup
    let camera_position = Vector3::new(0.0, 2.0, 8.0);
    let camera_target = Vector3::new(0.0, 0.0, 0.0);
    let camera_up = Vector3::new(0.0, 1.0, 0.0);
    let mut camera = Camera::new(camera_position, camera_target, camera_up);

    // Projection setup
    let fov_y = PI / 3.0; // 60 degrees
    let aspect = window_width as f32 / window_height as f32;
    let near = 0.1;
    let far = 100.0;

    // Model setup (rotating model at origin)
    let translation = Vector3::new(0.3, 0.2, 0.0);
    let mut rotation_y = 0.0f32;
    let rotation_speed = 0.005; // Radians per frame
    let scale = 0.25f32;

    // Light setup
    let light = Light::new(Vector3::new(5.0, 5.0, 5.0));

    let obj = Obj::load("assets/models/nave.obj").expect("Failed to load obj");
    let vertex_array = obj.get_vertex_array();

    let mut elapsed_time = 0.0f32;

    while !window.window_should_close() {
        // Get delta time from Raylib
        let delta_time = window.get_frame_time();
        elapsed_time += delta_time;

        // Process camera input
        camera.process_input(&window);

        // Update model rotation
        rotation_y += rotation_speed;

        framebuffer.clear();

        let rotation = Vector3::new(0.0, rotation_y, 0.0);
        let model_matrix = create_model_matrix(translation, scale, rotation);
        let view_matrix = camera.get_view_matrix();
        let projection_matrix = create_projection_matrix(fov_y, aspect, near, far);
        let viewport_matrix =
            create_viewport_matrix(0.0, 0.0, window_width as f32, window_height as f32);

        let uniforms = Uniforms {
            model_matrix,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            time: elapsed_time,
        };

        render(&mut framebuffer, &uniforms, &vertex_array, &light);

        // Call the encapsulated swap_buffers function
        framebuffer.swap_buffers(&mut window, &thread);

        thread::sleep(Duration::from_millis(16));
    }
}
