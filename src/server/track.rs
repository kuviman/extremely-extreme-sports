use super::*;

pub struct TrackGen {
    config: TrackConfig,
    rng: Box<dyn RngCore + Send>,
    last: Vec<Vec2<f32>>,
    last_len: [f32; 2],
    obstacle_options: Vec<(usize, ObstacleConfig)>,
}

impl TrackGen {
    pub fn new(config: &TrackConfig) -> Self {
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
        Self {
            config: config.clone(),
            rng: Box::new(StdRng::from_seed(global_rng().gen())),
            last: vec![Vec2::ZERO, Vec2::ZERO],
            last_len: [0.0; 2],
            obstacle_options,
        }
    }
    pub fn init(&self) -> Track {
        Track {
            obstacles: vec![],
            shape: vec![],
        }
    }
    pub fn update(&mut self, track: &mut Track, start: f32, end: f32) {
        while self.last[0].y > end {
            while self.last.len() < 4 {
                let mut cur = self.last.last().copied().unwrap();
                cur.y -= if self.last.len() == 2 {
                    self.config.spawn_area
                } else {
                    self.config.step
                };
                let var = if self.last.len() == 2 {
                    0.0
                } else {
                    self.rng.gen_range(0.0f32..=1.0).powf(self.config.curve_exp)
                        * self.config.max_curve
                        * if self.rng.gen_bool(0.5) { -1.0 } else { 1.0 }
                };
                cur.x += var * self.config.step;
                self.last.push(cur);
            }
            let sides: Vec<[Vec2<f32>; 2]> = self
                .last
                .iter()
                .copied()
                .map(|p| {
                    let width = if p.y == 0.0 {
                        self.config.spawn_width
                    } else {
                        self.config.width
                    };
                    [p - vec2(width, 0.0), p + vec2(width, 0.0)]
                })
                .collect();
            let left = CurveInterval {
                point_start: sides[1][0],
                point_end: sides[2][0],
                tangent_start: (sides[2][0] - sides[0][0]) * 0.5,
                tangent_end: (sides[3][0] - sides[1][0]) * 0.5,
            };
            let right = CurveInterval {
                point_start: sides[1][1],
                point_end: sides[2][1],
                tangent_start: (sides[2][1] - sides[0][1]) * 0.5,
                tangent_end: (sides[3][1] - sides[1][1]) * 0.5,
            };

            const N: usize = 10;
            for i in 0..N {
                let left = left.get(i as f32 / N as f32);
                let right = right.get(i as f32 / N as f32);
                let y = left.y;
                let left = left.x;
                let right = right.x;
                let mid = (left + right) / 2.0;
                if let Some(last) = track.shape.last() {
                    self.last_len[0] += (vec2(y, left) - vec2(last.y, last.left)).len();
                    self.last_len[1] += (vec2(y, right) - vec2(last.y, last.right)).len();
                }
                let safe = self.config.safe_middle
                    + (self.config.spawn_area + y).max(0.0) / self.config.spawn_area
                        * self.config.safe_middle;
                track.shape.push(ShapePoint {
                    y,
                    left,
                    right,
                    left_len: self.last_len[0],
                    right_len: self.last_len[1],
                    safe_left: mid - safe,
                    safe_right: mid + safe,
                });
            }

            let y1 = self.last[1].y;
            let y2 = self.last[2].y;

            'obstacles: for _ in
                0..((y1 - y2) * self.config.width * self.config.obstacle_density) as usize
            {
                let index = self
                    .obstacle_options
                    .choose_weighted(&mut self.rng, |(_, obstacle)| obstacle.spawn_weight)
                    .unwrap()
                    .0;
                let radius = self.obstacle_options[index].1.hitbox_radius / 20.0;
                let y = self.rng.gen_range(y2..y1);
                let shape_point = track.at(y);
                let x = self
                    .rng
                    .gen_range(shape_point.left + radius..shape_point.right - radius);
                if x + radius > shape_point.safe_left && x - radius < shape_point.safe_right {
                    continue 'obstacles;
                }
                let position = vec2(x, y);
                for obstacle in &track.obstacles {
                    if (obstacle.position - position).len()
                        < radius + obstacle.radius + self.config.distance_between_obstacles
                    {
                        continue 'obstacles;
                    }
                }
                track.obstacles.push(Obstacle {
                    index,
                    radius,
                    position,
                });
            }

            self.last.remove(0);
            assert_eq!(self.last.len(), 3);
        }
        track.shape.retain(|s| s.y < start);
        track.obstacles.retain(|o| o.position.y < start);
    }
}

impl Track {
    // pub fn old_new(seed: u64, config: &TrackConfig) -> Self {
    //     let mut rng = StdRng::seed_from_u64(seed);

    //     let shape = {
    //         let mut shape: Vec<ShapePoint> = Vec::new();
    //         let mut y = config.spawn_area;
    //         let mut left = Vec::new();
    //         let mut right = Vec::new();
    //         let mut ys = Vec::new();
    //         left.push(-config.spawn_width);
    //         right.push(config.spawn_width);
    //         ys.push(0.0);
    //         let mut mid = 0.0;
    //         while y < config.length {
    //             ys.push(y);
    //             left.push(mid - config.width);
    //             right.push(mid + config.width);
    //             const DELTA: f32 = 10.0;
    //             y += DELTA;
    //             let var = rng.gen_range(0.0f32..=1.0).powf(0.5);
    //             mid += if rng.gen_bool(0.5) { -1.0 } else { 1.0 } * var * DELTA * 1.2;
    //         }
    //         let n = ys.len();
    //         let left = CardinalSpline::new(
    //             ys.iter()
    //                 .copied()
    //                 .zip(left.into_iter())
    //                 .map(|(y, x)| vec2(x, y))
    //                 .collect(),
    //             0.5,
    //         );
    //         let right = CardinalSpline::new(
    //             ys.iter()
    //                 .copied()
    //                 .zip(right.into_iter())
    //                 .map(|(y, x)| vec2(x, y))
    //                 .collect(),
    //             0.5,
    //         );
    //         let mut left_len = 0.0;
    //         let mut right_len = 0.0;
    //         for (left, right) in left
    //             .intervals()
    //             .into_iter()
    //             .zip(right.intervals().into_iter())
    //         {
    //             const N: usize = 10;
    //             for i in 0..N {
    //                 let left = left.get(i as f32 / N as f32);
    //                 let right = right.get(i as f32 / N as f32);
    //                 assert_eq!(left.y, right.y);
    //                 let y = -left.y;
    //                 let left = left.x;
    //                 let right = right.x;
    //                 let mid = (left + right) / 2.0;
    //                 if let Some(last) = shape.last() {
    //                     left_len += (vec2(y, left) - vec2(last.y, last.left)).len();
    //                     right_len += (vec2(y, right) - vec2(last.y, last.right)).len();
    //                 }
    //                 let safe = config.safe_middle
    //                     + (config.spawn_area + y).max(0.0) / config.spawn_area * config.safe_middle;
    //                 shape.push(ShapePoint {
    //                     y,
    //                     left,
    //                     right,
    //                     left_len,
    //                     right_len,
    //                     safe_left: mid - safe,
    //                     safe_right: mid + safe,
    //                 });
    //             }
    //         }
    //         shape
    //     };

    //     let mut obstacles: Vec<Obstacle> = Vec::new();
    //     'obstacles: for _ in 0..(config.length * config.width * config.obstacle_density) as usize {
    //         let index = obstacle_options
    //             .choose_weighted(&mut rng, |(_, obstacle)| obstacle.spawn_weight)
    //             .unwrap()
    //             .0;
    //         let radius = obstacle_options[index].1.hitbox_radius / 20.0;
    //         let y = rng.gen_range(-config.length..0.0);
    //         let shape_point = Self::at_shape(&shape, y);
    //         let x = rng.gen_range(shape_point.left + radius..shape_point.right - radius);
    //         if x + radius > shape_point.safe_left && x - radius < shape_point.safe_right {
    //             continue 'obstacles;
    //         }
    //         let position = vec2(x, y);
    //         for obstacle in &obstacles {
    //             if (obstacle.position - position).len()
    //                 < radius + obstacle.radius + config.distance_between_obstacles
    //             {
    //                 continue 'obstacles;
    //             }
    //         }
    //         obstacles.push(Obstacle {
    //             index,
    //             radius,
    //             position,
    //         });
    //     }
    //     obstacles.sort_by_key(|o| -r32(o.position.y));
    //     Self { shape, obstacles }
    // }
}
