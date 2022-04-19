use super::*;

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
    config: PlayerConfig,
    keyboard: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PlayerConfig {
    pub hat: usize,
    pub coat: usize,
    pub pants: usize,
    pub equipment: usize,
    pub face: usize,
    pub custom: Option<String>,
}

impl PlayerConfig {
    pub fn random(assets: &PlayerAssets) -> Self {
        Self {
            hat: global_rng().gen_range(0..4),       // assets.hat.len()),
            coat: global_rng().gen_range(0..4),      // assets.coat.len()),
            pants: global_rng().gen_range(0..4),     // assets.pants.len()),
            face: global_rng().gen_range(0..4),      // assets.face.len()),
            equipment: global_rng().gen_range(0..2), // assets.equipment.len()),
            custom: None,
        }
    }
}

impl Lobby {
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        player_id: Id,
        model: simple_net::Remote<Model>,
    ) -> Self {
        Self {
            framebuffer_size: vec2(1, 1),
            geng: geng.clone(),
            camera: geng::Camera2d {
                center: vec2(0.0, 0.5),
                rotation: 0.0,
                fov: 2.0,
            },
            keyboard: false,
            assets: assets.clone(),
            player_id,
            model: Some(model),
            transition: None,
            name: match autosave::load("player_name.txt") {
                Some(name) => name,
                None => String::new(),
            },
            mouse: Vec2::ZERO,
            config: match autosave::load("player.json") {
                Some(config) => config,
                None => PlayerConfig::random(&assets.player),
            },
        }
    }
    fn buttons(&self) -> Vec<AABB<f32>> {
        if self.keyboard {
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
            for _ in "1234567890".chars() {
                result.push(AABB::point(vec2(x, y)).extend_positive(vec2(1.0, 1.0) * size));
                x += size;
            }
            x = initial_x;
            y -= size;
            for _ in "qwertyuiop".chars() {
                result.push(AABB::point(vec2(x, y)).extend_positive(vec2(1.0, 1.0) * size));
                x += size;
            }
            x = initial_x;
            y -= size;
            for _ in "asdfghjkl".chars() {
                result.push(AABB::point(vec2(x, y)).extend_positive(vec2(1.0, 1.0) * size));
                x += size;
            }
            x = initial_x;
            y -= size;
            for _ in "zxcvbnm".chars() {
                result.push(AABB::point(vec2(x, y)).extend_positive(vec2(1.0, 1.0) * size));
                x += size;
            }
            x = initial_x;
            y -= size;
            result.push(
                AABB::point(vec2(initial_x + 5.0 * size, initial_y + 2.0 * size))
                    .extend_positive(vec2("delete".len() as f32, 1.0) * size),
            );
            result
        } else {
            let size = 0.1;
            let mut result = vec![
                AABB::point(vec2(0.0, 0.8)).extend_positive(vec2("hat".len() as f32, 1.0) * size),
                AABB::point(vec2(0.0, 0.6)).extend_positive(vec2("face".len() as f32, 1.0) * size),
                AABB::point(vec2(0.0, 0.4)).extend_positive(vec2("coat".len() as f32, 1.0) * size),
                AABB::point(vec2(0.0, 0.2)).extend_positive(vec2("pants".len() as f32, 1.0) * size),
                AABB::point(vec2(0.0, 0.0))
                    .extend_positive(vec2("equipment".len() as f32, 1.0) * size),
                AABB::point(vec2(-0.5, -0.2))
                    .extend_positive(vec2("random".len() as f32, 1.0) * size),
                AABB::point(vec2(0.5, -0.4))
                    .extend_positive(vec2("play".len() as f32, 1.0) * size * 2.0),
            ];
            if self.assets.player.custom.contains_key(&self.name)
                || self.name == "potkirland"
                || self.name == "jitspoe"
            {
                result.push(
                    AABB::point(vec2(0.0, 1.3))
                        .extend_positive(vec2("secret".len() as f32, 1.0) * size * 1.0),
                );
            }
            result
        }
    }

    fn press_button(&mut self, index: usize) {
        if self.keyboard {
            if let Some(c) = "1234567890qwertyuiopasdfghjklzxcvbnm"
                .chars()
                .skip(index)
                .next()
            {
                if self.name.len() < 15 {
                    self.name.push(c);
                }
            } else {
                self.name.pop();
            }
            return;
        }
        // "hat", "face", "coat", "pants", "equipment"
        match index {
            0 => {
                self.config.hat += 1;
                self.config.hat %= 4; // self.assets.player.hat.len();
            }
            1 => {
                self.config.face += 1;
                self.config.face %= 4; //self.assets.player.face.len();
            }
            2 => {
                self.config.coat += 1;
                self.config.coat %= 4; // self.assets.player.coat.len();
            }
            3 => {
                self.config.pants += 1;
                self.config.pants %= 4; // self.assets.player.pants.len();
            }
            4 => {
                self.config.equipment += 1;
                self.config.equipment %= 2; // self.assets.player.equipment.len();
            }
            5 => {
                self.config = PlayerConfig::random(&self.assets.player);
            }
            6 => {
                self.transition = Some(geng::Transition::Switch(Box::new(Game::new(
                    &self.geng,
                    &self.assets,
                    self.player_id,
                    if self.name.is_empty() {
                        "unnamed".to_owned()
                    } else {
                        self.name.clone()
                    },
                    self.config.clone(),
                    self.model.take().unwrap(),
                ))));
            }
            7 => {
                if self.assets.player.custom.contains_key(&self.name) {
                    self.config.custom = Some(self.name.clone());
                }
                if self.name == "6fu" {
                    self.config.equipment = 3;
                }
                if self.name == "kidgiraffe" {
                    self.config.equipment = 2;
                }
                if self.name == "potkirland" {
                    self.config.pants = 4;
                    self.config.coat = 4;
                    self.config.face = 0;
                    self.config.hat = 3;
                }
                if self.name == "wendel" {
                    self.config.equipment = 5;
                }
                if self.name == "jared" {
                    self.config.equipment = 6;
                }
                if self.name == "jitspoe" {
                    self.config.equipment = 7;
                    self.config.pants = 3;
                    self.config.coat = 1;
                    self.config.face = 3;
                    self.config.hat = 0;
                }
            }
            _ => unreachable!(),
        }
        autosave::save("player.json", &self.config);
        autosave::save("player_name.txt", &self.name);
    }
}

impl geng::State for Lobby {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        self.camera.fov =
            2.0f32.max(2.7 * framebuffer.size().y as f32 / framebuffer.size().x as f32);
        ugli::clear(framebuffer, Some(Color::WHITE), None);

        let buttons = self.buttons();

        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::unit(&self.assets.player.equipment[self.config.equipment])
                .transform(Mat3::rotate(0.1))
                .translate(vec2(-0.5, 1.0)),
        );
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::unit(&self.assets.player.assemble(&self.geng, &self.config))
                .translate(vec2(-0.5, 0.0)),
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
        if !self.keyboard {
            for (button, text) in buttons.into_iter().zip([
                "hat",
                "face",
                "coat",
                "pants",
                "equipment",
                "random",
                "play",
                "secret",
            ]) {
                let mut pos = button.bottom_left();
                if button.contains(self.mouse)
                    && self
                        .geng
                        .window()
                        .is_button_pressed(geng::MouseButton::Left)
                {
                    pos.y -= button.height() * 0.2;
                }
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    pos,
                    button.height(),
                    text,
                    0.0,
                    if button.contains(self.mouse) {
                        Color::rgb(0.5, 0.5, 1.0)
                    } else {
                        Color::WHITE
                    },
                );
            }
        } else {
            for (button, text) in buttons.into_iter().zip(
                "1234567890qwertyuiopasdfghjklzxcvbnm"
                    .chars()
                    .map(|c| c.to_string())
                    .chain(std::iter::once("delete".to_owned())),
            ) {
                let mut pos = button.bottom_left();
                if button.contains(self.mouse)
                    && self
                        .geng
                        .window()
                        .is_button_pressed(geng::MouseButton::Left)
                {
                    pos.y -= button.height() * 0.2;
                }
                self.assets.font.draw(
                    framebuffer,
                    &self.camera,
                    pos,
                    button.height(),
                    &text,
                    0.0,
                    if button.contains(self.mouse) {
                        Color::rgb(0.5, 0.5, 1.0)
                    } else {
                        Color::WHITE
                    },
                );
            }
        }
    }

    fn update(&mut self, delta_time: f64) {
        if let Some(model) = &mut self.model {
            model.update();
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => {
                match key {
                    geng::Key::Space => {}
                    geng::Key::Backspace => {
                        self.name.pop();
                    }
                    _ => {
                        if self.name.len() < 15 {
                            let c = format!("{:?}", key);
                            let c = if c.len() == 1 {
                                Some(c.to_lowercase().chars().next().unwrap())
                            } else if let Some(c) = c.strip_prefix("Num") {
                                Some(c.chars().next().unwrap())
                            } else {
                                None
                            };
                            if let Some(c) = c {
                                self.name.push(c)
                            }
                        }
                    }
                }
                autosave::save("player_name.txt", &self.name);
            }
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
                    self.keyboard = !self.keyboard;
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
                    self.keyboard = !self.keyboard;
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
                if let Some(index) = self
                    .buttons()
                    .iter()
                    .position(|button| button.contains(self.mouse))
                {
                    self.press_button(index);
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
