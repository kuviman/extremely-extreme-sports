use super::*;

mod track;

use track::*;

pub struct Model {
    pub shared: SharedModel,
    pub track_gen: TrackGen,
}

impl Model {
    pub fn new() -> Self {
        discord::send_activity("Server started :green_circle:");
        let config: Config = Self::read_config();
        let track_gen = TrackGen::new(&config.track);
        Self {
            shared: SharedModel {
                reset_timer: 0.0,
                tick: 0,
                next_id: 0,
                avalanche_position: None,
                avalanche_speed: config.avalanche.min_speed,
                players: default(),
                track: track_gen.init(),
                config,
                winner: None,
                highscores: {
                    let path = std::path::Path::new("highscores.json");
                    if path.is_file() {
                        serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap()
                    } else {
                        default()
                    }
                },
                scores: default(),
            },
            track_gen,
        }
    }
    pub fn read_config() -> Config {
        match std::env::var("CONFIG") {
            Ok(path) => serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap(),
            Err(_) => serde_json::from_reader(
                std::fs::File::open(static_path().join("config.json")).unwrap(),
            )
            .unwrap(),
        }
    }
}

impl simple_net::Model for Model {
    type SharedState = SharedModel;
    fn shared_state(&self) -> &Self::SharedState {
        &self.shared
    }
    type PlayerId = Id;
    type Message = Message;
    type Event = Event;
    const TICKS_PER_SECOND: f32 = TICKS_PER_SECOND;
    fn new_player(&mut self, events: &mut Vec<Event>) -> Self::PlayerId {
        let player_id = self.shared.next_id;
        self.shared.next_id += 1;
        player_id
    }

    fn drop_player(&mut self, events: &mut Vec<Event>, player_id: &Self::PlayerId) {
        if let Some(player) = self.shared.players.remove(&player_id) {
            discord::send_activity(&format!(
                "{} left the server :woman_tipping_hand:",
                player.name
            ));
        }
    }

    fn handle_message(
        &mut self,
        events: &mut Vec<Event>,
        player_id: &Self::PlayerId,
        message: Message,
    ) {
        let player_id = *player_id;
        match message {
            Message::Disconnect => {
                self.drop_player(events, &player_id);
            }
            Message::UpdatePlayer(mut player) => {
                if player.id != player_id {
                    return;
                }
                if self.shared.players.get(&player_id).is_none() {
                    discord::send_activity(&format!(
                        "{} just joined the server :man_raising_hand:",
                        player.name
                    ));
                }
                self.shared.players.insert(player);
            }
            Message::Score(score) => {
                if let Some(player) = self.shared.players.get(&player_id) {
                    let last_score = self.shared.scores.get(&player.name).copied().unwrap_or(0);
                    if score > last_score {
                        self.shared.scores.insert(player.name.clone(), score);
                    }
                }
            }
            Message::StartTheRace => {
                if self.shared.avalanche_position.is_none() {
                    for player in &mut self.shared.players {
                        player.position.y = 0.0;
                    }
                    self.shared.scores.clear();
                    self.shared.avalanche_position = Some(self.shared.config.avalanche.start);
                }
            }
        }
    }

    fn tick(&mut self, events: &mut Vec<Event>) {
        let delta_time = 1.0 / TICKS_PER_SECOND;
        self.shared.tick += 1;
        if let Some(position) = &mut self.shared.avalanche_position {
            self.shared.avalanche_speed = (self.shared.avalanche_speed
                + delta_time * self.shared.config.avalanche.acceleration)
                .min(self.shared.config.avalanche.max_speed);
            *position -= self.shared.avalanche_speed * delta_time;
            if *position < self.shared.config.avalanche.start - 5.0 {
                if self.shared.players.iter().all(|player| {
                    (!player.is_riding && player.parachute.is_none())
                        || player.position.y > *position + self.shared.avalanche_speed * 2.0
                }) {
                    self.shared.reset_timer -= delta_time;
                    if self.shared.reset_timer < 0.0 {
                        self.shared.avalanche_position = None;
                        self.shared.avalanche_speed = self.shared.config.avalanche.min_speed;
                        self.track_gen = TrackGen::new(&self.shared.config.track);
                        self.shared.track = self.track_gen.init();
                        if !self.shared.scores.is_empty() {
                            let mut scores: Vec<(String, i32)> = self
                                .shared
                                .scores
                                .iter()
                                .map(|(a, b)| (a.clone(), *b))
                                .collect();
                            scores.sort_by_key(|(_name, score)| -score);
                            self.shared.winner = Some(scores[0].clone());
                            let mut text = "Race results:".to_owned();
                            for (index, (name, score)) in scores.iter().enumerate() {
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
                                self.shared.highscores.values().max().copied().unwrap_or(0);
                            for (name, score) in &self.shared.scores {
                                let score = *score;
                                if score > current_highest_score {
                                    discord::send_activity(&format!(
                                    "New highscore of {} by {} <:extremeBoom:963122644373368832>",
                                    score, name,
                                ));
                                }
                                if self.shared.highscores.get(name).copied().unwrap_or(0) < score {
                                    self.shared.highscores.insert(name.clone(), score);
                                }
                            }
                            serde_json::to_writer_pretty(
                                std::fs::File::create("highscores.json").unwrap(),
                                &self.shared.highscores,
                            )
                            .unwrap();

                            self.shared.scores.clear();
                        }
                    }
                } else {
                    self.shared.reset_timer = 0.0;
                }
            }
        }
        let pos = self.shared.avalanche_position.unwrap_or(0.0);
        const OFF: f32 = 300.0;
        fn round(x: f32) -> f32 {
            const X: f32 = 100.0;
            x.div_euclid(X) * X
        }
        self.track_gen.update(
            &mut self.shared.track,
            round(pos + OFF),
            round(pos - self.shared.config.avalanche.start - OFF),
        );
    }
}
