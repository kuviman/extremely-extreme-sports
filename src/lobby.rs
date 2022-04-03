use super::*;

pub struct Lobby {
    geng: Geng,
    assets: Rc<Assets>,
    player_id: Id,
    model: Option<simple_net::Remote<Model>>,
    transition: Option<geng::Transition>,
    name: String,
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
            assets: assets.clone(),
            player_id,
            model: Some(model),
            transition: None,
            name: String::new(),
        }
    }
}

impl geng::State for Lobby {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::WHITE), None);

        let camera = geng::Camera2d {
            center: vec2(0.0, 0.5),
            rotation: 0.0,
            fov: 2.0,
        };

        self.geng.draw_2d(
            framebuffer,
            &camera,
            &draw_2d::TexturedQuad::unit(&self.assets.player),
        );
        self.assets
            .font
            .draw(framebuffer, &camera, vec2(0.0, 1.0), 0.2, &self.name, 0.5);
    }

    fn update(&mut self, delta_time: f64) {
        if let Some(model) = &mut self.model {
            model.update();
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Space => {
                    self.transition = Some(geng::Transition::Switch(Box::new(Game::new(
                        &self.geng,
                        &self.assets,
                        self.player_id,
                        self.name.clone(),
                        self.model.take().unwrap(),
                    ))))
                }
                geng::Key::Backspace => {
                    self.name.pop();
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
                        self.name.push(c)
                    }
                }
            },
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
