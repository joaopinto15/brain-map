//! The keymap, as a decision. One place says what a key means; the window only carries
//! it out, so what the page does with a keystroke is checkable without opening one.

use eframe::egui::Key;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Action {
    /// Space: the leader, cleared by whatever key follows it.
    Arm,
    ToggleExplorer,
    FocusSearch,
    /// Drop the selection, the filter and the open note.
    Release,
    Replay,
    ToggleFull,
    NextMatch,
    PrevMatch,
    /// The explorer, walked the way vi walks a list: `j` and `k` move a row, `h` shuts a
    /// folder or steps out of it, `l` opens one or reads the note, `g` and `G` are the
    /// ends. With a note open the same keys move the reader, since the note is what the
    /// panel is showing.
    Down,
    Up,
    Out,
    Into,
    Top,
    Bottom,
    /// A fixed screen distance, so it moves the same amount at any zoom.
    Pan(f64, f64),
}

/// What a key does. `in_field` is a keystroke inside an input or a select: typing must
/// never trigger a binding, since `r`, `f` and `/` are all letters someone will type.
///
/// Only the key the leader binds is consumed. Swallowing the rest would mean one stray
/// space silently killed the next keystroke, which is worse than the chord is worth.
pub fn binding(key: &str, leader: bool, in_field: bool) -> Option<Action> {
    if in_field {
        return None;
    }
    if key == " " {
        return Some(Action::Arm);
    }
    if leader && key == "e" {
        return Some(Action::ToggleExplorer);
    }
    match key {
        "/" => Some(Action::FocusSearch),
        "Escape" => Some(Action::Release),
        "r" | "R" => Some(Action::Replay),
        "f" | "F" => Some(Action::ToggleFull),
        "n" => Some(Action::NextMatch),
        "N" => Some(Action::PrevMatch),
        "j" => Some(Action::Down),
        "k" => Some(Action::Up),
        "h" => Some(Action::Out),
        "l" | "Enter" => Some(Action::Into),
        "g" => Some(Action::Top),
        "G" => Some(Action::Bottom),
        // The camera keeps the arrows: hjkl belong to the tree now, and a graph still has
        // to be pannable without a mouse.
        "Left" => Some(Action::Pan(-1.0, 0.0)),
        "Right" => Some(Action::Pan(1.0, 0.0)),
        "Up" => Some(Action::Pan(0.0, -1.0)),
        "Down" => Some(Action::Pan(0.0, 1.0)),
        _ => None,
    }
}

/// egui names a key as a variant; the keymap names it the way a keyboard is printed.
/// `None` is a key the map has no word for, which is every key it does not bind.
pub fn key_name(key: Key, shift: bool) -> Option<String> {
    let named = match key {
        Key::Escape => "Escape",
        Key::Space => " ",
        Key::Slash => "/",
        Key::Enter => "Enter",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.to_string());
    }
    // Every remaining binding is a letter, and shift is what tells `n` from `N`.
    let name = format!("{key:?}");
    let letter = name.len() == 1 && name.chars().all(|c| c.is_ascii_alphabetic());
    letter.then(|| match shift {
        true => name,
        false => name.to_lowercase(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keypress_is_named_the_way_the_map_spells_it() {
        assert_eq!(key_name(Key::R, true).as_deref(), Some("R"));
        assert_eq!(key_name(Key::R, false).as_deref(), Some("r"));
        assert_eq!(
            key_name(Key::Slash, true).as_deref(),
            Some("/"),
            "shift is how you type it"
        );
        assert_eq!(key_name(Key::Space, false).as_deref(), Some(" "));
        assert_eq!(key_name(Key::Escape, false).as_deref(), Some("Escape"));
        assert_eq!(key_name(Key::F5, false), None, "nothing the map binds");
        // The named keys and the letters have to agree with `binding`, or a key that is
        // pressed is not the key that is bound.
        for (key, shift) in [
            (Key::Slash, false),
            (Key::Escape, false),
            (Key::Space, false),
        ] {
            let name = key_name(key, shift).expect("a name");
            assert!(
                binding(&name, false, false).is_some(),
                "{name} names nothing"
            );
        }
    }

    #[test]
    fn the_keys_are_the_keys_the_hint_promises() {
        assert_eq!(binding("/", false, false), Some(Action::FocusSearch));
        assert_eq!(binding("R", false, false), Some(Action::Replay));
        assert_eq!(binding("F", false, false), Some(Action::ToggleFull));
        assert_eq!(binding("n", false, false), Some(Action::NextMatch));
        assert_eq!(binding("N", false, false), Some(Action::PrevMatch));
        assert_eq!(binding("Escape", false, false), Some(Action::Release));
        assert_eq!(binding("Left", false, false), Some(Action::Pan(-1.0, 0.0)));
        assert_eq!(binding("Down", false, false), Some(Action::Pan(0.0, 1.0)));
    }

    #[test]
    fn the_tree_is_walked_the_way_vi_walks_one() {
        assert_eq!(binding("j", false, false), Some(Action::Down));
        assert_eq!(binding("k", false, false), Some(Action::Up));
        assert_eq!(binding("h", false, false), Some(Action::Out));
        assert_eq!(binding("l", false, false), Some(Action::Into));
        assert_eq!(binding("Enter", false, false), Some(Action::Into));
        assert_eq!(binding("g", false, false), Some(Action::Top));
        assert_eq!(binding("G", false, false), Some(Action::Bottom));
        // The names have to be the ones a keypress is given, or nothing fires.
        for key in [Key::Enter, Key::ArrowDown, Key::ArrowUp] {
            let name = key_name(key, false).expect("a name");
            assert!(
                binding(&name, false, false).is_some(),
                "{name} names nothing"
            );
        }
    }

    #[test]
    fn space_leads_only_the_key_it_binds() {
        assert_eq!(binding(" ", false, false), Some(Action::Arm));
        assert_eq!(binding("e", true, false), Some(Action::ToggleExplorer));
        assert_eq!(binding("e", false, false), None, "e alone does nothing");
        // A leader left armed by a stray space must not eat the next key.
        assert_eq!(binding("/", true, false), Some(Action::FocusSearch));
        assert_eq!(binding("R", true, false), Some(Action::Replay));
    }

    #[test]
    fn typing_never_triggers_a_binding() {
        for key in ["r", "f", "/", "n", "j", " ", "Escape"] {
            assert_eq!(binding(key, false, true), None, "{key} fired while typing");
        }
    }
}
