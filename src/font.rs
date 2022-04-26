use super::*;

const CHARS: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

pub struct Font {
    geng: Geng,
    indices: HashMap<char, usize>,
    atlas: geng::TextureAtlas,
}

impl Font {
    pub fn draw(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &geng::Camera2d,
        pos: Vec2<f32>,
        size: f32,
        text: &str,
        align: f32,
        color: Color<f32>,
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
        self.geng.draw_2d(
            framebuffer,
            camera,
            &draw_2d::TexturedPolygon::with_mode(
                {
                    let mut vs = Vec::new();
                    for c in text.chars() {
                        if c == ' ' {
                            pos.x += size;
                            continue;
                        }
                        let uv = self.atlas.uv(self.indices[&c]);
                        let ps = AABB::point(pos).extend_positive(vec2(size, size));
                        pos.x += size * 0.8;
                        vs.push(draw_2d::TexturedVertex {
                            a_pos: vec2(ps.x_min, ps.y_min),
                            a_color: color,
                            a_vt: vec2(uv.x_min, uv.y_min),
                        });
                        vs.push(draw_2d::TexturedVertex {
                            a_pos: vec2(ps.x_max, ps.y_min),
                            a_color: color,
                            a_vt: vec2(uv.x_max, uv.y_min),
                        });
                        vs.push(draw_2d::TexturedVertex {
                            a_pos: vec2(ps.x_max, ps.y_max),
                            a_color: color,
                            a_vt: vec2(uv.x_max, uv.y_max),
                        });

                        vs.push(draw_2d::TexturedVertex {
                            a_pos: vec2(ps.x_min, ps.y_min),
                            a_color: color,
                            a_vt: vec2(uv.x_min, uv.y_min),
                        });
                        vs.push(draw_2d::TexturedVertex {
                            a_pos: vec2(ps.x_max, ps.y_max),
                            a_color: color,
                            a_vt: vec2(uv.x_max, uv.y_max),
                        });
                        vs.push(draw_2d::TexturedVertex {
                            a_pos: vec2(ps.x_min, ps.y_max),
                            a_color: color,
                            a_vt: vec2(uv.x_min, uv.y_max),
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

impl geng::LoadAsset for Font {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let geng = geng.clone();
        let path = path.to_owned();
        async move {
            let mut textures = Vec::new();
            let mut indices = HashMap::new();
            for c in CHARS.chars() {
                indices.insert(c, textures.len());
                textures.push(<ugli::Texture as geng::LoadAsset>::load(
                    &geng,
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
                geng: geng.clone(),
                atlas: geng::TextureAtlas::new(geng.ugli(), &textures, ugli::Filter::Nearest),
            })
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = None;
}
