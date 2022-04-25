use super::*;

#[derive(Deref, DerefMut)]
pub struct Texture(ugli::Texture);

impl ugli::AsUniform for Texture {
    type Uniform = ugli::Texture;
    fn as_uniform(&self) -> &Self::Uniform {
        &self.0
    }
}

impl std::borrow::Borrow<ugli::Texture> for Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}

impl std::borrow::Borrow<ugli::Texture> for &'_ Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}

impl From<ugli::Texture> for Texture {
    fn from(mut texture: ugli::Texture) -> Self {
        texture.set_filter(ugli::Filter::Nearest);
        Self(texture)
    }
}

impl geng::LoadAsset for Texture {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let texture = ugli::Texture::load(geng, path);
        async move { Ok(texture.await?.into()) }.boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}
