use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    ToolPen,
    ToolPencil,
    ToolHighlighter,
    ToolEraser,
    ToolLasso,
    ToolSelect,
    ToolPan,
    Undo,
    Redo,
    SelectAll,
    DeleteSelection,
    DuplicateSelection,
    ClearSelection,
    Copy,
    Cut,
    Paste,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    BringForward,
    SendBackward,
    BringToFront,
    SendToBack,
    HoldPan,
}

impl Command {
    pub const ALL: [Command; 25] = [
        Command::ToolPen,
        Command::ToolPencil,
        Command::ToolHighlighter,
        Command::ToolEraser,
        Command::ToolLasso,
        Command::ToolSelect,
        Command::ToolPan,
        Command::Undo,
        Command::Redo,
        Command::SelectAll,
        Command::DeleteSelection,
        Command::DuplicateSelection,
        Command::ClearSelection,
        Command::Copy,
        Command::Cut,
        Command::Paste,
        Command::NudgeLeft,
        Command::NudgeRight,
        Command::NudgeUp,
        Command::NudgeDown,
        Command::BringForward,
        Command::SendBackward,
        Command::BringToFront,
        Command::SendToBack,
        Command::HoldPan,
    ];

    pub fn to_id(&self) -> &'static str {
        match self {
            Command::ToolPen => "tool_pen",
            Command::ToolPencil => "tool_pencil",
            Command::ToolHighlighter => "tool_highlighter",
            Command::ToolEraser => "tool_eraser",
            Command::ToolLasso => "tool_lasso",
            Command::ToolSelect => "tool_select",
            Command::ToolPan => "tool_pan",
            Command::Undo => "undo",
            Command::Redo => "redo",
            Command::SelectAll => "select_all",
            Command::DeleteSelection => "delete_selection",
            Command::DuplicateSelection => "duplicate_selection",
            Command::ClearSelection => "clear_selection",
            Command::Copy => "copy",
            Command::Cut => "cut",
            Command::Paste => "paste",
            Command::NudgeLeft => "nudge_left",
            Command::NudgeRight => "nudge_right",
            Command::NudgeUp => "nudge_up",
            Command::NudgeDown => "nudge_down",
            Command::BringForward => "bring_forward",
            Command::SendBackward => "send_backward",
            Command::BringToFront => "bring_to_front",
            Command::SendToBack => "send_to_back",
            Command::HoldPan => "hold_pan",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "tool_pen" => Some(Command::ToolPen),
            "tool_pencil" => Some(Command::ToolPencil),
            "tool_highlighter" => Some(Command::ToolHighlighter),
            "tool_eraser" => Some(Command::ToolEraser),
            "tool_lasso" => Some(Command::ToolLasso),
            "tool_select" => Some(Command::ToolSelect),
            "tool_pan" => Some(Command::ToolPan),
            "undo" => Some(Command::Undo),
            "redo" => Some(Command::Redo),
            "select_all" => Some(Command::SelectAll),
            "delete_selection" => Some(Command::DeleteSelection),
            "duplicate_selection" => Some(Command::DuplicateSelection),
            "clear_selection" => Some(Command::ClearSelection),
            "copy" => Some(Command::Copy),
            "cut" => Some(Command::Cut),
            "paste" => Some(Command::Paste),
            "nudge_left" => Some(Command::NudgeLeft),
            "nudge_right" => Some(Command::NudgeRight),
            "nudge_up" => Some(Command::NudgeUp),
            "nudge_down" => Some(Command::NudgeDown),
            "bring_forward" => Some(Command::BringForward),
            "send_backward" => Some(Command::SendBackward),
            "bring_to_front" => Some(Command::BringToFront),
            "send_to_back" => Some(Command::SendToBack),
            "hold_pan" => Some(Command::HoldPan),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Command::ToolPen => "Pen Tool",
            Command::ToolPencil => "Pencil Tool",
            Command::ToolHighlighter => "Highlighter Tool",
            Command::ToolEraser => "Eraser Tool",
            Command::ToolLasso => "Lasso Tool",
            Command::ToolSelect => "Select Tool",
            Command::ToolPan => "Pan Tool",
            Command::Undo => "Undo",
            Command::Redo => "Redo",
            Command::SelectAll => "Select All",
            Command::DeleteSelection => "Delete Selection",
            Command::DuplicateSelection => "Duplicate Selection",
            Command::ClearSelection => "Clear / Cancel Selection",
            Command::Copy => "Copy",
            Command::Cut => "Cut",
            Command::Paste => "Paste",
            Command::NudgeLeft => "Nudge Left",
            Command::NudgeRight => "Nudge Right",
            Command::NudgeUp => "Nudge Up",
            Command::NudgeDown => "Nudge Down",
            Command::BringForward => "Bring Forward",
            Command::SendBackward => "Send Backward",
            Command::BringToFront => "Bring to Front",
            Command::SendToBack => "Send to Back",
            Command::HoldPan => "Temporary Pan (Hold Space)",
        }
    }

    pub fn allows_repeat(&self) -> bool {
        matches!(
            self,
            Command::NudgeLeft | Command::NudgeRight | Command::NudgeUp | Command::NudgeDown
        )
    }

    pub fn group(&self) -> &'static str {
        match self {
            Command::ToolPen
            | Command::ToolPencil
            | Command::ToolHighlighter
            | Command::ToolEraser
            | Command::ToolLasso
            | Command::ToolSelect
            | Command::ToolPan => "Tools",
            Command::Undo | Command::Redo => "History",
            Command::SelectAll
            | Command::DeleteSelection
            | Command::DuplicateSelection
            | Command::ClearSelection => "Selection",
            Command::Copy
            | Command::Cut
            | Command::Paste
            | Command::NudgeLeft
            | Command::NudgeRight
            | Command::NudgeUp
            | Command::NudgeDown
            | Command::BringForward
            | Command::SendBackward
            | Command::BringToFront
            | Command::SendToBack => "Editing",
            Command::HoldPan => "Navigation",
        }
    }
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_id())
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Command::from_id(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown command: {}", s)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: String,
    pub primary: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyChord {
    pub fn new(code: impl Into<String>, primary: bool, shift: bool, alt: bool) -> Self {
        Self {
            code: code.into(),
            primary,
            shift,
            alt,
        }
    }

    pub fn simple(code: impl Into<String>) -> Self {
        Self::new(code, false, false, false)
    }

    pub fn primary(code: impl Into<String>) -> Self {
        Self::new(code, true, false, false)
    }

    pub fn primary_shift(code: impl Into<String>) -> Self {
        Self::new(code, true, true, false)
    }

    pub fn is_modifier_only(&self) -> bool {
        let code = self.code.as_str();
        code == "ControlLeft"
            || code == "ControlRight"
            || code == "ShiftLeft"
            || code == "ShiftRight"
            || code == "AltLeft"
            || code == "AltRight"
            || code == "MetaLeft"
            || code == "MetaRight"
    }

    pub fn to_display_string(&self, is_mac: bool) -> String {
        let mut parts = Vec::new();
        if self.primary {
            parts.push(if is_mac { "Cmd" } else { "Ctrl" });
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }
        let key_name = if self.code.starts_with("Key") {
            &self.code[3..]
        } else if self.code.starts_with("Digit") {
            &self.code[5..]
        } else {
            &self.code
        };
        parts.push(key_name);
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictError {
    Conflict(Command),
    ModifierOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShortcutMap {
    pub map: HashMap<Command, Vec<KeyChord>>,
}

impl ShortcutMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn bindings(&self, command: Command) -> &[KeyChord] {
        self.map.get(&command).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn command_for_chord(&self, chord: &KeyChord) -> Option<Command> {
        for (cmd, chords) in &self.map {
            if chords.contains(chord) {
                return Some(*cmd);
            }
        }
        None
    }

    pub fn detect_conflict(&self, chord: &KeyChord) -> Option<Command> {
        self.command_for_chord(chord)
    }

    pub fn add_binding(&mut self, command: Command, chord: KeyChord) -> Result<(), ConflictError> {
        if chord.is_modifier_only() {
            return Err(ConflictError::ModifierOnly);
        }
        if let Some(conflicting_cmd) = self.detect_conflict(&chord) {
            if conflicting_cmd != command {
                return Err(ConflictError::Conflict(conflicting_cmd));
            }
        }
        if command == Command::HoldPan {
            self.map.insert(Command::HoldPan, vec![chord]);
            return Ok(());
        }
        let list = self.map.entry(command).or_insert_with(Vec::new);
        if !list.contains(&chord) {
            list.push(chord);
        }
        Ok(())
    }

    pub fn remove_binding(&mut self, command: Command, chord: &KeyChord) -> bool {
        if let Some(list) = self.map.get_mut(&command) {
            let initial_len = list.len();
            list.retain(|c| c != chord);
            return list.len() < initial_len;
        }
        false
    }

    pub fn defaults() -> Self {
        let mut sm = ShortcutMap::new();
        let _ = sm.add_binding(Command::ToolPen, KeyChord::simple("KeyP"));
        let _ = sm.add_binding(Command::ToolPencil, KeyChord::simple("KeyN"));
        let _ = sm.add_binding(Command::ToolHighlighter, KeyChord::simple("KeyM"));
        let _ = sm.add_binding(Command::ToolEraser, KeyChord::simple("KeyE"));
        let _ = sm.add_binding(Command::ToolLasso, KeyChord::simple("KeyL"));
        let _ = sm.add_binding(Command::ToolSelect, KeyChord::simple("KeyV"));
        let _ = sm.add_binding(Command::ToolPan, KeyChord::simple("KeyH"));
        let _ = sm.add_binding(Command::HoldPan, KeyChord::simple("Space"));
        let _ = sm.add_binding(Command::Undo, KeyChord::primary("KeyZ"));
        let _ = sm.add_binding(Command::Redo, KeyChord::primary_shift("KeyZ"));
        let _ = sm.add_binding(Command::Redo, KeyChord::primary("KeyY"));
        let _ = sm.add_binding(Command::SelectAll, KeyChord::primary("KeyA"));
        let _ = sm.add_binding(Command::DeleteSelection, KeyChord::simple("Delete"));
        let _ = sm.add_binding(Command::DeleteSelection, KeyChord::simple("Backspace"));
        let _ = sm.add_binding(Command::DuplicateSelection, KeyChord::primary("KeyD"));
        let _ = sm.add_binding(Command::ClearSelection, KeyChord::simple("Escape"));
        let _ = sm.add_binding(Command::Copy, KeyChord::primary("KeyC"));
        let _ = sm.add_binding(Command::Cut, KeyChord::primary("KeyX"));
        let _ = sm.add_binding(Command::Paste, KeyChord::primary("KeyV"));
        let _ = sm.add_binding(Command::NudgeLeft, KeyChord::simple("ArrowLeft"));
        let _ = sm.add_binding(Command::NudgeRight, KeyChord::simple("ArrowRight"));
        let _ = sm.add_binding(Command::NudgeUp, KeyChord::simple("ArrowUp"));
        let _ = sm.add_binding(Command::NudgeDown, KeyChord::simple("ArrowDown"));
        let _ = sm.add_binding(Command::BringForward, KeyChord::primary("BracketRight"));
        let _ = sm.add_binding(Command::SendBackward, KeyChord::primary("BracketLeft"));
        let _ = sm.add_binding(
            Command::BringToFront,
            KeyChord::primary_shift("BracketRight"),
        );
        let _ = sm.add_binding(Command::SendToBack, KeyChord::primary_shift("BracketLeft"));
        sm
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub version: u32,
    pub shortcuts: ShortcutMap,
}

impl AppSettings {
    pub fn new() -> Self {
        Self {
            version: 1,
            shortcuts: ShortcutMap::defaults(),
        }
    }

    pub fn is_version_supported(version: u32) -> bool {
        version == 1
    }

    pub fn validate_and_repair(&mut self) {
        let mut clean_map = ShortcutMap::new();
        for cmd in &Command::ALL {
            if let Some(chords) = self.shortcuts.map.get(cmd) {
                for chord in chords {
                    if !chord.is_modifier_only() {
                        if clean_map.detect_conflict(chord).is_none() {
                            let _ = clean_map.add_binding(*cmd, chord.clone());
                        }
                    }
                }
            }
        }
        self.shortcuts = clean_map;
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempPanController {
    pub trigger_code: Option<String>,
    pub released: bool,
}

impl TempPanController {
    pub fn new() -> Self {
        Self {
            trigger_code: None,
            released: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.trigger_code.is_some()
    }

    pub fn handle_keydown(&mut self, code: &str, is_idle: bool) -> bool {
        if self.is_active() {
            return false;
        }
        if !is_idle {
            return false;
        }
        self.trigger_code = Some(code.to_string());
        self.released = false;
        true
    }

    pub fn handle_keyup(&mut self, code: &str, is_dragging_pan: bool) -> bool {
        if let Some(ref trigger) = self.trigger_code {
            if trigger == code {
                self.released = true;
                if !is_dragging_pan {
                    self.trigger_code = None;
                    self.released = false;
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_gesture_end(&mut self) -> bool {
        if self.released {
            self.trigger_code = None;
            self.released = false;
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.trigger_code = None;
        self.released = false;
    }
}

impl Default for TempPanController {
    fn default() -> Self {
        Self::new()
    }
}
