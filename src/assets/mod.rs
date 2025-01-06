use super::*;

mod texture;

pub use texture::*;

#[derive(geng::asset::Load)]
// #[load(sequential)]
pub struct PlayerAssets {
    #[load(load_with = "load_items(&manager, base_path.join(\"coat\"))")]
    pub coat: HashMap<String, skin::ItemConfig>,
    #[load(load_with = "load_items(&manager, base_path.join(\"hat\"))")]
    pub hat: HashMap<String, skin::ItemConfig>,
    #[load(load_with = "load_items(&manager, base_path.join(\"pants\"))")]
    pub pants: HashMap<String, skin::ItemConfig>,
    #[load(load_with = "load_items(&manager, base_path.join(\"face\"))")]
    pub face: HashMap<String, skin::ItemConfig>,
    #[load(load_with = "load_equipment(&manager, base_path.join(\"equipment\"))")]
    pub equipment: HashMap<String, ugli::Texture>,
    pub body: skin::ItemConfig,
    #[load(load_with = "load_secret(&manager, base_path.to_owned())")]
    pub secret: HashMap<String, skin::SecretConfig>,
    pub parachute: skin::ItemConfig,
}

async fn load_equipment(
    manager: &geng::asset::Manager,
    base_path: std::path::PathBuf,
) -> anyhow::Result<HashMap<String, ugli::Texture>> {
    let json: String = manager.load_string(base_path.join("_list.json")).await?;
    let list: Vec<String> = serde_json::from_str(&json)?;
    let result = future::join_all(
        list.iter()
            .map(|path| manager.load(base_path.join(format!("{}.png", path)))),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    Ok(list.into_iter().zip(result).collect())
}

async fn load_items(
    manager: &geng::asset::Manager,
    base_path: std::path::PathBuf,
) -> anyhow::Result<HashMap<String, skin::ItemConfig>> {
    let json: String = manager.load_string(base_path.join("_list.json")).await?;
    let list: Vec<String> = serde_json::from_str(&json)?;
    let result = future::join_all(
        list.iter()
            .map(|path| manager.load(base_path.join(format!("{}.json", path)))),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    Ok(list.into_iter().zip(result).collect())
}

async fn load_secret(
    manager: &geng::asset::Manager,
    path: std::path::PathBuf,
) -> anyhow::Result<HashMap<String, skin::SecretConfig>> {
    let base_path = path.join("secret");
    let json: String =
        geng::asset::Load::load(manager, &base_path.join("_list.json"), &default()).await?;
    let list: Vec<String> = serde_json::from_str(&json)?;
    let result = future::join_all(list.iter().map(|path| {
        geng::asset::Load::load(
            manager,
            &base_path.join(path).join("config.json"),
            &default(),
        )
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    Ok(list.into_iter().zip(result).collect())
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub player: PlayerAssets,
    #[load(load_with = "load_obstacles(&manager, &base_path)")]
    pub obstacles: Vec<ObstacleAssets>,
    pub texture_program: ugli::Program,
    pub shadow: ugli::Program,
    pub particle: Texture,
    pub particle_program: ugli::Program,
    pub border: Texture,
    pub background: Texture,
    pub detonator: Texture,
    pub detonator2: Texture,
    pub detonate_text: Texture,
    pub spectating_text: Texture,
    pub walk: Texture,
    pub ava_warning: Texture,
    pub font: Font,
    #[load(list = "1..=3", path = "crash_sound*.wav")]
    pub crash_sounds: Vec<geng::Sound>,
    #[load(list = "1..=4", path = "emotes/*.png")]
    pub emotes: Vec<Texture>,
    pub ride_sound: geng::Sound,
    pub boom: Texture,
    pub spawn: Texture,
    pub boom_sound: geng::Sound,
    pub avalanche_sound: geng::Sound,
    pub spawn_sound: geng::Sound,
    #[load(path = "LD-50.mp3")]
    pub music: geng::Sound,
    #[load(load_with = "async { Ok::<_, anyhow::Error>(HashMap::new()) }")]
    pub textures: HashMap<String, Texture>,
}

impl Assets {
    pub async fn process(&mut self, geng: &Geng) {
        self.border.set_wrap_mode(ugli::WrapMode::Repeat);
        self.ride_sound.looped = true;
        self.avalanche_sound.looped = true;
        self.music.looped = true;
        let mut paths = Vec::new();
        paths.extend(
            self.player
                .body
                .parts
                .iter()
                .map(|part| part.texture.as_str()),
        );
        paths.extend(
            self.player
                .parachute
                .parts
                .iter()
                .map(|part| part.texture.as_str()),
        );
        paths.extend(
            self.player
                .hat
                .values()
                .flat_map(|item| item.parts.iter().map(|part| part.texture.as_str())),
        );
        paths.extend(
            self.player
                .face
                .values()
                .flat_map(|item| item.parts.iter().map(|part| part.texture.as_str())),
        );
        paths.extend(
            self.player
                .coat
                .values()
                .flat_map(|item| item.parts.iter().map(|part| part.texture.as_str())),
        );
        paths.extend(
            self.player
                .pants
                .values()
                .flat_map(|item| item.parts.iter().map(|part| part.texture.as_str())),
        );
        for secret in self.player.secret.values() {
            if let Some(parts) = &secret.parts {
                paths.extend(parts.iter().map(|part| part.texture.as_str()));
            }
            if let Some(name) = &secret.hat {
                if name.ends_with(".png") {
                    paths.push(name.as_str());
                }
            }
            if let Some(name) = &secret.coat {
                if name.ends_with(".png") {
                    paths.push(name.as_str());
                }
            }
            if let Some(name) = &secret.pants {
                if name.ends_with(".png") {
                    paths.push(name.as_str());
                }
            }
            if let Some(name) = &secret.equipment {
                if name.ends_with(".png") {
                    paths.push(name.as_str());
                }
            }
            if let Some(name) = &secret.face {
                if name.ends_with(".png") {
                    paths.push(name.as_str());
                }
            }
        }
        let result = future::join_all(paths.iter().map(|path| {
            geng::asset::Load::load(geng.asset_manager(), &assets_path().join(path), &default())
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        for (path, texture) in paths.into_iter().zip(result) {
            self.textures.insert(path.to_owned(), texture);
        }
    }
}

async fn load_obstacles(
    manager: &geng::asset::Manager,
    base_path: &std::path::Path,
) -> anyhow::Result<Vec<ObstacleAssets>> {
    let list =
        <String as geng::asset::Load>::load(manager, &base_path.join("obstacles.json"), &default())
            .await?;
    let list: Vec<String> = serde_json::from_str(&list)?;
    let mut result = Vec::new();
    for t in list {
        result.push(geng::asset::Load::load(manager, &base_path.join(t), &default()).await?);
    }
    Ok(result)
}

#[derive(geng::asset::Load, Deserialize)]
#[load(serde = "json")]
pub struct ObstacleConfig {
    pub hitbox_origin: vec2<f32>,
    pub hitbox_radius: f32,
    pub spawn_weight: f32,
}

pub struct ObstacleAssets {
    pub config: ObstacleConfig,
    pub texture: Texture,
}

impl geng::asset::Load for ObstacleAssets {
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        _options: &(),
    ) -> geng::asset::Future<Self> {
        let config = <ObstacleConfig as geng::asset::Load>::load(
            manager,
            &{
                let mut path = path.to_owned();
                path.set_extension("json");
                path
            },
            &default(),
        );
        let texture = <Texture as geng::asset::Load>::load(
            manager,
            &{
                let mut path = path.to_owned();
                path.set_extension("png");
                path
            },
            &default(),
        );
        async move {
            let mut result = Self {
                config: config.await?,
                texture: texture.await?,
            };
            result.config.hitbox_origin.y =
                result.texture.size().y as f32 - 1.0 - result.config.hitbox_origin.y;
            Ok(result)
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = None;
    type Options = ();
}
