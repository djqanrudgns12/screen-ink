//! 활성 창을 고정하거나 풀기 전에 확인하는 규칙.

use crate::i18n::{static_text, Lang};

pub const PROP_SCREEN_INK: &str = "ScreenInk_TopmostPinned";
pub const PROP_POWERTOYS: &str = "AlwaysOnTop_Pinned";
pub const UNAVAILABLE: &str = "topmost.unavailable";
pub const POWERTOYS: &str = "topmost.powerToys";
pub const ALREADY_TOPMOST: &str = "topmost.alreadyTopmost";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateWindow {
    pub handle: u64,
    pub pid: u32,
    pub exe: String,
    pub class: String,
    pub visible: bool,
    pub minimized: bool,
    pub cloaked: bool,
    pub topmost: bool,
    pub has_our_prop: bool,
    pub has_powertoys_prop: bool,
    pub fullscreen_d3d_game: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopmostDecision {
    Pin,
    Unpin,
    IgnoreSilent,
    /// 섬에서 보여 줄 한 줄 안내.
    Skip(&'static str),
}

pub fn decide(win: &CandidateWindow, own_pid: u32) -> TopmostDecision {
    decide_for(win, own_pid, Lang::Ko)
}

pub fn decide_for(win: &CandidateWindow, own_pid: u32, lang: Lang) -> TopmostDecision {
    use TopmostDecision::*;
    let shell = matches!(win.class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
        || (win.class == "Windows.UI.Core.CoreWindow" && matches!(win.exe.as_str(), "startmenuexperiencehost.exe" | "shellexperiencehost.exe"));
    if win.fullscreen_d3d_game {
        IgnoreSilent
    } else if win.pid == own_pid || shell || !win.visible || win.minimized || win.cloaked {
        Skip(static_text(lang, UNAVAILABLE))
    } else if win.has_our_prop {
        Unpin
    } else if win.has_powertoys_prop {
        Skip(static_text(lang, POWERTOYS))
    } else if win.topmost {
        Skip(static_text(lang, ALREADY_TOPMOST))
    } else {
        Pin
    }
}

pub fn display_name(exe: &str) -> String {
    let base = exe.strip_suffix(".exe").unwrap_or(exe);
    let mut chars = base.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TopmostDecision::*;

    #[test]
    fn 고정과_건너뛰기의_우선순위() {
        let mut w = CandidateWindow {
            visible: true,
            pid: 7,
            ..Default::default()
        };
        assert_eq!(decide(&w, 1), Pin);
        w.topmost = true;
        assert_eq!(decide(&w, 1), Skip("이 창은 원래 맨 위에 있어요"));
        w.has_powertoys_prop = true;
        assert_eq!(decide(&w, 1), Skip("PowerToys가 고정한 창이에요"));
        w.has_our_prop = true;
        assert_eq!(decide(&w, 1), Unpin);
        assert_eq!(decide(&w, 7), Skip("이 창은 고정할 수 없어요"));
        for class in ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"] {
            w.class = class.into();
            assert_eq!(decide(&w, 1), Skip("이 창은 고정할 수 없어요"));
        }
        w.class = "Windows.UI.Core.CoreWindow".into();
        assert_eq!(decide(&w, 1), Unpin);
        w.exe = "startmenuexperiencehost.exe".into();
        assert_eq!(decide(&w, 1), Skip("이 창은 고정할 수 없어요"));
        w.class.clear();
        for (visible, minimized, cloaked) in [(false, false, false), (true, true, false), (true, false, true)] {
            w.visible = visible;
            w.minimized = minimized;
            w.cloaked = cloaked;
            assert_eq!(decide(&w, 1), Skip("이 창은 고정할 수 없어요"));
        }
        w.fullscreen_d3d_game = true;
        assert_eq!(decide(&w, 7), IgnoreSilent);
        assert_eq!(display_name("chrome.exe"), "Chrome");
    }
}
