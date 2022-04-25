use super::*;

mod track;

pub use track::*;

pub type Id = i64;

#[derive(Debug, Serialize, Deserialize, Diff, Clone, PartialEq)]
pub struct Model {
    pub tick: u64,
    pub next_id: Id,
    pub avalanche_position: Option<f32>,
    pub avalanche_speed: f32,
    pub players: Collection<Player>,
    #[diff = "eq"]
    pub track: Track,
    #[diff = "eq"]
    pub winner: Option<(String, f32)>,
    #[diff = "eq"]
    pub highscores: HashMap<String, i32>,
    #[diff = "eq"]
    pub scores: Vec<(String, i32)>,
}

impl Model {
    pub const AVALANCHE_MIN_SPEED: f32 = 7.0;
    pub const AVALANCHE_MAX_SPEED: f32 = 11.0;
    pub const AVALANCHE_ACCELERATION: f32 =
        (Self::AVALANCHE_MAX_SPEED - Self::AVALANCHE_MIN_SPEED) / 60.0;
    const AVALANCHE_START: f32 = 30.0;
    pub fn new() -> Self {
        discord::send_activity("Server started :green_circle:");
        Self {
            tick: 0,
            next_id: 0,
            avalanche_position: None,
            avalanche_speed: Self::AVALANCHE_MIN_SPEED,
            players: default(),
            track: Track::new_from_env(),
            winner: None,
            highscores: {
                let path = std::path::Path::new("highscores.json");
                if path.is_file() {
                    serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap()
                } else {
                    default()
                }
            },
            scores: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Message {
    UpdatePlayer(Player),
    Score(i32),
    StartTheRace,
}
pub const TRACK_WIDTH: f32 = 10.0;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {}

pub const TICKS_PER_SECOND: f32 = 10.0;

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
        if let Some(player) = self.players.remove(&player_id) {
            discord::send_activity(&format!(
                "{} left the server :woman_tipping_hand:",
                player.name
            ));
        }
    }

    fn handle_message(&mut self, player_id: &Self::PlayerId, message: Message) {
        let player_id = *player_id;
        match message {
            Message::UpdatePlayer(mut player) => {
                if player.id != player_id {
                    return;
                }
                if self.players.get(&player_id).is_none() {
                    discord::send_activity(&format!(
                        "{} just joined the server :man_raising_hand:",
                        player.name
                    ));
                }
                self.players.insert(player);
            }
            Message::Score(score) => {
                if let Some(player) = self.players.get(&player_id) {
                    self.scores.push((player.name.clone(), score));
                }
            }
            Message::StartTheRace => {
                if self.avalanche_position.is_none() {
                    for player in &mut self.players {
                        player.position.y = 0.0;
                    }
                    self.scores.clear();
                    self.avalanche_position = Some(Self::AVALANCHE_START);
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
                    !player.is_riding || player.position.y > *position + self.avalanche_speed * 2.0
                }) {
                    self.avalanche_position = None;
                    self.avalanche_speed = Self::AVALANCHE_MIN_SPEED;
                    self.track = Track::new_from_env();
                    if !self.scores.is_empty() {
                        self.scores.sort_by_key(|(_name, score)| -score);
                        let mut text = "Race results:".to_owned();
                        for (index, (name, score)) in self.scores.iter().enumerate() {
                            text.push('\n');
                            text.push_str(&(index + 1).to_string());
                            text.push_str(". ");
                            text.push_str(name);
                            text.push_str(" - ");
                            text.push_str(&score.to_string());
                        }
                        text.push_str("\n<:extremeBoom:963122644373368832>");
                        discord::send_activity(&text);

                        let current_highest_score =
                            self.highscores.values().max().copied().unwrap_or(0);
                        for (name, score) in &self.scores {
                            let score = *score;
                            if score > current_highest_score {
                                discord::send_activity(&format!(
                                    "New highscore of {} by {} <:extremeBoom:963122644373368832>",
                                    score, name,
                                ));
                            }
                            if self.highscores.get(name).copied().unwrap_or(0) < score {
                                self.highscores.insert(name.clone(), score);
                            }
                        }
                        serde_json::to_writer_pretty(
                            std::fs::File::create("highscores.json").unwrap(),
                            &self.highscores,
                        )
                        .unwrap();

                        self.scores.clear();
                    }
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
    pub emote: Option<(f32, usize)>,
    #[diff = "eq"]
    pub name: String,
    pub position: Vec2<f32>,
    #[diff = "eq"]
    pub config: PlayerConfig,
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
    pub ride_volume: f32,
}

impl Player {
    pub const ROTATION_SPEED: f32 = 2.0 * f32::PI;
    pub const ROTATION_LIMIT: f32 = f32::PI / 3.0;
    pub const MAX_SPEED: f32 = 10.0;
    pub const MAX_WALK_SPEED: f32 = 3.0;
    pub const FRICTION: f32 = 5.0;
    pub const DOWNHILL_ACCELERATION: f32 = 5.0;
    pub const WALK_ACCELERATION: f32 = 20.0;
    pub const CRASH_DECELERATION: f32 = 10.0;
    pub fn update_walk(&mut self, delta_time: f32) {
        let target_speed = self.input * Self::MAX_WALK_SPEED;
        self.velocity.x +=
            (target_speed - self.velocity.x).clamp_abs(Self::WALK_ACCELERATION * delta_time);
        self.position += self.velocity * delta_time;
        self.ride_volume = 0.0;
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
            self.ride_volume = force.abs() / 10.0;
            self.velocity += normal * force * delta_time;
            self.ski_velocity = self.velocity;
            self.ski_rotation = self.rotation;
            self.crash_position = self.position;
        } else {
            self.ride_volume = 0.0;
            self.crash_timer += delta_time;
            self.velocity -= self
                .velocity
                .clamp_len(..=Self::CRASH_DECELERATION * delta_time);
        }
        self.position += self.velocity * delta_time;
    }

    pub fn respawn(&mut self) {
        *self = Player {
            position: vec2(global_rng().gen_range(-TRACK_WIDTH..=TRACK_WIDTH), 0.0),
            rotation: 0.0,
            velocity: Vec2::ZERO,
            crashed: false,
            crash_timer: 0.0,
            is_riding: false,
            seen_no_avalanche: false,
            name: self.name.clone(),
            config: self.config.clone(),
            ..*self
        };
    }
}
