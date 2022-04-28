#![allow(warnings)]
use geng::prelude::*;

mod assets;
mod discord;
mod font;
mod lobby;
mod model;
mod simple_net;
mod skin;

use assets::*;
use font::*;
use lobby::*;
use model::*;

const DISCORD_LINK: &'static str = "https://discord.gg/DZaEMPpANY";

#[derive(ugli::Vertex)]
pub struct Particle {
    i_pos: Vec2<f32>,
    i_vel: Vec2<f32>,
    i_time: f32,
    i_size: f32,
    i_opacity: f32,
}

pub struct Game {
    framebuffer_size: Vec2<usize>,
    touch_control: Option<Vec2<f32>>,
    touches: usize,
    time: f32,
    volume: f64,
    explosion_time: Option<f32>,
    last_model_tick: u64,
    geng: Geng,
    assets: Rc<Assets>,
    player_id: Id,
    camera: geng::Camera2d,
    model: simple_net::Remote<Model>,
    players: Collection<Player>,
    interpolated_players: Collection<Player>,
    next_particle: f32,
    trail_texture: (ugli::Texture, Quad<f32>),
    particles: ugli::VertexBuffer<Particle>,
    show_player_names: bool,
    explosion_particles: ugli::VertexBuffer<Particle>,
    quad_geometry: ugli::VertexBuffer<draw_2d::Vertex>,
    ride_sound_effect: geng::SoundEffect,
    avalanche_sound_effect: geng::SoundEffect,
    music: Option<geng::SoundEffect>,
    spawn_particles: Vec<(f32, Vec2<f32>)>,
}

impl Game {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        player_id: Id,
        name: Option<String>,
        config: Option<skin::Config>,
        model: simple_net::Remote<Model>,
        auto_sound: bool,
    ) -> Self {
        Self {
            music: Some(assets.music.play()),
            touches: 0,
            touch_control: None,
            framebuffer_size: vec2(1, 1),
            interpolated_players: default(),
            time: 0.0,
            volume: 0.5,
            show_player_names: true,
            explosion_time: None,
            geng: geng.clone(),
            assets: assets.clone(),
            camera: geng::Camera2d {
                center: vec2(0.0, 0.0),
                rotation: 0.0,
                fov: 20.0,
            },
            spawn_particles: Vec::new(),
            model,
            player_id,
            last_model_tick: u64::MAX,
            players: {
                let mut result = Collection::new();
                if let (Some(name), Some(config)) = (name, config) {
                    result.insert(Player {
                        emote: None,
                        id: player_id,
                        name,
                        old_config: PlayerConfig::random(&assets.player),
                        config,
                        crash_position: Vec2::ZERO,
                        is_riding: false,
                        seen_no_avalanche: false,
                        ski_rotation: 0.0,
                        crash_timer: 0.0,
                        ride_volume: 0.0,
                        position: vec2(global_rng().gen_range(-TRACK_WIDTH..=TRACK_WIDTH), 0.0),
                        radius: 0.3,
                        rotation: 0.0,
                        input: 0.0,
                        velocity: Vec2::ZERO,
                        crashed: false,
                        ski_velocity: Vec2::ZERO,
                    });
                }
                result
            },
            ride_sound_effect: {
                let mut effect = assets.ride_sound.effect();
                effect.set_volume(0.0);
                effect.play();
                effect
            },
            avalanche_sound_effect: {
                let mut effect = assets.avalanche_sound.effect();
                effect.set_volume(0.0);
                effect.play();
                effect
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

    fn draw_player(&self, framebuffer: &mut ugli::Framebuffer, player: &Player) {
        let equipment: &ugli::Texture = &self.assets.player.equipment[player.old_config.equipment];
        if !player.crashed && player.is_riding {
            self.draw_texture(
                framebuffer,
                equipment,
                Mat3::translate(player.position) * Mat3::rotate(player.rotation),
                Color::WHITE,
            );
        } else if !player.crashed {
            self.draw_texture(
                framebuffer,
                equipment,
                Mat3::translate(player.position + vec2(0.0, 1.0)),
                Color::WHITE,
            );
        }

        if player.crashed {
            let t = player.crash_timer.min(1.0);
            self.draw_texture(
                framebuffer,
                equipment,
                Mat3::translate(
                    player.crash_position
                        + player.ski_velocity * t
                        + vec2(0.0, (1.0 - (t * 2.0 - 1.0).sqr()) * 5.0),
                ) * Mat3::rotate(player.ski_rotation + t * 5.0),
                Color::WHITE,
            );
        }

        let final_matrix = Mat3::translate(
            player.position
                + if player.is_riding {
                    vec2(0.0, 0.0)
                } else {
                    vec2(
                        0.0,
                        player.velocity.len().min(0.1) * (self.time * 15.0).sin().abs(),
                    )
                },
        ) * Mat3::rotate((player.crash_timer * 7.0).min(f32::PI / 2.0))
            * Mat3::scale_uniform(1.0 / 64.0);
        let turn = player.rotation / Player::ROTATION_LIMIT;
        let speed = (player.velocity.len() / Player::MAX_SPEED).min(1.0);
        let mut part_matrices: HashMap<&str, Mat3<f32>> = HashMap::new();
        for part in &player.config.parts {
            let texture = &self.assets.textures[&part.texture];
            let parent_matrix = match &part.parent {
                Some(name) => part_matrices[name.as_str()],
                None => Mat3::identity(),
            };
            let position = (parent_matrix
                * part
                    .position
                    .interpolate(turn, speed, self.time)
                    .extend(1.0))
            .xy();
            // Mat3::translate(position)
            let matrix = parent_matrix
                * Mat3::translate(part.position.interpolate(turn, speed, self.time))
                * Mat3::rotate(part.rotation.interpolate(turn, speed, self.time) * f32::PI / 180.0)
                * Mat3::scale(part.scale.interpolate(turn, speed, self.time))
                * Mat3::translate(-part.origin);
            if let Some(name) = &part.name {
                part_matrices.insert(name.as_str(), matrix);
            }
            let matrix = matrix
                * Mat3::scale(texture.size().map(|x| x as f32) / 2.0)
                * Mat3::translate(vec2(1.0, 1.0));
            self.draw_texture(framebuffer, texture, final_matrix * matrix, Color::WHITE);
        }

        if let Some((_, index)) = player.emote {
            self.draw_texture(
                framebuffer,
                &self.assets.emotes[index],
                Mat3::translate(player.position + vec2(0.0, 1.8)) * Mat3::scale_uniform(0.3),
                Color::WHITE,
            );
        }
    }
    fn play_sound(&self, sound: &geng::Sound, pos: Vec2<f32>) {
        let mut effect = sound.effect();
        effect.set_volume(
            (1.0 - ((pos - self.camera.center).len() / 10.0).sqr()).max(0.0) as f64 * self.volume,
        );
        effect.play()
    }

    fn update_interpolated(&mut self, delta_time: f32) {
        self.interpolated_players
            .retain(|player| self.players.get(&player.id).is_some());
        for player in &self.players {
            if self.interpolated_players.get(&player.id).is_none() || player.id == self.player_id {
                self.interpolated_players.insert(player.clone());
            }
            let i = self.interpolated_players.get_mut(&player.id).unwrap();
            const EXPECTED_PING: f32 = 0.3;
            *i = Player {
                id: player.id,
                emote: player.emote,
                name: player.name.clone(),
                position: i.position + (player.position - i.position) / EXPECTED_PING * delta_time,
                old_config: player.old_config.clone(),
                config: player.config.clone(),
                radius: player.radius,
                rotation: player.rotation,
                input: player.input,
                velocity: i.velocity + (player.velocity - i.velocity) / EXPECTED_PING * delta_time,
                crashed: player.crashed,
                crash_timer: player.crash_timer,
                ski_velocity: player.ski_velocity,
                ski_rotation: player.ski_rotation,
                is_riding: player.is_riding,
                seen_no_avalanche: player.seen_no_avalanche,
                crash_position: player.crash_position,
                ride_volume: player.ride_volume,
            };
        }
    }
}

impl geng::State for Game {
    fn handle_event(&mut self, event: geng::Event) {
        if self.music.is_none() {
            self.music = Some(self.assets.music.play());
        }
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Space => {
                    let my_player = self.players.get(&self.player_id).unwrap();
                    if my_player.position.x >= 0.0 && my_player.position.x < 1.0 {
                        self.model.send(Message::StartTheRace);
                    }
                }
                geng::Key::H => {
                    self.show_player_names = !self.show_player_names;
                }
                geng::Key::K => {
                    self.players.get_mut(&self.player_id).unwrap().crashed = true;
                }
                geng::Key::R => {
                    self.players.get_mut(&self.player_id).unwrap().respawn();
                }
                geng::Key::Num1 => {
                    self.players.get_mut(&self.player_id).unwrap().emote = Some((0.0, 0));
                }
                geng::Key::Num2 => {
                    self.players.get_mut(&self.player_id).unwrap().emote = Some((0.0, 1));
                }
                geng::Key::Num3 => {
                    self.players.get_mut(&self.player_id).unwrap().emote = Some((0.0, 2));
                }
                geng::Key::Num4 => {
                    self.players.get_mut(&self.player_id).unwrap().emote = Some((0.0, 3));
                }
                _ => {}
            },
            geng::Event::MouseDown {
                position,
                button: geng::MouseButton::Left,
            } => {
                self.touch_control = Some(position.map(|x| x as f32));
                if self.touch_control.unwrap().y > self.framebuffer_size.y as f32 / 2.0 {
                    if let Some(my_player) = self.players.get(&self.player_id) {
                        if my_player.position.x >= 0.0 && my_player.position.x < 1.0 {
                            self.model.send(Message::StartTheRace);
                        }
                    }
                }
            }
            geng::Event::TouchStart { touches } => {
                self.touches = touches.len();
                self.touch_control = Some(touches[0].position.map(|x| x as f32));
                if self.touch_control.unwrap().y > self.framebuffer_size.y as f32 / 2.0 {
                    if let Some(my_player) = self.players.get(&self.player_id) {
                        if my_player.position.x >= 0.0 && my_player.position.x < 1.0 {
                            self.model.send(Message::StartTheRace);
                        }
                    }
                }
            }
            geng::Event::TouchMove { touches } => {
                self.touches = touches.len();
                self.touch_control = Some(touches[0].position.map(|x| x as f32));
            }
            geng::Event::TouchEnd { touches } => {
                if touches.is_empty() {
                } else {
                    self.touch_control = Some(touches[0].position.map(|x| x as f32));
                }
                if self.touches == 1 {
                    self.touch_control = None;
                }
            }
            geng::Event::MouseMove { position, .. } => {
                if self
                    .geng
                    .window()
                    .is_button_pressed(geng::MouseButton::Left)
                {
                    self.touch_control = Some(position.map(|x| x as f32));
                }
            }
            geng::Event::MouseUp { .. } => {
                self.touch_control = None;
            }
            _ => {}
        }
    }
    fn update(&mut self, delta_time: f64) {
        if self.geng.window().is_key_pressed(geng::Key::PageUp) {
            self.volume += delta_time;
        }
        if self.geng.window().is_key_pressed(geng::Key::PageDown) {
            self.volume -= delta_time;
        }
        self.volume = self.volume.clamp(0.0, 1.0);

        let delta_time = delta_time as f32;
        self.time += delta_time;

        self.update_interpolated(delta_time);

        if let Some(time) = &mut self.explosion_time {
            *time += delta_time;
            if *time > 1.0 {
                self.explosion_time = None;
            }
        }

        for (t, _) in &mut self.spawn_particles {
            *t += delta_time * 3.0;
        }
        self.spawn_particles.retain(|(t, _)| *t < 1.0);

        let mut sounds: Vec<(&[geng::Sound], Vec2<f32>)> = Vec::new();

        if let Some(me) = self.players.get_mut(&self.player_id) {
            if let Some((time, _)) = &mut me.emote {
                *time += delta_time;
                if *time > 1.0 {
                    me.emote = None;
                }
            }

            me.input = 0.0;

            if let Some(pos) = self.touch_control {
                me.input += ((pos.x - self.framebuffer_size.x as f32 / 2.0)
                    / (self.framebuffer_size.x as f32 / 4.0))
                    .clamp(-1.0, 1.0);
            }

            if self.geng.window().is_key_pressed(geng::Key::A)
                || self.geng.window().is_key_pressed(geng::Key::Left)
            {
                me.input -= 1.0;
            }
            if self.geng.window().is_key_pressed(geng::Key::D)
                || self.geng.window().is_key_pressed(geng::Key::Right)
            {
                me.input += 1.0;
            }
        }
        {
            let model = self.model.get();

            let my_player = self.interpolated_players.get(&self.player_id);
            let mut target_player = my_player;
            if my_player.is_none()
                || (!my_player.as_ref().unwrap().is_riding && model.avalanche_position.is_some())
            {
                if let Some(player) = self
                    .interpolated_players
                    .iter()
                    .min_by_key(|player| r32(player.position.y))
                {
                    target_player = Some(player);
                }
            }
            let mut target_center = if let Some(target_player) = target_player {
                target_player.position + target_player.velocity * 0.5
            } else {
                vec2(0.0, 0.0)
            };
            self.camera.center +=
                (target_center - self.camera.center) * (3.0 * delta_time).min(1.0);

            if model.tick != self.last_model_tick {
                self.last_model_tick = model.tick;
                for player in &model.players {
                    if player.id != self.player_id {
                        if self.players.get(&player.id).is_none() {
                            self.spawn_particles.push((0.0, player.position));
                            let mut sfx = self.assets.spawn_sound.effect();
                            sfx.set_volume(self.volume);
                            sfx.play();
                        }
                        self.players.insert(player.clone());
                    }
                }
                for player in &self.players {
                    if player.id != self.player_id && model.players.get(&player.id).is_none() {
                        self.spawn_particles.push((0.0, player.position));
                        let mut sfx = self.assets.spawn_sound.effect();
                        sfx.set_volume(self.volume);
                        sfx.play();
                    }
                }
                self.players.retain(|player| {
                    model.players.get(&player.id).is_some() || player.id == self.player_id
                });
            }
            if model.avalanche_position.is_none() {
                if let Some(player) = self.players.get_mut(&self.player_id) {
                    player.seen_no_avalanche = true;
                }
            }
            if let Some(me) = self.players.get_mut(&self.player_id) {
                if me.seen_no_avalanche && model.avalanche_position.is_some() {
                    if !me.is_riding {
                        for _ in 0..100 {
                            self.explosion_time = Some(0.0);
                            let mut sfx = self.assets.boom_sound.effect();
                            sfx.set_volume(self.volume);
                            sfx.play();
                            break;
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
                        me.is_riding = true;
                    }
                }
                if me.crash_timer > 2.0 {
                    self.model
                        .send(Message::Score((-me.position.y * 100.0) as i32));
                    me.respawn();
                }
            }
            for player in &mut self.players {
                let shape_point = model.track.at(player.position.y);
                if !player.is_riding {
                    player.update_walk(delta_time);
                    player.position.x = player.position.x.clamp(
                        shape_point.safe_left + player.radius,
                        shape_point.safe_right - player.radius,
                    );
                } else {
                    player.update_riding(delta_time);
                    for obstacle in &model.track.obstacles {
                        let delta_pos = player.position - obstacle.position;
                        let peneration = player.radius + obstacle.radius - delta_pos.len();
                        if peneration > 0.0 {
                            let normal = delta_pos.normalize_or_zero();
                            player.position += normal * peneration;
                            player.velocity -= normal * Vec2::dot(player.velocity, normal);
                            if !player.crashed {
                                player.crashed = true;
                                sounds.push((&self.assets.crash_sounds, player.position));
                            }
                        }
                    }
                    if player.position.x < shape_point.left + player.radius
                        || player.position.x > shape_point.right - player.radius
                    {
                        // if player.position.x.abs() > TRACK_WIDTH - player.radius {
                        if !player.crashed {
                            player.crashed = true;
                            sounds.push((&self.assets.crash_sounds, player.position));
                        }
                    }
                    if let Some(position) = model.avalanche_position {
                        if player.position.y > position {
                            if !player.crashed {
                                player.crashed = true;
                                sounds.push((&self.assets.crash_sounds, player.position));
                            }
                        }
                    }
                    player.position.x = player.position.x.clamp(
                        shape_point.left + player.radius,
                        shape_point.right - player.radius,
                    );
                }
            }
            self.next_particle -= delta_time;
            while self.next_particle < 0.0 {
                self.next_particle += 1.0 / 100.0;
                let mut particles = Vec::new();
                for player in &self.interpolated_players {
                    if player.is_riding {
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
                }
                self.particles.extend(particles);
                if let Some(pos) = model.avalanche_position {
                    for _ in 0..10 {
                        self.particles.push(Particle {
                            i_pos: vec2(
                                self.camera.center.x
                                    + global_rng().gen_range(-TRACK_WIDTH..=TRACK_WIDTH),
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
        if let Some(player) = self.players.get(&self.player_id) {
            self.model.send(Message::UpdatePlayer(player.clone()));
        }

        for event in self.model.update() {
            // TODO handle
        }

        for (sounds, pos) in sounds {
            self.play_sound(sounds.choose(&mut global_rng()).unwrap(), pos);
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        let model = self.model.get();
        let my_player = self.players.get(&self.player_id);
        // self
        //     .interpolated_players
        //     .get(&self.player_id)
        //     .unwrap_or(self.players.get(&self.player_id).unwrap());
        let mut target_player = my_player;
        if my_player.is_none()
            || (!my_player.as_ref().unwrap().is_riding && model.avalanche_position.is_some())
        {
            if let Some(player) = self
                .interpolated_players
                .iter()
                .min_by_key(|player| r32(player.position.y))
            {
                target_player = Some(player);
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
        let view_width = view_area.bounding_box().width();
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::Quad::new(
                AABB::<f32>::point(vec2(self.camera.center.x, 0.0))
                    .extend_left(view_width)
                    .extend_right(view_width)
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
        if let Some(my_player) = &my_player {
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
        }

        let framebuffer_size = framebuffer.size();

        let c2 = Color::rgba(0.9, 0.9, 0.95, 0.0);
        let c1 = Color::rgba(0.9, 0.9, 0.95, 0.9);

        // TODO: outside border
        // self.geng.draw_2d(
        //     framebuffer,
        //     &self.camera,
        //     &draw_2d::Quad::new(
        //         AABB::point(vec2(TRACK_WIDTH, 0.0))
        //             .extend_right(TRACK_WIDTH * 5.0)
        //             .extend_up(self.camera.center.y - self.camera.fov),
        //         c1,
        //     ),
        // );
        // self.geng.draw_2d(
        //     framebuffer,
        //     &self.camera,
        //     &draw_2d::Quad::new(
        //         AABB::point(vec2(-TRACK_WIDTH, 0.0))
        //             .extend_right(-TRACK_WIDTH * 5.0)
        //             .extend_up(self.camera.center.y - self.camera.fov),
        //         c1,
        //     ),
        // );

        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::Chain::new(
                Chain::new(
                    model
                        .track
                        .shape
                        .iter()
                        .map(|point| vec2(point.safe_left, point.y))
                        .collect(),
                ),
                0.1,
                Color::rgba(0.0, 0.0, 0.0, 0.3),
                0,
            ),
        );
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::Chain::new(
                Chain::new(
                    model
                        .track
                        .shape
                        .iter()
                        .map(|point| vec2(point.safe_right, point.y))
                        .collect(),
                ),
                0.1,
                Color::rgba(0.0, 0.0, 0.0, 0.3),
                0,
            ),
        );
        if true {
            const OFF: f32 = 2.0;
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedPolygon::strip(
                    model
                        .track
                        .shape
                        .windows(2)
                        .flat_map(|window| {
                            let a = &window[0];
                            let b = &window[1];
                            let n = -(vec2(b.left, b.y) - vec2(a.left, a.y))
                                .rotate_90()
                                .normalize();
                            [
                                draw_2d::TexturedVertex {
                                    a_pos: vec2(a.left, a.y),
                                    a_color: Color::WHITE,
                                    a_vt: vec2(0.0, a.left_len / 2.0),
                                },
                                draw_2d::TexturedVertex {
                                    a_pos: vec2(a.left, a.y) + n * OFF,
                                    a_color: Color::WHITE,
                                    a_vt: vec2(1.0, a.left_len / 2.0),
                                },
                            ]
                        })
                        .collect(),
                    &self.assets.border,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedPolygon::strip(
                    model
                        .track
                        .shape
                        .windows(2)
                        .flat_map(|window| {
                            let a = &window[0];
                            let b = &window[1];
                            let n = (vec2(b.right, b.y) - vec2(a.right, a.y))
                                .rotate_90()
                                .normalize();
                            [
                                draw_2d::TexturedVertex {
                                    a_pos: vec2(a.right, a.y),
                                    a_color: Color::WHITE,
                                    a_vt: vec2(0.0, a.right_len / 2.0),
                                },
                                draw_2d::TexturedVertex {
                                    a_pos: vec2(a.right, a.y) + n * OFF,
                                    a_color: Color::WHITE,
                                    a_vt: vec2(1.0, a.right_len / 2.0),
                                },
                            ]
                        })
                        .collect(),
                    &self.assets.border,
                ),
            );
        } else {
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

        for player in &self.interpolated_players {
            self.draw_shadow(
                framebuffer,
                Mat3::translate(player.position) * Mat3::scale_uniform(player.radius),
                Color::rgba(0.5, 0.5, 0.5, 0.5),
            );
        }
        if true || self.players.get(&self.player_id).unwrap().is_riding {
            for obstacle in &model.track.obstacles {
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

        for player in &self.interpolated_players {
            self.draw_player(framebuffer, player);
        }

        for &(t, pos) in &self.spawn_particles {
            self.draw_texture(
                framebuffer,
                &self.assets.spawn,
                Mat3::translate(pos + vec2(0.0, 0.5)) * Mat3::scale_uniform(t),
                Color::rgba(0.5, 0.5, 1.0, 1.0 - t),
            );
        }

        if true || self.players.get(&self.player_id).unwrap().is_riding {
            for obstacle in &model.track.obstacles {
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
                        a_pos: vec2(self.camera.center.x - view_width, position - 3.0),
                        a_color: c2,
                    },
                    draw_2d::ColoredVertex {
                        a_pos: vec2(self.camera.center.x - view_width, position),
                        a_color: c1,
                    },
                    draw_2d::ColoredVertex {
                        a_pos: vec2(self.camera.center.x + view_width, position),
                        a_color: c1,
                    },
                    draw_2d::ColoredVertex {
                        a_pos: vec2(self.camera.center.x + view_width, position - 3.0),
                        a_color: c2,
                    },
                ]),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(
                    AABB::point(vec2(self.camera.center.x, position))
                        .extend_left(TRACK_WIDTH * 5.0)
                        .extend_right(TRACK_WIDTH * 5.0)
                        .extend_up(100.0),
                    c1,
                ),
            );
        }
        if let Some(my_player) = &my_player {
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
        }
        if self.show_player_names {
            for player in &self.interpolated_players {
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    player.position + vec2(0.0, 1.0),
                    0.5,
                    &player.name,
                    0.5,
                    Color::WHITE,
                );
            }
        }
        if let Some(pos) = model.avalanche_position {
            let pos = pos - self.camera.center.y - self.camera.fov / 2.0;
            // if pos > 1.0 {
            let alpha = (1.0 - (pos - 1.0) / 5.0).clamp(0.0, 1.0);
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::colored(
                    AABB::<f32>::point(self.camera.center + vec2(0.0, 8.0))
                        .extend_symmetric(self.assets.ava_warning.size().map(|x| x as f32) * 0.05),
                    &self.assets.ava_warning,
                    Color::rgba(1.0, 1.0, 1.0, alpha),
                ),
            );
            // }
            // if pos > 1.0 {
            //     self.assets.font.draw(
            //         framebuffer,
            //         &self.camera,
            //         self.camera.center + vec2(0.0, 8.0),
            //         1.0,
            //         &format!("avalanche is {}m behind", pos as i32),
            //         0.5,
            //     );
            // }
        } else if let Some((name, score)) = &model.winner {
            if self.show_player_names {
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    self.camera.center + vec2(0.0, 8.0),
                    1.0,
                    &format!("winner is {}", name),
                    0.5,
                    Color::WHITE,
                );
            }
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                self.camera.center + vec2(0.0, 7.0),
                1.0,
                &format!("winner scored {}", (*score * 100.0) as i32),
                0.5,
                Color::WHITE,
            );
        }
        if let Some(target_player) = target_player {
            if target_player.is_riding {
                self.ride_sound_effect.set_volume(
                    (target_player.velocity.len() / Player::MAX_SPEED * 0.05
                        + target_player.ride_volume.min(1.0) * 0.1) as f64
                        * self.volume,
                );
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    self.camera.center + vec2(0.0, -9.0),
                    1.0,
                    &format!("score {}", (-target_player.position.y * 100.0) as i32),
                    0.5,
                    Color::WHITE,
                );
                // self.assets.font.draw(
                //     framebuffer,
                //     &self.camera,
                //     self.camera.center + vec2(0.0, -10.0),
                //     1.0,
                //     &format!("speed {}m per second", (-target_player.velocity.y) as i32),
                //     0.5,
                // );
            } else {
                self.ride_sound_effect.set_volume(0.0);
            }
        } else {
            self.ride_sound_effect.set_volume(0.0);
        }
        if let Some(pos) = model.avalanche_position {
            self.avalanche_sound_effect.set_volume(
                (1.0 - ((pos - self.camera.center.y).abs() * 2.0 / self.camera.fov).powf(1.0))
                    .clamp(0.0, 1.0) as f64
                    * self.volume,
            );
        } else {
            self.avalanche_sound_effect.set_volume(0.0);
        }

        if let Some(music) = &mut self.music {
            music.set_volume(self.volume);
        }

        if let Some(time) = self.explosion_time {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::colored(
                    AABB::<f32>::point(vec2(0.0, 5.0)).extend_symmetric(
                        self.assets.boom.size().map(|x| x as f32) * 0.05 * (1.0 + time),
                    ),
                    &self.assets.boom,
                    Color::rgba(1.0, 1.0, 1.0, 1.0 - time.sqr()),
                ),
            );
        }
    }
}

#[derive(clap::Parser, Clone)]
struct Opt {
    #[clap(long)]
    addr: Option<String>,
    #[clap(long)]
    server: bool,
    #[clap(long)]
    with_server: bool,
    #[clap(long)]
    spectator: bool,
    #[clap(long)]
    auto_sound: bool,
}

impl Opt {
    pub fn addr(&self) -> &str {
        match &self.addr {
            Some(addr) => addr,
            None => option_env!("SERVER_ADDR").unwrap_or("127.0.0.1:1155"),
        }
    }
}

fn main() {
    // logger::init().unwrap();
    let opt: Opt = program_args::parse();
    let model_constructor = Model::new;
    let game_constructor = {
        let opt = opt.clone();
        move |geng: &Geng, player_id, model| {
            geng::LoadingScreen::new(
                &geng,
                geng::EmptyLoadingScreen,
                <Assets as geng::LoadAsset>::load(&geng, &static_path()),
                {
                    let geng = geng.clone();
                    move |assets| {
                        let mut assets = assets.expect("Failed to load assets");
                        assets.border.set_wrap_mode(ugli::WrapMode::Repeat);
                        assets.ride_sound.looped = true;
                        assets.avalanche_sound.looped = true;
                        assets.music.looped = true;
                        if opt.spectator {
                            Box::new(Game::new(
                                &geng,
                                &Rc::new(assets),
                                player_id,
                                None,
                                None,
                                model,
                                opt.auto_sound,
                            )) as Box<dyn geng::State>
                        } else {
                            Box::new(Lobby::new(&geng, &Rc::new(assets), player_id, model))
                        }
                    }
                },
            )
        }
    };
    if opt.server {
        #[cfg(not(target_arch = "wasm32"))]
        simple_net::Server::new(opt.addr(), model_constructor()).run();
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        let server = if opt.with_server {
            let server = simple_net::Server::new(opt.addr(), model_constructor());
            let server_handle = server.handle();
            let server_thread = std::thread::spawn(move || {
                server.run();
            });
            Some((server_handle, server_thread))
        } else {
            None
        };

        let geng = Geng::new("Extremely Extreme Sports");
        let state = simple_net::ConnectingState::new(&geng, opt.addr(), {
            let geng = geng.clone();
            move |player_id, model| game_constructor(&geng, player_id, model)
        });
        geng::run(&geng, state);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some((server_handle, server_thread)) = server {
            server_handle.shutdown();
            server_thread.join().unwrap();
        }
    }
}
