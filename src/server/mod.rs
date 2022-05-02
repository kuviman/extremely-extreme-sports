use super::*;

mod track;

impl Model {
    pub fn new() -> Self {
        discord::send_activity("Server started :green_circle:");
        Self {
            tick: 0,
            next_id: 0,
            avalanche_position: None,
            avalanche_speed: Self::AVALANCHE_MIN_SPEED,
            players: default(),
            track: Track::new_from_env(),
            winner: None,
            highscores: {
                let path = std::path::Path::new("highscores.json");
                if path.is_file() {
                    serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap()
                } else {
                    default()
                }
            },
            scores: vec![],
        }
    }
}

impl simple_net::Model for Model {
    type PlayerId = Id;
    type Message = Message;
    type Event = Event;
    const TICKS_PER_SECOND: f32 = TICKS_PER_SECOND;
    fn new_player(&mut self) -> Self::PlayerId {
        let player_id = self.next_id;
        self.next_id += 1;
        player_id
    }

    fn drop_player(&mut self, player_id: &Self::PlayerId) {
        if let Some(player) = self.players.remove(&player_id) {
            discord::send_activity(&format!(
                "{} left the server :woman_tipping_hand:",
                player.name
            ));
        }
    }

    fn handle_message(&mut self, player_id: &Self::PlayerId, message: Message) {
        let player_id = *player_id;
        match message {
            Message::UpdatePlayer(mut player) => {
                if player.id != player_id {
                    return;
                }
                if self.players.get(&player_id).is_none() {
                    discord::send_activity(&format!(
                        "{} just joined the server :man_raising_hand:",
                        player.name
                    ));
                }
                self.players.insert(player);
            }
            Message::Score(score) => {
                if let Some(player) = self.players.get(&player_id) {
                    self.scores.push((player.name.clone(), score));
                }
            }
            Message::StartTheRace => {
                if self.avalanche_position.is_none() {
                    for player in &mut self.players {
                        player.position.y = 0.0;
                    }
                    self.scores.clear();
                    self.avalanche_position = Some(Self::AVALANCHE_START);
                }
            }
        }
    }

    fn tick(&mut self, events: &mut Vec<Event>) {
        let delta_time = 1.0 / TICKS_PER_SECOND;
        self.tick += 1;
        if let Some(position) = &mut self.avalanche_position {
            self.avalanche_speed = (self.avalanche_speed
                + delta_time * Self::AVALANCHE_ACCELERATION)
                .min(Self::AVALANCHE_MAX_SPEED);
            *position -= self.avalanche_speed * delta_time;
            if *position < Self::AVALANCHE_START - 5.0 {
                if self.players.iter().all(|player| {
                    !player.is_riding || player.position.y > *position + self.avalanche_speed * 2.0
                }) {
                    self.avalanche_position = None;
                    self.avalanche_speed = Self::AVALANCHE_MIN_SPEED;
                    self.track = Track::new_from_env();
                    if !self.scores.is_empty() {
                        self.scores.sort_by_key(|(_name, score)| -score);
                        let mut text = "Race results:".to_owned();
                        for (index, (name, score)) in self.scores.iter().enumerate() {
                            text.push('\n');
                            text.push_str(&(index + 1).to_string());
                            text.push_str(". ");
                            text.push_str(name);
                            text.push_str(" - ");
                            text.push_str(&score.to_string());
                        }
                        text.push_str("\n<:extremeBoom:963122644373368832>");
                        discord::send_activity(&text);

                        let current_highest_score =
                            self.highscores.values().max().copied().unwrap_or(0);
                        for (name, score) in &self.scores {
                            let score = *score;
                            if score > current_highest_score {
                                discord::send_activity(&format!(
                                    "New highscore of {} by {} <:extremeBoom:963122644373368832>",
                                    score, name,
                                ));
                            }
                            if self.highscores.get(name).copied().unwrap_or(0) < score {
                                self.highscores.insert(name.clone(), score);
                            }
                        }
                        serde_json::to_writer_pretty(
                            std::fs::File::create("highscores.json").unwrap(),
                            &self.highscores,
                        )
                        .unwrap();

                        self.scores.clear();
                    }
                }
            }
            if let Some(winner) = self
                .players
                .iter()
                .filter(|player| player.is_riding)
                .min_by_key(|player| r32(player.position.y))
                .map(|player| (player.name.clone(), -player.position.y))
            {
                self.winner = Some(winner);
            }
        }
    }
}
