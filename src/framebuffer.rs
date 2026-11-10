use raylib::prelude::*;

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    image: Image,
    background_color: Vector3,
    texture: Option<Texture2D>,
    depth_buffer: Vec<f32>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let image = Image::gen_image_color(width as i32, height as i32, Color::BLACK);
        let buffer_size = (width * height) as usize;
        let depth_buffer = vec![f32::INFINITY; buffer_size]; // Initialize with far plane
        Framebuffer {
            width,
            height,
            image,
            background_color: Vector3::zero(),
            texture: None,
            depth_buffer,
        }
    }

    pub fn init_texture(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        self.texture = Some(rl.load_texture_from_image(thread, &self.image).unwrap());
    }

    pub fn clear(&mut self) {
        let bg_color = Color::new(
            (self.background_color.x * 255.0) as u8,
            (self.background_color.y * 255.0) as u8,
            (self.background_color.z * 255.0) as u8,
            255,
        );
        self.image.clear_background(bg_color);

        // Clear depth buffer to far plane
        self.depth_buffer.fill(f32::INFINITY);
    }

    pub fn point(&mut self, x: i32, y: i32, color: Vector3, depth: f32) -> bool {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            let index = (y * self.width as i32 + x) as usize;

            // Depth test: only draw if this fragment is closer
            if depth < self.depth_buffer[index] {
                self.depth_buffer[index] = depth;

                let pixel_color = Color::new(
                    (color.x.clamp(0.0, 1.0) * 255.0) as u8,
                    (color.y.clamp(0.0, 1.0) * 255.0) as u8,
                    (color.z.clamp(0.0, 1.0) * 255.0) as u8,
                    255,
                );
                self.image.draw_pixel(x, y, pixel_color);
                return true;
            }
        }
        false
    }

    pub fn set_background_color(&mut self, color: Vector3) {
        self.background_color = color;
    }

    pub fn swap_buffers(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        if let Some(texture) = &mut self.texture {
            let colors = self.image.get_image_data();
            // Safely cast the &[Color] slice to a &[u8] slice for the update function
            let data: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    colors.as_ptr() as *const u8,
                    colors.len() * 4, // Each Color is 4 bytes (r,g,b,a)
                )
            };
            texture.update_texture(data).unwrap();

            let mut d = rl.begin_drawing(thread);
            d.clear_background(Color::BLACK);
            d.draw_texture(texture, 0, 0, Color::WHITE);
        } else {
            panic!("Framebuffer texture has not been initialized. Call init_texture after creating the RaylibHandle.");
        }
    }
}