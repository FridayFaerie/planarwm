use crate::river::river_seat_v1::Modifiers;
use xkbcommon::xkb::{self, KEYSYM_NO_FLAGS};

#[derive(Debug, Clone)]
pub enum Action {
    None,
    Pan,
    View { x: i32, y: i32 },
    Spawn { program: String, args: Vec<String> },
    SpawnShell { command: String },
    Close,
    CenterFocused,
    Move,
    Resize,
    ToggleMaximize,
    NextSlide,
    PrevSlide,
    MoveToNextSlide,
    MoveToPrevSlide,
    NextWindow,
    PrevWindow,
    CycleTiling,
    Exit,
}

pub fn parse_modifiers(s: &str) -> Option<Modifiers> {
    let mut mods = Modifiers::None;
    let mut seen_any = false;

    for part in s.split(['+', '-']) {
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
    let ks = xkb::keysym_from_name(s.trim(), KEYSYM_NO_FLAGS);
    //KEYSYM_CASE_INSENSITIVE exists, but it's recommended only to use that as a fallback
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
        "center_focused" => Some(Action::CenterFocused),
        "move" => Some(Action::Move),
        "resize" => Some(Action::Resize),
        "maximize" => Some(Action::ToggleMaximize),
        "next_slide" => Some(Action::NextSlide),
        "moveto_next_slide" => Some(Action::MoveToNextSlide),
        "moveto_prev_slide" => Some(Action::MoveToPrevSlide),
        "prev_slide" => Some(Action::PrevSlide),
        "next_window" => Some(Action::NextWindow),
        "prev_window" => Some(Action::PrevWindow),
        "cycle_tiling" => Some(Action::CycleTiling),
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
