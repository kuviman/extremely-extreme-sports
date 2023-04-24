use super::*;

#[derive(Deref, DerefMut)]
pub struct Texture(ugli::Texture);

impl ugli::Uniform for Texture {
    fn apply(&self, gl: &ugli::raw::Context, info: &ugli::UniformInfo) {
        ugli::Texture::apply(self, gl, info)
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

impl geng::asset::Load for Texture {
    fn load(manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        let texture = ugli::Texture::load(manager, path);
        async move { Ok(texture.await?.into()) }.boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}
