use super::*;

pub enum UiMessage {
    Input(char),
    Delete,
    Back,
    RandomSkin,
    ChangeHat,
    ChangeFace,
    ChangeCoat,
    ChangePants,
    ChangeEquipment,
    SecretSkin,
    Leaderboard,
    Play,
    Customize,
    Spectate,
    JoinDiscord,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Main,
    Leaderboard,
    Customizer,
    Keyboard,
}

pub struct Lobby {
    geng: Geng,
    framebuffer_size: Vec2<usize>,
    assets: Rc<Assets>,
    player_id: Id,
    model: Option<simple_net::Remote<Model>>,
    transition: Option<geng::Transition>,
    name: String,
    camera: geng::Camera2d,
    mouse: Vec2<f32>,
    config: skin::Config,
    state: State,
    skin_renderer: skin::Renderer,
}

impl Lobby {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        player_id: Id,
        model: simple_net::Remote<Model>,
    ) -> Self {
        let random_config = skin::Config::random(&assets.player);
        let config = match autosave::load("player.json") {
            Some(mut config) => {
                let correct = |config: &skin::Config| -> bool {
                    if let Some(name) = &config.secret {
                        if !assets.player.secret.contains_key(name) {
                            return false;
                        }
                    }
                    if let Some(name) = &config.hat {
                        if !assets.player.hat.contains_key(name) {
                            return false;
                        }
                    }
                    if let Some(name) = &config.face {
                        if !assets.player.face.contains_key(name) {
                            return false;
                        }
                    }
                    if let Some(name) = &config.coat {
                        if !assets.player.coat.contains_key(name) {
                            return false;
                        }
                    }
                    if let Some(name) = &config.pants {
                        if !assets.player.pants.contains_key(name) {
                            return false;
                        }
                    }
                    if let Some(name) = &config.equipment {
                        if !assets.player.equipment.contains_key(name) {
                            return false;
                        }
                    }
                    true
                };
                if !correct(&config) {
                    config = random_config;
                }
                config
            }
            None => random_config,
        };
        Self {
            framebuffer_size: vec2(1, 1),
            geng: geng.clone(),
            camera: geng::Camera2d {
                center: vec2(0.0, 0.5),
                rotation: 0.0,
                fov: 2.0,
            },
            assets: assets.clone(),
            player_id,
            model: Some(model),
            transition: None,
            name: match autosave::load("player_name.txt") {
                Some(name) => name,
                None => String::new(),
            },
            mouse: Vec2::ZERO,
            skin_renderer: skin::Renderer::new(geng, &config, assets),
            config,
            state: State::Main,
        }
    }
    fn handle_ui(&mut self, message: UiMessage) {
        fn change_skin_item<T>(item: &mut Option<String>, options: &HashMap<String, T>) {
            let options: Vec<&str> = options.keys().map(|s| s.as_str()).collect();
            if let Some(name) = item {
                let current = options.iter().position(|s| s == name).unwrap_or(0);
                *name = options[(current + 1) % options.len()].to_owned();
            }
        }
        match message {
            UiMessage::Input(c) => {
                if self.name.len() < 15 {
                    self.name.push(c);
                }
            }
            UiMessage::Delete => {
                self.name.pop();
            }
            UiMessage::Back => self.state = State::Main,
            UiMessage::RandomSkin => self.config = skin::Config::random(&self.assets.player),
            UiMessage::ChangeHat => change_skin_item(&mut self.config.hat, &self.assets.player.hat),
            UiMessage::ChangeFace => {
                change_skin_item(&mut self.config.face, &self.assets.player.face);
            }
            UiMessage::ChangeCoat => {
                change_skin_item(&mut self.config.coat, &self.assets.player.coat);
            }
            UiMessage::ChangePants => {
                change_skin_item(&mut self.config.pants, &self.assets.player.pants);
            }
            UiMessage::ChangeEquipment => {
                change_skin_item(&mut self.config.equipment, &self.assets.player.equipment);
            }
            UiMessage::SecretSkin => {
                if let Some(config) = self.assets.player.secret.get(&self.name) {
                    self.config = skin::Config {
                        secret: if config.parts.is_some() {
                            Some(self.name.clone())
                        } else {
                            None
                        },
                        hat: config.hat.clone(),
                        coat: config.coat.clone(),
                        pants: config.pants.clone(),
                        equipment: config.equipment.clone(),
                        face: config.face.clone(),
                    };
                }
            }
            UiMessage::Leaderboard => self.state = State::Leaderboard,
            UiMessage::Play => {
                self.transition = Some(geng::Transition::Switch(Box::new(Game::new(
                    &self.geng,
                    &self.assets,
                    self.player_id,
                    Some(if self.name.is_empty() {
                        "unnamed".to_owned()
                    } else {
                        self.name.clone()
                    }),
                    Some(self.config.clone()),
                    self.model.take().unwrap(),
                    false,
                ))));
            }
            UiMessage::Customize => {
                self.state = State::Customizer;
            }
            UiMessage::Spectate => {
                self.transition = Some(geng::Transition::Switch(Box::new(Game::new(
                    &self.geng,
                    &self.assets,
                    self.player_id,
                    None,
                    None,
                    self.model.take().unwrap(),
                    true,
                ))));
            }
            UiMessage::JoinDiscord => {
                #[cfg(target_arch = "wasm32")]
                {
                    if let Ok(Some(w)) = web_sys::window()
                        .unwrap()
                        .open_with_url_and_target(DISCORD_LINK, "_blank")
                    {
                        w.focus();
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    open::that(DISCORD_LINK).unwrap();
                }
            }
        }
        self.skin_renderer = skin::Renderer::new(&self.geng, &self.config, &self.assets);
        autosave::save("player.json", &self.config);
        autosave::save("player_name.txt", &self.name);
    }
    fn buttons(&self) -> Vec<ui::Button<UiMessage>> {
        match self.state {
            State::Keyboard => {
                let mut result = vec![];
                let mut initial_x = 0.0;
                let mut initial_y = 0.5;
                let mut size = 0.1;
                if self.framebuffer_size.x < self.framebuffer_size.y {
                    initial_x = -1.25;
                    initial_y = -0.5;
                    size = 0.25;
                }
                let mut x = initial_x;
                let mut y = initial_y;
                for c in "1234567890".chars() {
                    result.push(ui::Button::new(
                        &c.to_string(),
                        vec2(x, y),
                        size,
                        0.0,
                        UiMessage::Input(c),
                    ));
                    x += size;
                }
                x = initial_x;
                y -= size;
                for c in "qwertyuiop".chars() {
                    result.push(ui::Button::new(
                        &c.to_string(),
                        vec2(x, y),
                        size,
                        0.0,
                        UiMessage::Input(c),
                    ));
                    x += size;
                }
                x = initial_x;
                y -= size;
                for c in "asdfghjkl".chars() {
                    result.push(ui::Button::new(
                        &c.to_string(),
                        vec2(x, y),
                        size,
                        0.0,
                        UiMessage::Input(c),
                    ));
                    x += size;
                }
                x = initial_x;
                y -= size;
                for c in "zxcvbnm".chars() {
                    result.push(ui::Button::new(
                        &c.to_string(),
                        vec2(x, y),
                        size,
                        0.0,
                        UiMessage::Input(c),
                    ));
                    x += size;
                }
                x = initial_x;
                y -= size;
                result.push(ui::Button::new(
                    "delete",
                    vec2(initial_x + 5.0 * size, initial_y + 2.0 * size),
                    size,
                    0.5,
                    UiMessage::Delete,
                ));
                result.push(ui::Button::new(
                    "back",
                    vec2(initial_x + 6.0 * size, initial_y + 4.0 * size),
                    size,
                    0.5,
                    UiMessage::Back,
                ));
                result
            }
            State::Customizer => {
                let size = 0.1;
                let mut result = vec![
                    ui::Button::new("hat", vec2(0.0, 0.8), size, 0.0, UiMessage::ChangeHat),
                    ui::Button::new("face", vec2(0.0, 0.6), size, 0.0, UiMessage::ChangeFace),
                    ui::Button::new("coat", vec2(0.0, 0.4), size, 0.0, UiMessage::ChangeCoat),
                    ui::Button::new("pants", vec2(0.0, 0.2), size, 0.0, UiMessage::ChangePants),
                    ui::Button::new(
                        "equipment",
                        vec2(0.0, 0.0),
                        size,
                        0.0,
                        UiMessage::ChangeEquipment,
                    ),
                    ui::Button::new("random", vec2(-0.5, -0.2), size, 0.0, UiMessage::RandomSkin),
                    ui::Button::new("back", vec2(0.5, -0.4), size * 2.0, 0.0, UiMessage::Back),
                ];
                if self.assets.player.secret.contains_key(&self.name) {
                    result.push(ui::Button::new(
                        "secret",
                        vec2(0.0, 1.3),
                        size,
                        0.0,
                        UiMessage::SecretSkin,
                    ));
                }
                result
            }
            State::Leaderboard => {
                let size = 0.1;
                let mut result = vec![ui::Button::new(
                    "secret",
                    vec2(0.0, -0.3),
                    size,
                    0.0,
                    UiMessage::SecretSkin,
                )];
                result
            }
            State::Main => {
                let size = 0.1;
                let mut result = vec![
                    ui::Button::new("customize", vec2(0.0, 0.8), size, 0.0, UiMessage::Customize),
                    ui::Button::new("play", vec2(0.0, 0.4), size * 2.0, 0.0, UiMessage::Play),
                    ui::Button::new("spectate", vec2(0.0, 0.0), size, 0.0, UiMessage::Spectate),
                    ui::Button::new(
                        "join discord",
                        vec2(0.0, -0.3),
                        size,
                        0.0,
                        UiMessage::JoinDiscord,
                    ),
                    ui::Button::new(
                        "leaderboard",
                        vec2(0.0, 0.2),
                        size,
                        0.0,
                        UiMessage::Leaderboard,
                    ),
                ];
                if self.assets.player.secret.contains_key(&self.name) {
                    result.push(ui::Button::new(
                        "secret",
                        vec2(0.0, 1.3),
                        size,
                        0.0,
                        UiMessage::SecretSkin,
                    ));
                }
                result
            }
        }
    }
}

impl geng::State for Lobby {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        self.camera.fov =
            2.0f32.max(2.7 * framebuffer.size().y as f32 / framebuffer.size().x as f32);
        ugli::clear(framebuffer, Some(Color::WHITE), None);

        match self.state {
            State::Leaderboard => {
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    vec2(0.0, 1.2),
                    0.2,
                    "leaderboard",
                    0.5,
                    Color::GRAY,
                );
                if let Some(model) = &self.model {
                    let mut rows = Vec::new();
                    let model = model.get();
                    for (name, score) in &model.highscores {
                        rows.push((name.clone(), *score));
                    }
                    rows.sort_by_key(|(name, score)| (-*score, name.clone()));
                    while rows.len() > 10 {
                        rows.pop();
                    }
                    if !rows.iter().any(|(name, _score)| name == &self.name) {
                        if let Some(&score) = model.highscores.get(&self.name) {
                            rows.push((self.name.clone(), score));
                        }
                    }
                    let mut y = 1.0;
                    let highlight = Color::rgb(0.5, 0.5, 1.0);
                    for (index, (name, score)) in rows.iter().enumerate() {
                        if index == 10 {
                            y -= 0.1;
                        }
                        let color = if *name == self.name {
                            highlight
                        } else {
                            Color::WHITE
                        };
                        if index < 10 {
                            self.assets.font.draw(
                                framebuffer,
                                &self.camera,
                                vec2(-1.0, y),
                                0.1,
                                &(index + 1).to_string(),
                                1.0,
                                color,
                            );
                        }
                        y -= 0.1;
                    }
                    let mut y = 1.0;
                    for (index, (name, score)) in rows.iter().enumerate() {
                        if index == 10 {
                            y -= 0.1;
                        }
                        let color = if *name == self.name {
                            highlight
                        } else {
                            Color::WHITE
                        };
                        self.assets.font.draw(
                            framebuffer,
                            &self.camera,
                            vec2(-0.9, y),
                            0.1,
                            name,
                            0.0,
                            color,
                        );
                        y -= 0.1;
                    }
                    let mut y = 1.0;
                    for (index, (name, score)) in rows.iter().enumerate() {
                        if index == 10 {
                            y -= 0.1;
                        }

                        let color = if *name == self.name {
                            highlight
                        } else {
                            Color::WHITE
                        };
                        self.assets.font.draw(
                            framebuffer,
                            &self.camera,
                            vec2(1.0, y),
                            0.1,
                            &score.to_string(),
                            1.0,
                            color,
                        );
                        y -= 0.1;
                    }
                }
            }
            _ => {
                // Draw player
                self.skin_renderer.draw(
                    framebuffer,
                    &self.camera,
                    &skin::DrawInstance {
                        position: vec2(-0.5, 0.0),
                        rotation: 0.0,
                        velocity: Vec2::ZERO,
                        crashed: false,
                        crash_timer: 0.0,
                        ski_velocity: Vec2::ZERO,
                        ski_rotation: 0.0,
                        is_riding: false,
                        crash_position: Vec2::ZERO,
                    },
                );

                let c = if AABB::point(vec2(0.5, 1.1))
                    .extend_up(0.1)
                    .extend_left(2.0)
                    .extend_right(2.0)
                    .contains(self.mouse)
                {
                    Some(Color::rgb(0.5, 0.5, 1.0))
                } else {
                    None
                };
                if self.name.is_empty() {
                    self.assets.font.draw(
                        framebuffer,
                        &self.camera,
                        vec2(0.5, 1.1),
                        0.1,
                        "type your name",
                        0.5,
                        c.unwrap_or(Color::RED),
                    );
                } else {
                    self.assets.font.draw(
                        framebuffer,
                        &self.camera,
                        vec2(0.5, 1.1),
                        0.1,
                        &self.name,
                        0.5,
                        c.unwrap_or(Color::WHITE),
                    );
                }
            }
        }

        for button in self.buttons() {
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
                    Color::rgb(0.5, 0.5, 1.0)
                } else {
                    Color::WHITE
                },
            );
        }
    }

    fn update(&mut self, delta_time: f64) {
        if let Some(model) = &mut self.model {
            model.update();
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Space => {}
                geng::Key::Backspace => {
                    self.handle_ui(UiMessage::Delete);
                }
                _ => {
                    let c = format!("{:?}", key);
                    let c = if c.len() == 1 {
                        Some(c.to_lowercase().chars().next().unwrap())
                    } else if let Some(c) = c.strip_prefix("Num") {
                        Some(c.chars().next().unwrap())
                    } else {
                        None
                    };
                    if let Some(c) = c {
                        self.handle_ui(UiMessage::Input(c));
                    }
                }
            },
            geng::Event::MouseDown {
                position,
                button: geng::MouseButton::Left,
            } => {
                self.mouse = self.camera.screen_to_world(
                    self.framebuffer_size.map(|x| x as f32),
                    position.map(|x| x as f32),
                );
                if AABB::point(vec2(0.5, 1.1))
                    .extend_up(0.1)
                    .extend_left(2.0)
                    .extend_right(2.0)
                    .contains(self.mouse)
                {
                    self.state = match self.state {
                        State::Keyboard => State::Main,
                        _ => State::Keyboard,
                    };
                }
            }
            geng::Event::MouseMove { position, .. } => {
                self.mouse = self.camera.screen_to_world(
                    self.framebuffer_size.map(|x| x as f32),
                    position.map(|x| x as f32),
                );
            }
            geng::Event::TouchStart { touches } => {
                self.mouse = self.camera.screen_to_world(
                    self.framebuffer_size.map(|x| x as f32),
                    touches[0].position.map(|x| x as f32),
                );
                if AABB::point(vec2(0.5, 1.1))
                    .extend_up(0.1)
                    .extend_left(2.0)
                    .extend_right(2.0)
                    .contains(self.mouse)
                {
                    self.state = match self.state {
                        State::Keyboard => State::Main,
                        _ => State::Keyboard,
                    };
                }
            }
            geng::Event::TouchMove { touches } => {
                self.mouse = self.camera.screen_to_world(
                    self.framebuffer_size.map(|x| x as f32),
                    touches[0].position.map(|x| x as f32),
                );
            }
            geng::Event::MouseUp {
                button: geng::MouseButton::Left,
                ..
            }
            | geng::Event::TouchEnd { .. } => {
                if let Some(button) = self
                    .buttons()
                    .into_iter()
                    .find(|button| button.aabb().contains(self.mouse))
                {
                    self.handle_ui(button.message);
                }
            }
            _ => {}
        }
    }

    fn transition(&mut self) -> Option<geng::Transition> {
        self.transition.take()
    }

    fn ui<'a>(&'a mut self, cx: &'a geng::ui::Controller) -> Box<dyn geng::ui::Widget + 'a> {
        #![allow(unused_variables)]
        Box::new(geng::ui::Void)
    }
}
