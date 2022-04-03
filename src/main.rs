use geng::net::simple as simple_net;
use geng::prelude::*;

mod assets;
mod font;

use assets::*;
use font::*;

type Id = i64;

#[derive(Debug, Serialize, Deserialize, HasId, Diff, Clone, PartialEq)]
pub struct Obstacle {
    pub id: Id,
    pub index: usize,
    pub radius: f32,
    pub position: Vec2<f32>,
}

#[derive(Debug, Serialize, Deserialize, Diff, Clone, PartialEq)]
pub struct Model {
    tick: u64,
    next_id: Id,
    avalanche_position: Option<f32>,
    avalanche_speed: f32,
    players: Collection<Player>,
    #[diff = "eq"]
    obstacles: Vec<Obstacle>,
    #[diff = "eq"]
    winner: Option<(String, f32)>,
}

impl Model {
    pub const AVALANCHE_MIN_SPEED: f32 = 7.0;
    pub const AVALANCHE_MAX_SPEED: f32 = 11.0;
    pub const AVALANCHE_ACCELERATION: f32 =
        (Self::AVALANCHE_MAX_SPEED - Self::AVALANCHE_MIN_SPEED) / 60.0;
    const AVALANCHE_START: f32 = 20.0;
    const SPAWN_AREA: f32 = 15.0;
    pub fn new() -> Self {
        Self {
            tick: 0,
            next_id: 0,
            avalanche_position: None,
            avalanche_speed: Self::AVALANCHE_MIN_SPEED,
            players: default(),
            obstacles: default(),
            winner: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Message {
    UpdatePlayer(Player),
    StartTheRace,
}
const TRACK_WIDTH: f32 = 10.0;

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
                    let list: Vec<String> = serde_json::from_reader(
                        std::fs::File::open(static_path().join("obstacles.json")).unwrap(),
                    )
                    .unwrap();
                    let obstacles: Vec<(usize, ObstacleConfig)> = list
                        .into_iter()
                        .map(|path| {
                            serde_json::from_reader(
                                std::fs::File::open(static_path().join(format!("{}.json", path)))
                                    .unwrap(),
                            )
                            .unwrap()
                        })
                        .enumerate()
                        .collect();
                    'obstacles: for _ in 0..(TRACK_LEN * TRACK_WIDTH * OBSTACLES_DENSITY) as usize {
                        let index = obstacles
                            .choose_weighted(&mut global_rng(), |(_, obstacle)| {
                                obstacle.spawn_weight
                            })
                            .unwrap()
                            .0;
                        let radius = obstacles[index].1.hitbox_radius / 20.0;
                        let w = TRACK_WIDTH - radius;
                        let x = global_rng().gen_range(-w..w);
                        let y = global_rng().gen_range(-TRACK_LEN..-Self::SPAWN_AREA);
                        let position = vec2(x, y);
                        for obstacle in &self.obstacles {
                            if (obstacle.position - position).len() < radius + obstacle.radius {
                                continue 'obstacles;
                            }
                        }
                        self.obstacles.push(Obstacle {
                            id: self.next_id,
                            index,
                            radius,
                            position,
                        });
                        self.next_id += 1;
                    }
                    self.obstacles.sort_by_key(|o| -r32(o.position.y));
                }
            }
        }
    }

    fn tick(&mut self, events: &mut Vec<Event>) {
        let delta_time = 1.0 / TICKS_PER_SECOND;
        self.tick += 1;
        if let Some(position) = &mut self.avalanche_position {
            self.avalanche_speed = (self.avalanche_speed
                + delta_time * Self::AVALANCHE_ACCELERATION)
                .min(Self::AVALANCHE_MAX_SPEED);
            *position -= self.avalanche_speed * delta_time;
            if *position < Self::AVALANCHE_START - 5.0 {
                if self.players.iter().all(|player| {
                    !player.is_riding || player.position.y > *position + self.avalanche_speed * 5.0
                }) {
                    self.avalanche_position = None;
                    self.avalanche_speed = Self::AVALANCHE_MIN_SPEED;
                    self.obstacles.clear();
                }
            }
            if let Some(winner) = self
                .players
                .iter()
                .filter(|player| player.is_riding)
                .min_by_key(|player| r32(player.position.y))
                .map(|player| (player.name.clone(), -player.position.y))
            {
                self.winner = Some(winner);
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, HasId, Diff, Clone, PartialEq)]
pub struct Player {
    pub id: Id,
    #[diff = "eq"]
    pub name: String,
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
    pub crash_position: Vec2<f32>,
}

impl Player {
    const ROTATION_SPEED: f32 = 2.0 * f32::PI;
    const ROTATION_LIMIT: f32 = f32::PI / 3.0;
    const MAX_SPEED: f32 = 10.0;
    const MAX_WALK_SPEED: f32 = 3.0;
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
            let target_rotation = (self.input * f32::PI).clamp_abs(Self::ROTATION_LIMIT);
            self.rotation +=
                (target_rotation - self.rotation).clamp_abs(Self::ROTATION_SPEED * delta_time);
            self.velocity.y += (-Self::MAX_SPEED - self.velocity.y)
                .clamp_abs(Self::DOWNHILL_ACCELERATION * delta_time);
            let normal = vec2(1.0, 0.0).rotate(self.rotation);
            let force = -Vec2::dot(self.velocity, normal) * Self::FRICTION;
            self.velocity += normal * force * delta_time;
            self.ski_velocity = self.velocity;
            self.ski_rotation = self.rotation;
            self.crash_position = self.position;
        } else {
            self.crash_timer += delta_time;
            self.velocity -= self
                .velocity
                .clamp_len(..=Self::CRASH_DECELERATION * delta_time);
        }
        self.position += self.velocity * delta_time;
    }

    fn respawn(&mut self) {
        *self = Player {
            position: vec2(0.0, 0.0),
            rotation: 0.0,
            velocity: Vec2::ZERO,
            crashed: false,
            crash_timer: 0.0,
            is_riding: false,
            seen_no_avalanche: false,
            name: self.name.clone(),
            ..*self
        };
    }
}

#[derive(ugli::Vertex)]
pub struct Particle {
    i_pos: Vec2<f32>,
    i_vel: Vec2<f32>,
    i_time: f32,
    i_size: f32,
    i_opacity: f32,
}

pub struct Game {
    time: f32,
    last_model_tick: u64,
    geng: Geng,
    assets: Rc<Assets>,
    player_id: Id,
    camera: geng::Camera2d,
    model: simple_net::Remote<Model>,
    players: Collection<Player>,
    next_particle: f32,
    trail_texture: (ugli::Texture, Quad<f32>),
    particles: ugli::VertexBuffer<Particle>,
    explosion_particles: ugli::VertexBuffer<Particle>,
    quad_geometry: ugli::VertexBuffer<draw_2d::Vertex>,
}

impl Game {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        player_id: Id,
        model: simple_net::Remote<Model>,
    ) -> Self {
        Self {
            time: 0.0,
            geng: geng.clone(),
            assets: assets.clone(),
            camera: geng::Camera2d {
                center: vec2(0.0, 0.0),
                rotation: 0.0,
                fov: 20.0,
            },
            model,
            player_id,
            last_model_tick: u64::MAX,
            players: {
                let mut result = Collection::new();
                result.insert(Player {
                    id: player_id,
                    name: "kuviman".to_owned(),
                    crash_position: Vec2::ZERO,
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
                });
                result
            },
            trail_texture: (
                ugli::Texture::new_with(geng.ugli(), vec2(1, 1), |_| Color::TRANSPARENT_WHITE),
                Quad::unit(),
            ),
            particles: ugli::VertexBuffer::new_dynamic(geng.ugli(), vec![]),
            explosion_particles: ugli::VertexBuffer::new_dynamic(geng.ugli(), vec![]),
            quad_geometry: ugli::VertexBuffer::new_static(
                geng.ugli(),
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
            next_particle: 0.0,
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
            &self.quad_geometry,
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
        if player.is_riding {
            self.draw_texture(
                framebuffer,
                &self.assets.ski,
                Mat3::translate(player.position) * Mat3::rotate(player.rotation),
                Color::rgb(0.8, 0.8, 0.8),
            );
        }
    }

    fn draw_player(&self, framebuffer: &mut ugli::Framebuffer, player: &Player) {
        if !player.crashed && player.is_riding {
            self.draw_texture(
                framebuffer,
                &self.assets.ski,
                Mat3::translate(player.position) * Mat3::rotate(player.rotation),
                Color::BLACK,
            );
        }

        if player.crashed {
            let t = player.crash_timer.min(1.0);
            self.draw_texture(
                framebuffer,
                &self.assets.ski,
                Mat3::translate(
                    player.crash_position
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
}

impl geng::State for Game {
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Space => {
                    let my_player = self.players.get(&self.player_id).unwrap();
                    if my_player.position.x >= 0.0 && my_player.position.x < 1.0 {
                        self.model.send(Message::StartTheRace);
                    }
                }
                geng::Key::K => {
                    self.players.get_mut(&self.player_id).unwrap().crashed = true;
                }
                geng::Key::R => {
                    self.players.get_mut(&self.player_id).unwrap().respawn();
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        self.time += delta_time;

        self.players.get_mut(&self.player_id).unwrap().input = 0.0;
        if self.geng.window().is_key_pressed(geng::Key::A)
            || self.geng.window().is_key_pressed(geng::Key::Left)
        {
            self.players.get_mut(&self.player_id).unwrap().input -= 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::D)
            || self.geng.window().is_key_pressed(geng::Key::Right)
        {
            self.players.get_mut(&self.player_id).unwrap().input += 1.0;
        }
        {
            let model = self.model.get();

            let my_player = self.players.get(&self.player_id).unwrap();
            let mut target_player = my_player;
            if !my_player.is_riding && model.avalanche_position.is_some() {
                if let Some(player) = self
                    .players
                    .iter()
                    .min_by_key(|player| r32(player.position.y))
                {
                    target_player = player;
                }
            }
            let target_center = vec2(
                target_player.position.x,
                target_player.position.y + target_player.velocity.y * 0.3,
            );
            self.camera.center += (target_center - self.camera.center) * (1.0 - delta_time * 0.1);

            if model.tick != self.last_model_tick {
                self.last_model_tick = model.tick;
                for player in &model.players {
                    if player.id != self.player_id {
                        self.players.insert(player.clone());
                    }
                }
                self.players.retain(|player| {
                    model.players.get(&player.id).is_some() || player.id == self.player_id
                });
            }
            if model.avalanche_position.is_none() {
                self.players
                    .get_mut(&self.player_id)
                    .unwrap()
                    .seen_no_avalanche = true;
            }
            if self
                .players
                .get_mut(&self.player_id)
                .unwrap()
                .seen_no_avalanche
                && model.avalanche_position.is_some()
            {
                let player = self.players.get_mut(&self.player_id).unwrap();
                if !player.is_riding {
                    for _ in 0..100 {
                        self.explosion_particles.push(Particle {
                            i_pos: vec2(0.0, 5.0),
                            i_vel: vec2(global_rng().gen_range(0.0f32..=1.0).powf(0.2), 0.0)
                                .rotate(global_rng().gen_range(-f32::PI..=f32::PI))
                                * 5.0,
                            i_time: self.time,
                            i_size: 0.4,
                            i_opacity: 0.3,
                        })
                    }
                    player.is_riding = true;
                }
            }
            if self.players.get_mut(&self.player_id).unwrap().crash_timer > 5.0
                && model.avalanche_position.is_none()
            {
                self.players.get_mut(&self.player_id).unwrap().respawn();
            }
            for player in &mut self.players {
                if !player.is_riding {
                    player.update_walk(delta_time);
                } else {
                    player.update_riding(delta_time);
                    for obstacle in &model.obstacles {
                        let delta_pos = player.position - obstacle.position;
                        let peneration = player.radius + obstacle.radius - delta_pos.len();
                        if peneration > 0.0 {
                            let normal = delta_pos.normalize_or_zero();
                            player.position += normal * peneration;
                            player.velocity -= normal * Vec2::dot(player.velocity, normal);
                            player.crashed = true;
                        }
                    }
                    if player.position.x.abs() > TRACK_WIDTH - player.radius {
                        player.crashed = true;
                    }
                    if let Some(position) = model.avalanche_position {
                        if player.position.y > position {
                            player.crashed = true;
                        }
                    }
                }
                player.position.x = player.position.x.clamp_abs(TRACK_WIDTH - player.radius);
            }
            self.next_particle -= delta_time;
            while self.next_particle < 0.0 {
                self.next_particle += 1.0 / 100.0;
                let mut particles = Vec::new();
                for player in &self.players {
                    particles.push(Particle {
                        i_pos: player.position,
                        // i_vel: vec2(
                        //     global_rng().gen_range(-1.0..=1.0),
                        //     global_rng().gen_range(-1.0..=1.0),
                        // ) / 3.0,
                        i_vel: Vec2::ZERO,
                        i_time: self.time,
                        i_size: 0.2,
                        i_opacity: 1.0,
                    });
                    let normal = vec2(1.0, 0.0).rotate(player.rotation);
                    let force = Vec2::dot(player.velocity, normal).abs();
                    particles.push(Particle {
                        i_pos: player.position,
                        i_vel: vec2(
                            global_rng().gen_range(-1.0..=1.0),
                            global_rng().gen_range(-1.0..=1.0),
                        ) / 2.0
                            + player.velocity,
                        i_time: self.time,
                        i_size: 0.4,
                        i_opacity: 0.5 * force / Player::MAX_SPEED,
                    });
                }
                self.particles.extend(particles);
                if let Some(pos) = model.avalanche_position {
                    for _ in 0..10 {
                        self.particles.push(Particle {
                            i_pos: vec2(
                                global_rng().gen_range(-TRACK_WIDTH..=TRACK_WIDTH),
                                pos + global_rng().gen_range(-3.0..=0.0),
                            ),
                            i_vel: vec2(
                                global_rng().gen_range(-1.0..=1.0),
                                global_rng().gen_range(-1.0..=1.0),
                            ),
                            i_time: self.time,
                            i_size: 0.4,
                            i_opacity: 0.5,
                        });
                    }
                }
            }
        }
        self.particles
            .retain(|particle| particle.i_time > self.time - 1.0);
        for particle in &mut *self.particles {
            particle.i_pos += particle.i_vel * delta_time;
            particle.i_vel -= particle.i_vel.clamp_len(..=delta_time * 5.0);
        }
        self.explosion_particles
            .retain(|particle| particle.i_time > self.time - 1.0);
        for particle in &mut *self.explosion_particles {
            particle.i_pos += particle.i_vel * delta_time;
            particle.i_vel -= particle.i_vel.clamp_len(..=delta_time * 5.0);
        }
        self.model.send(Message::UpdatePlayer(
            self.players.get(&self.player_id).unwrap().clone(),
        ));

        for event in self.model.update() {
            // TODO handle
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let model = self.model.get();
        let my_player = self.players.get(&self.player_id).unwrap();
        let mut target_player = my_player;
        if !my_player.is_riding && model.avalanche_position.is_some() {
            if let Some(player) = self
                .players
                .iter()
                .min_by_key(|player| r32(player.position.y))
            {
                target_player = player;
            }
        }

        // let mut new_trail_texture =
        //     ugli::Texture::new_uninitialized(self.geng.ugli(), framebuffer.size());
        // {
        //     new_trail_texture.set_filter(ugli::Filter::Nearest);
        //     let mut framebuffer = ugli::Framebuffer::new_color(
        //         self.geng.ugli(),
        //         ugli::ColorAttachment::Texture(&mut new_trail_texture),
        //     );
        //     let framebuffer = &mut framebuffer;
        //     ugli::clear(framebuffer, Some(Color::TRANSPARENT_WHITE), None);
        //     self.draw_texture(
        //         framebuffer,
        //         &self.trail_texture.0,
        //         self.trail_texture.1.transform,
        //         Color::WHITE,
        //     );
        //     for player in self.iter_players() {
        //         self.draw_player_trail(framebuffer, &player);
        //     }
        // }
        let view_area = self.camera.view_area(framebuffer.size().map(|x| x as f32));
        // self.trail_texture = (new_trail_texture, view_area);

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
            &draw_2d::Quad::new(
                AABB::<f32>::point(Vec2::ZERO)
                    .extend_left(TRACK_WIDTH * 2.0)
                    .extend_right(TRACK_WIDTH * 2.0)
                    .extend_up(100.0),
                Color::rgb(145.0 / 255.0, 249.0 / 255.0, 1.0),
            ),
        );
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::new(
                AABB::<f32>::point(Vec2::ZERO)
                    .extend_symmetric(self.assets.background.size().map(|x| x as f32) * 0.05),
                &self.assets.background,
            ),
        );
        {
            let texture = if model.avalanche_position.is_none() {
                &self.assets.detonator
            } else {
                &self.assets.detonator2
            };
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::new(
                    AABB::<f32>::point(Vec2::ZERO)
                        .extend_positive(texture.size().map(|x| x as f32) * 0.05),
                    texture,
                ),
            );
        }
        if model.avalanche_position.is_none()
            && my_player.position.x >= 0.0
            && my_player.position.x < 1.0
        {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::new(
                    AABB::<f32>::point(self.camera.center + vec2(0.0, 4.0)).extend_symmetric(
                        self.assets.detonate_text.size().map(|x| x as f32) * 0.05,
                    ),
                    &self.assets.detonate_text,
                ),
            );
        }

        let framebuffer_size = framebuffer.size();

        let c2 = Color::rgba(0.9, 0.9, 0.95, 0.0);
        let c1 = Color::rgba(0.9, 0.9, 0.95, 0.9);

        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::Quad::new(
                AABB::point(vec2(TRACK_WIDTH, 0.0))
                    .extend_right(TRACK_WIDTH * 2.0)
                    .extend_up(self.camera.center.y - self.camera.fov),
                c1,
            ),
        );
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::Quad::new(
                AABB::point(vec2(-TRACK_WIDTH, 0.0))
                    .extend_right(-TRACK_WIDTH * 2.0)
                    .extend_up(self.camera.center.y - self.camera.fov),
                c1,
            ),
        );
        {
            let p = |x: f32, y: f32| draw_2d::TexturedVertex {
                a_pos: vec2(TRACK_WIDTH + x * 2.0, y),
                a_color: Color::WHITE,
                a_vt: vec2(x * 0.9, y / 2.0),
            };
            let y = self.camera.center.y - self.camera.fov;
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedPolygon::new(
                    vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, y), p(0.0, y)],
                    &self.assets.border,
                ),
            );
            let p = |x: f32, y: f32| draw_2d::TexturedVertex {
                a_pos: vec2(-TRACK_WIDTH - x * 2.0, y),
                a_color: Color::WHITE,
                a_vt: vec2(x * 0.9, y / 2.0),
            };
            let y = self.camera.center.y - self.camera.fov;
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedPolygon::new(
                    vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, y), p(0.0, y)],
                    &self.assets.border,
                ),
            );
        }

        ugli::draw(
            framebuffer,
            &self.assets.particle_program,
            ugli::DrawMode::TriangleFan,
            ugli::instanced(&self.quad_geometry, &self.particles),
            (
                ugli::uniforms! {
                    u_time: self.time,
                    u_texture: &self.assets.particle,
                    u_color: Color::rgba(0.8, 0.8, 0.85, 0.7),
                },
                geng::camera2d_uniforms(&self.camera, framebuffer_size.map(|x| x as f32)),
            ),
            &ugli::DrawParameters {
                blend_mode: Some(default()),
                ..default()
            },
        );

        ugli::draw(
            framebuffer,
            &self.assets.particle_program,
            ugli::DrawMode::TriangleFan,
            ugli::instanced(&self.quad_geometry, &self.explosion_particles),
            (
                ugli::uniforms! {
                    u_time: self.time,
                    u_texture: &self.assets.particle,
                    u_color: Color::rgb(1.0, 0.5, 0.0),
                },
                geng::camera2d_uniforms(&self.camera, framebuffer_size.map(|x| x as f32)),
            ),
            &ugli::DrawParameters {
                blend_mode: Some(default()),
                ..default()
            },
        );

        // self.draw_texture(
        //     framebuffer,
        //     &self.trail_texture.0,
        //     self.trail_texture.1.transform,
        //     Color::WHITE,
        // );

        for player in &self.players {
            self.draw_shadow(
                framebuffer,
                Mat3::translate(player.position) * Mat3::scale_uniform(player.radius),
                Color::rgba(0.5, 0.5, 0.5, 0.5),
            );
        }
        if true || self.players.get(&self.player_id).unwrap().is_riding {
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

        for player in &self.players {
            self.draw_player(framebuffer, player);
        }

        if true || self.players.get(&self.player_id).unwrap().is_riding {
            for obstacle in &model.obstacles {
                if !in_view(obstacle.position) {
                    continue;
                }
                self.draw_obstacle(
                    framebuffer,
                    &self.assets.obstacles[obstacle.index],
                    Mat3::translate(obstacle.position) * Mat3::scale_uniform(obstacle.radius),
                    Color::WHITE,
                );
            }
        }
        if let Some(position) = model.avalanche_position {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Polygon::new_gradient(vec![
                    draw_2d::ColoredVertex {
                        a_pos: vec2(-TRACK_WIDTH * 2.0, position - 3.0),
                        a_color: c2,
                    },
                    draw_2d::ColoredVertex {
                        a_pos: vec2(-TRACK_WIDTH * 2.0, position),
                        a_color: c1,
                    },
                    draw_2d::ColoredVertex {
                        a_pos: vec2(TRACK_WIDTH * 2.0, position),
                        a_color: c1,
                    },
                    draw_2d::ColoredVertex {
                        a_pos: vec2(TRACK_WIDTH * 2.0, position - 3.0),
                        a_color: c2,
                    },
                ]),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(
                    AABB::point(vec2(0.0, position))
                        .extend_left(TRACK_WIDTH * 2.0)
                        .extend_right(TRACK_WIDTH * 2.0)
                        .extend_up(100.0),
                    c1,
                ),
            );
        }
        if !my_player.is_riding && model.avalanche_position.is_some() {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::new(
                    AABB::<f32>::point(self.camera.center + vec2(0.0, -5.0)).extend_symmetric(
                        self.assets.spectating_text.size().map(|x| x as f32) * 0.05,
                    ),
                    &self.assets.spectating_text,
                ),
            );
        }
        for player in &self.players {
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                player.position + vec2(0.0, 1.0),
                0.5,
                &player.name,
                0.5,
            );
        }
        if let Some(pos) = model.avalanche_position {
            let pos = pos - self.camera.center.y - self.camera.fov / 2.0;
            if pos > 1.0 {
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    self.camera.center + vec2(0.0, 8.0),
                    1.0,
                    &format!("avalanche is {}m behind", pos as i32),
                    0.5,
                );
            }
        } else if let Some((name, score)) = &model.winner {
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                self.camera.center + vec2(0.0, 8.0),
                1.0,
                &format!("winner is {}", name),
                0.5,
            );
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                self.camera.center + vec2(0.0, 7.0),
                1.0,
                &format!("resulted {}m", *score as i32),
                0.5,
            );
        }
        if target_player.is_riding {
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                self.camera.center + vec2(0.0, -9.0),
                1.0,
                &format!("score {}m", (-target_player.position.y) as i32),
                0.5,
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
                    let mut assets = assets.expect("Failed to load assets");
                    assets.border.set_filter(ugli::Filter::Nearest);
                    assets.border.set_wrap_mode(ugli::WrapMode::Repeat);
                    assets.background.set_filter(ugli::Filter::Nearest);
                    assets.detonator.set_filter(ugli::Filter::Nearest);
                    assets.detonator2.set_filter(ugli::Filter::Nearest);
                    assets.detonate_text.set_filter(ugli::Filter::Nearest);
                    assets.spectating_text.set_filter(ugli::Filter::Nearest);
                    Game::new(&geng, &Rc::new(assets), player_id, model)
                }
            },
        )
    });
}
