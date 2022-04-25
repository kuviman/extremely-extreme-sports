use super::*;

#[derive(Debug, Serialize, Deserialize, Diff, Clone, PartialEq)]
pub struct Obstacle {
    pub index: usize,
    pub radius: f32,
    pub position: Vec2<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Track {
    pub obstacles: Vec<Obstacle>,
}

impl Track {
    pub fn new(seed: i32) -> Self {
        let mut rng = global_rng();
        const TRACK_LEN: f32 = 1000.0;
        const OBSTACLES_DENSITY: f32 = 0.1;
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
            let w = TRACK_WIDTH - radius;
            let x = rng.gen_range(-w..w);
            let y = rng.gen_range(-TRACK_LEN..-Model::SPAWN_AREA);
            let position = vec2(x, y);
            for obstacle in &obstacles {
                if (obstacle.position - position).len() < radius + obstacle.radius {
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
        Self { obstacles }
    }
}
