use super::*;

impl Player {
    pub fn update_walk(&mut self, config: &PlayerConfig, delta_time: f32) {
        let target_speed = self.input.clamp_len(0.0..=1.0) * config.max_walk_speed;
        self.velocity +=
            (target_speed - self.velocity).clamp_len(..=config.walk_acceleration * delta_time);
        self.position += self.velocity * delta_time;
        self.ride_volume = 0.0;
    }
    pub fn update_riding(&mut self, config: &PlayerConfig, delta_time: f32) {
        match &mut self.state {
            PlayerState::Crash { timer, .. } => {
                self.ride_volume = 0.0;
                *timer += delta_time;
                self.velocity -= self
                    .velocity
                    .clamp_len(..=config.crash_deceleration * delta_time);
            }
            _ => {
                let target_rotation =
                    (config.rotation_limit * self.input.x).clamp_abs(config.rotation_limit);
                self.rotation +=
                    (target_rotation - self.rotation).clamp_abs(config.rotation_speed * delta_time);
                self.velocity.y += (-config.max_speed - self.velocity.y)
                    .clamp_abs(config.downhill_acceleration * delta_time);
                let normal = vec2(1.0, 0.0).rotate(self.rotation);
                let force = -vec2::dot(self.velocity, normal) * config.friction;
                self.ride_volume = force.abs() / 10.0;
                self.velocity += normal * force * delta_time;
                self.velocity = self.velocity.clamp_len(..=config.max_speed);
            }
        }
        self.position += self.velocity * delta_time;
    }

    pub fn respawn(&mut self) {
        *self = Player {
            position: vec2(thread_rng().gen_range(-10.0..=10.0), 0.0),
            rotation: Angle::ZERO,
            velocity: vec2::ZERO,
            start_y: 0.0,
            state: PlayerState::SpawnWalk,
            seen_no_avalanche: false,
            name: self.name.clone(),
            config: self.config.clone(),
            ..*self
        };
    }
}
