mod ai;
mod game;

use std::{iter, marker::PhantomData, rc::Rc, time::Duration};

use async_io::Timer;
use async_task::Runnable;
use softbuffer::{Context, Surface};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Shader, Stroke, Transform};
use winit::{application::ApplicationHandler, dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, MouseButton}, event_loop::{EventLoop, EventLoopProxy}, window::{Window, WindowAttributes}};

use crate::{ai::maximize, game::{Player, State}};

const N: u32 = 3;

fn draw_x(builder: &mut PathBuilder, x: u32, y: u32) {
    builder.move_to(x as f32 + 0.2, y as f32 + 0.2);
    builder.line_to((x + 1) as f32 - 0.2, (y + 1) as f32 - 0.2);
    builder.move_to((x + 1) as f32 - 0.2, y as f32 + 0.2);
    builder.line_to(x as f32 + 0.2, (y + 1) as f32 - 0.2);
}

fn draw_o(builder: &mut PathBuilder, x: u32, y: u32) {
    builder.push_circle((x as f32) + 0.5, (y as f32) + 0.5, 0.3);
}

enum AsyncEvent {
    Runnable(Runnable),
    Ready(Box<dyn FnOnce(&mut App)>)
}

struct App {
    pxy: EventLoopProxy<AsyncEvent>,
    last_mouse_pos: PhysicalPosition<f64>,
    board: State,
    win: Option<Rc<Window>>,
    sfc: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    fb: Pixmap,

    _phantom: PhantomData<*mut ()>
}

impl App {
    fn new(pxy: EventLoopProxy<AsyncEvent>) -> Self {
        let board = State::default();
        println!("1");
        let (_, (x, y)) = maximize(board, Player::X);
        println!("2");

        Self {
            pxy,
            last_mouse_pos: Default::default(),
            board: board.do_move(x, y).unwrap(),
            win: None,
            sfc: None,
            fb: Pixmap::new(1, 1).unwrap(),
            _phantom: PhantomData
        }
    }

    #[expect(dead_code)]
    fn spawn<T: 'static>(&self, fut: impl Future<Output = T> + 'static) {
        let pxy = self.pxy.clone();
        let (r, t) = unsafe { async_task::spawn_unchecked(
            fut,
            move |r| { let _ = pxy.send_event(AsyncEvent::Runnable(r)); }
        ) };
        t.detach();
        r.schedule();
    }

    fn spawn_cb<T: 'static>(&self, fut: impl Future<Output = T> + 'static, cb: impl for<'a> FnOnce(&'a mut App, T) + 'static) {
        let pxy1 = self.pxy.clone();
        let pxy2 = self.pxy.clone();
        let (r, t) = unsafe { async_task::spawn_unchecked(
            async move {
                let res = fut.await;
                let cb = Box::new(move |this: &mut App| {
                    cb(this, res);
                });
                let _ = pxy1.send_event(AsyncEvent::Ready(cb));
            },
            move |r| { let _ = pxy2.send_event(AsyncEvent::Runnable(r)); }
        ) };
        t.detach();
        r.schedule();
    }
}

impl ApplicationHandler<AsyncEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        assert!(self.win.is_none());

        let win = Rc::new(event_loop.create_window(WindowAttributes::default()
            .with_resizable(false)
            .with_inner_size(PhysicalSize::new(100 * N, 100 * N))).unwrap());

        let ctx = Context::new(win.clone()).unwrap();
        let mut sfc = Surface::new(&ctx, win.clone()).unwrap();

        let sz = win.inner_size();
        sfc.resize(sz.width.try_into().unwrap(), sz.height.try_into().unwrap()).unwrap();
        let fb = Pixmap::new(sz.width, sz.height).unwrap();
        
        self.win = Some(win);
        self.sfc = Some(sfc);
        self.fb = fb;
        
        self.spawn_cb(Timer::after(Duration::from_secs(1)), |a, _| println!("WHAT THE FUCK {:?}", a as *mut _));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent::*;

        match event {
            CloseRequested => event_loop.exit(),
            RedrawRequested => {
                let fb = &mut self.fb;
                fb.fill(Color::WHITE);
                let mut paint = Paint::default();
                let stroke = Stroke {
                    width: 5.,
                    ..Default::default()
                };
                let mut path_buffer = PathBuilder::new();

                for k in 1..N {
                    let k = k as f32;
                    path_buffer.move_to(k, 0.1);
                    path_buffer.line_to(k, (N as f32) - 0.1);
                    path_buffer.move_to(0.1, k);
                    path_buffer.line_to((N as f32) - 0.1, k);
                }

                if !path_buffer.is_empty() {
                    let path = path_buffer.finish().unwrap().transform(Transform::from_scale(100., 100.)).unwrap();
                    paint.shader = Shader::SolidColor(Color::from_rgba8(255, 159, 244, 255));
                    fb.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                    path_buffer = path.clear();
                }

                for (i, player) in self.board.board().into_iter().enumerate() {
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
                    let path = path_buffer.finish().unwrap().transform(Transform::from_scale(100., 100.)).unwrap();
                    paint.shader = Shader::SolidColor(Color::from_rgba8(216, 159, 255, 255));
                    fb.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }

                let sfc = self.sfc.as_mut().unwrap();
                let mut buf = sfc.buffer_mut().unwrap();
                for (dst, src) in iter::zip(&mut buf[..], fb.pixels()) {
                    let src = src.demultiply();
                    *dst = u32::from(src.red()) << 16 | u32::from(src.green()) << 8 | u32::from(src.blue());
                }
                buf.present().unwrap();
            },
            CursorMoved { device_id: _, position } => {
                self.last_mouse_pos = position;
            },
            MouseInput { device_id: _, state: ElementState::Pressed, button: MouseButton::Left } => {
                let x = (self.last_mouse_pos.x / 100.) as u32;
                let y = (self.last_mouse_pos.y / 100.) as u32;

                if self.board.turn() == Player::O && self.board.score().is_none() && let Ok(mut nst) = self.board.do_move(x, y) {
                    let (_, (x, y)) = maximize(nst, Player::X);
                    if x != 255 && y != 255 {
                        nst = nst.do_move(x, y).unwrap();
                    }
                    self.board = nst;
                    self.win.as_ref().unwrap().request_redraw();
                }
            },
            _ => ()
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: AsyncEvent) {
        match event {
            AsyncEvent::Runnable(r) => { r.run(); },
            AsyncEvent::Ready(cb) => { cb(self); }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let evt = EventLoop::with_user_event().build()?;
    let mut app = App::new(evt.create_proxy());
    evt.run_app(&mut app)?;
    Ok(())
}
