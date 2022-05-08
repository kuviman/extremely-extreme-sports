use super::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TrackConfig {
    pub length: f32,
    pub width: f32,
    pub safe_middle: f32,
    pub obstacle_density: f32,
    pub distance_between_obstacles: f32,
    pub spawn_area: f32,
    pub spawn_width: f32,
}

#[derive(Debug, Serialize, Deserialize, Diff, Clone, PartialEq)]
pub struct Obstacle {
    pub index: usize,
    pub radius: f32,
    pub position: Vec2<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ShapePoint {
    pub y: f32,
    pub left: f32,
    pub right: f32,
    pub left_len: f32,
    pub right_len: f32,
    pub safe_left: f32,
    pub safe_right: f32,
}

impl ShapePoint {
    pub fn middle(&self) -> f32 {
        (self.safe_left + self.safe_right) / 2.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Track {
    pub obstacles: Vec<Obstacle>,
    pub shape: Vec<ShapePoint>,
}

impl Track {
    pub fn query_obstacles(&self, start: f32, end: f32) -> &[Obstacle] {
        let start = match self
            .obstacles
            .binary_search_by_key(&r32(-start), |o| -r32(o.position.y))
        {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        let end = match self
            .obstacles
            .binary_search_by_key(&r32(-end), |o| -r32(o.position.y))
        {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        &self.obstacles[start..end]
    }
    pub fn query_shape(&self, start: f32, end: f32) -> &[ShapePoint] {
        let start = match self.shape.binary_search_by_key(&r32(-start), |p| -r32(p.y)) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        let end = match self.shape.binary_search_by_key(&r32(-end), |p| -r32(p.y)) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        &self.shape[start..end]
    }
    pub fn at_shape(shape: &Vec<ShapePoint>, y: f32) -> ShapePoint {
        let idx = match shape.binary_search_by_key(&r32(-y), |point| r32(-point.y)) {
            Ok(idx) => idx,
            Err(idx) => idx.max(1) - 1,
        }
        .min(shape.len() - 2);
        fn lerp(a: f32, b: f32, t: f32) -> f32 {
            a + (b - a) * t
        }
        let left = lerp(
            shape[idx].left,
            shape[idx + 1].left,
            (y - shape[idx].y) / (shape[idx + 1].y - shape[idx].y),
        );
        let right = lerp(
            shape[idx].right,
            shape[idx + 1].right,
            (y - shape[idx].y) / (shape[idx + 1].y - shape[idx].y),
        );
        let safe_left = lerp(
            shape[idx].safe_left,
            shape[idx + 1].safe_left,
            (y - shape[idx].y) / (shape[idx + 1].y - shape[idx].y),
        );
        let safe_right = lerp(
            shape[idx].safe_right,
            shape[idx + 1].safe_right,
            (y - shape[idx].y) / (shape[idx + 1].y - shape[idx].y),
        );
        ShapePoint {
            y,
            left,
            right,
            left_len: 0.0,
            right_len: 0.0,
            safe_left,
            safe_right,
        }
    }
    pub fn at(&self, y: f32) -> ShapePoint {
        Self::at_shape(&self.shape, y)
    }
}
