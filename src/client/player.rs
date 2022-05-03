use super::*;

impl Player {
    pub fn update_walk(&mut self, delta_time: f32) {
        let target_speed = self.input * Self::MAX_WALK_SPEED;
        self.velocity.x +=
            (target_speed - self.velocity.x).clamp_abs(Self::WALK_ACCELERATION * delta_time);
        self.position += self.velocity * delta_time;
        self.ride_volume = 0.0;
    }
    pub fn update_riding(&mut self, delta_time: f32) {
        if !self.crashed {
            let target_rotation =
                (self.input * Self::ROTATION_LIMIT).clamp_abs(Self::ROTATION_LIMIT);
            self.rotation +=
                (target_rotation - self.rotation).clamp_abs(Self::ROTATION_SPEED * delta_time);
            self.velocity.y += (-Self::MAX_SPEED - self.velocity.y)
                .clamp_abs(Self::DOWNHILL_ACCELERATION * delta_time);
            let normal = vec2(1.0, 0.0).rotate(self.rotation);
            let force = -Vec2::dot(self.velocity, normal) * Self::FRICTION;
            self.ride_volume = force.abs() / 10.0;
            self.velocity += normal * force * delta_time;
            self.velocity = self.velocity.clamp_len(..=Self::MAX_SPEED);
            self.ski_velocity = self.velocity;
            self.ski_rotation = self.rotation;
            self.crash_position = self.position;
        } else {
            self.ride_volume = 0.0;
            self.crash_timer += delta_time;
            self.velocity -= self
                .velocity
                .clamp_len(..=Self::CRASH_DECELERATION * delta_time);
        }
        self.position += self.velocity * delta_time;
    }

    pub fn respawn(&mut self) {
        *self = Player {
            position: vec2(global_rng().gen_range(-TRACK_WIDTH..=TRACK_WIDTH), 0.0),
            rotation: 0.0,
            velocity: Vec2::ZERO,
            crashed: false,
            crash_timer: 0.0,
            is_riding: false,
            seen_no_avalanche: false,
            name: self.name.clone(),
            config: self.config.clone(),
            ..*self
        };
    }
}