mod ai;
mod game;
mod rend;

use std::{cell::Cell, iter, marker::PhantomData, rc::Rc, time::Duration};

use async_io::Timer;
use async_task::Runnable;
use softbuffer::{Context, Surface};
use tiny_skia::{Color, FillRule, IntSize, Mask, NonZeroRect, PathBuilder, Pixmap, Point, Rect, Transform};
use winit::{application::ApplicationHandler, dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, MouseButton}, event_loop::{EventLoop, EventLoopProxy, OwnedDisplayHandle}, window::{Window, WindowAttributes}};

use crate::{ai::maximize, game::{Player, State}, rend::Renderer};

const N: u32 = 3;

enum AsyncEvent {
    Runnable(Runnable)
}

struct App {
    pxy: EventLoopProxy<AsyncEvent>,
    async_cb: Rc<Cell<Option<Box<dyn FnOnce(&mut App)>>>>,
    last_mouse_pos: PhysicalPosition<f64>,
    board: State,
    sfc: Option<softbuffer::Surface<OwnedDisplayHandle, Window>>,
    fb: Option<Pixmap>,
    mask: Option<Mask>,
    transform: Transform,
    rend: Renderer,

    _phantom: PhantomData<*mut ()>
}

impl App {
    fn new(pxy: EventLoopProxy<AsyncEvent>) -> Self {
        let board = State::default();
        println!("1");
        let (x, y) = maximize(board, Player::X).1.unwrap();
        println!("2");

        Self {
            pxy,
            async_cb: Rc::new(Cell::new(None)),
            last_mouse_pos: Default::default(),
            board: board.do_move(x, y).unwrap(),
            sfc: None,
            fb: None,
            mask: None,
            transform: Transform::identity(),
            rend: Renderer::default(),
            _phantom: PhantomData
        }
    }

    #[expect(dead_code)]
    fn spawn<T: 'static>(&self, fut: impl Future<Output = T> + 'static) {
        let pxy = self.pxy.clone();
        let (r, t) = async_task::spawn_local(
            fut,
            move |r| { let _ = pxy.send_event(AsyncEvent::Runnable(r)); }
        );
        t.detach();
        r.schedule();
    }

    fn spawn_cb<T: 'static>(&self, fut: impl Future<Output = T> + 'static, cb: impl for<'a> FnOnce(&'a mut App, T) + 'static) {
        let pxy = self.pxy.clone();
        let async_cb = self.async_cb.clone();
        let (r, t) = async_task::spawn_local(
            async move {
                let res = fut.await;
                let cb = Box::new(move |this: &mut App| {
                    cb(this, res);
                });
                let prev = async_cb.replace(Some(cb));
                assert!(prev.is_none());
            },
            move |r| { let _ = pxy.send_event(AsyncEvent::Runnable(r)); }
        );
        t.detach();
        r.schedule();
    }

    fn on_resize(&mut self, w: u32, h: u32) {
        let (x, y, s) = if w >= h {
            ((w - h) / 2, 0, h)
        } else {
            (0, (h - w) / 2, w)
        };

        self.transform = Transform::from_bbox(NonZeroRect::from_xywh(x as f32, y as f32, s as f32 / 100., s as f32 / 100.).unwrap());

        self.sfc.as_mut().unwrap().resize(w.try_into().unwrap(), h.try_into().unwrap()).unwrap();
        
        let sz = IntSize::from_wh(w, h).unwrap();
        self.fb = Some(match self.fb.take() {
            None => Pixmap::new(w, h).unwrap(),
            Some(fb) => {
                let mut fb = fb.take();
                fb.resize(4 * (w as usize) * (h as usize), 0);
                Pixmap::from_vec(fb, sz).unwrap()
            }
        });

        self.mask = Some({
            let mut mask = match self.mask.take() {
                None => Mask::new(w, h).unwrap(),
                Some(mask) => {
                    let mut mask = mask.take();
                    mask.resize((w as usize) * (h as usize), 0);
                    Mask::from_vec(mask, sz).unwrap()
                }
            };
            mask.fill_path(
                &PathBuilder::from_rect(Rect::from_xywh(0., 0., 100., 100.).unwrap()),
                FillRule::Winding,
                false,
                self.transform
            );
            mask
        });
    }
}

impl ApplicationHandler<AsyncEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        assert!(self.sfc.is_none());

        let win = event_loop.create_window(WindowAttributes::default()
            .with_resizable(true)
            .with_inner_size(PhysicalSize::new(300, 300))).unwrap();

        let ctx = Context::new(event_loop.owned_display_handle()).unwrap();
        let sfc = Surface::new(&ctx, win).unwrap();

        let sz = sfc.window().inner_size();
        
        self.sfc = Some(sfc);

        self.on_resize(sz.width, sz.height);
        
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
                self.rend.prepare(&self.board);

                let fb = self.fb.as_mut().unwrap();
                fb.fill(Color::WHITE);

                self.rend.render(&mut fb.as_mut(), self.transform, self.mask.as_ref());

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
                println!("clicky");
                let mut pt = Point { x: self.last_mouse_pos.x as f32, y: self.last_mouse_pos.y as f32 };
                self.transform.invert().unwrap().map_point(&mut pt);

                if pt.x < 0. || pt.x > 100. || pt.y < 0. || pt.y > 100. {
                    return
                }

                let x = (pt.x * (N as f32) / 100.) as u8;
                let y = (pt.y * (N as f32) / 100.) as u8;

                if self.board.turn() == Player::O && self.board.score().is_none() && let Ok(nst) = self.board.do_move(x, y) {
                    self.board = nst;
                    self.sfc.as_ref().unwrap().window().request_redraw();

                    self.spawn_cb(
                        async move {
                            Timer::after(Duration::from_millis(200)).await;
                            let (_, pos) = maximize(nst, Player::X);
                            if let Some((x, y)) = pos {
                                Some(nst.do_move(x, y).unwrap())
                            } else {
                                None
                            }
                        }, |this, nst| {
                            if let Some(nst) = nst {
                                this.board = nst;
                                this.sfc.as_ref().unwrap().window().request_redraw();
                            }
                        }
                    );
                }
            },
            Resized(PhysicalSize { width, height }) => self.on_resize(width, height),
            _ => ()
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: AsyncEvent) {
        match event {
            AsyncEvent::Runnable(r) => {
                r.run();
                if let Some(cb) = self.async_cb.take() {
                    cb(self);
                }
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let evt = EventLoop::with_user_event().build()?;
    let mut app = App::new(evt.create_proxy());
    evt.run_app(&mut app)?;
    Ok(())
}
