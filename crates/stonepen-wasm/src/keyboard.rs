use stonepen_core::KeyChord;
use web_sys::KeyboardEvent;

pub fn parse_event_to_chord(e: &KeyboardEvent) -> KeyChord {
    let code = e.code();
    let primary = e.ctrl_key() || e.meta_key();
    let shift = e.shift_key();
    let alt = e.alt_key();
    KeyChord::new(code, primary, shift, alt)
}
