use super::*;

pub struct Button<T> {
    pub text: String,
    pub position: Vec2<f32>,
    pub size: f32,
    pub message: T,
}

impl<T> Button<T> {
    pub fn new(text: &str, position: Vec2<f32>, size: f32, align: f32, message: T) -> Self {
        let width = text.len() as f32 * 0.8;
        Self {
            text: text.to_owned(),
            position: vec2(position.x - width * align * size, position.y),
            size,
            message,
        }
    }
    pub fn aabb(&self) -> AABB<f32> {
        AABB::point(self.position)
            .extend_positive(vec2(self.text.len() as f32 * 0.8, 1.0) * self.size)
    }
}
