use keyboard_types::KeyboardEvent;
use crate::pointer::PointerEvent;

/*#[cfg(feature = "std")]
use std::time::Instant;*/

/// Result of [`WindowEventReducer::reduce`].
#[derive(Clone, Debug)]
pub enum UiEvent {
    /// Resulting [`KeyboardEvent`].
    Keyboard(KeyboardEvent),
    /// Resulting [`PointerEvent`].
    Pointer(PointerEvent),
    /// Not relevant.
    Na,
}

/*#[derive(Clone, Debug)]
pub struct UiEvent {
    // https://dom.spec.whatwg.org/#dom-event-timestamp
    #[cfg(feature = "std")]
    timestamp: Instant,
}*/