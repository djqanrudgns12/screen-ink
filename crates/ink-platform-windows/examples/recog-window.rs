//! 시험 창 하나만 WGC로 떠서 재생 사례로 저장한다. 창 밖은 단색이며 제목·요소 이름을 읽지 않는다.
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ink_app::case::{Case, CaseMonitor, CASE_SCHEMA};
    use ink_app::{ElementSource, Frame, ScreenSource};
    use ink_domain::geometry::PhysRect;
    use ink_platform_windows::{screen, uia::UiaIndex, wgc_window::WgcScreen};
    use std::{collections::BTreeMap, env, path::PathBuf, sync::mpsc, time::Duration};
    use windows_sys::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };

    let args: Vec<String> = env::args().skip(1).collect();
    let with_hidden = args.len() == 7 && args[4] == "--cloaked";
    if !(matches!(args.len(), 4 | 8) || with_hidden) {
        return Err(
            "인자: 사례ID PID HWND 메모 [창 안의 x y 너비 높이 | --cloaked PID HWND]".into(),
        );
    }
    let id = &args[0];
    if id.is_empty() || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
        return Err("사례 ID에는 영문·숫자·붙임표만 써요".into());
    }
    let dir = PathBuf::from(env::var("SCREEN_INK_CASE_DIR")?);
    let pid = args[1].parse::<u32>()?;
    let handle = args[2].parse::<u64>()?;
    // 이 스레드에서 창·모니터 모두 물리 픽셀로 읽는다.
    unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let cursor = screen::cursor_pos();
    let layout = screen::monitor_layout();
    let snapshot = screen::top_windows();
    let window = snapshot
        .iter()
        .find(|w| w.pid == pid && w.handle == handle && w.visible && !w.minimized && !w.cloaked)
        .ok_or("주어진 프로세스에 보이는 시험 창이 없어요")?;
    let r = window.rect;
    let region = if args.len() == 8 {
        let local: Vec<i32> = args[4..]
            .iter()
            .map(|v| v.parse())
            .collect::<Result<_, _>>()?;
        let roi = PhysRect::new(r.x + local[0], r.y + local[1], local[2], local[3]);
        if roi.intersect(&r) != Some(roi) {
            return Err("시험 영역이 창 밖이에요".into());
        }
        roi
    } else {
        r
    };
    let (tx, rx) = mpsc::channel();
    let index = UiaIndex::spawn(move |_, _, _| {
        let _ = tx.send(());
    });
    index.request(handle, &window.exe);
    rx.recv_timeout(Duration::from_secs(10))?;
    let children = screen::child_rects(handle);
    let shot = WgcScreen::new().capture_window(handle, r)?;
    if (shot.width as i32, shot.height as i32) != (r.w, r.h) {
        return Err(format!(
            "창과 영상 크기가 달라요: {r:?}, {}×{}",
            shot.width, shot.height
        )
        .into());
    }
    let mut monitors = Vec::new();
    let mut frames = Vec::new();
    for monitor in layout.monitors {
        let b = monitor.bounds;
        let mut pixels = vec![255; (b.area() * 4) as usize];
        if let Some(part) = b.intersect(&region) {
            for y in part.y..part.bottom() {
                let src = (((y - r.y) * r.w + part.x - r.x) * 4) as usize;
                let dst = (((y - b.y) * b.w + part.x - b.x) * 4) as usize;
                let n = (part.w * 4) as usize;
                pixels[dst..dst + n].copy_from_slice(&shot.bgra[src..src + n]);
            }
        }
        frames.push(Frame {
            monitor_id: monitor.id,
            width: b.w as u32,
            height: b.h as u32,
            bgra: pixels.into(),
        });
        monitors.push(CaseMonitor {
            frame: Case::frame_name(id, monitor.id),
            monitor,
        });
    }
    let elements = index.elements(handle);
    // 실행마다 달라지는 번호를 고정한다. 재생 프로세스의 PID와 겹쳐 창이 제외되는 일도 막는다.
    let hidden = if with_hidden {
        let other_pid = args[5].parse::<u32>()?;
        let other = args[6].parse::<u64>()?;
        if !snapshot
            .iter()
            .any(|w| w.handle == other && w.pid == other_pid && w.cloaked)
        {
            return Err("다른 데스크톱의 시험 창에서 숨김 속성을 확인하지 못했어요".into());
        }
        Some(other)
    } else {
        None
    };
    let windows = snapshot
        .into_iter()
        .filter_map(|mut w| {
            let id = if w.handle == handle {
                1
            } else if Some(w.handle) == hidden {
                2
            } else {
                return None;
            };
            w.handle = id;
            w.pid = u32::MAX - (id - 1) as u32;
            Some(w)
        })
        .collect();
    let case = Case {
        schema: CASE_SCHEMA,
        id: id.clone(),
        note: args[3].clone(),
        monitors,
        cursor,
        windows,
        elements: BTreeMap::from([(1, elements)]),
        children: BTreeMap::from([(1, children)]),
        points: Vec::new(),
    };
    ink_platform_fake::replay::save(&dir, &case, &frames)?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("이 수집 도구는 Windows에서 실행해요");
}
