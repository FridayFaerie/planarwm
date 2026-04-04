use crate::river::river_seat_v1::Modifiers;
use xkbcommon::xkb::{self, KEYSYM_CASE_INSENSITIVE};

#[derive(Debug, Clone)]
pub enum Action {
    None,
    Pan,
    View { x: i32, y: i32 },
    Spawn { program: String, args: Vec<String> },
    SpawnShell { command: String },
    Close,
    Focus,
    FocusNext,
    Move,
    Resize,
    ToggleMaximize,
    Exit,
}

pub fn parse_modifiers(s: &str) -> Option<Modifiers> {
    let mut mods = Modifiers::None;
    let mut seen_any = false;

    for part in s.split(|c: char| c == '+' || c == '-') {
        let part = part.trim().to_ascii_lowercase();
        if part.is_empty() {
            continue;
        }

        let m = match part.as_str() {
            "none" => Modifiers::None,
            "shift" => Modifiers::Shift,
            "ctrl" => Modifiers::Ctrl,
            "alt" => Modifiers::Mod1,
            "super" => Modifiers::Mod4,
            "mod3" => Modifiers::Mod3,
            "mod5" => Modifiers::Mod5,
            _ => return None,
        };

        mods = mods.union(m);
        seen_any = true;
    }

    if seen_any { Some(mods) } else { None }
}

pub fn parse_keysym(s: &str) -> Option<u32> {
    let ks = xkb::keysym_from_name(s.trim(), KEYSYM_CASE_INSENSITIVE);
    // it is recommended to first call this function without this flag; and if that fails, only then to try with this flag, while possibly warning the user he had misspelled the name, and might get wrong results.
    // but I shall not :>
    if ks != xkb::keysyms::KEY_NoSymbol.into() {
        return Some(ks.into());
    }
    None
}

pub fn parse_action(keyword: &str) -> Option<Action> {
    let keyword = keyword.trim();

    match keyword {
        "pan" => Some(Action::Pan),
        "close" => Some(Action::Close),
        "focus" => Some(Action::Focus),
        "focus_next" => Some(Action::FocusNext),
        "move" => Some(Action::Move),
        "resize" => Some(Action::Resize),
        "maximize" => Some(Action::ToggleMaximize),
        "exit" => Some(Action::Exit),
        _ if keyword.starts_with("spawn ") => {
            let rest = &keyword["spawn ".len()..];
            let mut parts = rest.split_whitespace();
            let program = parts.next()?.to_string();
            let args = parts.map(|s| s.to_string()).collect();
            Some(Action::Spawn { program, args })
        }
        _ if keyword.starts_with("shell ") => {
            let command = keyword["shell ".len()..].trim().to_string();
            Some(Action::SpawnShell { command })
        }
        _ if keyword.starts_with("view ") => {
            let mut parts = keyword["view ".len()..].split_whitespace();
            let x = parts.next()?.parse().ok()?;
            let y = parts.next()?.parse().ok()?;
            Some(Action::View { x, y })
        }
        _ => None,
    }
}
