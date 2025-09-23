mod ai;
mod game;
mod rend;

use std::{iter, marker::PhantomData, rc::Rc, time::Duration};

use async_io::Timer;
use async_task::Runnable;
use softbuffer::{Context, Surface};
use tiny_skia::{Color, FillRule, Mask, NonZeroRect, PathBuilder, Pixmap, Point, Rect, Transform};
use winit::{application::ApplicationHandler, dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, MouseButton}, event_loop::{EventLoop, EventLoopProxy}, window::{Window, WindowAttributes}};

use crate::{ai::maximize, game::{Player, State}, rend::Renderer};

const N: u32 = 3;

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
    mask: Mask,
    transform: Transform,
    rend: Renderer,

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
            mask: Mask::new(1, 1).unwrap(),
            transform: Transform::identity(),
            rend: Renderer::default(),
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

    fn on_resize(&mut self, w: u32, h: u32) {
        self.sfc.as_mut().unwrap().resize(w.try_into().unwrap(), h.try_into().unwrap()).unwrap();
        self.fb = Pixmap::new(w, h).unwrap(); // todo: reuse buffer
        self.mask = Mask::new(w, h).unwrap();

        let (x, y, s) = if w >= h {
            ((w - h) / 2, 0, h)
        } else {
            (0, (h - w) / 2, w)
        };

        self.mask.fill_path(
            &PathBuilder::from_rect(Rect::from_xywh(x as f32, y as f32, s as f32, s as f32).unwrap()),
            FillRule::Winding,
            false,
            Transform::identity()
        );
        self.transform = Transform::from_bbox(NonZeroRect::from_xywh(x as f32, y as f32, s as f32 / 100., s as f32 / 100.).unwrap())
    }
}

impl ApplicationHandler<AsyncEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        assert!(self.win.is_none());

        let win = Rc::new(event_loop.create_window(WindowAttributes::default()
            .with_resizable(true)
            .with_inner_size(PhysicalSize::new(300, 300))).unwrap());

        let ctx = Context::new(win.clone()).unwrap();
        let sfc = Surface::new(&ctx, win.clone()).unwrap();

        let sz = win.inner_size();
        
        self.win = Some(win);
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

                let fb = &mut self.fb;
                fb.fill(Color::WHITE);

                self.rend.render(&mut fb.as_mut(), self.transform, Some(&self.mask));

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
                let mut pt = Point { x: self.last_mouse_pos.x as f32, y: self.last_mouse_pos.y as f32 };
                self.transform.invert().unwrap().map_point(&mut pt);

                if pt.x < 0. || pt.x > 100. || pt.y < 0. || pt.y > 100. {
                    return
                }

                let x = (pt.x * (N as f32) / 100.) as u8;
                let y = (pt.y * (N as f32) / 100.) as u8;

                if self.board.turn() == Player::O && self.board.score().is_none() && let Ok(mut nst) = self.board.do_move(x, y) {
                    let (_, (x, y)) = maximize(nst, Player::X);
                    if x != 255 && y != 255 {
                        nst = nst.do_move(x, y).unwrap();
                    }
                    self.board = nst;
                    self.win.as_ref().unwrap().request_redraw();
                }
            },
            Resized(PhysicalSize { width, height }) => self.on_resize(width, height),
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
