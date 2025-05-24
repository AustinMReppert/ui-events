use keyboard_types::KeyboardEvent;
use crate::pointer::PointerEvent;

/// Result of [`WindowEventReducer::reduce`].
#[derive(Clone, Debug)]
pub enum UiEvent {
    /// Resulting [`KeyboardEvent`].
    Keyboard(KeyboardEvent),
    /// Resulting [`PointerEvent`].
    Pointer(PointerEvent),
}