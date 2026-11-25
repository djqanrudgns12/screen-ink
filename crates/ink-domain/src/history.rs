//! 범위 기록(05 §4): 사각형만(이미지·창 제목·앱 이름 없음). 같은 모니터 구성일 때만 다시 쓴다.

use crate::geometry::PhysRect;
use serde::{Deserialize, Serialize};

pub const KEEP_DAYS: u64 = 30;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub rect: PhysRect,
    /// 모니터 구성 지문(`MonitorLayout::fingerprint`)
    pub layout: String,
    /// element · window · drag · full
    pub mode: String,
    /// 유닉스 초
    pub at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    /// 최신이 앞
    pub entries: Vec<HistoryEntry>,
}

impl History {
    /// 맨 앞에 추가. 같은 사각형이 연달아 오면 합치고, `max`를 넘으면 끝을 버린다(0 = 기록 끔).
    pub fn push(&mut self, e: HistoryEntry, max: usize) {
        if max == 0 {
            self.entries.clear();
            return;
        }
        if self.entries.first().is_some_and(|f| f.rect == e.rect && f.layout == e.layout) {
            self.entries[0].at = e.at;
        } else {
            self.entries.insert(0, e);
        }
        self.entries.truncate(max);
    }

    /// 지금 구성에서 쓸 수 있는 사각형(최신이 앞).
    pub fn for_layout(&self, layout: &str) -> Vec<PhysRect> {
        self.entries.iter().filter(|e| e.layout == layout).map(|e| e.rect).collect()
    }

    /// 시작할 때 30일 넘은 것을 조용히 정리. 지운 개수.
    pub fn prune(&mut self, now: u64) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| now.saturating_sub(e.at) <= KEEP_DAYS * 86_400);
        before - self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(x: i32, layout: &str, at: u64) -> HistoryEntry {
        HistoryEntry { rect: PhysRect::new(x, 0, 10, 10), layout: layout.into(), mode: "drag".into(), at }
    }

    #[test]
    fn 앞에_쌓고_연속_중복은_합치고_넘치면_버린다() {
        let mut h = History::default();
        h.push(e(1, "A", 1), 3);
        h.push(e(1, "A", 2), 3);
        h.push(e(2, "A", 3), 3);
        h.push(e(3, "B", 4), 3);
        h.push(e(4, "A", 5), 3);
        assert_eq!(h.entries.len(), 3);
        assert_eq!(h.for_layout("A").iter().map(|r| r.x).collect::<Vec<_>>(), vec![4, 2]);
        h.push(e(5, "A", 6), 0);
        assert!(h.entries.is_empty(), "0 = 기록 끔");
    }

    #[test]
    fn 오래된_것은_정리() {
        let mut h = History { entries: vec![e(1, "A", 100), e(2, "A", 100 + 31 * 86_400)] };
        assert_eq!(h.prune(100 + 31 * 86_400), 1);
        assert_eq!(h.entries[0].rect.x, 2);
    }
}
