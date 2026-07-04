use web_sys::KeyboardEvent;

pub struct KeyAction {
    pub undo: bool,
    pub redo: bool,
    pub delete: bool,
    pub escape: bool,
    pub duplicate: bool,
}

pub fn parse_key(e: &KeyboardEvent) -> KeyAction {
    let key = e.key();
    let ctrl = e.ctrl_key() || e.meta_key();
    KeyAction {
        undo: ctrl && key == "z" && !e.shift_key(),
        redo: ctrl && (key == "y" || (key == "z" && e.shift_key())),
        delete: key == "Delete" || key == "Backspace",
        escape: key == "Escape",
        duplicate: ctrl && (key == "d" || key == "D"),
    }
}
