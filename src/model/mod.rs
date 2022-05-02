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
    pub const AVALANCHE_START: f32 = 30.0;
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

#[derive(Debug, Serialize, Deserialize, HasId, Diff, Clone, PartialEq)]
pub struct Player {
    pub id: Id,
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
}
