use std::{cell::Cell, sync::LazyLock};

use tiny_skia::{Color, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, PixmapMut, Shader, Stroke, Transform};

use crate::{game::{Player, State}, N};

static GRID_PAINT: LazyLock<Paint> = LazyLock::new(|| Paint {
    shader: Shader::SolidColor(Color::from_rgba8(255, 159, 244, 255)),
    ..Default::default()
});

static TILE_PAINT: LazyLock<Paint> = LazyLock::new(|| Paint {
    shader: Shader::SolidColor(Color::from_rgba8(216, 159, 255, 255)),
    ..Default::default()
});

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

#[derive(Default)]
pub struct Renderer {
    path_buffers: Cell<Vec<PathBuilder>>,
    paths: Cell<Vec<Drawable>>
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
        {
            let mut path_buffer = self.path_buffers.get_mut().pop().unwrap_or_default();

            for k in 1..N {
                let k = k as f32;
                path_buffer.move_to(k, 0.1);
                path_buffer.line_to(k, (N as f32) - 0.1);
                path_buffer.move_to(0.1, k);
                path_buffer.line_to((N as f32) - 0.1, k);
            }

            if !path_buffer.is_empty() {
                let path = path_buffer.finish().unwrap().transform(Transform::from_scale(100. / N as f32, 100. / N as f32)).unwrap();
                self.paths.get_mut().push(Drawable::Stroke(
                    path,
                    &GRID_PAINT,
                    STROKE
                ));
            } else {
                self.path_buffers.get_mut().push(path_buffer);
            }
        }

        {
            let mut path_buffer = self.path_buffers.get_mut().pop().unwrap_or_default();

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
                self.paths.get_mut().push(Drawable::Stroke(
                    path,
                    &TILE_PAINT,
                    STROKE
                ));
            } else {
                self.path_buffers.get_mut().push(path_buffer);
            }
        }
    }

    pub fn render(&self, target: &mut PixmapMut<'_>, world_transform: Transform, world_mask: Option<&Mask>) {
        let mut path_buffers = self.path_buffers.take();

        for drawable in self.paths.take() {
            match drawable {
                Drawable::Stroke(path, paint, stroke) => {
                    target.stroke_path(&path, paint, stroke, world_transform, world_mask);
                    path_buffers.push(path.clear());
                }
            }
        }

        self.path_buffers.set(path_buffers);
    }
}