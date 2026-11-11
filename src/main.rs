mod camera;
mod fragment;
mod framebuffer;
mod light;
mod line;
mod matrix;
mod obj;
mod shaders;
mod sphere;
mod triangle;
mod vertex;

use crate::camera::Camera;
use crate::light::Light;
use crate::matrix::{create_model_matrix, create_projection_matrix, create_viewport_matrix};
use crate::sphere::generate_uv_sphere;
use framebuffer::Framebuffer;
use obj::Obj;
use raylib::prelude::*;
use shaders::{fragment_shader, vertex_shader};
use std::f32::consts::PI;
use triangle::triangle;
use vertex::Vertex;

const WINDOW_WIDTH: i32 = 640;
const WINDOW_HEIGHT: i32 = 480;

#[derive(Clone, Copy)]
pub enum ShaderKind {
    Ship,
    Sun,
    Terran,
    Desert,
    Ice,
}

pub struct Uniforms {
    pub model_matrix: Matrix,
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub viewport_matrix: Matrix,
    pub time: f32,
    pub shader_kind: ShaderKind,
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

#[allow(clippy::too_many_arguments)]
fn render_object(
    framebuffer: &mut Framebuffer,
    vertices: &[Vertex],
    light: &Light,
    view_matrix: Matrix,
    projection_matrix: Matrix,
    viewport_matrix: Matrix,
    time: f32,
    translation: Vector3,
    scale: f32,
    rotation: Vector3,
    base_color: Vector3,
    shader_kind: ShaderKind,
) {
    let uniforms = Uniforms {
        model_matrix: create_model_matrix(translation, scale, rotation),
        view_matrix,
        projection_matrix,
        viewport_matrix,
        time,
        shader_kind,
    };
    render(framebuffer, &uniforms, vertices, light, base_color);
}

fn draw_stars(framebuffer: &mut Framebuffer) {
    for i in 0..180_i32 {
        let x = (i * 83 + i * i * 7 + 31).rem_euclid(WINDOW_WIDTH);
        let y = (i * 47 + i * i * 3 + 17).rem_euclid(WINDOW_HEIGHT);
        let brightness = 0.45 + (i % 7) as f32 * 0.075;
        framebuffer.point(
            x,
            y,
            Vector3::new(brightness * 0.8, brightness * 0.9, brightness),
            1.0,
        );
    }
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

    let ship_obj = Obj::load("assets/models/nave.obj").expect("Failed to load ship OBJ");
    let ship_vertices = ship_obj.get_vertex_array();
    let sphere_vertices = generate_uv_sphere(8, 12);

    let mut ship_position = Vector3::new(0.0, 0.5, 15.0);
    let mut ship_yaw = 0.0_f32;
    let mut camera_distance = 6.0_f32;
    let mut camera = Camera::new(
        Vector3::new(0.0, 4.0, 21.0),
        Vector3::new(0.0, 0.5, 12.0),
        Vector3::new(0.0, 1.0, 0.0),
    );

    let projection_matrix = create_projection_matrix(
        PI / 3.0,
        WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
        0.1,
        120.0,
    );
    let viewport_matrix = create_viewport_matrix(
        0.0,
        0.0,
        WINDOW_WIDTH as f32,
        WINDOW_HEIGHT as f32,
    );
    let mut elapsed_time = 0.0_f32;

    while !window.window_should_close() {
        let delta_time = window.get_frame_time().min(0.05);
        elapsed_time += delta_time;

        let turn_speed = 1.15;
        let movement_speed = if window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            20.0
        } else {
            10.0
        };
        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            ship_yaw += turn_speed * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            ship_yaw -= turn_speed * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            camera_distance = (camera_distance - 5.0 * delta_time).max(3.2);
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            camera_distance = (camera_distance + 5.0 * delta_time).min(14.0);
        }

        let forward = Vector3::new(ship_yaw.sin(), 0.0, -ship_yaw.cos());
        let right = Vector3::new(-forward.z, 0.0, forward.x);
        if window.is_key_down(KeyboardKey::KEY_W) {
            ship_position += forward * movement_speed * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_S) {
            ship_position -= forward * movement_speed * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_A) {
            ship_position -= right * movement_speed * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_D) {
            ship_position += right * movement_speed * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_SPACE) {
            ship_position.y += movement_speed * delta_time;
        }
        if window.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
            ship_position.y -= movement_speed * delta_time;
        }

        camera.eye = ship_position
            - forward * camera_distance
            + Vector3::new(0.0, camera_distance * 0.42, 0.0);
        camera.target = ship_position + forward * 3.5;

        framebuffer.clear();
        draw_stars(&mut framebuffer);
        let view_matrix = camera.get_view_matrix();
        let light = Light::new(Vector3::zero());

        // The OBJ is not centered at its origin. Rotate its centering offset with the ship
        // so it remains visually locked to the chase camera while turning.
        let ship_center_correction = Vector3::new(
            0.164 * ship_yaw.cos(),
            0.096,
            -0.164 * ship_yaw.sin(),
        );
        render_object(
            &mut framebuffer,
            &sphere_vertices,
            &light,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            elapsed_time,
            Vector3::zero(),
            1.4,
            Vector3::new(0.0, elapsed_time * 0.25, 0.0),
            Vector3::new(1.0, 0.55, 0.08),
            ShaderKind::Sun,
        );

        let terran_angle = elapsed_time * 0.32;
        let terran_position = Vector3::new(terran_angle.cos() * 5.0, 0.0, terran_angle.sin() * 5.0);
        render_object(
            &mut framebuffer,
            &sphere_vertices,
            &light,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            elapsed_time,
            terran_position,
            0.7,
            Vector3::new(0.1, elapsed_time * 0.8, 0.0),
            Vector3::new(0.08, 0.75, 0.48),
            ShaderKind::Terran,
        );

        let desert_angle = elapsed_time * 0.21 + 2.2;
        let desert_position = Vector3::new(desert_angle.cos() * 8.0, 0.6, desert_angle.sin() * 8.0);
        render_object(
            &mut framebuffer,
            &sphere_vertices,
            &light,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            elapsed_time,
            desert_position,
            0.95,
            Vector3::new(0.15, elapsed_time * 0.45, 0.0),
            Vector3::new(0.95, 0.43, 0.08),
            ShaderKind::Desert,
        );

        let ice_angle = elapsed_time * 0.14 + 4.1;
        let ice_position = Vector3::new(ice_angle.cos() * 11.0, -0.5, ice_angle.sin() * 11.0);
        render_object(
            &mut framebuffer,
            &sphere_vertices,
            &light,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            elapsed_time,
            ice_position,
            0.82,
            Vector3::new(0.0, elapsed_time * 0.3, 0.2),
            Vector3::new(0.45, 0.82, 1.0),
            ShaderKind::Ice,
        );

        render_object(
            &mut framebuffer,
            &ship_vertices,
            &light,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            elapsed_time,
            ship_position + ship_center_correction,
            0.12,
            Vector3::new(0.0, ship_yaw, 0.0),
            Vector3::new(0.12, 0.82, 0.95),
            ShaderKind::Ship,
        );

        framebuffer.swap_buffers(&mut window, &thread);
    }
}
