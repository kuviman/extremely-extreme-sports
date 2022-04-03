use super::*;

#[derive(Deref)]
pub struct Texture(#[deref] ugli::Texture);

impl std::borrow::Borrow<ugli::Texture> for &'_ Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}

impl geng::LoadAsset for Texture {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let texture = ugli::Texture::load(geng, path);
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Self(texture))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

#[derive(geng::Assets)]
pub struct Assets {
    pub player: Texture,
    pub ski: Texture,
    #[asset(load_with = "load_obstacles(&geng, &base_path)")]
    pub obstacles: Vec<ObstacleAssets>,
    pub texture_program: ugli::Program,
    pub shadow: ugli::Program,
    pub particle: ugli::Texture,
    pub particle_program: ugli::Program,
    pub border: ugli::Texture,
    pub background: ugli::Texture,
    pub detonator: ugli::Texture,
    pub detonator2: ugli::Texture,
    pub detonate_text: ugli::Texture,
    pub spectating_text: ugli::Texture,
    pub ava_warning: ugli::Texture,
    pub font: Font,
    #[asset(range = "1..=3", path = "crash_sound*.wav")]
    pub crash_sounds: Vec<geng::Sound>,
    pub ride_sound: geng::Sound,
    pub boom: ugli::Texture,
    pub boom_sound: geng::Sound,
    pub avalanche_sound: geng::Sound,
}

async fn load_obstacles(
    geng: &Geng,
    base_path: &std::path::Path,
) -> anyhow::Result<Vec<ObstacleAssets>> {
    let list = <String as geng::LoadAsset>::load(geng, &base_path.join("obstacles.json")).await?;
    let list: Vec<String> = serde_json::from_str(&list)?;
    let mut result = Vec::new();
    for t in list {
        result.push(geng::LoadAsset::load(geng, &base_path.join(t)).await?);
    }
    Ok(result)
}

#[derive(geng::Assets, Deserialize)]
#[asset(json)]
pub struct ObstacleConfig {
    pub hitbox_origin: Vec2<f32>,
    pub hitbox_radius: f32,
    pub spawn_weight: f32,
}

pub struct ObstacleAssets {
    pub config: ObstacleConfig,
    pub texture: ugli::Texture,
}

impl geng::LoadAsset for ObstacleAssets {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let config = <ObstacleConfig as geng::LoadAsset>::load(geng, &{
            let mut path = path.to_owned();
            path.set_extension("json");
            path
        });
        let texture = <ugli::Texture as geng::LoadAsset>::load(geng, &{
            let mut path = path.to_owned();
            path.set_extension("png");
            path
        });
        async move {
            let mut result = Self {
                config: config.await?,
                texture: texture.await?,
            };
            result.texture.set_filter(ugli::Filter::Nearest);
            result.config.hitbox_origin.y =
                result.texture.size().y as f32 - 1.0 - result.config.hitbox_origin.y;
            Ok(result)
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;
}
