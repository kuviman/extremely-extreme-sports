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

const DISCORD_LINK: &str = "https://discord.gg/DZaEMPpANY";

fn assets_path() -> std::path::PathBuf {
    run_dir().join("assets")
}

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
        ugli::clear(framebuffer, Some(Rgba::WHITE), None, None);
        self.geng.default_font().draw(
            framebuffer,
            &geng::PixelPerfectCamera,
            "Loading assets...",
            vec2::splat(geng::TextAlign::CENTER),
            mat3::translate(framebuffer_size.map(|x| x as f32) / 2.0) * mat3::scale_uniform(40.0),
            Rgba::BLACK,
        );
    }
}

fn main() {
    logger::init();
    geng::setup_panic_handler();
    let mut opt: Opt = cli::parse();
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
            geng::LoadingScreen::new(&geng, LoadingScreen::new(geng), {
                let geng = geng.clone();
                async move {
                    let mut assets: Assets =
                        geng.asset_manager().load(assets_path()).await.unwrap();
                    assets.process(&geng).await;
                    client::run(&geng, &Rc::new(assets), player_id, &opt, model)
                }
            })
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
        geng.run(state);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some((server_handle, server_thread)) = server {
            server_handle.shutdown();
            server_thread.join().unwrap();
        }
    }
}
