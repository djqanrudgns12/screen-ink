//! 진입 계측(M1, R-CAP-2): 단축키 → 오버레이가 정지 화면을 그린 순간까지.
//! 로그 한 줄 `overlay ready [N ms] (capture N, show N, …)` — Snipaste 로그와 같은 집계를 `bench/`에서 한다.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryTiming {
    /// 모니터 배치 읽기
    pub probe_ms: f64,
    /// 창 목록(정지 순간)
    pub windows_ms: f64,
    /// 모니터별 정지 이미지
    pub capture_ms: f64,
    /// 오버레이 창 보이기 명령까지
    pub show_ms: f64,
    /// 네이티브 정지 화면 창이 그려진 순간(단축키 기준 누적) — 있으면 이것이 진입 시간
    #[serde(default)]
    pub frame_ms: Option<f64>,
    /// UI(웹뷰 오버레이)가 그렸다고 알린 순간(단축키 기준 누적)
    pub ready_ms: Option<f64>,
    pub monitors: usize,
    pub windows: usize,
    /// UI 쪽: 세션 알림 → 이미지 load / load → 그림(다음 프레임 둘)
    #[serde(default)]
    pub ui_load_ms: Option<f64>,
    #[serde(default)]
    pub ui_paint_ms: Option<f64>,
}

impl EntryTiming {
    /// 로그 한 줄. 창 제목·파일 이름 없이 숫자만(05 §9).
    /// 진입 시간 = 정지 화면이 보인 순간(네이티브면 frame, 아니면 UI 그림).
    pub fn entry_ms(&self) -> f64 {
        self.frame_ms.or(self.ready_ms).unwrap_or(self.probe_ms + self.windows_ms + self.capture_ms + self.show_ms)
    }

    pub fn log_line(&self) -> String {
        let total = self.entry_ms();
        let mut line = format!(
            "overlay ready [{} ms] (capture {}, show {}, windows {} ms/{}, monitors {}",
            total.round() as i64,
            self.capture_ms.round() as i64,
            self.show_ms.round() as i64,
            self.windows_ms.round() as i64,
            self.windows,
            self.monitors
        );
        if self.frame_ms.is_some() {
            if let Some(r) = self.ready_ms {
                line.push_str(&format!(", ui {}", r.round() as i64));
            }
        }
        if let (Some(l), Some(p)) = (self.ui_load_ms, self.ui_paint_ms) {
            line.push_str(&format!(", ui load {}, paint {}", l.round() as i64, p.round() as i64));
        }
        line.push(')');
        line
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub n: usize,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub max: f64,
}

/// 가장 가까운 순위 백분위(Snipaste 집계와 같은 방식).
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank.min(sorted.len()) - 1]
}

pub fn stats(samples: &[f64]) -> Option<Stats> {
    if samples.is_empty() {
        return None;
    }
    let mut s: Vec<f64> = samples.iter().copied().filter(|x| x.is_finite()).collect();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(Stats { n: s.len(), p50: percentile(&s, 50.0), p90: percentile(&s, 90.0), p95: percentile(&s, 95.0), max: *s.last()? })
}

/// 로그에서 `overlay ready [N ms]`의 N을 뽑는다(벤치 집계·자기 시험).
pub fn parse_ready_ms(line: &str) -> Option<f64> {
    let rest = line.split("overlay ready [").nth(1)?;
    rest.split(" ms]").next()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 로그_줄은_숫자만_담고_다시_읽힌다() {
        let mut t = EntryTiming { probe_ms: 0.4, windows_ms: 2.2, capture_ms: 41.6, show_ms: 3.1, ready_ms: Some(72.5), monitors: 2, windows: 31, ..Default::default() };
        let line = t.log_line();
        assert_eq!(line, "overlay ready [73 ms] (capture 42, show 3, windows 2 ms/31, monitors 2)");
        assert_eq!(parse_ready_ms(&format!("[INFO] {line}")), Some(73.0));
        t.ui_load_ms = Some(12.4);
        t.ui_paint_ms = Some(16.6);
        assert!(t.log_line().ends_with("monitors 2, ui load 12, paint 17)"));
        // 네이티브 정지 화면이면 그 순간이 진입 시간, UI는 따로
        t.frame_ms = Some(55.2);
        t.ui_load_ms = None;
        assert_eq!(t.entry_ms(), 55.2);
        assert_eq!(t.log_line(), "overlay ready [55 ms] (capture 42, show 3, windows 2 ms/31, monitors 2, ui 73)");
    }

    #[test]
    fn 백분위는_가장_가까운_순위() {
        let v: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let s = stats(&v).unwrap();
        assert_eq!((s.n, s.p50, s.p90, s.p95, s.max), (100, 50.0, 90.0, 95.0, 100.0));
        assert_eq!(stats(&[7.0]).unwrap().p95, 7.0);
        assert!(stats(&[]).is_none());
    }
}
