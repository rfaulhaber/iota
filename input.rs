/// Input abstraction layer for editor commands
/// This decouples the editor from any specific frontend (terminal, GUI, web, etc.)
/// Design shamelessly stolen from crossterm's KeyEvent structure
use std::str::FromStr;

/// Key codes - what key was pressed (similar to crossterm::KeyCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    Delete,
    Escape,
    F(u8), // Function keys F1-F12
}

/// A key event with modifiers (similar to crossterm::KeyEvent)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorKey {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Modifier keys (similar to crossterm::KeyModifiers but using a struct instead of bitflags)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool, // Command on Mac, Super/Windows key on Linux/Windows
}

impl KeyModifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
    };

    pub const CONTROL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
        meta: false,
    };

    pub const ALT: Self = Self {
        ctrl: false,
        alt: true,
        shift: false,
        meta: false,
    };

    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
        meta: false,
    };

    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.meta
    }
}

impl EditorKey {
    /// Create a key with no modifiers
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Create a key with modifiers
    pub fn with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Check if this is a plain character (no control modifiers)
    pub fn is_plain_char(&self) -> bool {
        matches!(self.code, KeyCode::Char(_))
            && !self.modifiers.ctrl
            && !self.modifiers.alt
            && !self.modifiers.meta
    }
}

/// A sequence of key presses (for keychording like Emacs C-x C-s)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySequence {
    pub keys: Vec<EditorKey>,
}

impl KeySequence {
    pub fn new(keys: Vec<EditorKey>) -> Self {
        Self { keys }
    }

    pub fn single(key: EditorKey) -> Self {
        Self { keys: vec![key] }
    }
}

/// Error type for parsing key specifications
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseKeyError {
    Empty,
    UnknownModifier(String),
    UnknownKey(String),
    InvalidFunctionKey(String),
}

impl std::fmt::Display for ParseKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseKeyError::Empty => write!(f, "empty key specification"),
            ParseKeyError::UnknownModifier(m) => write!(f, "unknown modifier: {}", m),
            ParseKeyError::UnknownKey(k) => write!(f, "unknown key: {}", k),
            ParseKeyError::InvalidFunctionKey(k) => write!(f, "invalid function key: {}", k),
        }
    }
}

impl std::error::Error for ParseKeyError {}

impl FromStr for EditorKey {
    type Err = ParseKeyError;

    /// Parse a key specification string into an EditorKey.
    ///
    /// Supports both long-form and Emacs-style modifiers:
    /// - Control: "Ctrl-" or "C-"
    /// - Alt/Meta: "Alt-" or "M-"
    /// - Shift: "Shift-" or "S-"
    /// - Super: "Super-" or "s-"
    ///
    /// Examples:
    /// - "Ctrl-c" → Ctrl+c
    /// - "C-c" → Ctrl+c
    /// - "C-M-s" → Ctrl+Alt+s
    /// - "Shift-Left" → Shift+Left Arrow
    /// - "F1" → F1 key
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseKeyError::Empty);
        }

        let parts: Vec<&str> = s.split('-').collect();
        if parts.is_empty() {
            return Err(ParseKeyError::Empty);
        }

        let mut modifiers = KeyModifiers::default();
        let key_part = parts[parts.len() - 1];

        // Process all parts except the last as potential modifiers
        for &part in &parts[..parts.len() - 1] {
            match part {
                "Ctrl" | "C" => modifiers.ctrl = true,
                "Alt" | "M" => modifiers.alt = true,
                "Shift" | "S" => modifiers.shift = true,
                "Super" | "s" => modifiers.meta = true,
                _ => return Err(ParseKeyError::UnknownModifier(part.to_string())),
            }
        }

        // Parse the key code from the last part
        let code = parse_key_code(key_part)?;

        Ok(EditorKey { code, modifiers })
    }
}

impl FromStr for KeySequence {
    type Err = ParseKeyError;

    /// Parse a space-separated sequence of key specifications.
    ///
    /// Examples:
    /// - "C-x C-s" → [Ctrl+x, Ctrl+s]
    /// - "C-c C-c" → [Ctrl+c, Ctrl+c]
    /// - "M-x" → [Alt+x]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ParseKeyError::Empty);
        }

        let keys: Result<Vec<EditorKey>, ParseKeyError> = trimmed
            .split_whitespace()
            .map(|key_spec| key_spec.parse())
            .collect();

        Ok(KeySequence { keys: keys? })
    }
}

/// Parse a key code from a string
fn parse_key_code(s: &str) -> Result<KeyCode, ParseKeyError> {
    match s {
        // Special keys
        "Enter" => Ok(KeyCode::Enter),
        "Backspace" => Ok(KeyCode::Backspace),
        "Delete" => Ok(KeyCode::Delete),
        "Escape" | "Esc" => Ok(KeyCode::Escape),
        "Tab" => Ok(KeyCode::Tab),
        "Home" => Ok(KeyCode::Home),
        "End" => Ok(KeyCode::End),
        "PageUp" => Ok(KeyCode::PageUp),
        "PageDown" => Ok(KeyCode::PageDown),
        "Left" => Ok(KeyCode::Left),
        "Right" => Ok(KeyCode::Right),
        "Up" => Ok(KeyCode::Up),
        "Down" => Ok(KeyCode::Down),

        // Function keys (F1-F12)
        s if s.starts_with('F') && s.len() > 1 && s[1..].chars().all(|c| c.is_ascii_digit()) => {
            let num_str = &s[1..];
            let num: u8 = num_str
                .parse()
                .map_err(|_| ParseKeyError::InvalidFunctionKey(s.to_string()))?;
            if num >= 1 && num <= 12 {
                Ok(KeyCode::F(num))
            } else {
                Err(ParseKeyError::InvalidFunctionKey(s.to_string()))
            }
        }

        // Single character
        s if s.len() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap())),

        // Unknown
        _ => Err(ParseKeyError::UnknownKey(s.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_combinations() {
        // Test Ctrl+C
        let key = EditorKey::with_modifiers(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key.code, KeyCode::Char('c'));
        assert!(key.modifiers.ctrl);
        assert!(!key.modifiers.alt);
        assert!(!key.is_plain_char());

        // Test Alt+X (Meta in Emacs)
        let key = EditorKey::with_modifiers(KeyCode::Char('x'), KeyModifiers::ALT);
        assert_eq!(key.code, KeyCode::Char('x'));
        assert!(!key.modifiers.ctrl);
        assert!(key.modifiers.alt);
        assert!(!key.is_plain_char());

        // Test Ctrl+Alt+C
        let mods = KeyModifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        let key = EditorKey::with_modifiers(KeyCode::Char('c'), mods);
        assert_eq!(key.code, KeyCode::Char('c'));
        assert!(key.modifiers.ctrl);
        assert!(key.modifiers.alt);
        assert!(!key.is_plain_char());

        // Test plain character (should be printable)
        let key = EditorKey::new(KeyCode::Char('a'));
        assert_eq!(key.code, KeyCode::Char('a'));
        assert!(!key.modifiers.ctrl);
        assert!(!key.modifiers.alt);
        assert!(key.modifiers.is_empty());
        assert!(key.is_plain_char());
    }

    #[test]
    fn test_modifier_equality() {
        // Same modifiers should be equal
        let key1 = EditorKey::with_modifiers(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let key2 = EditorKey::with_modifiers(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(key1, key2);

        // Different modifiers should not be equal
        let key1 = EditorKey::with_modifiers(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let key2 = EditorKey::with_modifiers(KeyCode::Char('s'), KeyModifiers::ALT);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_arrow_keys() {
        // Plain arrow key
        let key = EditorKey::new(KeyCode::Left);
        assert_eq!(key.code, KeyCode::Left);
        assert!(key.modifiers.is_empty());

        // Ctrl+Arrow
        let key = EditorKey::with_modifiers(KeyCode::Right, KeyModifiers::CONTROL);
        assert_eq!(key.code, KeyCode::Right);
        assert!(key.modifiers.ctrl);
    }

    #[test]
    fn test_special_keys() {
        let key = EditorKey::new(KeyCode::Enter);
        assert_eq!(key.code, KeyCode::Enter);
        assert!(key.modifiers.is_empty());

        let key = EditorKey::new(KeyCode::Backspace);
        assert_eq!(key.code, KeyCode::Backspace);
    }

    #[test]
    fn test_parse_simple_char() {
        let key: EditorKey = "a".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('a'));
        assert!(key.modifiers.is_empty());

        let key: EditorKey = "Z".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('Z'));
        assert!(key.modifiers.is_empty());
    }

    #[test]
    fn test_parse_ctrl_key() {
        // Long form
        let key: EditorKey = "Ctrl-c".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('c'));
        assert!(key.modifiers.ctrl);
        assert!(!key.modifiers.alt);
        assert!(!key.modifiers.shift);

        // Emacs style
        let key: EditorKey = "C-c".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('c'));
        assert!(key.modifiers.ctrl);
        assert!(!key.modifiers.alt);

        // Case sensitive - uppercase C
        let key: EditorKey = "Ctrl-C".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('C'));
        assert!(key.modifiers.ctrl);
    }

    #[test]
    fn test_parse_alt_key() {
        // Long form
        let key: EditorKey = "Alt-x".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('x'));
        assert!(key.modifiers.alt);
        assert!(!key.modifiers.ctrl);

        // Emacs style (Meta)
        let key: EditorKey = "M-x".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('x'));
        assert!(key.modifiers.alt);
    }

    #[test]
    fn test_parse_shift_key() {
        let key: EditorKey = "Shift-Left".parse().unwrap();
        assert_eq!(key.code, KeyCode::Left);
        assert!(key.modifiers.shift);

        let key: EditorKey = "S-a".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('a'));
        assert!(key.modifiers.shift);
    }

    #[test]
    fn test_parse_super_key() {
        let key: EditorKey = "Super-s".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('s'));
        assert!(key.modifiers.meta);

        // Emacs style (lowercase s for super)
        let key: EditorKey = "s-s".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('s'));
        assert!(key.modifiers.meta);
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        // Ctrl+Alt
        let key: EditorKey = "C-M-s".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('s'));
        assert!(key.modifiers.ctrl);
        assert!(key.modifiers.alt);
        assert!(!key.modifiers.shift);

        // Ctrl+Shift
        let key: EditorKey = "Ctrl-Shift-a".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('a'));
        assert!(key.modifiers.ctrl);
        assert!(key.modifiers.shift);

        // All modifiers
        let key: EditorKey = "C-M-S-s-x".parse().unwrap();
        assert_eq!(key.code, KeyCode::Char('x'));
        assert!(key.modifiers.ctrl);
        assert!(key.modifiers.alt);
        assert!(key.modifiers.shift);
        assert!(key.modifiers.meta);
    }

    #[test]
    fn test_parse_special_keys() {
        let key: EditorKey = "Enter".parse().unwrap();
        assert_eq!(key.code, KeyCode::Enter);

        let key: EditorKey = "Backspace".parse().unwrap();
        assert_eq!(key.code, KeyCode::Backspace);

        let key: EditorKey = "Delete".parse().unwrap();
        assert_eq!(key.code, KeyCode::Delete);

        let key: EditorKey = "Escape".parse().unwrap();
        assert_eq!(key.code, KeyCode::Escape);

        let key: EditorKey = "Esc".parse().unwrap();
        assert_eq!(key.code, KeyCode::Escape);

        let key: EditorKey = "Tab".parse().unwrap();
        assert_eq!(key.code, KeyCode::Tab);
    }

    #[test]
    fn test_parse_arrow_keys() {
        let key: EditorKey = "Left".parse().unwrap();
        assert_eq!(key.code, KeyCode::Left);

        let key: EditorKey = "Right".parse().unwrap();
        assert_eq!(key.code, KeyCode::Right);

        let key: EditorKey = "Up".parse().unwrap();
        assert_eq!(key.code, KeyCode::Up);

        let key: EditorKey = "Down".parse().unwrap();
        assert_eq!(key.code, KeyCode::Down);

        // With modifiers
        let key: EditorKey = "C-Left".parse().unwrap();
        assert_eq!(key.code, KeyCode::Left);
        assert!(key.modifiers.ctrl);
    }

    #[test]
    fn test_parse_function_keys() {
        let key: EditorKey = "F1".parse().unwrap();
        assert_eq!(key.code, KeyCode::F(1));

        let key: EditorKey = "F12".parse().unwrap();
        assert_eq!(key.code, KeyCode::F(12));

        // With modifiers
        let key: EditorKey = "C-F5".parse().unwrap();
        assert_eq!(key.code, KeyCode::F(5));
        assert!(key.modifiers.ctrl);
    }

    #[test]
    fn test_parse_errors() {
        // Empty string
        assert_eq!("".parse::<EditorKey>().unwrap_err(), ParseKeyError::Empty);

        // Unknown modifier
        assert!(matches!(
            "Foo-c".parse::<EditorKey>().unwrap_err(),
            ParseKeyError::UnknownModifier(_)
        ));

        // Unknown key
        assert!(matches!(
            "Unknown".parse::<EditorKey>().unwrap_err(),
            ParseKeyError::UnknownKey(_)
        ));

        // Invalid function key
        assert!(matches!(
            "F13".parse::<EditorKey>().unwrap_err(),
            ParseKeyError::InvalidFunctionKey(_)
        ));

        assert!(matches!(
            "F0".parse::<EditorKey>().unwrap_err(),
            ParseKeyError::InvalidFunctionKey(_)
        ));

        // "Fabc" doesn't match function key pattern (not all digits after F)
        assert!(matches!(
            "Fabc".parse::<EditorKey>().unwrap_err(),
            ParseKeyError::UnknownKey(_)
        ));
    }

    #[test]
    fn test_parse_key_sequence() {
        // Single key
        let seq: KeySequence = "C-x".parse().unwrap();
        assert_eq!(seq.keys.len(), 1);
        assert_eq!(seq.keys[0].code, KeyCode::Char('x'));
        assert!(seq.keys[0].modifiers.ctrl);

        // Two keys
        let seq: KeySequence = "C-x C-s".parse().unwrap();
        assert_eq!(seq.keys.len(), 2);
        assert_eq!(seq.keys[0].code, KeyCode::Char('x'));
        assert!(seq.keys[0].modifiers.ctrl);
        assert_eq!(seq.keys[1].code, KeyCode::Char('s'));
        assert!(seq.keys[1].modifiers.ctrl);

        // Multiple keys with different modifiers
        let seq: KeySequence = "C-c C-c M-x".parse().unwrap();
        assert_eq!(seq.keys.len(), 3);
        assert_eq!(seq.keys[0].code, KeyCode::Char('c'));
        assert!(seq.keys[0].modifiers.ctrl);
        assert_eq!(seq.keys[1].code, KeyCode::Char('c'));
        assert!(seq.keys[1].modifiers.ctrl);
        assert_eq!(seq.keys[2].code, KeyCode::Char('x'));
        assert!(seq.keys[2].modifiers.alt);
    }

    #[test]
    fn test_parse_key_sequence_with_special_keys() {
        let seq: KeySequence = "C-x Enter".parse().unwrap();
        assert_eq!(seq.keys.len(), 2);
        assert_eq!(seq.keys[0].code, KeyCode::Char('x'));
        assert!(seq.keys[0].modifiers.ctrl);
        assert_eq!(seq.keys[1].code, KeyCode::Enter);
        assert!(seq.keys[1].modifiers.is_empty());
    }

    #[test]
    fn test_parse_key_sequence_errors() {
        // Empty string
        assert_eq!("".parse::<KeySequence>().unwrap_err(), ParseKeyError::Empty);

        // Invalid key in sequence
        assert!(matches!(
            "C-x Foo".parse::<KeySequence>().unwrap_err(),
            ParseKeyError::UnknownKey(_)
        ));
    }

    #[test]
    fn test_case_sensitivity() {
        // lowercase c
        let key1: EditorKey = "C-c".parse().unwrap();
        assert_eq!(key1.code, KeyCode::Char('c'));

        // uppercase C
        let key2: EditorKey = "C-C".parse().unwrap();
        assert_eq!(key2.code, KeyCode::Char('C'));

        // They should be different
        assert_ne!(key1, key2);
    }
}
