// Copyright 2025 the UI Events Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Simple example.

use ui_events_winit::WindowEventReducer;

#[cfg(target_arch = "wasm32")]
use {wasm_bindgen::JsCast, winit::platform::web::WindowAttributesExtWebSys};

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
use ui_events_winit::WindowEventTranslation;
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

        #[cfg(target_arch = "wasm32")]
        let window_attributes = {
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();

            window_attributes.with_canvas(Some(canvas))
        };

        let window = event_loop.create_window(window_attributes).unwrap();
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        info!("winit_event: {event:?}");

        if let Some(event) = self.event_reducer.reduce(&event) {
            match event {
                WindowEventTranslation::Keyboard(keyboard_event) => {
                    info!("Keyboard event: {:?}", keyboard_event);
                }
                WindowEventTranslation::Pointer(pointer_event) => match pointer_event {
                    PointerEvent::Down(pointer_button_event) => {
                        info!("Pointer down: {:?}", pointer_button_event);
                    }
                    PointerEvent::Up(pointer_button_event) => {
                        info!("Pointer up: {:?}", pointer_button_event);
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
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                window.pre_present_notify();
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
