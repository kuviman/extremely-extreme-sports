use geng::prelude::*;

#[derive(Deref)]
pub struct Texture(#[deref] ugli::Texture);

impl std::borrow::Borrow<ugli::Texture> for &'_ Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}

impl geng::LoadAsset for Texture {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let texture = ugli::Texture::load(geng, path);
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Self(texture))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

#[derive(geng::Assets)]
pub struct Assets {
    pub player: Texture,
    pub ski: Texture,
    pub tree: Rc<Texture>,
    pub texture_program: ugli::Program,
}

pub struct Player {
    pub position: Vec2<f32>,
    pub radius: f32,
    pub rotation: f32,
    pub target_rotation: f32,
    pub velocity: Vec2<f32>,
}

impl Player {
    const ROTATION_SPEED: f32 = 2.0 * f32::PI;
    const ROTATION_LIMIT: f32 = f32::PI / 3.0;
    const MAX_SPEED: f32 = 10.0;
    const FRICTION: f32 = 3.0;
    const DOWNHILL_ACCELERATION: f32 = 5.0;
    pub fn update(&mut self, delta_time: f32) {
        self.target_rotation = self.target_rotation.clamp_abs(Self::ROTATION_LIMIT);
        self.rotation +=
            (self.target_rotation - self.rotation).clamp_abs(Self::ROTATION_SPEED * delta_time);
        self.velocity.y += (-Self::MAX_SPEED - self.velocity.y)
            .clamp_abs(Self::DOWNHILL_ACCELERATION * delta_time);
        self.position += self.velocity * delta_time;
        let normal = vec2(1.0, 0.0).rotate(self.rotation);
        let force = -Vec2::dot(self.velocity, normal) * Self::FRICTION;
        self.velocity += normal * force * delta_time;
    }
}

pub struct Obstacle {
    pub position: Vec2<f32>,
    pub radius: f32,
    pub texture: Rc<Texture>,
}

pub struct Game {
    geng: Geng,
    assets: Rc<Assets>,
    camera: geng::Camera2d,
    time: f32,
    player: Player,
    obstacles: Vec<Obstacle>,
    trail_texture: (ugli::Texture, Quad<f32>),
}

impl Game {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            camera: geng::Camera2d {
                center: vec2(0.0, 0.0),
                rotation: 0.0,
                fov: 10.0,
            },
            time: 0.0,
            player: Player {
                position: Vec2::ZERO,
                radius: 1.0,
                rotation: 0.0,
                target_rotation: 0.0,
                velocity: Vec2::ZERO,
            },
            obstacles: {
                let mut result = Vec::new();
                for _ in 0..100 {
                    result.push(Obstacle {
                        position: vec2(
                            global_rng().gen_range(-10.0..=10.0),
                            global_rng().gen_range(-100.0..0.0),
                        ),
                        radius: 1.0,
                        texture: assets.tree.clone(),
                    })
                }
                result
            },
            trail_texture: (
                ugli::Texture::new_with(geng.ugli(), vec2(1, 1), |_| Color::TRANSPARENT_WHITE),
                Quad::unit(),
            ),
        }
    }

    fn draw_texture(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        texture: &ugli::Texture,
        transform: Mat3<f32>,
        color: Color<f32>,
    ) {
        let framebuffer_size = framebuffer.size();
        ugli::draw(
            framebuffer,
            &self.assets.texture_program,
            ugli::DrawMode::TriangleFan,
            &ugli::VertexBuffer::new_dynamic(
                self.geng.ugli(),
                vec![
                    draw_2d::Vertex {
                        a_pos: vec2(-1.0, -1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(1.0, -1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(1.0, 1.0),
                    },
                    draw_2d::Vertex {
                        a_pos: vec2(-1.0, 1.0),
                    },
                ],
            ),
            (
                ugli::uniforms! {
                    u_texture: texture,
                    u_model_matrix: transform,
                    u_color: color,
                },
                geng::camera2d_uniforms(&self.camera, framebuffer_size.map(|x| x as f32)),
            ),
            &ugli::DrawParameters { ..default() },
        );
    }
}

impl geng::State for Game {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        self.time += delta_time;
        self.player.target_rotation = 0.0;
        if self.geng.window().is_key_pressed(geng::Key::A) {
            self.player.target_rotation += f32::PI / 2.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::D) {
            self.player.target_rotation -= f32::PI / 2.0;
        }
        self.player.update(delta_time);
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.camera.center = self.player.position;

        let mut new_trail_texture =
            ugli::Texture::new_uninitialized(self.geng.ugli(), framebuffer.size());
        {
            new_trail_texture.set_filter(ugli::Filter::Nearest);
            let mut framebuffer = ugli::Framebuffer::new_color(
                self.geng.ugli(),
                ugli::ColorAttachment::Texture(&mut new_trail_texture),
            );
            let framebuffer = &mut framebuffer;
            ugli::clear(framebuffer, Some(Color::TRANSPARENT_WHITE), None);
            self.draw_texture(
                framebuffer,
                &self.trail_texture.0,
                self.trail_texture.1.transform,
                Color::WHITE,
            );
            self.draw_texture(
                framebuffer,
                &self.assets.ski,
                Mat3::translate(self.player.position) * Mat3::rotate(self.player.rotation),
                Color::rgb(0.8, 0.8, 0.8),
            );
        }
        self.trail_texture = (
            new_trail_texture,
            self.camera.view_area(framebuffer.size().map(|x| x as f32)),
        );

        ugli::clear(framebuffer, Some(Color::WHITE), None);

        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::unit(&self.trail_texture.0)
                .transform(self.trail_texture.1.transform),
        );

        self.draw_texture(
            framebuffer,
            &self.assets.ski,
            Mat3::translate(self.player.position) * Mat3::rotate(self.player.rotation),
            Color::BLACK,
        );
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::unit(&self.assets.player).translate(self.player.position),
        );
        for obstacle in &self.obstacles {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit(&**obstacle.texture).translate(obstacle.position),
            );
        }
    }
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    let geng = Geng::new("LD50");
    geng::run(
        &geng,
        geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            <Assets as geng::LoadAsset>::load(&geng, &static_path()),
            {
                let geng = geng.clone();
                move |assets| {
                    let assets = assets.expect("Failed to load assets");
                    Game::new(&geng, &Rc::new(assets))
                }
            },
        ),
    );
}
