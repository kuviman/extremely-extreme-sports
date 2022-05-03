use geng::prelude::*;

mod assets;
mod client;
mod discord;
mod font;
mod model;
mod server;
mod simple_net;
mod skin;
mod ui;

use assets::*;
use font::*;
use model::*;

const DISCORD_LINK: &'static str = "https://discord.gg/DZaEMPpANY";

#[derive(clap::Parser, Clone)]
pub struct Opt {
    #[clap(long)]
    addr: Option<String>,
    #[clap(long)]
    server: bool,
    #[clap(long)]
    with_server: bool,
    #[clap(long)]
    spectator: bool,
    #[clap(long)]
    auto_sound: bool,
}

impl Opt {
    pub fn addr(&self) -> &str {
        match &self.addr {
            Some(addr) => addr,
            None => option_env!("SERVER_ADDR").unwrap_or("127.0.0.1:1155"),
        }
    }
}

fn main() {
    // logger::init().unwrap();
    let opt: Opt = program_args::parse();
    let model_constructor = Model::new;
    let game_constructor = {
        let opt = opt.clone();
        move |geng: &Geng, player_id, model| {
            geng::LoadingScreen::new(
                &geng,
                geng::EmptyLoadingScreen,
                <Assets as geng::LoadAsset>::load(&geng, &static_path()).then({
                    let geng = geng.clone();
                    move |assets| async move {
                        match assets {
                            Ok(mut assets) => {
                                assets.process(&geng).await;
                                Ok(assets)
                            }
                            Err(e) => Err(e),
                        }
                    }
                }),
                {
                    let geng = geng.clone();
                    move |assets| {
                        let mut assets = assets.expect("Failed to load assets");
                        client::run(&geng, &Rc::new(assets), player_id, &opt, model)
                    }
                },
            )
        }
    };
    if opt.server {
        #[cfg(not(target_arch = "wasm32"))]
        simple_net::Server::new(opt.addr(), model_constructor()).run();
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        let server = if opt.with_server {
            let server = simple_net::Server::new(opt.addr(), model_constructor());
            let server_handle = server.handle();
            let server_thread = std::thread::spawn(move || {
                server.run();
            });
            Some((server_handle, server_thread))
        } else {
            None
        };

        let geng = Geng::new("Extremely Extreme Sports");
        let state = simple_net::ConnectingState::new(&geng, opt.addr(), {
            let geng = geng.clone();
            move |player_id, model| game_constructor(&geng, player_id, model)
        });
        geng::run(&geng, state);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some((server_handle, server_thread)) = server {
            server_handle.shutdown();
            server_thread.join().unwrap();
        }
    }
}
