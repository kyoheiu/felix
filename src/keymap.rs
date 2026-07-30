//! Data-driven keybindings.
//!
//! Historically every key was matched inline inside the giant loop in
//! `run.rs`. This module turns keys into data: a [`Key`] is one physical
//! keypress, a [`KeyChord`] is a sequence of them (`gg`, `dd`, `ge`...), and a
//! [`Keymap`] maps chords to [`Action`]s. The event loop consults the keymap
//! via [`Keymap::resolve`] instead of a hardcoded match, which is what lets
//! user-defined chords such as `ge` coexist with the built-in `gg`.
//!
//! The actual work each action performs lives in `run.rs::dispatch`; this
//! module is pure data with no dependency on `State`.

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::errors::FxError;

/// One physical keypress. Uppercase characters carry their case in `code`, so
/// the `SHIFT` modifier is normalized away for `Char` keys (mirroring the old
/// `KeyModifiers::NONE | KeyModifiers::SHIFT` arm in `run.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl Key {
    /// Build a key from a live crossterm event, normalizing `SHIFT` away for
    /// character keys (the character already encodes its case).
    pub fn from_event(code: KeyCode, mods: KeyModifiers) -> Self {
        let mods = match code {
            KeyCode::Char(_) => mods & !KeyModifiers::SHIFT,
            _ => mods,
        };
        Key { code, mods }
    }

    fn plain(code: KeyCode) -> Self {
        Key {
            code,
            mods: KeyModifiers::NONE,
        }
    }
}

/// A sequence of keys forming one binding. `j` is one key; `gg` and `ge` are
/// two.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChord(pub Vec<Key>);

/// Every remappable command, expressed as data.
///
/// Text editing *inside* an input mode (insert/rename/search/command line) is
/// intentionally excluded — those keys stay fixed for now. The `Enter*`/`Quit*`
/// variants are handed back to the loop, which runs the corresponding hardcoded
/// sub-loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Movement
    MoveDown,
    MoveUp,
    HalfPageDown,
    HalfPageUp,
    GoToTop,
    GoToBottom,
    // Navigation / open
    Open,
    OpenNewWindow,
    GoToParent,
    JumpForward,
    JumpBackward,
    // File operations
    Unpack,
    Delete,
    Yank,
    Put,
    Undo,
    Redo,
    // Toggles
    ToggleSort,
    ToggleHidden,
    TogglePreview,
    ToggleSplit,
    ToggleVisual,
    ResetSelection,
    // Search navigation
    SearchNext,
    SearchPrev,
    // Preview scrolling
    ScrollPreviewDown,
    ScrollPreviewUp,
    // Mode entry (handed back to the loop; sub-loops stay hardcoded)
    EnterInsert,
    EnterInsertDir,
    Rename,
    EnterSearch,
    EnterCommandLine,
    EnterRegister,
    ZoxideJump,
    // Exit
    Quit,
    QuitWithoutSave,
}

/// Result of looking up the currently pending key sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    /// The pending sequence is a complete binding.
    Action(Action),
    /// The pending sequence is a live prefix of one or more longer bindings;
    /// wait for the next key.
    Prefix,
    /// Dead end; drop the pending buffer.
    None,
}

/// Maps key chords to actions, with a fast set of live prefixes so the loop
/// knows when to keep reading.
///
/// `visual_overrides` handles the one place where visual mode needs a different
/// binding than normal mode: `d` and `y` are single-press actions in visual
/// mode but the first key of the `dd`/`yy` chords in normal mode. Every other
/// visual-vs-normal difference is behavioral and lives in `run.rs::dispatch`,
/// which branches on `state.v_start`.
#[derive(Debug, Clone)]
pub struct Keymap {
    map: HashMap<KeyChord, Action>,
    prefixes: HashSet<Vec<Key>>,
    visual_overrides: HashMap<Key, Action>,
}

impl Keymap {
    /// Build a keymap from `(chord, action)` pairs, deriving the prefix set.
    fn from_pairs(pairs: impl IntoIterator<Item = (KeyChord, Action)>) -> Self {
        let mut map = HashMap::new();
        for (chord, action) in pairs {
            map.insert(chord, action);
        }
        let prefixes = Self::compute_prefixes(&map);
        Keymap {
            map,
            prefixes,
            visual_overrides: HashMap::new(),
        }
    }

    /// Every strict prefix of every bound chord — the sequences on which the
    /// loop should wait for more input.
    fn compute_prefixes(map: &HashMap<KeyChord, Action>) -> HashSet<Vec<Key>> {
        let mut prefixes = HashSet::new();
        for chord in map.keys() {
            for i in 1..chord.0.len() {
                prefixes.insert(chord.0[..i].to_vec());
            }
        }
        prefixes
    }

    /// Insert or override a binding, keeping the prefix set consistent.
    pub fn insert(&mut self, chord: KeyChord, action: Action) {
        self.map.insert(chord, action);
        self.prefixes = Self::compute_prefixes(&self.map);
    }

    /// Build the built-in keymap and overlay user bindings from config.
    ///
    /// Each entry maps a chord string (`ge`, `C-d`) to an action name
    /// (`go_to_bottom`). Invalid entries — an unparseable chord or an unknown
    /// action — are skipped with a warning so a typo never crashes felix. A
    /// binding whose chord shadows a live prefix of another chord is warned
    /// about too (the longer chord becomes unreachable).
    pub fn from_config(bindings: &BTreeMap<String, String>) -> Self {
        let mut km = Self::default();
        for (chord_str, action_str) in bindings {
            let chord = match parse_chord(chord_str) {
                Ok(chord) => chord,
                Err(_) => {
                    log::warn!("Invalid keybinding chord: {:?}", chord_str);
                    continue;
                }
            };
            let action = match parse_action(action_str) {
                Some(action) => action,
                None => {
                    log::warn!("Unknown keybinding action: {:?}", action_str);
                    continue;
                }
            };
            km.insert(chord, action);
        }
        // Warn about chords that shadow a live prefix (e.g. binding both `g`
        // and `ge` makes `ge` unreachable, since `g` fires first).
        for chord in km.map.keys() {
            if km.prefixes.contains(&chord.0) {
                log::warn!(
                    "Keybinding {:?} shadows a longer chord and makes it unreachable",
                    chord
                );
            }
        }
        km
    }

    /// Look up the pending key sequence.
    ///
    /// In visual mode, a lone key with a visual override (e.g. `d`, `y`) fires
    /// immediately instead of being treated as a chord prefix. Otherwise: an
    /// exact match wins over "is a prefix", so a binding that is also the prefix
    /// of a longer one still fires (the longer one is unreachable — a shadow the
    /// config layer warns about).
    pub fn resolve(&self, pending: &[Key], visual: bool) -> Resolved {
        if visual {
            if let [key] = pending {
                if let Some(action) = self.visual_overrides.get(key) {
                    return Resolved::Action(*action);
                }
            }
        }
        if let Some(action) = self.map.get(&KeyChord(pending.to_vec())) {
            return Resolved::Action(*action);
        }
        if self.prefixes.contains(pending) {
            return Resolved::Prefix;
        }
        Resolved::None
    }
}

impl Default for Keymap {
    /// The built-in bindings — an exact port of what `run.rs` matched inline.
    fn default() -> Self {
        use Action::*;
        use KeyCode as Kc;

        let c = |ch: char| Key::plain(Kc::Char(ch));
        let ctrl = |ch: char| Key {
            code: Kc::Char(ch),
            mods: KeyModifiers::CONTROL,
        };
        let alt = |ch: char| Key {
            code: Kc::Char(ch),
            mods: KeyModifiers::ALT,
        };
        let key = |code: KeyCode| Key::plain(code);
        let one = |k: Key| KeyChord(vec![k]);
        let two = |a: Key, b: Key| KeyChord(vec![a, b]);

        let pairs = vec![
            // Movement
            (one(c('j')), MoveDown),
            (one(key(Kc::Down)), MoveDown),
            (one(c('k')), MoveUp),
            (one(key(Kc::Up)), MoveUp),
            (one(ctrl('d')), HalfPageDown),
            (one(ctrl('u')), HalfPageUp),
            (two(c('g'), c('g')), GoToTop),
            (one(c('G')), GoToBottom),
            // Navigation / open
            (one(c('l')), Open),
            (one(key(Kc::Enter)), Open),
            (one(key(Kc::Right)), Open),
            (one(c('o')), OpenNewWindow),
            (one(c('h')), GoToParent),
            (one(key(Kc::Left)), GoToParent),
            (one(key(Kc::Tab)), JumpForward),
            (one(ctrl('o')), JumpBackward),
            // File operations
            (one(c('e')), Unpack),
            (two(c('d'), c('d')), Delete),
            (two(c('y'), c('y')), Yank),
            (one(c('p')), Put),
            (one(c('u')), Undo),
            (one(ctrl('r')), Redo),
            // Toggles
            (one(c('t')), ToggleSort),
            (one(key(Kc::Backspace)), ToggleHidden),
            (one(c('v')), TogglePreview),
            (one(c('s')), ToggleSplit),
            (one(c('V')), ToggleVisual),
            (one(key(Kc::Esc)), ResetSelection),
            // Search navigation
            (one(c('n')), SearchNext),
            (one(c('N')), SearchPrev),
            // Preview scrolling (Alt+j / Alt+k and their arrow equivalents)
            (one(alt('j')), ScrollPreviewDown),
            (
                one(Key {
                    code: Kc::Down,
                    mods: KeyModifiers::ALT,
                }),
                ScrollPreviewDown,
            ),
            (one(alt('k')), ScrollPreviewUp),
            (
                one(Key {
                    code: Kc::Up,
                    mods: KeyModifiers::ALT,
                }),
                ScrollPreviewUp,
            ),
            // Mode entry
            (one(c('i')), EnterInsert),
            (one(c('I')), EnterInsertDir),
            (one(c('c')), Rename),
            (one(c('/')), EnterSearch),
            (one(c(':')), EnterCommandLine),
            (one(c('"')), EnterRegister),
            (one(c('z')), ZoxideJump),
            // Exit
            (two(c('Z'), c('Z')), Quit),
            (two(c('Z'), c('Q')), QuitWithoutSave),
        ];

        let mut km = Self::from_pairs(pairs);
        // In visual mode `d` and `y` act on the selection immediately rather
        // than starting the `dd`/`yy` chords.
        km.visual_overrides.insert(c('d'), Delete);
        km.visual_overrides.insert(c('y'), Yank);
        km
    }
}

/// Parse a binding string into a chord.
///
/// A whole string that names a single key — a special key (`Enter`, `Tab`,
/// `Backspace`, `Esc`, ...), a modified key (`C-d`, `A-j`, `S-x`), or a lone
/// character — becomes one [`Key`]. Anything else is treated as a sequence of
/// single-character keys, so `gg`, `dd`, and `ge` parse as two keys each.
pub fn parse_chord(s: &str) -> Result<KeyChord, FxError> {
    if s.is_empty() {
        return Err(FxError::Config("Empty keybinding".to_string()));
    }
    if let Some(key) = parse_single_key(s) {
        return Ok(KeyChord(vec![key]));
    }
    // Otherwise, a sequence of literal character keys.
    let keys: Vec<Key> = s.chars().map(|ch| Key::plain(KeyCode::Char(ch))).collect();
    Ok(KeyChord(keys))
}

/// Parse an action name (snake_case, e.g. `go_to_bottom`) into an [`Action`].
/// Returns `None` for an unknown name. Uses the same serde `rename_all` mapping
/// the config deserializer would.
pub fn parse_action(s: &str) -> Option<Action> {
    serde_yaml::from_str::<Action>(s).ok()
}

/// Parse a string that names exactly one key, or return `None` if it should be
/// treated as a multi-key sequence.
fn parse_single_key(s: &str) -> Option<Key> {
    // Modified form: "C-d", "A-j", "S-x".
    if let Some((prefix, rest)) = s.split_once('-') {
        if rest.chars().count() == 1 {
            let mods = match prefix {
                "C" | "c" => KeyModifiers::CONTROL,
                "A" | "a" | "M" | "m" => KeyModifiers::ALT,
                "S" | "s" => KeyModifiers::SHIFT,
                _ => return None,
            };
            let ch = rest.chars().next().unwrap();
            return Some(Key {
                code: KeyCode::Char(ch),
                mods,
            });
        }
    }
    // Named special keys.
    let code = match s {
        "Enter" | "CR" | "Return" => Some(KeyCode::Enter),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" | "BS" => Some(KeyCode::Backspace),
        "Esc" | "Escape" => Some(KeyCode::Esc),
        "Space" => Some(KeyCode::Char(' ')),
        "Up" => Some(KeyCode::Up),
        "Down" => Some(KeyCode::Down),
        "Left" => Some(KeyCode::Left),
        "Right" => Some(KeyCode::Right),
        _ => None,
    };
    if let Some(code) = code {
        return Some(Key::plain(code));
    }
    // A single literal character.
    let mut chars = s.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        return Some(Key::plain(KeyCode::Char(first)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(ch: char) -> Key {
        Key::plain(KeyCode::Char(ch))
    }

    #[test]
    fn parse_single_char() {
        assert_eq!(parse_chord("j").unwrap(), KeyChord(vec![c('j')]));
    }

    #[test]
    fn parse_multi_char_chord() {
        assert_eq!(parse_chord("gg").unwrap(), KeyChord(vec![c('g'), c('g')]));
        assert_eq!(parse_chord("ge").unwrap(), KeyChord(vec![c('g'), c('e')]));
        assert_eq!(parse_chord("ZZ").unwrap(), KeyChord(vec![c('Z'), c('Z')]));
    }

    #[test]
    fn parse_ctrl() {
        assert_eq!(
            parse_chord("C-d").unwrap(),
            KeyChord(vec![Key {
                code: KeyCode::Char('d'),
                mods: KeyModifiers::CONTROL
            }])
        );
    }

    #[test]
    fn parse_alt_and_special() {
        assert_eq!(
            parse_chord("A-j").unwrap(),
            KeyChord(vec![Key {
                code: KeyCode::Char('j'),
                mods: KeyModifiers::ALT
            }])
        );
        assert_eq!(
            parse_chord("Enter").unwrap(),
            KeyChord(vec![Key::plain(KeyCode::Enter)])
        );
        assert_eq!(
            parse_chord("Tab").unwrap(),
            KeyChord(vec![Key::plain(KeyCode::Tab)])
        );
    }

    #[test]
    fn shift_normalized_away_for_chars() {
        let k = Key::from_event(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(k, c('G'));
    }

    #[test]
    fn default_resolves_simple_and_chords() {
        let km = Keymap::default();
        assert_eq!(
            km.resolve(&[c('j')], false),
            Resolved::Action(Action::MoveDown)
        );
        assert_eq!(
            km.resolve(&[c('G')], false),
            Resolved::Action(Action::GoToBottom)
        );
        // gg is a chord: first g is a prefix, gg is the action.
        assert_eq!(km.resolve(&[c('g')], false), Resolved::Prefix);
        assert_eq!(
            km.resolve(&[c('g'), c('g')], false),
            Resolved::Action(Action::GoToTop)
        );
        // dead end
        assert_eq!(km.resolve(&[c('g'), c('x')], false), Resolved::None);
        assert_eq!(km.resolve(&[c('q')], false), Resolved::None);
    }

    #[test]
    fn user_can_add_ge_binding() {
        let mut km = Keymap::default();
        km.insert(parse_chord("ge").unwrap(), Action::GoToBottom);
        assert_eq!(km.resolve(&[c('g')], false), Resolved::Prefix);
        assert_eq!(
            km.resolve(&[c('g'), c('e')], false),
            Resolved::Action(Action::GoToBottom)
        );
        // existing gg still works
        assert_eq!(
            km.resolve(&[c('g'), c('g')], false),
            Resolved::Action(Action::GoToTop)
        );
    }

    #[test]
    fn ctrl_and_alt_do_not_collide_with_plain() {
        let km = Keymap::default();
        let ctrl_d = Key {
            code: KeyCode::Char('d'),
            mods: KeyModifiers::CONTROL,
        };
        assert_eq!(
            km.resolve(&[ctrl_d], false),
            Resolved::Action(Action::HalfPageDown)
        );
        let alt_down = Key {
            code: KeyCode::Down,
            mods: KeyModifiers::ALT,
        };
        assert_eq!(
            km.resolve(&[alt_down], false),
            Resolved::Action(Action::ScrollPreviewDown)
        );
        let plain_down = Key::plain(KeyCode::Down);
        assert_eq!(
            km.resolve(&[plain_down], false),
            Resolved::Action(Action::MoveDown)
        );
    }

    #[test]
    fn parse_action_names() {
        assert_eq!(parse_action("go_to_bottom"), Some(Action::GoToBottom));
        assert_eq!(parse_action("half_page_down"), Some(Action::HalfPageDown));
        assert_eq!(parse_action("toggle_hidden"), Some(Action::ToggleHidden));
        assert_eq!(parse_action("not_an_action"), None);
    }

    #[test]
    fn from_config_overlays_defaults() {
        let mut bindings = BTreeMap::new();
        bindings.insert("ge".to_string(), "go_to_bottom".to_string());
        // Invalid entries are skipped, not fatal.
        bindings.insert("zz_bad".to_string(), "not_an_action".to_string());
        let km = Keymap::from_config(&bindings);
        // User binding is active.
        assert_eq!(
            km.resolve(&[c('g'), c('e')], false),
            Resolved::Action(Action::GoToBottom)
        );
        // Built-in bindings remain intact.
        assert_eq!(
            km.resolve(&[c('g'), c('g')], false),
            Resolved::Action(Action::GoToTop)
        );
        assert_eq!(km.resolve(&[c('g')], false), Resolved::Prefix);
    }

    #[test]
    fn visual_mode_d_and_y_are_single_press() {
        let km = Keymap::default();
        // In normal mode, d is a prefix (dd chord).
        assert_eq!(km.resolve(&[c('d')], false), Resolved::Prefix);
        assert_eq!(km.resolve(&[c('y')], false), Resolved::Prefix);
        // In visual mode, a single d/y fires immediately.
        assert_eq!(
            km.resolve(&[c('d')], true),
            Resolved::Action(Action::Delete)
        );
        assert_eq!(km.resolve(&[c('y')], true), Resolved::Action(Action::Yank));
        // Movement chords still resolve the same in visual mode.
        assert_eq!(km.resolve(&[c('g')], true), Resolved::Prefix);
        assert_eq!(
            km.resolve(&[c('g'), c('g')], true),
            Resolved::Action(Action::GoToTop)
        );
    }
}
