//! 결과 쪽 어댑터: 클립보드(DIB+PNG) · 저장 폴더(PNG, 이름 규칙·중복·260자·사진 폴더 대체) · 시계 · 섬 썸네일.

use ink_app::{Clipboard, Clock, FileSink, SavedFile};
use ink_domain::naming::{self, NameContext};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, Runtime};

/// BGRA → PNG(RGB, 알파 없음 — R-OUT-3, 빠른 압축).
pub fn encode_png(w: u32, h: u32, bgra: &[u8]) -> Result<Vec<u8>, String> {
    let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
    for px in bgra.chunks_exact(4) {
        rgb.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgb).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// 클립보드 소유 창 = 섬 창(늘 있고, 숨어 있어도 된다).
pub struct TauriClipboard<R: Runtime> {
    /// Windows 클립보드 소유 창을 찾을 때만 쓴다
    #[cfg_attr(not(windows), allow(dead_code))]
    pub app: AppHandle<R>,
}

impl<R: Runtime> TauriClipboard<R> {
    #[cfg(windows)]
    fn owner(&self) -> isize {
        self.app.get_webview_window(crate::ISLAND).and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize).unwrap_or(0)
    }
}

impl<R: Runtime> Clipboard for TauriClipboard<R> {
    fn set_image(&mut self, w: u32, h: u32, bgra: &[u8]) -> Result<(), String> {
        #[cfg(windows)]
        {
            let png = encode_png(w, h, bgra).ok();
            ink_platform_windows::clipboard::set_image(self.owner(), w, h, bgra, png.as_deref())
        }
        #[cfg(target_os = "macos")]
        {
            let png = encode_png(w, h, bgra).ok();
            ink_platform_macos::clipboard::set_image(w, h, bgra, png.as_deref())
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            let _ = (w, h, bgra);
            Err("clipboard image not implemented on this platform yet".into())
        }
    }

    fn set_text(&mut self, text: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            ink_platform_windows::clipboard::set_text(self.owner(), text)
        }
        #[cfg(target_os = "macos")]
        {
            ink_platform_macos::clipboard::set_text(text)
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            let _ = text;
            Err("clipboard text not implemented on this platform yet".into())
        }
    }
}

/// 저장 폴더. 빈 값 = 사진 폴더\screen-ink. 폴더를 만들 수 없으면 사진 폴더로(R-OUT-2).
pub struct TauriFiles<R: Runtime> {
    pub app: AppHandle<R>,
}

impl<R: Runtime> TauriFiles<R> {
    fn pictures(&self) -> PathBuf {
        self.app.path().picture_dir().unwrap_or_else(|_| std::env::temp_dir()).join("screen-ink")
    }
}

fn write_unique(dir: &Path, stem: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let dir_s = dir.to_string_lossy();
    let stem = naming::fit_path(&dir_s, stem, "png");
    let name = naming::unique_file_name(&stem, "png", |n| dir.join(n).exists());
    let path = dir.join(name);
    // 새 파일로만(같은 순간 두 번 저장해도 덮지 않게)
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
    std::io::Write::write_all(&mut f, bytes)?;
    Ok(path)
}

impl<R: Runtime> FileSink for TauriFiles<R> {
    fn save_png(&mut self, folder: &str, stem: &str, w: u32, h: u32, bgra: &[u8]) -> Result<SavedFile, String> {
        let bytes = encode_png(w, h, bgra)?;
        let wanted = if folder.trim().is_empty() { None } else { Some(PathBuf::from(folder)) };
        let (path, fell_back) = match wanted.as_ref().map(|d| write_unique(d, stem, &bytes)) {
            Some(Ok(p)) => (p, false),
            Some(Err(e)) => {
                log::warn!("save folder unusable ({e}), using pictures");
                (write_unique(&self.pictures(), stem, &bytes).map_err(|e| e.to_string())?, true)
            }
            None => (write_unique(&self.pictures(), stem, &bytes).map_err(|e| e.to_string())?, true),
        };
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(SavedFile { name, path: path.to_string_lossy().into_owned(), fell_back })
    }

    fn write_case(&mut self, dir: &str, case: &ink_app::case::Case, frames: &[ink_app::Frame]) -> Result<String, String> {
        write_case(Path::new(dir), case, frames)
    }
}

/// 인식 시험 사례: `<id>.json` + 모니터별 무손실 PNG(개발용, `SCREEN_INK_CASE_DIR`).
pub fn write_case(dir: &Path, case: &ink_app::case::Case, frames: &[ink_app::Frame]) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for m in &case.monitors {
        let Some(f) = frames.iter().find(|f| f.monitor_id == m.monitor.id) else { continue };
        let png = encode_png(f.width, f.height, &f.bgra)?;
        std::fs::write(dir.join(&m.frame), png).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(case).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", case.id));
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn local(&self) -> NameContext {
        #[cfg(windows)]
        {
            ink_platform_windows::screen::local_time()
        }
        #[cfg(target_os = "macos")]
        {
            ink_platform_macos::local_time()
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            // 어댑터 없는 곳: UTC
            let secs = self.unix();
            let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
            NameContext { year: 1970, month: 1, day: 1, hour: h as u8, minute: m as u8, second: s as u8, app: None, n: 0 }
        }
    }

    fn unix(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }
}

/// (마지막 번호, 번호 → (폭, 높이, BGRA))
type Thumbs = (u64, HashMap<u64, (u32, u32, Vec<u8>)>);

/// 결과 섬 썸네일(`ink://thumb/<id>`) — 마지막 몇 개만.
#[derive(Default, Clone)]
pub struct ThumbStore(Arc<Mutex<Thumbs>>);

impl ThumbStore {
    pub fn put(&self, w: u32, h: u32, bgra: Vec<u8>) -> u64 {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        g.0 += 1;
        let id = g.0;
        g.1.retain(|k, _| *k + 4 > id);
        g.1.insert(id, (w, h, bgra));
        id
    }

    pub fn bmp(&self, id: u64) -> Option<Vec<u8>> {
        let g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        g.1.get(&id).map(|(w, h, px)| ink_domain::bmp::encode(*w, *h, px))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png는_rgb로_줄고_다시_읽힌다() {
        let bgra: Vec<u8> = (0..64 * 64).flat_map(|i| [(i % 256) as u8, 0, 255, 0]).collect();
        let p = encode_png(64, 64, &bgra).unwrap();
        assert_eq!(&p[1..4], b"PNG");
        let dec = png::Decoder::new(std::io::Cursor::new(&p));
        let mut r = dec.read_info().unwrap();
        let mut buf = vec![0; r.output_buffer_size().unwrap()];
        let info = r.next_frame(&mut buf).unwrap();
        assert_eq!((info.width, info.height, info.color_type), (64, 64, png::ColorType::Rgb));
        assert_eq!(&buf[0..3], &[255, 0, 0], "BGRA → RGB");
    }

    #[test]
    fn 사례는_json과_모니터별_png로_쓴다() {
        use ink_app::case::{Case, CaseMonitor, CASE_SCHEMA};
        use ink_domain::geometry::{Monitor, PhysPoint, PhysRect};
        let d = std::env::temp_dir().join(format!("screen-ink-case-w-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mon = Monitor { id: 3, bounds: PhysRect::new(0, 0, 4, 2), work: PhysRect::new(0, 0, 4, 2), scale: 1.0, primary: true };
        let case = Case {
            schema: CASE_SCHEMA,
            id: "c1".into(),
            note: String::new(),
            monitors: vec![CaseMonitor { monitor: mon, frame: Case::frame_name("c1", 3) }],
            cursor: PhysPoint { x: 1, y: 1 },
            windows: vec![],
            elements: Default::default(),
            children: Default::default(),
            points: vec![],
        };
        let frames = [ink_app::Frame { monitor_id: 3, width: 4, height: 2, bgra: std::sync::Arc::new(vec![9; 32]) }];
        let path = write_case(&d, &case, &frames).unwrap();
        assert!(path.ends_with("c1.json"));
        let back: Case = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, case);
        assert!(d.join("c1.m3.png").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn 저장은_새_파일로_같은_이름이면_2() {
        let d = std::env::temp_dir().join(format!("screen-ink-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let a = write_unique(&d, "수업_1", b"a").unwrap();
        let b = write_unique(&d, "수업_1", b"b").unwrap();
        assert_eq!(a.file_name().unwrap(), "수업_1.png");
        assert_eq!(b.file_name().unwrap(), "수업_1 (2).png");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn 썸네일은_최근_것만() {
        let t = ThumbStore::default();
        let ids: Vec<u64> = (0..6).map(|_| t.put(1, 1, vec![0, 0, 0, 255])).collect();
        assert!(t.bmp(ids[0]).is_none());
        assert!(t.bmp(ids[5]).is_some());
    }
}
