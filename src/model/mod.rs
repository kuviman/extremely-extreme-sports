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
pub struct PlayerConfig {
    pub rotation_speed: f32,
    pub rotation_limit: f32,
    pub max_speed: f32,
    pub max_walk_speed: f32,
    pub friction: f32,
    pub downhill_acceleration: f32,
    pub walk_acceleration: f32,
    pub crash_deceleration: f32,
    pub parachute_time: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub avalanche: AvalancheConfig,
    pub track: TrackConfig,
    pub player: PlayerConfig,
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
    Disconnect,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {}

pub const TICKS_PER_SECOND: f32 = 10.0;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq)]
pub enum PlayerState {
    SpawnWalk,
    Walk,
    Ride,
    Crash {
        timer: f32,
        ski_velocity: vec2<f32>,
        ski_rotation: f32,
        crash_position: vec2<f32>,
    },
    Parachute {
        timer: f32,
    },
}

#[derive(Debug, Serialize, Deserialize, HasId, Diff, Clone, PartialEq)]
pub struct Player {
    pub id: Id,
    pub start_y: f32,
    pub emote: Option<(f32, usize)>,
    #[diff = "eq"]
    pub name: String,
    pub position: vec2<f32>,
    #[diff = "eq"]
    pub config: skin::Config,
    pub radius: f32,
    pub rotation: f32,
    pub input: vec2<f32>,
    pub velocity: vec2<f32>,
    pub state: PlayerState,
    pub seen_no_avalanche: bool,
    pub ride_volume: f32,
}

impl Player {
    pub fn score(&self) -> i32 {
        ((self.start_y - self.position.y) * 100.0) as i32
    }
}
