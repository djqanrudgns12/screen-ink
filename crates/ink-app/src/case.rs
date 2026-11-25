//! 인식 시험 사례(08 §5.2 스냅숏 재생): 진입 순간의 정지 이미지·창 목록·A 색인을 창 제목·글자 없이 떠 둔 것.
//!
//! 앱은 개발자가 `SCREEN_INK_CASE_DIR`를 줬을 때만 확정한 범위를 정답으로 한 사례를 쓴다(사람 정답 도구).
//! 시험 문서 화면에서만 쓴다 — 학생·교직원 정보·실제 업무 화면 금지(08 §5.1). 재생은 `ink-platform-fake::replay`.

use ink_domain::geometry::{Monitor, PhysPoint, PhysRect};
use ink_domain::recog::ElementCand;
use ink_domain::snapshot::RawWindow;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CASE_SCHEMA: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Case {
    pub schema: u32,
    pub id: String,
    /// 무엇을 띄운 화면인지(사람이 적음, 예: "RB-04 Win32 시험 앱")
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub monitors: Vec<CaseMonitor>,
    /// 진입 순간 커서
    pub cursor: PhysPoint,
    /// 창 목록(앞 → 뒤, 거르기 전). 우리 앱 창은 뺀다.
    pub windows: Vec<RawWindow>,
    /// 창 → A 색인(사각형·컨트롤 종류만)
    #[serde(default)]
    pub elements: BTreeMap<u64, Vec<ElementCand>>,
    /// 창 → H 자식 창 사각형
    #[serde(default)]
    pub children: BTreeMap<u64, Vec<PhysRect>>,
    pub points: Vec<CasePoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseMonitor {
    #[serde(flatten)]
    pub monitor: Monitor,
    /// 같은 폴더의 PNG 파일 이름
    pub frame: String,
}

/// 커서 점 하나와 그 점의 정답 사각형.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CasePoint {
    pub at: PhysPoint,
    pub truth: PhysRect,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
}

impl Case {
    /// 모니터 이미지 파일 이름 규칙.
    pub fn frame_name(id: &str, monitor: u32) -> String {
        format!("{id}.m{monitor}.png")
    }
}
