use geng::net::simple as simple_net;
use geng::prelude::*;

mod assets;

use assets::*;

type Id = i64;

#[derive(Debug, Serialize, Deserialize, HasId, Diff, Clone, PartialEq)]
pub struct Obstacle {
    pub id: Id,
    pub radius: f32,
    pub position: Vec2<f32>,
}

#[derive(Debug, Serialize, Deserialize, Diff, Clone, PartialEq)]
pub struct Model {
    next_id: Id,
    avalanche_position: Option<f32>,
    players: Collection<Player>,
    obstacles: Collection<Obstacle>,
}

impl Model {
    pub const AVALANCHE_SPEED: f32 = 7.0;
    const AVALANCHE_START: f32 = 100.0;
    pub fn new() -> Self {
        Self {
            next_id: 0,
            avalanche_position: None,
            players: default(),
            obstacles: default(),
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
        let player_id = self.next_id;
        self.next_id += 1;
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
                    self.avalanche_position = Some(Self::AVALANCHE_START);
                    const TRACK_LEN: f32 = 1000.0;
                    const OBSTACLES_DENSITY: f32 = 0.1;
                    const TRACK_WIDTH: f32 = 20.0;
                    'obstacles: for _ in 0..(TRACK_LEN * TRACK_WIDTH * OBSTACLES_DENSITY) as usize {
                        let x = global_rng().gen_range(-TRACK_WIDTH..TRACK_WIDTH);
                        let y = global_rng().gen_range(-TRACK_LEN..0.0);
                        let position = vec2(x, y);
                        let radius = 1.0;
                        for obstacle in &self.obstacles {
                            if (obstacle.position - position).len() < radius + obstacle.radius {
                                continue 'obstacles;
                            }
                        }
                        self.obstacles.insert(Obstacle {
                            id: self.next_id,
                            radius,
                            position,
                        });
                        self.next_id += 1;
                    }
                }
            }
        }
    }

    fn tick(&mut self, events: &mut Vec<Event>) {
        let delta_time = 1.0 / TICKS_PER_SECOND;
        if let Some(position) = &mut self.avalanche_position {
            *position -= Self::AVALANCHE_SPEED * delta_time;
            if *position < Self::AVALANCHE_START - Self::AVALANCHE_SPEED * 10.0 {
                if self.players.iter().all(|player| !player.is_riding) {
                    self.avalanche_position = None;
                }
            }
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
    pub crash_timer: f32,
    pub ski_velocity: Vec2<f32>,
    pub ski_rotation: f32,
    pub is_riding: bool,
    pub seen_no_avalanche: bool,
}

impl Player {
    const ROTATION_SPEED: f32 = 2.0 * f32::PI;
    const ROTATION_LIMIT: f32 = f32::PI / 3.0;
    const MAX_SPEED: f32 = 10.0;
    const MAX_WALK_SPEED: f32 = 1.0;
    const FRICTION: f32 = 5.0;
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
            self.ski_velocity = self.velocity;
            self.ski_rotation = self.rotation;
        } else {
            self.crash_timer += delta_time;
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
                fov: 20.0,
            },
            model,
            player_id,
            player: Player {
                id: player_id,
                is_riding: false,
                seen_no_avalanche: false,
                ski_rotation: 0.0,
                crash_timer: 0.0,
                position: Vec2::ZERO,
                radius: 0.3,
                rotation: 0.0,
                input: 0.0,
                velocity: Vec2::ZERO,
                crashed: false,
                ski_velocity: Vec2::ZERO,
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

    fn draw_shadow(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        transform: Mat3<f32>,
        color: Color<f32>,
    ) {
        let framebuffer_size = framebuffer.size();
        ugli::draw(
            framebuffer,
            &self.assets.shadow,
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
                    u_model_matrix: transform,
                    u_color: color,
                },
                geng::camera2d_uniforms(&self.camera, framebuffer_size.map(|x| x as f32)),
            ),
            &ugli::DrawParameters {
                blend_mode: Some(default()),
                ..default()
            },
        );
    }

    fn draw_obstacle(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        obstacle: &ObstacleAssets,
        transform: Mat3<f32>,
        color: Color<f32>,
    ) {
        self.draw_texture(
            framebuffer,
            &obstacle.texture,
            transform
                * Mat3::scale_uniform(1.0 / obstacle.config.hitbox_radius)
                * Mat3::translate(-obstacle.config.hitbox_origin)
                * Mat3::scale(obstacle.texture.size().map(|x| x as f32))
                * Mat3::scale_uniform(0.5)
                * Mat3::translate(vec2(1.0, 1.0)),
            color,
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
        } else {
            let t = player.crash_timer.min(1.0);
            self.draw_texture(
                framebuffer,
                &self.assets.ski,
                Mat3::translate(
                    player.position
                        + player.ski_velocity * t
                        + vec2(0.0, (1.0 - (t * 2.0 - 1.0).sqr()) * 5.0),
                ) * Mat3::rotate(player.ski_rotation + t * 5.0),
                Color::BLACK,
            );
        }
        self.draw_texture(
            framebuffer,
            &self.assets.player,
            Mat3::translate(player.position)
                * Mat3::rotate((player.crash_timer * 7.0).min(f32::PI / 2.0)),
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
                geng::Key::R => {
                    self.player = Player {
                        position: vec2(0.0, 0.0),
                        rotation: 0.0,
                        velocity: Vec2::ZERO,
                        crashed: false,
                        crash_timer: 0.0,
                        is_riding: false,
                        seen_no_avalanche: false,
                        ..self.player
                    };
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
            println!("{:?}", model.avalanche_position);
            if model.avalanche_position.is_none() {
                self.player.seen_no_avalanche = true;
            }
            if self.player.seen_no_avalanche && model.avalanche_position.is_some() {
                self.player.is_riding = true;
            }
            if !self.player.is_riding {
                self.player.update_walk(delta_time);
            } else {
                self.player.update_riding(delta_time);
                for obstacle in &model.obstacles {
                    let delta_pos = self.player.position - obstacle.position;
                    let peneration = self.player.radius + obstacle.radius - delta_pos.len();
                    if peneration > 0.0 {
                        let normal = delta_pos.normalize_or_zero();
                        self.player.position += normal * peneration;
                        self.player.velocity -= normal * Vec2::dot(self.player.velocity, normal);
                        self.player.crashed = true;
                    }
                }
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
        let view_area = self.camera.view_area(framebuffer.size().map(|x| x as f32));
        self.trail_texture = (new_trail_texture, view_area);

        let in_view = |position: Vec2<f32>| -> bool {
            let position_in_view = view_area.transform.inverse() * position.extend(1.0);
            if position_in_view.x.abs() > 1.5 {
                return false;
            }
            if position_in_view.y.abs() > 1.5 {
                return false;
            }
            true
        };

        ugli::clear(framebuffer, Some(Color::WHITE), None);

        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::unit(&self.trail_texture.0)
                .transform(self.trail_texture.1.transform),
        );

        for player in self.iter_players() {
            self.draw_shadow(
                framebuffer,
                Mat3::translate(player.position) * Mat3::scale_uniform(player.radius),
                Color::rgba(0.5, 0.5, 0.5, 0.5),
            );
        }
        if self.player.is_riding {
            for obstacle in &model.obstacles {
                if !in_view(obstacle.position) {
                    continue;
                }
                self.draw_shadow(
                    framebuffer,
                    Mat3::translate(obstacle.position) * Mat3::scale_uniform(obstacle.radius),
                    Color::rgba(0.5, 0.5, 0.5, 0.5),
                );
            }
        }

        for player in self.iter_players() {
            self.draw_player(framebuffer, &player);
        }

        if self.player.is_riding {
            for obstacle in &model.obstacles {
                if !in_view(obstacle.position) {
                    continue;
                }
                self.draw_obstacle(
                    framebuffer,
                    &self.assets.tree,
                    Mat3::translate(obstacle.position) * Mat3::scale_uniform(obstacle.radius),
                    Color::WHITE,
                );
            }
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
