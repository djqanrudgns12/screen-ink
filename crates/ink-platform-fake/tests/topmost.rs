use ink_app::{
    ports::{BorderStyle, PinError, WindowPinner},
    HotkeyOutcome, InkService,
};
use ink_domain::{topmost::CandidateWindow, Platform};
use ink_platform_fake::FakeWorld;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Scene {
    active: u64,
    pinned: BTreeSet<u64>,
    border: u32,
    denied: bool,
}
#[derive(Default, Clone)]
struct Pinner(Arc<Mutex<Scene>>);

impl WindowPinner for Pinner {
    fn foreground(&self) -> Option<CandidateWindow> {
        let s = self.0.lock().unwrap();
        Some(CandidateWindow {
            handle: s.active,
            pid: 0,
            visible: true,
            exe: "chrome.exe".into(),
            has_our_prop: s.pinned.contains(&s.active),
            ..Default::default()
        })
    }
    fn pin(&mut self, h: u64, b: BorderStyle) -> Result<(), PinError> {
        let mut s = self.0.lock().unwrap();
        if s.denied {
            return Err(PinError::AccessDenied);
        }
        s.border = b.thickness_px;
        s.pinned.insert(h);
        Ok(())
    }
    fn unpin(&mut self, h: u64) -> Result<(), PinError> {
        self.0.lock().unwrap().pinned.remove(&h);
        Ok(())
    }
    fn update_border_style(&mut self, b: BorderStyle) {
        self.0.lock().unwrap().border = b.thickness_px;
    }
    fn unpin_all(&mut self) {
        self.0.lock().unwrap().pinned.clear();
    }
    fn is_alive(&self, h: u64) -> bool {
        self.0.lock().unwrap().pinned.contains(&h)
    }
    fn cleanup_orphans(&mut self) -> usize {
        let mut s = self.0.lock().unwrap();
        std::mem::take(&mut s.pinned).len()
    }
}

#[test]
fn 고정_풀기_모두풀기_창닫힘_시작정리_흐름() {
    let w = FakeWorld::new();
    let mut svc = InkService::new(w.ports(), Platform::Win, "시험");
    let p = Pinner::default();
    p.0.lock().unwrap().pinned.insert(99);
    svc.set_window_pinner(Box::new(p.clone()));
    svc.cleanup_on_start();
    svc.start();
    svc.tick();
    svc.announce_topmost_cleanup();
    assert_eq!(w.last_island().unwrap().text, "창 고정이 풀렸어요");
    let notices = w.islands().len();
    svc.announce_topmost_cleanup();
    assert_eq!(w.islands().len(), notices);
    assert_eq!(svc.on_hotkey("topmost"), HotkeyOutcome::Topmost);
    assert_eq!(svc.state().pinned_count, 1);
    assert_eq!(w.last_island().unwrap().text, "‘Chrome’을 맨 위에 고정했어요");
    svc.update_setting("topmost.border", &serde_json::json!(0));
    assert_eq!(p.0.lock().unwrap().border, 0);
    svc.undo();
    assert_eq!(p.0.lock().unwrap().border, 4);
    p.0.lock().unwrap().active = 2;
    svc.toggle_topmost();
    assert_eq!(svc.state().pinned_count, 2);
    let notices = w.islands().len();
    p.0.lock().unwrap().pinned.remove(&0);
    svc.tick();
    assert_eq!(svc.state().pinned_count, 1);
    assert_eq!(w.islands().len(), notices);
    svc.toggle_topmost();
    assert_eq!(w.last_island().unwrap().text, "고정을 풀었어요");
    assert_eq!(svc.state().pinned_count, 0);
    svc.toggle_topmost();
    p.0.lock().unwrap().active = 3;
    svc.toggle_topmost();
    svc.unpin_all(true);
    assert_eq!(w.last_island().unwrap().text, "고정한 창 2개를 풀었어요");
    assert_eq!(svc.state().pinned_count, 0);
    p.0.lock().unwrap().denied = true;
    svc.toggle_topmost();
    assert_eq!(w.last_island().unwrap().text, "관리자 권한 창이라 고정할 수 없어요");
    assert_eq!(svc.state().pinned_count, 0);
}

#[test]
fn 정상_종료와_모두_풀기_뒤에는_시작_정리_안내가_없다() {
    for announce in [false, true] {
        let p = Pinner::default();
        let w = FakeWorld::new();
        let mut svc = InkService::new(w.ports(), Platform::Win, "시험");
        svc.set_window_pinner(Box::new(p.clone()));
        svc.start();
        svc.toggle_topmost();
        assert_eq!(svc.state().pinned_count, 1);
        svc.unpin_all(announce);
        assert_eq!(svc.state().pinned_count, 0);

        let restarted = FakeWorld::new();
        let mut svc = InkService::new(restarted.ports(), Platform::Win, "시험");
        svc.set_window_pinner(Box::new(p));
        svc.cleanup_on_start();
        svc.start();
        let notices = restarted.islands().len();
        svc.announce_topmost_cleanup();
        assert_eq!(restarted.islands().len(), notices);
    }
}
