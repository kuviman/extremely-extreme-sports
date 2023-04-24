use super::*;

pub struct Button<T> {
    pub text: String,
    pub position: vec2<f32>,
    pub size: f32,
    pub message: T,
}

impl<T> Button<T> {
    pub fn new(text: &str, position: vec2<f32>, size: f32, align: f32, message: T) -> Self {
        let width = text.len() as f32 * 0.8;
        Self {
            text: text.to_owned(),
            position: vec2(position.x - width * align * size, position.y),
            size,
            message,
        }
    }
    pub fn aabb(&self) -> Aabb2<f32> {
        Aabb2::point(self.position)
            .extend_positive(vec2(self.text.len() as f32 * 0.8, 1.0) * self.size)
    }
}

pub struct Controller {
    geng: Geng,
    assets: Rc<Assets>,
    mouse: vec2<f32>,
    camera: geng::Camera2d,
    framebuffer_size: vec2<f32>,
}

impl Controller {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            mouse: vec2(0.0, 0.0),
            camera: geng::Camera2d {
                center: vec2(0.0, 0.0),
                rotation: 0.0,
                fov: 1.0,
            },
            framebuffer_size: vec2(1.0, 1.0),
        }
    }
    pub fn draw<T>(
        &mut self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &geng::Camera2d,
        buttons: Vec<Button<T>>,
    ) {
        self.camera = camera.clone();
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        for button in buttons {
            let mut position = button.position;
            let hovered = button.aabb().contains(self.mouse);
            if hovered
                && self
                    .geng
                    .window()
                    .is_button_pressed(geng::MouseButton::Left)
            {
                position.y -= button.size * 0.2;
            }
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                position,
                button.size,
                &button.text,
                0.0,
                if hovered {
                    Rgba::opaque(0.5, 0.5, 1.0)
                } else {
                    Rgba::WHITE
                },
            );
        }
    }
    pub fn handle_event<T: Clone>(
        &mut self,
        event: &geng::Event,
        buttons: Vec<Button<T>>,
    ) -> Vec<T> {
        match *event {
            geng::Event::MouseMove { position, .. }
            | geng::Event::MouseDown { position, .. }
            | geng::Event::MouseUp { position, .. } => {
                self.mouse = self
                    .camera
                    .screen_to_world(self.framebuffer_size, position.map(|x| x as f32));
            }
            geng::Event::TouchStart { ref touches, .. }
            | geng::Event::TouchMove { ref touches, .. }
            | geng::Event::TouchEnd { ref touches, .. } => {
                if let Some(touch) = touches.get(0) {
                    self.mouse = self
                        .camera
                        .screen_to_world(self.framebuffer_size, touch.position.map(|x| x as f32));
                }
            }
            _ => {}
        }
        let mut result = Vec::new();
        match *event {
            geng::Event::MouseUp {
                button: geng::MouseButton::Left,
                ..
            }
            | geng::Event::TouchEnd { .. } => {
                if let Some(button) = buttons
                    .into_iter()
                    .find(|button| button.aabb().contains(self.mouse))
                {
                    result.push(button.message.clone());
                }
            }
            _ => {}
        }
        result
    }
}
