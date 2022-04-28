use super::*;

pub trait WiggleThing: Sized + Copy + Add<Output = Self> + Mul<f32, Output = Self> {
    fn zero() -> Self;
}

impl WiggleThing for f32 {
    fn zero() -> Self {
        0.0
    }
}

impl WiggleThing for Vec2<f32> {
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
    pub fn interpolate(&self, turn: f32, speed: f32, time: f32) -> T {
        let mut wiggle = self.still;
        if let Some(max_speed) = self.max_speed {
            wiggle = wiggle * (1.0 - speed) + max_speed * speed;
        }
        if let Some(addition) = self.turn_addition {
            wiggle = wiggle + addition * turn;
        }
        wiggle.base + wiggle.amplitude() * (time * wiggle.frequency() * 2.0 * f32::PI).sin()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Part {
    pub name: Option<String>,
    pub parent: Option<String>,
    pub texture: String,
    pub origin: Vec2<f32>,
    pub position: Inter<Vec2<f32>>,
    pub rotation: Option<Inter<f32>>,
    pub scale: Option<Inter<Vec2<f32>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, geng::Assets)]
#[asset(json)]
pub struct Config {
    pub parts: Vec<Part>,
}
