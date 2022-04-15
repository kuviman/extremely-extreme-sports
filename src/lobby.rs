use super::*;

pub struct Lobby {
    geng: Geng,
    assets: Rc<Assets>,
    player_id: Id,
    model: Option<simple_net::Remote<Model>>,
    transition: Option<geng::Transition>,
    name: String,
    camera: geng::Camera2d,
    mouse: Vec2<f32>,
    config: PlayerConfig,
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
            name: String::new(),
            mouse: Vec2::ZERO,
            config: PlayerConfig::random(&assets.player),
        }
    }
    fn buttons(&self) -> Vec<AABB<f32>> {
        let size = 0.1;
        let mut result = vec![
            AABB::point(vec2(0.0, 0.8)).extend_positive(vec2("hat".len() as f32, 1.0) * size),
            AABB::point(vec2(0.0, 0.6)).extend_positive(vec2("face".len() as f32, 1.0) * size),
            AABB::point(vec2(0.0, 0.4)).extend_positive(vec2("coat".len() as f32, 1.0) * size),
            AABB::point(vec2(0.0, 0.2)).extend_positive(vec2("pants".len() as f32, 1.0) * size),
            AABB::point(vec2(0.0, 0.0)).extend_positive(vec2("equipment".len() as f32, 1.0) * size),
            AABB::point(vec2(-0.5, -0.2)).extend_positive(vec2("random".len() as f32, 1.0) * size),
            AABB::point(vec2(0.5, -0.4))
                .extend_positive(vec2("play".len() as f32, 1.0) * size * 2.0),
        ];
        if self.assets.player.custom.contains_key(&self.name) {
            result.push(
                AABB::point(vec2(0.0, 1.3))
                    .extend_positive(vec2("secret".len() as f32, 1.0) * size * 1.0),
            );
        }
        result
    }
}

impl geng::State for Lobby {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
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
        if self.name.is_empty() {
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                vec2(0.5, 1.1),
                0.1,
                "type your name",
                0.5,
                Color::RED,
            );
        } else {
            self.assets.font.draw(
                framebuffer,
                &self.camera,
                vec2(0.5, 1.1),
                0.1,
                &self.name,
                0.5,
                Color::WHITE,
            );
        }
        self.mouse = self.camera.screen_to_world(
            framebuffer.size().map(|x| x as f32),
            self.geng.window().mouse_pos().map(|x| x as f32),
        );
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
            },
            geng::Event::MouseUp {
                button: geng::MouseButton::Left,
                ..
            } => {
                if let Some(index) = self
                    .buttons()
                    .iter()
                    .position(|button| button.contains(self.mouse))
                {
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
                        }
                        _ => unreachable!(),
                    }
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
