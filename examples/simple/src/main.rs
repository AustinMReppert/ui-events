// Copyright 2025 the UI Events Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Simple example.

use anyhow::Result;
use ui_events_winit::WindowEventReducer;

fn main() -> Result<(), impl std::error::Error> {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    util::init();

    info!("Press 'Esc' to close the window.");

    let event_loop = EventLoop::new().unwrap();

    event_loop.run_app(&mut Simple::default())
}

use tracing::info;

use ui_events::pointer::PointerEvent;
use ui_events::UiEvent;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[path = "util.rs"]
mod util;

#[derive(Default, Debug)]
struct Simple {
    request_redraw: bool,
    wait_cancelled: bool,
    close_requested: bool,
    window: Option<Window>,
    event_reducer: WindowEventReducer,
}

impl ApplicationHandler for Simple {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        self.wait_cancelled = match cause {
            _ => false,
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default().with_title("Ui Events Winit Example");
        let window = event_loop.create_window(window_attributes).unwrap();
        self.event_reducer.set_scale_factor(&window);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        info!("winit_event: {event:?}");

        match self.event_reducer.reduce(&event) {
            UiEvent::Keyboard(_) => {}
            UiEvent::Pointer(pointer_event) => match pointer_event {
                PointerEvent::Down(pointer_button_update) => {
                    if pointer_button_update.is_primary() {
                        info!("Pointer down: {:?}", pointer_button_update);
                        pointer_button_update.state.position;
                    }
                }
                PointerEvent::Up(pointer_button_update) => {
                    if pointer_button_update.is_primary() {
                        info!("Pointer up: {:?}", pointer_button_update);
                    }
                }
                PointerEvent::Move(pointer_update) => {
                    info!("Pointer move: {:?}", pointer_update);
                }
                PointerEvent::Cancel(_) => {}
                PointerEvent::Enter(_) => {}
                PointerEvent::Leave(_) => {}
                PointerEvent::Scroll(pointer_scroll_update) => {
                    info!("Pointer scroll: {:?}", pointer_scroll_update);
                }
            },
            UiEvent::Na => {}
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                window.pre_present_notify();
                //fill::fill_window(window.as_ref());
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.request_redraw && !self.wait_cancelled && !self.close_requested {
            self.window.as_ref().unwrap().request_redraw();
        }

        event_loop.set_control_flow(ControlFlow::Wait);

        if self.close_requested {
            event_loop.exit();
        }
    }
}
