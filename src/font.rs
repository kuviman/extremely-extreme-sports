use super::*;

const CHARS: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

pub struct Font {
    draw2d: geng::draw2d::Helper,
    indices: HashMap<char, usize>,
    atlas: geng::TextureAtlas,
}

impl Font {
    pub fn draw(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &geng::Camera2d,
        pos: vec2<f32>,
        size: f32,
        text: &str,
        align: f32,
        color: Rgba<f32>,
    ) {
        if text.is_empty() {
            return;
        }
        let mut pos = pos;
        let mut width = 0.0;

        for c in text.chars() {
            if c == ' ' {
                width += size;
                continue;
            }
            width += size * 0.8;
        }
        pos.x -= width * align;
        self.draw2d.draw2d(
            framebuffer,
            camera,
            &draw2d::TexturedPolygon::with_mode(
                {
                    let mut vs = Vec::new();
                    for c in text.chars() {
                        if c == ' ' {
                            pos.x += size;
                            continue;
                        }
                        if !self.indices.contains_key(&c) {
                            continue;
                        }
                        let uv = self.atlas.uv(self.indices[&c]);
                        let ps = Aabb2::point(pos).extend_positive(vec2(size, size));
                        pos.x += size * 0.8;
                        vs.push(draw2d::TexturedVertex {
                            a_pos: vec2(ps.min.x, ps.min.y),
                            a_color: color,
                            a_vt: vec2(uv.min.x, uv.min.y),
                        });
                        vs.push(draw2d::TexturedVertex {
                            a_pos: vec2(ps.max.x, ps.min.y),
                            a_color: color,
                            a_vt: vec2(uv.max.x, uv.min.y),
                        });
                        vs.push(draw2d::TexturedVertex {
                            a_pos: vec2(ps.max.x, ps.max.y),
                            a_color: color,
                            a_vt: vec2(uv.max.x, uv.max.y),
                        });

                        vs.push(draw2d::TexturedVertex {
                            a_pos: vec2(ps.min.x, ps.min.y),
                            a_color: color,
                            a_vt: vec2(uv.min.x, uv.min.y),
                        });
                        vs.push(draw2d::TexturedVertex {
                            a_pos: vec2(ps.max.x, ps.max.y),
                            a_color: color,
                            a_vt: vec2(uv.max.x, uv.max.y),
                        });
                        vs.push(draw2d::TexturedVertex {
                            a_pos: vec2(ps.min.x, ps.max.y),
                            a_color: color,
                            a_vt: vec2(uv.min.x, uv.max.y),
                        });
                    }
                    vs
                },
                self.atlas.texture(),
                ugli::DrawMode::Triangles,
            ),
        );
    }
}

impl geng::asset::Load for Font {
    fn load(manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        let manager = manager.clone();
        let path = path.to_owned();
        async move {
            let mut textures = Vec::new();
            let mut indices = HashMap::new();
            for c in CHARS.chars() {
                indices.insert(c, textures.len());
                textures.push(<ugli::Texture as geng::asset::Load>::load(
                    &manager,
                    &path.join(format!("{}.png", c)),
                ));
            }
            let mut textures: Vec<ugli::Texture> = futures::future::join_all(textures)
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            for texture in &mut textures {
                texture.set_filter(ugli::Filter::Nearest);
            }
            let textures: Vec<&ugli::Texture> = textures.iter().collect();
            Ok(Self {
                indices,
                draw2d: geng::draw2d::Helper::new(manager.ugli(), false),
                atlas: geng::TextureAtlas::new(manager.ugli(), &textures, ugli::Filter::Nearest),
            })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;
}
