# E. 단계 6 F2 창 캡처 · 바로 캡처 키 · F3 화면 녹화 구현 시안 (`6.1`~`6.5`)

대상 항목(`progress.json`): `6.1` F2 WGC 창 캡처·대체 경로·검은 화면 판정 · `6.2` 바로 캡처 키(`Shift+F1`·`Ctrl+F1`·`Shift+F2`·`Alt+F1`) · `6.3` F3 범위 + `Recorder` · `6.4` 녹화 섬·빨간 꺾쇠·트레이 빨간 점 · `6.5` 자동 중지·임시 파일 복구·마이크.

---

## 1. 목표 · 범위 (요구사항 ID)

| 요구사항 | 우선 | 요약 |
|---|---|---|
| `R-WIN-1` | P0 | `F2` 커서 아래 창 강조 → 클릭 = 가려진 부분 포함 **그 창만** WGC `CreateForWindow`로 캡처(`SetIsBorderRequired(false)` 노란 테두리 끔), 실패 시 `PrintWindow(PW_RENDERFULLCONTENT)` → `PrintWindow(0)` → `BitBlt`. Win11 둥근 모서리 투명 배경 PNG |
| `R-WIN-2` | P0 | `F2` 400 ms 안 두 번 = 지금 활성 창 즉시 캡처 |
| `R-WIN-3` | P0 | 보호 콘텐츠(전부 검정) · 최소화 창 → 빈 이미지 복사 금지 + 섬 `"이 창은 가져올 수 없어요"` |
| `R-CAP-13`·`R-KEY-5` | P0/P1 | 바로 캡처 키 4종: `Shift+F1` 직전 범위 · `Ctrl+F1` 커서 모니터 · `Shift+F2` 활성 창 · `Alt+F1` 지연 캡처(3·5·10초 카운트다운 섬, 캡처 제외) |
| `R-REC-1` | P0 | `F3` → F1과 같은 범위 고르기(모니터 클릭 1회·창 클릭=창 추적·드래그=사각형) → 즉시 MP4(H.264) 녹화. 1080p 30 fps 10분 정상 재생, **시작 1초 안 중지도 파일 손상 0** |
| `R-REC-2` | P0 | 녹화 중 `Alt+1` 판서는 영상에 포함, 녹화 섬·빨간 꺾쇠·말풍선은 `WDA_EXCLUDEFROMCAPTURE`로 제외 |
| `R-REC-3` | P1 | 마이크 녹음(기본 끔, 장치 고르기, 장치 사라지면 자동 끔+안내), 커서 포함 여부 |
| `05 §8` | P0 | 잠금·절전·모니터 해제 → 자동 중지·저장, 비정상 종료 시 `%TEMP%\screen-ink\rec-*.mp4` 다음 시작 복구 제안 |

---

## 2. 정확한 형식

### 2.1 `6.1` F2 창 캡처 — 포트 · 구조체 · 대체 사슬

```rust
// crates/ink-app/src/ports.rs — ScreenSource 확장
pub trait ScreenSource: Send {
    fn capture(&mut self, layout: &MonitorLayout) -> Result<Vec<Frame>, String>;
    fn cursor_sprite(&mut self) -> Option<ink_domain::image::CursorSprite> { None }
    /// F2 창 단독 캡처(가려진 부분 포함, 둥근 모서리 알파 유지).
    /// 반환: (width, height, bgra, method_tag: "wgc"|"pw_full"|"pw_0"|"bitblt")
    fn capture_window(&mut self, _hwnd: u64, _bounds: PhysRect) -> Result<WindowShot, String> {
        Err("window capture not supported".into())
    }
}

#[derive(Clone, Debug)]
pub struct WindowShot {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    pub method: &'static str, // 디버그 로그용(창 제목 없음)
}
```

- **모듈 위치**: `crates/ink-platform-windows/src/wgc_window.rs` (신규).
- **순수 함수 (`crates/ink-domain/src/image.rs`)**:
  - `is_all_black(bgra: &[u8]) -> bool`: 전체 픽셀(또는 4×4 스트라이드 표본 후 의심 시 전수) `R==0 && G==0 && B==0`이면 `true` → 보호 콘텐츠 판정(`R-WIN-3`).
  - `mask_win11_round_corners(bgra: &mut [u8], w: u32, h: u32, radius_px: u32)`: `DWMWA_WINDOW_CORNER_PREFERENCE`가 `DWMWCP_ROUND`(8 px × scale) 또는 `ROUNDSMALL`(4 px × scale)이면 네 귀 바깥 알파를 0으로 안티앨리어싱 마스크(`R-CAP-25`·`R-WIN-1`).
- **F2 UI 차이 (`04 S10`)**:
  - `CaptureOverlay.svelte`가 `sessionKind === 'window'`일 때:
    - 커서 아래 창 1개만 강조(요소 사슬 없음, `W` 근거만 — `08` 재사용).
    - 주황 2 px 테두리 + 오른쪽 위 고정폭 크기 꼬리표(`1280×800`).
    - 클릭 = `CaptureInput::Command { name: "captureWindow" }` → `InkService::finish_window(handle, rect, exe)`.
    - 섬: 창 아이콘 · **창 캡처** · 앱 이름(창 제목 X) · `가려진 부분까지` / `클릭` 찍기 · `F2` 한 번 더 = 활성 창 · `Esc`.

### 2.2 `6.2` 바로 캡처 키 4종

| 슬롯 (`defaults-v1.json`) | Windows 기본 | 맥 기본 | 동작 (`crates/ink-app/src/capture.rs`) |
|---|---|---|---|
| `last` | `Shift+F1` | `Shift+F1` | `cap.ranges.for_layout(현재 지문).first()` 즉시 캡처 → 기본 결과(복사/저장). 없으면 섬 `"지난 범위가 없어요"` |
| `full` | `Ctrl+F1` | `Ctrl+Option+F1` | 커서 모니터 전체(`layout.nearest(cursor).bounds`) 즉시 캡처 → 기본 결과 |
| `active` | `Shift+F2` | `Shift+F2` | `SystemProbe::foreground_window()` → `capture_window` 즉시 실행(F2 두 번과 같은 경로) |
| `delay` | `Alt+F1` | `Option+F1` | 지연 캡처 카운트다운 시작(기본 3초, 트레이 메뉴에서 3·5·10초 선택 가능). 섬 `variant: "delay"`, `countdown: 3 → 2 → 1`, text `"초 뒤 범위 캡처"`, `Esc` 취소. 0초 도달 시 섬을 먼저 숨긴 뒤(`100 ms` 대기 없이 동기 `hide`) F1 진입 |

- `SystemProbe` 확장: `fn foreground_window(&self) -> Option<RawWindow> { None }` 추가.

### 2.3 `6.3`~`6.5` F3 화면 녹화 — 상태 기계 · 포트 · 파일

```rust
// crates/ink-domain/src/recording.rs (신규, 05 §8)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecPhase { Idle, Picking, Starting, Recording, Finalizing }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecTarget {
    Monitor { monitor_id: u32, rect: PhysRect },
    Region { monitor_id: u32, rect: PhysRect }, // WGC 모니터 캡처 후 크롭(짝수 w·h 보정)
    Window { handle: u64, rect: PhysRect },     // 창 추적(CreateForWindow)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub active: bool,
    pub phase: RecPhase,
    pub elapsed_ms: u64,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub mic_on: bool,
    pub rect: Option<PhysRect>,
}
```

```rust
// crates/ink-app/src/ports.rs 추가
#[derive(Clone, Debug)]
pub struct RecStartOpts {
    pub target: RecTarget,
    pub temp_path: std::path::PathBuf, // %TEMP%\screen-ink\rec-<id>.mp4
    pub fps: u32,                      // 30 | 60
    pub mic_device: Option<String>,    // None = 마이크 끔
    pub capture_cursor: bool,
}

#[derive(Clone, Debug)]
pub struct RecStats { pub elapsed_ms: u64, pub bytes: u64, pub frames: u64 }

pub trait Recorder: Send {
    fn start(&mut self, opts: RecStartOpts) -> Result<(), String>;
    fn stats(&self) -> RecStats;
    /// 정상 마무리(트랜스코더 flush + moov atom 기록). 시작 < 1초라도 최소 1프레임 flush 보장.
    fn stop(&mut self) -> Result<RecStats, String>;
    /// 버리기(임시 파일 삭제).
    fn abort(&mut self);
    /// 사용 가능한 마이크 장치 목록(`[(id, display_name)]`, 설정 창용).
    fn audio_inputs(&self) -> Vec<(String, String)> { Vec::new() }
}
```

| 파일 위치 | 책임 |
|---|---|
| `crates/ink-domain/src/recording.rs` | 상태 기계, H.264 짝수 해상도 보정(`even_rect`), 예상 잔여 분 계산(`free_bytes / (20MB/min * fps/30)`), 임시 파일 복구 목록 판정 |
| `crates/ink-app/src/record.rs` | `RecordState`, 녹화 시작·1초 상태 틱·중지·이름 규칙 이동·자동 중지(잠금·절전·모니터 해제)·시작 시 `%TEMP%\screen-ink\rec-*.mp4` 스캔·복구 제안 |
| `crates/ink-platform-windows/src/recorder.rs` | `WinRecorder`: `windows-capture` WGC + `VideoEncoder`(WinRT `MediaTranscoder` + `MediaStreamSource`, H.264 MP4) + WASAPI 마이크 캡처(AAC 믹스, `record.mic=true`일 때만) |
| `crates/tauri-plugin-screen-ink/src/rec_frame.rs` | 녹화 범위 표시 전용 투명 클릭 통과 창(`ink-rec-frame`, `WDA_EXCLUDEFROMCAPTURE`, 네 귀 빨간 꺾쇠 + 바깥 22% 어둡게 — `SelectionFrame state="recording"`) |
| `packages/ink-ui/src/lib/screens/RecordFrame.svelte` | `SelectionFrame state="recording"` 전용 얇은 뷰(`index.html?view=recframe`) |
| `packages/ink-client/src/index.js`, `tauri.js` | `InkAction`에 `'record.stop'`, `'record.discard'`, `'record.recoverSave'`, `'record.recoverDelete'`, `listAudioInputs()` 추가 |

### 2.4 화면 상태 · 문구 (`04 S11`)

- **녹화 중 모드 겹침 (`03 §3`)**:
  - `Mode`에 `recording`을 **독립 상태**로 분리(`service.rec_active: bool`)해 녹화 중에도 `Alt+1` 판서·`Alt+2` 실시간 확대·`F1` 캡처가 자유롭게 진입·종료된다(`05 §11` 8·9번).
  - 판서 오버레이(`ink-overlay-*`)는 `WDA_EXCLUDEFROMCAPTURE`를 **걸지 않아** 영상에 함께 찍히고, 섬(`ink-island`)과 빨간 꺾쇠(`ink-rec-frame`)는 **`WDA_EXCLUDEFROMCAPTURE`를 걸어** 영상에서 빠진다(`R-REC-2`).
- **녹화 섬 (`04 S11`)**:
  - 첫 녹화 1회(설정 `record.firstHintDone` 미완료 시): 2초 동안 `"판서는 영상에 함께 · 이 알약은 안 찍혀요"`를 보인 뒤 녹화 상태로 축소.
  - 녹화 중: 빨간 점(1.6초 맥박) · `00:42` · `1920×1080 · 30fps` · 마이크 꺼짐 아이콘 · `[■ 중지 F3]`.
  - 공간 부족 경고: `"남은 공간이 n분 정도예요"`(경고 후 녹화 그대로 시작, `02 §5`).
  - 완료: `"녹화를 저장했어요 · 00:42 · 18 MB"` + `[폴더 열기]`.
  - 복구(시작 시 미완성 임시 파일 발견): `"끝나지 않은 녹화가 있어요"` + `[저장 시도]` `[지우기]`.
- **트레이**: `InkState.recording.active = true` → 기존 `TauriNotifier`가 이미 `TrayMode::Recording`(큰 빨간 원)으로 자동 전환.

---

## 3. 알고리즘 · 결정 (대안과 고른 이유)

- **Windows 인코더**: `windows-capture` + WinRT `MediaTranscoder`/`MediaStreamSource`(H.264 하드웨어 가속 우선, 소프트웨어 자동 폴백 — ZoomIt과 동일 경로) 채택.
  - 대안 `ffmpeg.exe` 동봉 기각 이유: 설치 용량 +40~80 MB, GPL/LGPL 라이선스 부담, 학교 PC 백신 오탐 위험.
- **시작 1초 안 중지 안전장치 (`R-REC-1`)**:
  - 첫 비디오 프레임이 인코더에 1장 이상 들어가기 전에 `stop()`이 불리면 최대 `400 ms`까지 첫 프레임을 기다린 뒤 2프레임(최소 0.1초)을 밀어 넣고 스트림 EOF를 닫는다 → MP4 `moov` atom 누락/깨짐 0건.
- **짝수 해상도 보정**: H.264 YUV420은 가로·세로가 2의 배수여야 한다. 범위 크기가 홀수면 `w & !1`, `h & !1`(최소 16×16)로 내림 보정해 인코더 초기화 실패 방지.
- **임시 파일 → 원자적 이동 (`05 §8`)**: 녹화는 `%TEMP%\screen-ink\rec-<id>.mp4`에 쓰고 `Finalizing` 성공 시 `std::fs::rename`(다른 드라이브면 복사→검증→삭제)으로 저장 폴더에 옮긴다. 강제 종료되면 임시 파일이 남아 다음 실행 때 복구(MP4 재생 가능 여부 헤더 검사 후 이동 또는 삭제)를 제안한다.

---

## 4. 시험 · 합격 기준

| 시험 / 측정 | 이름 예시 | 합격 기준 |
|---|---|---|
| Rust 단위 (`ink-domain`) | `검은화면_보호콘텐츠_판정_및_둥근모서리_알파마스크` | 전 픽셀 검정 `true`, 1픽셀이라도 색 있으면 `false`, 모서리 밖 알파 0 |
| Rust 단위 (`ink-domain`) | `녹화_해상도_짝수보정_및_남은공간_예상분_계산` | 1919×1079 → 1918×1078, 100MB/30fps → 5분 |
| Rust 통합 (`ink-platform-fake`) | `f2_400ms_두번_활성창_및_바로캡처_4종` | `Shift+F1`/`Ctrl+F1`/`Shift+F2`/`Alt+F1` 각각 기대 대상 캡처 |
| Rust 통합 (`ink-platform-fake`) | `녹화중_잠금_절전_모니터해제_자동중지_저장` | `SystemEvent::Lock` → `recorder.stop()` 호출 + 임시 파일 이동 + 결과 섬 |
| Rust 통합 (`ink-platform-fake`) | `녹화중_Alt1_판서_겹침_허용_및_임시파일_복구제안` | 녹화 유지 상태로 `Mode::Draw` 진입·종료, 시작 시 `rec-*.mp4` 발견 → 복구 섬 |
| Windows 실기 (S7) | 가려진 창 F2 + DRM 창(Netflix/보호 창) F2 | 가려진 부분까지 선명, 노란 테두리 0 px, 보호 창 → 안내 섬·클립보드 미변경 |
| Windows 실기 (S13) | 1080p 30 fps 10분 연속 녹화 + 0.3초 즉시 중지 + 녹화 중 판서 | Windows 기본 미디어 플레이어 정상 재생, 영상에 판서만 보이고 섬·빨간 꺾쇠 0 px |

---

## 5. 위험 · 대응

| 위험 | 발생 신호 | 대응 |
|---|---|---|
| **구형 학교 PC에서 `SetIsBorderRequired(false)` 미지원** (Win10 2004 이하) | WGC 결과에 노란 테두리가 찍힘 | API 호출 실패(HRESULT) 감지 즉시 `PrintWindow(PW_RENDERFULLCONTENT)` 대체 경로로 폴백 |
| **WGC `CreateForWindow`가 특정 Electron/UWP 창에서 빈 프레임** | `is_all_black` 참인데 보호 창이 아님 | WGC가 전 픽셀 검정을 주면 1회 `PrintWindow(PW_RENDERFULLCONTENT)` 재시도 → 그래도 검정일 때만 보호 콘텐츠로 판정 |
| **WASAPI 마이크와 WGC 비디오 타임스탬프 드리프트** | 10분 녹화 끝에서 음성 싱크 밀림 | 비디오·오디오 모두 단일 단조 시계(`QPC` / `Instant`) 기준으로 PTS 부여, 마이크 미지원·오류 시 비디오만 계속 녹화하고 섬 1회 안내 |
| **하드웨어 H.264 인코더 드라이버 크래시** | 일부 Intel 내장 그래픽에서 `MediaTranscoder` 실패 | `HardwareAccelerationEnabled(true)` 실패 시 `false`(소프트웨어 인코더)로 자동 1회 재시도, 둘 다 실패 시 `"녹화를 시작할 수 없어요(코덱)"` 안내 |

---

## 6. 작업일 · 난이도 · 먼저 끝나야 할 것

| 항목 | 작업일 | 난이도 | 선행 |
|---|---:|---|---|
| `6.1` F2 WGC 창 캡처·대체 경로·검은 화면 판정 | 0.60일 | 상 | 단계 3e (끝남) |
| `6.2` 바로 캡처 키 4종 | 0.60일 | 하 | `6.1`(Shift+F2 활성 창) |
| `6.3` F3 범위 + `Recorder` | 0.60일 | 상 | 단계 3e (끝남) |
| `6.4` 녹화 섬·빨간 꺾쇠·트레이 빨간 점 | 0.60일 | 중 | `6.3`, 판서 겹침 확인은 **A** |
| `6.5` 자동 중지·임시 파일 복구·마이크 | 0.60일 | 상 | `6.3` |
| **묶음 합계** | **3.00일** | **상** | 단계 3e 완료(끝남), 녹화+판서 확인은 **A** |
