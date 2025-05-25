// Copyright 2025 the UI Events Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This crate bridges [`winit`]'s native input events (mouse, touch, keyboard, etc.)
//! into the [`ui-events`] model.
//!
//! The primary entry point is [`WindowEventReducer`].
//!
//! [`ui-events`]: https://docs.rs/ui-events/

// LINEBENDER LINT SET - lib.rs - v3
// See https://linebender.org/wiki/canonical-lints/
// These lints shouldn't apply to examples or tests.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
// These lints shouldn't apply to examples.
#![warn(clippy::print_stdout, clippy::print_stderr)]
// Targeting e.g. 32-bit means structs containing usize can give false positives for 64-bit.
#![cfg_attr(target_pointer_width = "64", warn(clippy::trivially_copy_pass_by_ref))]
// END LINEBENDER LINT SET
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![no_std]

pub mod keyboard;
pub mod pointer;

extern crate alloc;
use alloc::{vec, vec::Vec};

extern crate std;
use std::time::Instant;

use ui_events::pointer::{PointerButtonUpdate, PointerScrollUpdate};
use ui_events::{
    pointer::{PointerEvent, PointerId, PointerInfo, PointerState, PointerType, PointerUpdate},
    ScrollDelta, UiEvent,
};
use winit::{
    event::{ElementState, Force, MouseScrollDelta, Touch, TouchPhase, WindowEvent},
    keyboard::ModifiersState,
};
use winit::window::Window;

/// Manages stateful transformations of winit [`WindowEvent`].
///
/// Store a single instance of this per window, then call [`WindowEventReducer::reduce`]
/// on each [`WindowEvent`] for that window.
/// Use the [`WindowEventTranslation`] value to receive [`PointerEvent`]s and [`KeyboardEvent`]s.
///
/// This handles:
///  - [`ModifiersChanged`][`WindowEvent::ModifiersChanged`]
///  - [`KeyboardInput`][`WindowEvent::KeyboardInput`]
///  - [`Touch`][`WindowEvent::Touch`]
///  - [`MouseInput`][`WindowEvent::MouseInput`]
///  - [`MouseWheel`][`WindowEvent::MouseWheel`]
///  - [`CursorMoved`][`WindowEvent::CursorMoved`]
///  - [`CursorEntered`][`WindowEvent::CursorEntered`]
///  - [`CursorLeft`][`WindowEvent::CursorLeft`]
#[derive(Debug, Default)]
pub struct WindowEventReducer {
    /// State of modifiers.
    modifiers: ModifiersState,
    /// State of the primary mouse pointer.
    primary_state: PointerState,
    /// Click and tap counter.
    counter: TapCounter,
    /// First time an event was received..
    first_instant: Option<Instant>,
    /// Scale factor.
    scale_factor: Option<f64>,
}

#[allow(clippy::cast_possible_truncation)]
impl WindowEventReducer {
    /// Process a [`WindowEvent`].
    pub fn reduce(&mut self, window_event: &WindowEvent) -> Option<UiEvent> {
        const PRIMARY_MOUSE: PointerInfo = PointerInfo {
            pointer_id: Some(PointerId::PRIMARY),
            // TODO: Maybe transmute device.
            persistent_device_id: None,
            pointer_type: PointerType::Mouse,
        };

        let time = Instant::now()
            .duration_since(*self.first_instant.get_or_insert_with(Instant::now))
            .as_nanos() as u64;

        self.primary_state.time = time;

        match window_event {
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
                self.primary_state.modifiers = keyboard::from_winit_modifier_state(self.modifiers);
                None
            }
            WindowEvent::KeyboardInput { event, .. } => Some(UiEvent::Keyboard(
                keyboard::from_winit_keyboard_event(event.clone(), self.modifiers),
            )),
            WindowEvent::CursorEntered { .. } => {
                Some(UiEvent::Pointer(PointerEvent::Enter(PRIMARY_MOUSE)))
            }
            WindowEvent::CursorLeft { .. } => {
                Some(UiEvent::Pointer(PointerEvent::Leave(PRIMARY_MOUSE)))
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = position.to_logical(self.scale_factor.unwrap_or(1.0));
                self.primary_state.position = kurbo::Point::new(logical.x, logical.y);

                Some(UiEvent::Pointer(self.counter.attach_count(
                    PointerEvent::Move(PointerUpdate {
                        pointer: PRIMARY_MOUSE,
                        current: self.primary_state.clone(),
                        coalesced: vec![],
                        predicted: vec![],
                    }),
                )))
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                let button = pointer::try_from_winit_button(*button);
                if let Some(button) = button {
                    self.primary_state.buttons.insert(button);
                }

                Some(UiEvent::Pointer(self.counter.attach_count(
                    PointerEvent::Down(PointerButtonUpdate {
                        pointer: PRIMARY_MOUSE,
                        button,
                        state: self.primary_state.clone(),
                    }),
                )))
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } => {
                let button = pointer::try_from_winit_button(*button);
                if let Some(button) = button {
                    self.primary_state.buttons.remove(button);
                }

                Some(UiEvent::Pointer(self.counter.attach_count(
                    PointerEvent::Up(PointerButtonUpdate {
                        pointer: PRIMARY_MOUSE,
                        button,
                        state: self.primary_state.clone(),
                    }),
                )))
            }
            WindowEvent::MouseWheel { delta, .. } => Some(UiEvent::Pointer(PointerEvent::Scroll(PointerScrollUpdate {
                pointer: PRIMARY_MOUSE,
                delta: match *delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(p) => {
                        let logical = p.to_logical(self.scale_factor.unwrap_or(1.0));
                        ScrollDelta::PixelDelta(logical.x, logical.y)
                    },
                },
                state: self.primary_state.clone(),
            }))),
            WindowEvent::Touch(Touch {
                phase,
                id,
                location,
                force,
                ..
            }) => {
                let pointer = PointerInfo {
                    pointer_id: PointerId::new(id.saturating_add(1)),
                    pointer_type: PointerType::Touch,
                    persistent_device_id: None,
                };

                use TouchPhase::*;

                let logical_location = location.to_logical(self.scale_factor.unwrap_or(1.0));
                
                let state = PointerState {
                    time,
                    position: kurbo::Point::new(logical_location.x, logical_location.y),
                    modifiers: self.primary_state.modifiers,
                    pressure: if matches!(phase, Ended | Cancelled) {
                        0.0
                    } else {
                        match force {
                            Some(Force::Calibrated { force, .. }) => (force * 0.5) as f32,
                            Some(Force::Normalized(q)) => *q as f32,
                            _ => 0.5,
                        }
                    },
                    ..Default::default()
                };

                Some(UiEvent::Pointer(self.counter.attach_count(match phase {
                    Started => PointerEvent::Down(PointerButtonUpdate {
                        pointer,
                        button: None,
                        state,
                    }),
                    Moved => PointerEvent::Move(PointerUpdate {
                        pointer,
                        current: state,
                        coalesced: vec![],
                        predicted: vec![],
                    }),
                    Cancelled => PointerEvent::Cancel(pointer),
                    Ended => PointerEvent::Up(PointerButtonUpdate {
                        pointer,
                        button: None,
                        state,
                    }),
                })))
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = Some(*scale_factor);
                None
            },
            _ => None,
        }
    }

    /// Set the scale factor for the window.
    pub fn set_scale_factor(&mut self, window: &Window) {
        self.scale_factor = Some(window.scale_factor());
    }
}

#[derive(Clone, Debug)]
struct TapState {
    /// Pointer ID used to attach tap counts to [`PointerEvent::Move`].
    pointer_id: Option<PointerId>,
    /// Nanosecond timestamp when the tap went Down.
    down_time: u64,
    /// Nanosecond timestamp when the tap went Up.
    ///
    /// Resets to `down_time` when tap goes Down.
    up_time: u64,
    /// The local tap count as of the last Down phase.
    count: u8,
    /// x coordinate.
    x: f64,
    /// y coordinate.
    y: f64,
}

#[derive(Debug, Default)]
struct TapCounter {
    taps: Vec<TapState>,
}

impl TapCounter {
    /// Enhance a [`PointerEvent`] with a `count`.
    fn attach_count(&mut self, e: PointerEvent) -> PointerEvent {
        match e {
            PointerEvent::Down(pointer_button_update) => {
                let e = if let Some(i) =
                    self.taps.iter().position(|TapState { x, y, up_time, .. }| {
                        let dx = (x - pointer_button_update.state.position.x).abs();
                        let dy = (y - pointer_button_update.state.position.y).abs();
                        (dx * dx + dy * dy).sqrt() < 4.0
                            && (up_time + 500_000_000) > pointer_button_update.state.time
                    }) {
                    let count = self.taps[i].count + 1;
                    self.taps[i].count = count;
                    self.taps[i].pointer_id = pointer_button_update.pointer.pointer_id;
                    self.taps[i].down_time = pointer_button_update.state.time;
                    self.taps[i].up_time = pointer_button_update.state.time;
                    self.taps[i].x = pointer_button_update.state.position.x;
                    self.taps[i].y = pointer_button_update.state.position.y;

                    PointerEvent::Down(PointerButtonUpdate {
                        button: pointer_button_update.button,
                        pointer: pointer_button_update.pointer,
                        state: PointerState {
                            count,
                            ..pointer_button_update.state
                        },
                    })
                } else {
                    let s = TapState {
                        pointer_id: pointer_button_update.pointer.pointer_id,
                        down_time: pointer_button_update.state.time,
                        up_time: pointer_button_update.state.time,
                        count: 1,
                        x: pointer_button_update.state.position.x,
                        y: pointer_button_update.state.position.y,
                    };
                    self.taps.push(s);
                    PointerEvent::Down(PointerButtonUpdate {
                        button: pointer_button_update.button,
                        pointer: pointer_button_update.pointer,
                        state: PointerState {
                            count: 1,
                            ..pointer_button_update.state
                        },
                    })
                };
                self.clear_expired(pointer_button_update.state.time);
                e
            }
            PointerEvent::Up(ref pointer_button_update) => {
                if let Some(i) = self.taps.iter().position(|TapState { pointer_id, .. }| {
                    *pointer_id == pointer_button_update.pointer.pointer_id
                }) {
                    self.taps[i].up_time = pointer_button_update.state.time;
                    PointerEvent::Up(PointerButtonUpdate {
                        button: pointer_button_update.button,
                        pointer: pointer_button_update.pointer,
                        state: PointerState {
                            count: self.taps[i].count,
                            ..pointer_button_update.state.clone()
                        },
                    })
                } else {
                    e.clone()
                }
            }
            PointerEvent::Move(PointerUpdate {
                pointer,
                ref current,
                ref coalesced,
                ref predicted,
            }) => {
                if let Some(TapState { count, .. }) = self
                    .taps
                    .iter()
                    .find(
                        |TapState {
                             pointer_id,
                             down_time,
                             up_time,
                             ..
                         }| {
                            *pointer_id == pointer.pointer_id && down_time == up_time
                        },
                    )
                    .cloned()
                {
                    PointerEvent::Move(PointerUpdate {
                        pointer,
                        current: PointerState {
                            count,
                            ..current.clone()
                        },
                        coalesced: coalesced
                            .iter()
                            .cloned()
                            .map(|u| PointerState { count, ..u })
                            .collect(),
                        predicted: predicted
                            .iter()
                            .cloned()
                            .map(|u| PointerState { count, ..u })
                            .collect(),
                    })
                } else {
                    e
                }
            }
            PointerEvent::Cancel(p) | PointerEvent::Leave(p) => {
                self.taps
                    .retain(|TapState { pointer_id, .. }| *pointer_id != p.pointer_id);
                e.clone()
            }
            PointerEvent::Enter(..) | PointerEvent::Scroll { .. } => e.clone(),
        }
    }

    /// Clear expired taps.
    ///
    /// `t` is the time of the last received event.
    /// All events have the same time base on Android, so this is valid here.
    fn clear_expired(&mut self, t: u64) {
        self.taps.retain(
            |TapState {
                 down_time, up_time, ..
             }| { down_time == up_time || (up_time + 500_000_000) > t },
        );
    }
}

#[cfg(test)]
mod tests {
    // CI will fail unless cargo nextest can execute at least one test per workspace.
    // Delete this dummy test once we have an actual real test.
    #[test]
    fn dummy_test_until_we_have_a_real_test() {}
}
