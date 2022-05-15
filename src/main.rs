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
use server::Model;

const DISCORD_LINK: &'static str = "https://discord.gg/DZaEMPpANY";

#[derive(clap::Parser, Clone)]
pub struct Opt {
    #[clap(long)]
    server: Option<String>,
    #[clap(long)]
    connect: Option<String>,
    #[clap(long)]
    spectator: bool,
    #[clap(long)]
    auto_sound: bool,
}

struct LoadingScreen {
    geng: Geng,
}

impl LoadingScreen {
    fn new(geng: &Geng) -> Self {
        Self { geng: geng.clone() }
    }
}

impl geng::ProgressScreen for LoadingScreen {}

impl geng::State for LoadingScreen {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Color::WHITE), None);
        self.geng.default_font().draw(
            framebuffer,
            &geng::PixelPerfectCamera,
            "Loading assets...",
            framebuffer_size.map(|x| x as f32) / 2.0,
            geng::TextAlign::CENTER,
            40.0,
            Color::BLACK,
        );
    }
}

fn main() {
    // logger::init().unwrap();
    let mut opt: Opt = program_args::parse();
    if opt.connect.is_none() && opt.server.is_none() {
        if cfg!(target_arch = "wasm32") {
            opt.connect = Some(
                option_env!("CONNECT")
                    .expect("Set CONNECT compile time env var")
                    .to_owned(),
            );
        } else {
            opt.server = Some("127.0.0.1:1155".to_owned());
            opt.connect = Some("ws://127.0.0.1:1155".to_owned());
        }
    }
    let model_constructor = Model::new;
    let game_constructor = {
        let opt = opt.clone();
        move |geng: &Geng, player_id, model| {
            geng::LoadingScreen::new(
                &geng,
                LoadingScreen::new(geng),
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
    if opt.server.is_some() && opt.connect.is_none() {
        #[cfg(not(target_arch = "wasm32"))]
        simple_net::Server::new(opt.server.as_deref().unwrap(), model_constructor()).run();
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        let server = if let Some(addr) = &opt.server {
            let server = simple_net::Server::new(addr, model_constructor());
            let server_handle = server.handle();
            let server_thread = std::thread::spawn(move || {
                server.run();
            });
            Some((server_handle, server_thread))
        } else {
            None
        };

        let geng = Geng::new_with(geng::ContextOptions {
            title: "Extremely Extreme Sports".to_owned(),
            antialias: false,
            ..default()
        });
        let state = simple_net::ConnectingState::new(&geng, opt.connect.as_deref().unwrap(), {
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
