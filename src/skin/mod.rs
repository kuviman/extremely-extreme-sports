use super::*;

pub trait WiggleThing: Sized + Copy + Add<Output = Self> + Mul<f32, Output = Self> {
    fn zero() -> Self;
}

impl WiggleThing for f32 {
    fn zero() -> Self {
        0.0
    }
}

impl WiggleThing for vec2<f32> {
    fn zero() -> Self {
        vec2(0.0, 0.0)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Wiggle<T> {
    pub base: T,
    pub amplitude: Option<T>,
    pub frequency: Option<f32>,
}

impl<T: WiggleThing> Wiggle<T> {
    fn amplitude(&self) -> T {
        self.amplitude.unwrap_or(T::zero())
    }
    fn frequency(&self) -> f32 {
        self.frequency.unwrap_or(0.0)
    }
    fn get(&self, phase: f32) -> T {
        self.base + self.amplitude() * phase.sin()
    }
}

impl<T: WiggleThing> Add for Wiggle<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            base: self.base + rhs.base,
            amplitude: Some(self.amplitude() + rhs.amplitude()),
            frequency: Some(self.frequency() + rhs.frequency()),
        }
    }
}

impl<T: WiggleThing> Mul<f32> for Wiggle<T> {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            base: self.base * rhs,
            amplitude: Some(self.amplitude() * rhs),
            frequency: Some(self.frequency() * rhs),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Inter<T> {
    pub still: Wiggle<T>,
    pub max_speed: Option<Wiggle<T>>,
    pub turn_addition: Option<Wiggle<T>>,
}

impl<T: WiggleThing> Inter<T> {
    pub fn interpolate(&self, turn: f32, speed: f32) -> Wiggle<T> {
        let mut wiggle = self.still;
        if let Some(max_speed) = self.max_speed {
            wiggle = wiggle * (1.0 - speed) + max_speed * speed;
        }
        if let Some(addition) = self.turn_addition {
            wiggle = wiggle + addition * turn;
        }
        wiggle
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Part {
    pub name: Option<String>,
    pub parent: Option<String>,
    #[serde(default)]
    pub z: i32,
    pub texture: String,
    pub origin: vec2<f32>,
    pub position: Inter<vec2<f32>>,
    pub rotation: Option<Inter<f32>>,
    pub scale: Option<Inter<vec2<f32>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, geng::asset::Load)]
#[load(json)]
pub struct SecretConfig {
    pub parts: Option<Vec<Part>>,
    pub hat: Option<String>,
    pub coat: Option<String>,
    pub pants: Option<String>,
    pub equipment: Option<String>,
    pub face: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, geng::asset::Load)]
#[load(json)]
pub struct ItemConfig {
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, geng::asset::Load)]
// #[serde(tag = "type")]
#[load(json)]
pub struct Config {
    pub secret: Option<String>,
    pub hat: Option<String>,
    pub coat: Option<String>,
    pub pants: Option<String>,
    pub equipment: Option<String>,
    pub face: Option<String>,
}

impl Config {
    pub fn parts<'a>(
        &'a self,
        assets: &'a Assets,
        state: &PlayerState,
    ) -> Box<dyn Iterator<Item = &'a Part> + 'a> {
        let mut result = Vec::new();
        if let Some(name) = &self.secret {
            result.extend(
                assets.player.secret[name]
                    .parts
                    .iter()
                    .flat_map(|parts| parts.iter()),
            );
        } else {
            result.extend(assets.player.body.parts.iter());
        }
        if let Some(name) = &self.face {
            result.extend(assets.player.face[name].parts.iter());
        }
        if let Some(name) = &self.hat {
            result.extend(assets.player.hat[name].parts.iter());
        }
        if let Some(name) = &self.pants {
            result.extend(assets.player.pants[name].parts.iter());
        }
        if let Some(name) = &self.coat {
            result.extend(assets.player.coat[name].parts.iter());
        }
        if let PlayerState::Parachute { .. } = state {
            result.extend(&assets.player.parachute.parts);
        }
        Box::new(result.into_iter())
    }
    pub fn random(assets: &assets::PlayerAssets) -> Self {
        let mut rng = thread_rng();
        let rng = &mut rng;
        Self {
            secret: None,
            hat: Some(assets.hat.keys().choose(rng).unwrap().to_owned()),
            coat: Some(assets.coat.keys().choose(rng).unwrap().to_owned()),
            pants: Some(assets.pants.keys().choose(rng).unwrap().to_owned()),
            equipment: Some(assets.equipment.keys().choose(rng).unwrap().to_owned()),
            face: Some(assets.face.keys().choose(rng).unwrap().to_owned()),
        }
    }
}

struct PartState {
    phase: f32,
    frequency: f32,
}

struct State {
    position: Vec<PartState>,
    rotation: Vec<PartState>,
    scale: Vec<PartState>,
}

pub struct Renderer {
    assets: Rc<Assets>,
    config: Config,
    quad_geometry: ugli::VertexBuffer<draw2d::Vertex>,
    time: f32,
    state: RefCell<State>,
}

pub struct DrawInstance {
    pub position: vec2<f32>,
    pub rotation: f32,
    pub velocity: vec2<f32>,
    pub state: PlayerState,
}

impl Renderer {
    pub fn new(geng: &Geng, config: &Config, assets: &Rc<Assets>) -> Self {
        Self {
            assets: assets.clone(),
            config: config.clone(),
            quad_geometry: ugli::VertexBuffer::new_static(
                geng.ugli(),
                vec![
                    draw2d::Vertex {
                        a_pos: vec2(-1.0, -1.0),
                    },
                    draw2d::Vertex {
                        a_pos: vec2(1.0, -1.0),
                    },
                    draw2d::Vertex {
                        a_pos: vec2(1.0, 1.0),
                    },
                    draw2d::Vertex {
                        a_pos: vec2(-1.0, 1.0),
                    },
                ],
            ),
            time: 0.0,
            state: RefCell::new(State {
                position: config
                    .parts(assets, &PlayerState::Parachute { timer: 0.0 })
                    .map(|_| PartState {
                        phase: thread_rng().gen_range(0.0..=2.0 * f32::PI),
                        frequency: 0.0,
                    })
                    .collect(),
                rotation: config
                    .parts(assets, &PlayerState::Parachute { timer: 0.0 })
                    .map(|_| PartState {
                        phase: thread_rng().gen_range(0.0..=2.0 * f32::PI),
                        frequency: 0.0,
                    })
                    .collect(),
                scale: config
                    .parts(assets, &PlayerState::Parachute { timer: 0.0 })
                    .map(|_| PartState {
                        phase: thread_rng().gen_range(0.0..=2.0 * f32::PI),
                        frequency: 0.0,
                    })
                    .collect(),
            }),
        }
    }
    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
        let mut state = self.state.borrow_mut();
        for part in &mut state.position {
            part.phase += delta_time * part.frequency * 2.0 * f32::PI;
        }
        for part in &mut state.rotation {
            part.phase += delta_time * part.frequency * 2.0 * f32::PI;
        }
        for part in &mut state.scale {
            part.phase += delta_time * part.frequency * 2.0 * f32::PI;
        }
    }
    pub fn draw(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        config: &PlayerConfig,
        player: &DrawInstance,
    ) {
        let draw_position = player.position
            + match player.state {
                PlayerState::Ride | PlayerState::Crash { .. } => vec2(0.0, 0.0),
                PlayerState::Walk | PlayerState::SpawnWalk => vec2(
                    0.0,
                    player.velocity.len().min(0.1) * (self.time * 15.0).sin().abs(),
                ),
                PlayerState::Parachute { timer } => vec2(0.0, 10.0 * timer / config.parachute_time),
            };
        let mut draw_texture = |texture: &ugli::Texture, transform: mat3<f32>, color: Rgba<f32>| {
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
                    camera.uniforms(framebuffer_size.map(|x| x as f32)),
                ),
                &ugli::DrawParameters { ..default() },
            );
        };

        let equipment: Option<&ugli::Texture> = self.config.equipment.as_ref().map(|name| {
            self.assets
                .player
                .equipment
                .get(name)
                .unwrap_or_else(|| &self.assets.textures[name])
        });
        if let Some(equipment) = equipment {
            if let PlayerState::Ride | PlayerState::Parachute { .. } = player.state {
                draw_texture(
                    equipment,
                    mat3::translate(draw_position) * mat3::rotate(player.rotation),
                    Rgba::WHITE,
                );
            } else if let PlayerState::Crash {
                timer,
                ski_velocity,
                ski_rotation,
                crash_position,
            } = player.state
            {
                let t = timer.min(1.0);
                draw_texture(
                    equipment,
                    mat3::translate(
                        crash_position
                            + ski_velocity * t
                            + vec2(0.0, (1.0 - (t * 2.0 - 1.0).sqr()) * 5.0),
                    ) * mat3::rotate(ski_rotation + t * 5.0),
                    Rgba::WHITE,
                );
            } else {
                draw_texture(
                    equipment,
                    mat3::translate(draw_position + vec2(0.0, 1.0)),
                    Rgba::WHITE,
                );
            }
        }

        let final_matrix = mat3::translate(draw_position)
            * mat3::rotate(
                (match player.state {
                    PlayerState::Crash { timer, .. } => timer,
                    _ => 0.0,
                } * 7.0)
                    .min(f32::PI / 2.0),
            )
            * mat3::scale_uniform(1.0 / 64.0);
        let turn = if player.state == PlayerState::Ride {
            player.rotation / config.rotation_limit
        } else {
            player.velocity.x / config.max_walk_speed
        };
        let speed = if player.state != PlayerState::SpawnWalk && player.state != PlayerState::Walk {
            (player.velocity.len() / config.max_speed).min(1.0)
        } else {
            0.0
        };
        let mut part_matrices: HashMap<&str, mat3<f32>> = HashMap::new();
        let mut state = self.state.borrow_mut();
        struct Q<'a> {
            texture: &'a ugli::Texture,
            matrix: mat3<f32>,
            z: i32,
        }
        let mut q = Vec::new();
        for (i, part) in self.config.parts(&self.assets, &player.state).enumerate() {
            let texture = &self.assets.textures[&part.texture];
            let parent_matrix = match &part.parent {
                Some(name) => part_matrices
                    .get(name.as_str())
                    .copied()
                    .unwrap_or(mat3::identity()),
                None => mat3::identity(),
            };
            let position_wiggle = part.position.interpolate(turn, speed);
            state.position[i].frequency = position_wiggle.frequency.unwrap_or(0.0);
            let mut matrix =
                parent_matrix * mat3::translate(position_wiggle.get(state.position[i].phase));
            if let Some(rotation) = &part.rotation {
                let rotation_wiggle = rotation.interpolate(turn, speed);
                state.rotation[i].frequency = rotation_wiggle.frequency.unwrap_or(0.0);
                matrix *=
                    mat3::rotate(rotation_wiggle.get(state.rotation[i].phase) * f32::PI / 180.0);
            }
            if let Some(scale) = &part.scale {
                let scale_wiggle = scale.interpolate(turn, speed);
                state.scale[i].frequency = scale_wiggle.frequency.unwrap_or(0.0);
                matrix *= mat3::scale(scale_wiggle.get(state.scale[i].phase));
            }
            matrix *= mat3::translate(-part.origin);
            if let Some(name) = &part.name {
                part_matrices.insert(name.as_str(), matrix);
            }
            let matrix = matrix
                * mat3::scale(texture.size().map(|x| x as f32) / 2.0)
                * mat3::translate(vec2(1.0, 1.0));
            q.push(Q {
                texture,
                matrix: final_matrix * matrix,
                z: part.z,
            });
        }
        q.sort_by_key(|q| q.z);
        for q in q {
            draw_texture(q.texture, q.matrix, Rgba::WHITE);
        }
    }
}
