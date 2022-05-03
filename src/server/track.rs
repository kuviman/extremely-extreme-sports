use super::*;

impl Track {
    pub fn new_from_env(config: &TrackConfig) -> Self {
        let seed = match std::env::var("SEED") {
            Ok(seed) => seed.parse().unwrap(),
            Err(_) => global_rng().gen(),
        };
        Self::new(seed, config)
    }
    pub fn new(seed: u64, config: &TrackConfig) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let shape = {
            let mut shape: Vec<ShapePoint> = Vec::new();
            let mut y = config.spawn_area;
            let mut left = Vec::new();
            let mut right = Vec::new();
            let mut ys = Vec::new();
            left.push(-config.spawn_width);
            right.push(config.spawn_width);
            ys.push(0.0);
            let mut mid = 0.0;
            while y < config.length {
                ys.push(y);
                left.push(mid - config.width);
                right.push(mid + config.width);
                const DELTA: f32 = 10.0;
                y += DELTA;
                let var = rng.gen_range(0.0f32..=1.0).powf(0.5);
                mid += if rng.gen_bool(0.5) { -1.0 } else { 1.0 } * var * DELTA * 1.2;
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
                    let safe = config.safe_middle
                        + (config.spawn_area + y).max(0.0) / config.spawn_area * config.safe_middle;
                    shape.push(ShapePoint {
                        y,
                        left,
                        right,
                        left_len,
                        right_len,
                        safe_left: mid - safe,
                        safe_right: mid + safe,
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
        'obstacles: for _ in 0..(config.length * config.width * config.obstacle_density) as usize {
            let index = obstacle_options
                .choose_weighted(&mut rng, |(_, obstacle)| obstacle.spawn_weight)
                .unwrap()
                .0;
            let radius = obstacle_options[index].1.hitbox_radius / 20.0;
            let y = rng.gen_range(-config.length..0.0);
            let shape_point = Self::at_shape(&shape, y);
            let x = rng.gen_range(shape_point.left + radius..shape_point.right - radius);
            if x + radius > shape_point.safe_left && x - radius < shape_point.safe_right {
                continue 'obstacles;
            }
            let position = vec2(x, y);
            for obstacle in &obstacles {
                if (obstacle.position - position).len()
                    < radius + obstacle.radius + config.distance_between_obstacles
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
}
