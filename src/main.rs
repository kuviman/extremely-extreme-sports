use geng::net::simple as simple_net;
use geng::prelude::*;

mod assets;

use assets::*;

type Id = i64;

#[derive(Debug, Serialize, Deserialize, Diff, Clone, PartialEq)]
pub struct Model {
    next_player_id: Id,
    avalanche_position: Option<f32>,
    players: Collection<Player>,
}

impl Model {
    pub const AVALANCHE_SPEED: f32 = 10.0;
    pub fn new() -> Self {
        Self {
            next_player_id: 0,
            avalanche_position: None,
            players: default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Message {
    UpdatePlayer(Player),
    StartTheRace,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {}

const TICKS_PER_SECOND: f32 = 10.0;

impl simple_net::Model for Model {
    type PlayerId = Id;
    type Message = Message;
    type Event = Event;
    const TICKS_PER_SECOND: f32 = TICKS_PER_SECOND;
    fn new_player(&mut self) -> Self::PlayerId {
        let player_id = self.next_player_id;
        self.next_player_id += 1;
        player_id
    }

    fn drop_player(&mut self, player_id: &Self::PlayerId) {
        self.players.remove(&player_id);
    }

    fn handle_message(&mut self, player_id: &Self::PlayerId, message: Message) {
        let player_id = *player_id;
        match message {
            Message::UpdatePlayer(mut player) => {
                if player.id != player_id {
                    return;
                }
                self.players.insert(player);
            }
            Message::StartTheRace => {
                if self.avalanche_position.is_none() {
                    self.avalanche_position = Some(100.0);
                }
            }
        }
    }

    fn tick(&mut self, events: &mut Vec<Event>) {
        let delta_time = 1.0 / TICKS_PER_SECOND;
        if let Some(position) = &mut self.avalanche_position {
            *position -= Self::AVALANCHE_SPEED * delta_time;
        }
    }
}

#[derive(Debug, Serialize, Deserialize, HasId, Diff, Clone, PartialEq)]
pub struct Player {
    pub id: Id,
    pub position: Vec2<f32>,
    pub radius: f32,
    pub rotation: f32,
    pub input: f32,
    pub velocity: Vec2<f32>,
    pub crashed: bool,
}

impl Player {
    const ROTATION_SPEED: f32 = 2.0 * f32::PI;
    const ROTATION_LIMIT: f32 = f32::PI / 3.0;
    const MAX_SPEED: f32 = 10.0;
    const MAX_WALK_SPEED: f32 = 1.0;
    const FRICTION: f32 = 3.0;
    const DOWNHILL_ACCELERATION: f32 = 5.0;
    const WALK_ACCELERATION: f32 = 10.0;
    const CRASH_DECELERATION: f32 = 3.0;
    pub fn update_walk(&mut self, delta_time: f32) {
        let target_speed = self.input.clamp_abs(Self::MAX_WALK_SPEED);
        self.velocity.x +=
            (target_speed - self.velocity.x).clamp_abs(Self::WALK_ACCELERATION * delta_time);
        self.position += self.velocity * delta_time;
    }
    pub fn update_riding(&mut self, delta_time: f32) {
        if !self.crashed {
            let target_rotation = (-self.input * f32::PI).clamp_abs(Self::ROTATION_LIMIT);
            self.rotation +=
                (target_rotation - self.rotation).clamp_abs(Self::ROTATION_SPEED * delta_time);
            self.velocity.y += (-Self::MAX_SPEED - self.velocity.y)
                .clamp_abs(Self::DOWNHILL_ACCELERATION * delta_time);
            let normal = vec2(1.0, 0.0).rotate(self.rotation);
            let force = -Vec2::dot(self.velocity, normal) * Self::FRICTION;
            self.velocity += normal * force * delta_time;
        } else {
            self.velocity -= self
                .velocity
                .clamp_len(..=Self::CRASH_DECELERATION * delta_time);
        }
        self.position += self.velocity * delta_time;
    }
}

pub struct Game {
    geng: Geng,
    assets: Rc<Assets>,
    player_id: Id,
    camera: geng::Camera2d,
    model: simple_net::Remote<Model>,
    player: Player,
    trail_texture: (ugli::Texture, Quad<f32>),
}

impl Game {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        player_id: Id,
        model: simple_net::Remote<Model>,
    ) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            camera: geng::Camera2d {
                center: vec2(0.0, 0.0),
                rotation: 0.0,
                fov: 10.0,
            },
            model,
            player_id,
            player: Player {
                id: player_id,
                position: Vec2::ZERO,
                radius: 1.0,
                rotation: 0.0,
                input: 0.0,
                velocity: Vec2::ZERO,
                crashed: false,
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

    fn draw_player_trail(&self, framebuffer: &mut ugli::Framebuffer, player: &Player) {
        if !player.crashed {
            self.draw_texture(
                framebuffer,
                &self.assets.ski,
                Mat3::translate(player.position) * Mat3::rotate(player.rotation),
                Color::rgb(0.8, 0.8, 0.8),
            );
        }
    }

    fn draw_player(&self, framebuffer: &mut ugli::Framebuffer, player: &Player) {
        if !player.crashed {
            self.draw_texture(
                framebuffer,
                &self.assets.ski,
                Mat3::translate(player.position) * Mat3::rotate(player.rotation),
                Color::BLACK,
            );
        }
        self.draw_texture(
            framebuffer,
            &self.assets.player,
            Mat3::translate(player.position),
            Color::WHITE,
        );
    }

    fn iter_players(&self) -> impl Iterator<Item = Player> {
        std::iter::once(&self.player)
            .chain(
                self.model
                    .get()
                    .players
                    .iter()
                    .filter(move |player| player.id != self.player.id),
            )
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl geng::State for Game {
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Space => {
                    self.model.send(Message::StartTheRace);
                }
                geng::Key::K => {
                    self.player.crashed = true;
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        self.player.input = 0.0;
        if self.geng.window().is_key_pressed(geng::Key::A) {
            self.player.input -= 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::D) {
            self.player.input += 1.0;
        }
        {
            let model = self.model.get();
            if model.avalanche_position.is_none() {
                self.player.update_walk(delta_time);
            } else {
                self.player.update_riding(delta_time);
            }
            if let Some(position) = model.avalanche_position {
                if self.player.position.y > position {
                    self.player.crashed = true;
                }
            }
        }
        self.model.send(Message::UpdatePlayer(self.player.clone()));

        for event in self.model.update() {
            // TODO handle
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let model = self.model.get();
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
            for player in self.iter_players() {
                self.draw_player_trail(framebuffer, &player);
            }
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
        for player in self.iter_players() {
            self.draw_player(framebuffer, &player);
        }

        if let Some(position) = model.avalanche_position {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::unit(Color::rgba(0.5, 0.5, 0.5, 0.5))
                    .translate(vec2(0.0, 1.0))
                    .scale(vec2(1000.0, 10.0))
                    .translate(vec2(0.0, position)),
            );
        }
    }
}

fn main() {
    geng::net::simple::run("LD50", Model::new, |geng, player_id, model| {
        geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            <Assets as geng::LoadAsset>::load(&geng, &static_path()),
            {
                let geng = geng.clone();
                move |assets| {
                    let assets = assets.expect("Failed to load assets");
                    Game::new(&geng, &Rc::new(assets), player_id, model)
                }
            },
        )
    });
}
