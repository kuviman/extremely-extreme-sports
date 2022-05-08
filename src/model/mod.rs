use super::*;

mod track;

pub use track::*;

pub type Id = i64;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AvalancheConfig {
    pub min_speed: f32,
    pub max_speed: f32,
    pub acceleration: f32,
    pub start: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub avalanche: AvalancheConfig,
    pub track: TrackConfig,
}

#[derive(Debug, Serialize, Deserialize, Diff, Clone, PartialEq)]
pub struct SharedModel {
    pub tick: u64,
    pub next_id: Id,
    #[diff = "eq"]
    pub config: Config,
    pub avalanche_position: Option<f32>,
    pub avalanche_speed: f32,
    pub players: Collection<Player>,
    #[diff = "eq"]
    pub track: Track,
    #[diff = "eq"]
    pub winner: Option<(String, i32)>,
    #[diff = "eq"]
    pub highscores: HashMap<String, i32>,
    #[diff = "eq"]
    pub scores: HashMap<String, i32>,
    pub reset_timer: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Message {
    UpdatePlayer(Player),
    Score(i32),
    StartTheRace,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {}

pub const TICKS_PER_SECOND: f32 = 10.0;

#[derive(Debug, Serialize, Deserialize, HasId, Diff, Clone, PartialEq)]
pub struct Player {
    pub id: Id,
    pub start_y: f32,
    pub emote: Option<(f32, usize)>,
    #[diff = "eq"]
    pub name: String,
    pub position: Vec2<f32>,
    #[diff = "eq"]
    pub config: skin::Config,
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
    pub parachute: Option<f32>,
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
    pub const PARACHUTE_TIME: f32 = 2.0;

    pub fn score(&self) -> i32 {
        ((self.start_y - self.position.y) * 100.0) as i32
    }
}
