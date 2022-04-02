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
    pub tree: ObstacleAssets,
    pub texture_program: ugli::Program,
    pub shadow: ugli::Program,
}

#[derive(geng::Assets, Deserialize)]
#[asset(json)]
pub struct ObstacleConfig {
    pub hitbox_origin: Vec2<f32>,
    pub hitbox_radius: f32,
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
            result.config.hitbox_origin.y =
                result.texture.size().y as f32 - 1.0 - result.config.hitbox_origin.y;
            Ok(result)
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;
}
