use super::*;

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Track {
    pub obstacles: Vec<Obstacle>,
    pub shape: Vec<ShapePoint>,
}

impl Track {
    pub fn new_from_env() -> Self {
        let seed = match std::env::var("SEED") {
            Ok(seed) => seed.parse().unwrap(),
            Err(_) => global_rng().gen(),
        };
        Self::new(seed)
    }
    pub fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        const TRACK_LEN: f32 = 1500.0;
        const TRACK_WIDTH: f32 = 30.0;
        const SAFE_MIDDLE: f32 = 5.0;
        const OBSTACLES_DENSITY: f32 = 0.2;
        const DISTANCE_BETWEEN_OBSTACLES: f32 = 0.5;
        const SPAWN_AREA: f32 = 10.0;
        const SPAWN_WIDTH: f32 = 10.0;

        let shape = {
            let mut shape: Vec<ShapePoint> = Vec::new();
            let mut y = SPAWN_AREA;
            let mut left = Vec::new();
            let mut right = Vec::new();
            let mut ys = Vec::new();
            left.push(-SPAWN_WIDTH);
            right.push(SPAWN_WIDTH);
            ys.push(0.0);
            let mut mid = 0.0;
            while y < TRACK_LEN {
                ys.push(y);
                left.push(mid - TRACK_WIDTH);
                right.push(mid + TRACK_WIDTH);
                const DELTA: f32 = 20.0;
                y += DELTA;
                mid += rng.gen_range(-1.0..=1.0) * DELTA * 1.0;
            }
            let n = ys.len();
            let left = CardinalSpline::new(
                ys.iter()
                    .copied()
                    .zip(left.into_iter())
                    .map(|(y, x)| vec2(x, y))
                    .collect(),
                0.5,
            );
            let right = CardinalSpline::new(
                ys.iter()
                    .copied()
                    .zip(right.into_iter())
                    .map(|(y, x)| vec2(x, y))
                    .collect(),
                0.5,
            );
            let mut left_len = 0.0;
            let mut right_len = 0.0;
            for (left, right) in left
                .intervals()
                .into_iter()
                .zip(right.intervals().into_iter())
            {
                const N: usize = 10;
                for i in 0..N {
                    let left = left.get(i as f32 / N as f32);
                    let right = right.get(i as f32 / N as f32);
                    assert_eq!(left.y, right.y);
                    let y = -left.y;
                    let left = left.x;
                    let right = right.x;
                    let mid = (left + right) / 2.0;
                    if let Some(last) = shape.last() {
                        left_len += (vec2(y, left) - vec2(last.y, last.left)).len();
                        right_len += (vec2(y, right) - vec2(last.y, last.right)).len();
                    }
                    shape.push(ShapePoint {
                        y,
                        left,
                        right,
                        left_len,
                        right_len,
                        safe_left: mid - SAFE_MIDDLE,
                        safe_right: mid + SAFE_MIDDLE,
                    });
                }
            }
            shape
        };

        let list: Vec<String> = serde_json::from_reader(
            std::fs::File::open(static_path().join("obstacles.json")).unwrap(),
        )
        .unwrap();
        let obstacle_options: Vec<(usize, ObstacleConfig)> = list
            .into_iter()
            .map(|path| {
                serde_json::from_reader(
                    std::fs::File::open(static_path().join(format!("{}.json", path))).unwrap(),
                )
                .unwrap()
            })
            .enumerate()
            .collect();
        let mut obstacles: Vec<Obstacle> = Vec::new();
        'obstacles: for _ in 0..(TRACK_LEN * TRACK_WIDTH * OBSTACLES_DENSITY) as usize {
            let index = obstacle_options
                .choose_weighted(&mut rng, |(_, obstacle)| obstacle.spawn_weight)
                .unwrap()
                .0;
            let radius = obstacle_options[index].1.hitbox_radius / 20.0;
            let y = rng.gen_range(-TRACK_LEN..0.0);
            let shape_point = Self::at_shape(&shape, y);
            let x = rng.gen_range(shape_point.left + radius..shape_point.right - radius);
            if x + radius > shape_point.safe_left && x - radius < shape_point.safe_right {
                continue 'obstacles;
            }
            let position = vec2(x, y);
            for obstacle in &obstacles {
                if (obstacle.position - position).len()
                    < radius + obstacle.radius + DISTANCE_BETWEEN_OBSTACLES
                {
                    continue 'obstacles;
                }
            }
            obstacles.push(Obstacle {
                index,
                radius,
                position,
            });
        }
        obstacles.sort_by_key(|o| -r32(o.position.y));
        Self { shape, obstacles }
    }
    fn at_shape(shape: &Vec<ShapePoint>, y: f32) -> ShapePoint {
        let idx = match shape.binary_search_by_key(&r32(-y), |point| r32(-point.y)) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
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
