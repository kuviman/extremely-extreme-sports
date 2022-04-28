use super::*;

trait WiggleThing: Sized + Copy + Add<Output = Self> + Mul<f32, Output = Self> {}

impl<T: Add<Output = T> + Mul<f32, Output = T> + Copy> WiggleThing for T {}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Wiggle<T> {
    pub base: T,
    pub amplitude: T,
    pub frequency: f32,
}

impl<T: WiggleThing> Add for Wiggle<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            base: self.base + rhs.base,
            amplitude: self.amplitude + rhs.amplitude,
            frequency: self.frequency + rhs.frequency,
        }
    }
}

impl<T: WiggleThing> Mul<f32> for Wiggle<T> {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            base: self.base * rhs,
            amplitude: self.amplitude * rhs,
            frequency: self.frequency * rhs,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Inter<T> {
    pub still: Wiggle<T>,
    pub max_speed: Wiggle<T>,
    pub turn_addition: Wiggle<T>,
}

impl<T: WiggleThing> Inter<T> {
    pub fn interpolate(&self, turn: f32, speed: f32, time: f32) -> T {
        let wiggle =
            self.still * (1.0 - speed) + self.max_speed * speed + self.turn_addition * turn;
        wiggle.base + wiggle.amplitude * (time * wiggle.frequency * 2.0 * f32::PI).sin()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Part {
    pub name: Option<String>,
    pub parent: Option<String>,
    pub texture: String,
    pub origin: Vec2<f32>,
    pub position: Inter<Vec2<f32>>,
    pub rotation: Inter<f32>,
    pub scale: Inter<Vec2<f32>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, geng::Assets)]
#[asset(json)]
pub struct Config {
    pub parts: Vec<Part>,
}
