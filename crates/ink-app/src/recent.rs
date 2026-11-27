//! 트레이의 최근 캡처. 픽셀과 작은 미리보기는 앱 메모리에만 둔다.
use crate::capture::LastCapture;
use crate::ports::Frame;
use crate::service::InkService;
use crate::state::RecentCaptureView;
use ink_domain::geometry::PhysRect;
use ink_domain::image::{self, FrameRef};
use std::collections::VecDeque;
use std::sync::Arc;

const LIMIT: usize = 5;
const TTL_MS: u64 = 30 * 60 * 1000;

pub(crate) struct RecentCapture {
    view: RecentCaptureView,
    expires_ms: u64,
    width: u32,
    height: u32,
    pixels: Arc<Vec<u8>>,
    thumb: Vec<u8>,
}

impl RecentCapture {
    /// 판서 바탕은 모니터 크기다. 선택한 그림만 비율을 유지해 가운데 놓는다.
    pub(crate) fn draw_frame(&self, monitor_id: u32, bounds: PhysRect) -> (Frame, PhysRect) {
        let (mw, mh) = (bounds.w as u32, bounds.h as u32);
        if (self.width, self.height) == (mw, mh) {
            return (Frame { monitor_id, width: mw, height: mh, bgra: self.pixels.clone() }, bounds);
        }
        let scaled;
        let (w, h, pixels) = if self.width > mw || self.height > mh {
            let (w, h, p) = image::thumbnail(self.width, self.height, &self.pixels, mw, mh);
            scaled = p;
            (w, h, scaled.as_slice())
        } else {
            (self.width, self.height, self.pixels.as_slice())
        };
        let rect = PhysRect::new(bounds.x + (bounds.w - w as i32) / 2, bounds.y + (bounds.h - h as i32) / 2, w as i32, h as i32);
        let (_, _, canvas) = image::crop(&[FrameRef { bounds: rect, w, h, bgra: pixels }], bounds);
        (Frame { monitor_id, width: mw, height: mh, bgra: Arc::new(canvas) }, rect)
    }
}

#[derive(Default)]
pub(crate) struct RecentCaptures {
    entries: VecDeque<RecentCapture>,
    seq: u64,
}

impl RecentCaptures {
    pub(crate) fn views(&self, now: u64) -> Vec<RecentCaptureView> {
        self.entries.iter().filter(|e| now < e.expires_ms).map(|e| e.view.clone()).collect()
    }

    pub(crate) fn get(&self, id: u64, now: u64) -> Option<&RecentCapture> {
        self.entries.iter().find(|e| e.view.id == id && now < e.expires_ms)
    }
}

impl InkService {
    pub(crate) fn remember_capture(&mut self, capture: &LastCapture) -> String {
        let now = self.clock_ms();
        let captured_at = self.ports.clock.unix();
        let recent = &mut self.cap.recent;
        recent.seq += 1;
        let id = recent.seq;
        let thumb_url = self.ports.overlay.recent_thumbnail_url(id);
        let (w, h, px) = image::thumbnail(capture.w, capture.h, &capture.bgra, 96, 96);
        recent.entries.push_front(RecentCapture {
            view: RecentCaptureView { id, captured_at, thumb_url: thumb_url.clone() },
            expires_ms: now + TTL_MS,
            width: capture.w,
            height: capture.h,
            pixels: capture.bgra.clone(),
            thumb: ink_domain::bmp::encode(w, h, &px),
        });
        recent.entries.truncate(LIMIT);
        self.release_unlisted_result();
        thumb_url
    }

    fn release_unlisted_result(&mut self) {
        if self.cap.last.as_ref().is_some_and(|last| !self.cap.recent.entries.iter().any(|e| Arc::ptr_eq(&e.pixels, &last.bgra))) {
            self.cap.last = None;
        }
    }

    pub fn recent_thumbnail(&self, id: u64) -> Option<Vec<u8>> {
        self.cap.recent.get(id, self.clock_ms()).map(|e| e.thumb.clone())
    }

    pub(crate) fn expire_recent(&mut self, now: u64) {
        if self.cap.recent.entries.back().is_some_and(|e| now >= e.expires_ms) {
            self.cap.recent.entries.retain(|e| now < e.expires_ms);
            self.release_unlisted_result();
            self.emit_state();
        }
    }

    pub(crate) fn clear_recent(&mut self) {
        if !self.cap.recent.entries.is_empty() {
            self.cap.recent.entries.clear();
            self.cap.last = None;
            self.emit_state();
        }
    }
}
