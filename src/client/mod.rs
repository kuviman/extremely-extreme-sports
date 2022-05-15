use super::*;

mod game;
mod lobby;
mod player;

use game::Game;
use lobby::Lobby;

pub fn run(
    geng: &Geng,
    assets: &Rc<Assets>,
    player_id: Id,
    opt: &Opt,
    model: simple_net::Remote<Model>,
) -> Box<dyn geng::State> {
    if opt.spectator {
        Box::new(Game::new(&geng, assets, player_id, None, None, model))
    } else {
        Box::new(Lobby::new(&geng, assets, player_id, model))
    }
}
