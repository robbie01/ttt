use tiny_skia::{Color, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, PixmapMut, Shader, Stroke, Transform};

use crate::{game::{Player, State}, N};

const GRID_COLOR: Color = unsafe { Color::from_rgba_unchecked(1., 159./255., 244./255., 1.) };
const TILE_COLOR: Color = unsafe { Color::from_rgba_unchecked(216./255., 159./255., 1., 1.) };

const GRID_PAINT: &Paint = &Paint {
    shader: Shader::SolidColor(GRID_COLOR),
    blend_mode: tiny_skia::BlendMode::SourceOver,
    anti_alias: true,
    force_hq_pipeline: false,
    colorspace: tiny_skia::ColorSpace::Linear
};

const TILE_PAINT: &Paint = &Paint {
    shader: Shader::SolidColor(TILE_COLOR),
    blend_mode: tiny_skia::BlendMode::SourceOver,
    anti_alias: true,
    force_hq_pipeline: false,
    colorspace: tiny_skia::ColorSpace::Linear
};

const STROKE: &Stroke = &Stroke {
    width: 5. / 3.,
    miter_limit: 4.,
    line_cap: LineCap::Round,
    line_join: LineJoin::Miter,
    dash: None
};

enum Drawable {
    Stroke(Path, &'static Paint<'static>, &'static Stroke)
}

impl From<Drawable> for PathBuilder {
    fn from(value: Drawable) -> Self {
        match value {
            Drawable::Stroke(path, _, _) => path
        }.clear()
    }
}

#[derive(Default)]
pub struct Renderer {
    path_buffers: Vec<PathBuilder>,
    paths: Vec<Drawable>
}

fn draw_x(builder: &mut PathBuilder, x: u32, y: u32) {
    builder.move_to(x as f32 + 0.2, y as f32 + 0.2);
    builder.line_to((x + 1) as f32 - 0.2, (y + 1) as f32 - 0.2);
    builder.move_to((x + 1) as f32 - 0.2, y as f32 + 0.2);
    builder.line_to(x as f32 + 0.2, (y + 1) as f32 - 0.2);
}

fn draw_o(builder: &mut PathBuilder, x: u32, y: u32) {
    builder.push_circle((x as f32) + 0.5, (y as f32) + 0.5, 0.3);
}

impl Renderer {
    pub fn prepare(&mut self, st: &State) {
        self.path_buffers.extend(self.paths.drain(..).map(Into::into));

        {
            let mut path_buffer = self.path_buffers.pop().unwrap_or_default();

            for k in 1..N {
                let k = k as f32;
                path_buffer.move_to(k, 0.1);
                path_buffer.line_to(k, (N as f32) - 0.1);
                path_buffer.move_to(0.1, k);
                path_buffer.line_to((N as f32) - 0.1, k);
            }

            if !path_buffer.is_empty() {
                let path = path_buffer.finish().unwrap().transform(Transform::from_scale(100. / N as f32, 100. / N as f32)).unwrap();
                self.paths.push(Drawable::Stroke(
                    path,
                    GRID_PAINT,
                    STROKE
                ));
            } else {
                self.path_buffers.push(path_buffer);
            }
        }

        {
            let mut path_buffer = self.path_buffers.pop().unwrap_or_default();

            for (i, player) in st.board().into_iter().enumerate() {
                let i = i as u32;
                let x = i % N;
                let y = i / N;
                if player == Some(Player::X) {
                    draw_x(&mut path_buffer, x, y);
                } else if player == Some(Player::O) {
                    draw_o(&mut path_buffer, x, y);
                }
            }

            if !path_buffer.is_empty() {
                let path = path_buffer.finish().unwrap().transform(Transform::from_scale(100. / N as f32, 100. / N as f32)).unwrap();
                self.paths.push(Drawable::Stroke(
                    path,
                    TILE_PAINT,
                    STROKE
                ));
            } else {
                self.path_buffers.push(path_buffer);
            }
        }
    }

    pub fn render(&self, target: &mut PixmapMut<'_>, world_transform: Transform, world_mask: Option<&Mask>) {
        for drawable in &self.paths {
            match *drawable {
                Drawable::Stroke(ref path, paint, stroke) => {
                    target.stroke_path(path, paint, stroke, world_transform, world_mask);
                }
            }
        }
    }
}