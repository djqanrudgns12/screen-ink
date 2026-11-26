# screen-ink 기술 가능성 스파이크 보고서 (2026-11-26)

이 PC에서 Rust로 실제 컴파일하고 실행해 확인한 사실만 적었다. 설계안은 다루지 않는다.

- **PC:** Windows 11 Education 25H2(10.0.26200) · i5-13500 · Intel UHD Graphics 770(드라이버 32.0.101.6129) · 모니터 2대(1920×1080 @60Hz, 둘 다 100%)
- **도구:** rustc 1.94.1(stable-x86_64-pc-windows-msvc) · `windows` 0.62.2 · `windows-sys` 0.59.0 · `uiautomation` 0.25.3 · `windows-capture` 2.0.1
- **위치:** 모든 시험 프로젝트는 `D:\dev-cache\screen-ink\spikes\<이름>`에 있다. 따로 된 cargo 프로젝트이고, 빌드 폴더는 하나(`spikes\target`)를 같이 쓴다.
- **지킨 것**
  - 화면에 보이는 창은 만들지 않았다. 시험용 창은 모두 DWM 가림(cloak), 화면 밖 배치, 숨김 중 하나였다.
  - 마우스·키보드를 움직이지 않았다.
  - 캡처한 화소는 디스크에 쓰지 않았다. 크기, 시간, 검은지 여부만 기록했다.
  - 설치, 서명, 인증서, 레지스트리 쓰기를 하지 않았다. 레지스트리는 정책 키 몇 개를 **읽기만** 했다.
  - 저장소는 건드리지 않았다. 아이콘 PNG와 release exe를 spikes 폴더로 **복사만** 했다.

| 프로젝트 | 항목 | 다시 실행 |
|---|---|---|
| `wgc-raw` | 6.1 WGC | `target\release\wgc-raw.exe`(자기 창만) · `--skip-own --app`(보이는 앱 창 하나, 통계만) · `--foreign`(작업 표시줄) |
| `wc-encode` | 6.3 windows-capture 인코더 | `wc-encode.exe 5 fast` · `wc-encode.exe 10 paced` |
| `mf-encode` | 6.3 Media Foundation 직접 | `mf-encode.exe gpu 10 paced` · `mf-encode.exe cpu 5 fast` |
| `magnify` | 5.1 Magnification | `magnify.exe` · `magnify.exe --cloaked` |
| `msix` | D2/D3 | `layout\`(exe + Assets + AppxManifest.xml) → `out\screen-ink_0.1.0.0_x64.msix` |
| `updates` | D10 | `updates.exe` |
| `msaa` | 3b-msaa | `msaa.exe`(지금 커서 점) |
| `dpi` | 3b-dpi | `dpi.exe` · `dpi.exe --all` |

빌드는 PowerShell에서 `$env:CARGO_TARGET_DIR='D:\dev-cache\screen-ink\spikes\target'; cargo build --release`로 한다.

`--never` 인자는 컴파일 확인용으로만 넣어 둔 경로다. 설치·업데이트·전체 화면 확대처럼 실제로 부르면 안 되는 호출을 묶어 두었으니 **실행하지 말 것.**

---

## 1. 6.1 F2 — WGC로 HWND 하나 → BGRA 프레임

### 결론: 된다

- 테두리 끄기, 첫 프레임 시간 측정, 검은 프레임 판정을 모두 이 PC에서 실제로 해 보았다.
- 별도 crate 없이 `windows` 0.62만으로 충분하다.

### 쓸 API·crate·features

`windows = "0.62"`의 features:

    "Foundation", "Foundation_Metadata", "Graphics", "Graphics_Capture", "Graphics_DirectX",
    "Graphics_DirectX_Direct3D11", "Security_Authorization_AppCapabilityAccess", "UI",
    "Win32_Foundation", "Win32_Graphics_Direct3D", "Win32_Graphics_Direct3D11", "Win32_Graphics_Dwm",
    "Win32_Graphics_Dxgi", "Win32_Graphics_Dxgi_Common", "Win32_Graphics_Gdi",
    "Win32_System_LibraryLoader", "Win32_System_WinRT", "Win32_System_WinRT_Direct3D11",
    "Win32_System_WinRT_Graphics_Capture", "Win32_UI_HiDpi", "Win32_UI_WindowsAndMessaging"

호출 흐름:
1. `IGraphicsCaptureItemInterop::CreateForWindow`
2. `Direct3D11CaptureFramePool::CreateFreeThreaded(B8G8R8A8UIntNormalized, 2, size)`
3. `CreateCaptureSession`
4. `SetIsBorderRequired(false)` · `SetIsCursorCaptureEnabled(false)`
5. `StartCapture`
6. `TryGetNextFrame`
7. `IDirect3DDxgiInterfaceAccess::GetInterface::<ID3D11Texture2D>()`
8. staging 텍스처로 `CopyResource` → `Map`

테두리 권한은 `AppCapability::Create("graphicsCaptureWithoutBorder").CheckAccess()`로 확인한다. 사용자에게 묻지 않는 호출이다.

### 최소 코드 (컴파일·실행 확인, `wgc-raw/src/main.rs`)

    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd)? };
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &wdev, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, item.Size()?)?;
    let session = pool.CreateCaptureSession(&item)?;
    session.SetIsBorderRequired(false)?;          // Win10 2104(20348)+
    let border = session.IsBorderRequired()?;     // false로 읽혀도 증거는 아니다(함정 참고)
    session.SetIsCursorCaptureEnabled(false)?;
    session.StartCapture()?;
    let frame = loop { if let Ok(f) = pool.TryGetNextFrame() { break f; } /* 메시지 펌프 + 250µs 대기, 시간 제한 */ };
    let tex: ID3D11Texture2D = unsafe { frame.Surface()?.cast::<IDirect3DDxgiInterfaceAccess>()?.GetInterface()? };
    // desc.Usage=STAGING, CPUAccessFlags=READ → CreateTexture2D → CopyResource → Map → 표본 검사

장치는 `D3D11CreateDevice(HARDWARE, BGRA_SUPPORT)`로 만든다. `IDXGIDevice`를 `CreateDirect3D11DeviceFromDXGIDevice`에 넘기고 결과를 `.cast::<IDirect3DDevice>()`로 바꾼다.

### 실측 수치

**권한(패키지 없이 실행):** 이 PC에서는 모두 막힘 없이 된다.
- `GraphicsCaptureSession::IsSupported` = true
- `IsBorderRequired`·`IsCursorCaptureEnabled`·`MinUpdateInterval`·`IncludeSecondaryWindows` 속성이 모두 있다.
- `CheckAccess(graphicsCaptureWithoutBorder)` = **4(Allowed)** · `graphicsCaptureProgrammatic` = 4
- `SetIsBorderRequired(false)` 성공 → 다시 읽으면 false

**보이는 남의 앱 창(Chromium 계열, 1920×1032)**

새 프로세스 5개 × 3회 = 15회 기준.

| 단계 | 시간 |
|---|---|
| D3D11 장치 + WinRT 래핑 | 40~75 ms(드물게 195 ms). 프로세스당 1회 |
| `CreateForWindow` | 1.7~2.5 ms |
| 준비(item + pool + session + 테두리 설정) | 11~18 ms |
| **첫 프레임**(StartCapture 기준) | **7~19 ms** |
| GPU→CPU 읽기 + 통계(1920×1032) | 2.5~4.4 ms |
| **합계(장치가 있을 때)** | **≈ 25~40 ms** |

이상치도 있었다.
- 다른 창(1906×1025)을 잡은 회차에서 첫 프레임이 **299 ms·307 ms·949 ms**였고, 한 번은 **1500 ms 안에 프레임이 오지 않았다.**
- 같은 회차에서 GPU 읽기가 707 ms인 적도 있었다.
- 작업 표시줄(`Shell_TrayWnd`, 1920×48)은 첫 시도에 1.5초 동안 프레임이 없었고, 두 번째는 710 ms였다. 첫 남의 창 캡처는 차가운 비용(CreateForWindow 171 ms + Session 180 ms)이 붙었다.

**우리 프로세스의 창(같은 스레드가 만든 창)**
- `CreateForWindow`가 매번 **38~48 ms**로 남의 창(2 ms)보다 20배 느렸다.
- 추정 원인: 대상 창 스레드가 호출 스레드 자신이라 그 스레드의 메시지 처리를 기다린다. [확인 필요]
- `TryCreateFromWindowId`(UI feature)는 55~277 ms로 느렸다 → interop 경로를 쓴다.

**검은 프레임(보호 화면) 판정 — 우리 창으로 재현**

| 대상 | 프레임 | 화소 |
|---|---|---|
| DWM 가린 창(보통) | 7~13 ms에 옴 | 정상 내용(최대 RGB 204, alpha 255) → **가린 창·다른 가상 데스크톱 창도 캡처된다** |
| 가린 창 + `WDA_MONITOR` | 12 ms에 옴 | **RGB 전부 0, alpha 255** |
| 가린 창 + `WDA_EXCLUDEFROMCAPTURE` | 17 ms에 옴 | **RGB 전부 0, alpha 255** |
| 가상 화면 밖 창(보통) | 9~18 ms에 옴 | **RGB 0, alpha 0**(그려진 적 없는 표면) |
| 가상 화면 밖 창 + 표시 선호도 | **1.5초 동안 안 옴** | — |

`GetWindowDisplayAffinity(남의 창)`도 성공한다. 다른 프로세스 창을 읽었을 때 결과는 0이었다. 남의 창이 보호를 걸었을 때 0이 아닌 값이 나오는지는 [확인 필요].

### 함정

1. **테두리를 정말 껐는지 API만으로는 증명할 수 없다.**
   - 문서: 사용자가 거부한 경우에도 `false` 설정은 **성공하고, 값은 무시되어 테두리가 그려진다**.
   - 다른 앱이 같은 창에 true를 걸어도 테두리가 나온다.
   - 판정은 `AppCapability::CheckAccess(graphicsCaptureWithoutBorder) == Allowed`로 해야 한다. 이 PC에서는 패키지 없이 실행하면 Allowed였다.
2. **MSIX 패키지가 되면 조건이 바뀐다.**
   - 매니페스트에 `uap11:Capability graphicsCaptureWithoutBorder`를 넣어야 한다.
   - 그 위에 `GraphicsCaptureAccess::RequestAccessAsync(Borderless)`를 불러야 하는데, 문서상 **사용자 동의 창이 뜬다.**
   - 이번 스파이크에서는 창이 뜰 수 있어 이 호출을 컴파일만 했다.
3. **첫 프레임은 보장되지 않는다.**
   - 대부분 20 ms 안이지만 0.3~1초 걸리거나 오지 않는 창이 있었다(작업 표시줄, 일부 Chromium 창).
   - 반드시 시간 제한을 두고 대체 경로로 넘어가야 한다.
4. **화면 밖에 있는 창은 alpha 0의 빈 프레임을 준다**(검은 화면과 다르다). 판정할 때 alpha도 본다.
5. **검은 프레임 판정은 "RGB가 정확히 0"으로 한다.**
   - 문턱값(≤16)으로 하면 Windows Terminal 같은 어두운 배경 앱(약 12,12,12)이 잘못 걸린다.
   - 보호 화면은 정확히 (0,0,0,255)였다.
6. **우리 창을 캡처할 때(핀·섬 등)는 그 창 스레드에서 `CreateForWindow`를 부르지 않는다**(+40 ms).
7. `windows` 0.62에서 `BOOL`은 `windows::core::BOOL`이다. `Win32::Foundation::BOOL`이 아니다.
8. **windows-capture 2.0.1은 edition 2024 crate다.**
   - let-chains를 쓰므로 Rust 1.88 이상이 필요하다. 저장소 `rust-version = "1.80"`과 맞지 않는다.
   - `DrawBorderSettings::WithoutBorder`는 있지만 `RequestAccessAsync`는 부르지 않는다.
   - `start()` 때마다 D3D 장치를 새로 만들고, 호출 스레드를 DispatcherQueue 루프로 가져간다.
   - 컴파일 확인: `wc-encode/src/capture_probe.rs`

### 추천 구현 방식

1. `windows` 0.62로 직접 짠다.
2. D3D11 장치는 앱 시작 때(또는 F2 첫 진입 때) 1회 만들어 재사용한다.
3. F2 고르기 중 커서 아래 창이 정해지면 item·pool·session을 미리 만들어 둔다(≈15 ms를 숨김).
4. 확정하면 `StartCapture` → 첫 프레임을 **150 ms** 기다린다.
5. 오지 않거나 RGB가 전부 0이면 순서대로 판단한다.
   1. `GetWindowDisplayAffinity`가 0이 아니면 보호 화면이다.
   2. 아니면 대체 경로(정지 스냅숏에서 창 사각형 자르기)로 간다.
   3. 대체 결과도 0이면 보호 화면 안내를 띄운다.
6. 패키지판은 첫 실행 때 `RequestAccessAsync(Borderless)`를 부른다.

### [확인 필요]

- 테두리가 실제로 안 그려지는지 **눈으로** 확인해야 한다. 사용자가 보는 앞에서 보이는 창 하나로 시험한다.
  - 이번에는 화면을 볼 수 없었다.
  - 근거는 `CheckAccess=Allowed`와 다시 읽은 값뿐이다.
- MSIX로 설치했을 때 `RequestAccessAsync(Borderless)`의 동의 창 문구와 거부 시 동작
- 학교 PC MDM의 개인정보 정책(`LetAppsAccessGraphicsCaptureWithoutBorder` 계열)이 패키지판에 걸리는지
- DRM 영상(Edge·넷플릭스 등 보호 스왑체인)도 (0,0,0,255)로 오는지

---

## 2. 6.3 Recorder — 1080p30 하드웨어 H.264

### 결론: 된다 — Media Foundation을 직접 쓰는 쪽을 권한다

- 이 PC에는 Intel Quick Sync 하드웨어 인코더가 있다. 싱크 라이터가 실제로 그 인코더를 골랐다(`hardware_url=true`).
- D3D11 텍스처(WGC 프레임과 같은 모양)를 넣으면 30 fps 실시간에 CPU가 한 코어의 9~12%만 든다.
- 두 방식 모두 이 PC에서 빌드되고 재생 가능한 MP4가 나온다.

### 이 PC의 인코더 (`MFTEnumEx`)

- H.264 하드웨어: **Intel® Quick Sync Video H.264 Encoder MFT** — 2개 항목, `MFT_ENUM_FLAG_ASYNCMFT`(0x4)
- HEVC 하드웨어: Intel® Hardware H265 Encoder MFT — 2개 항목
- H.264 소프트웨어: H264 Encoder MFT — 1개
- 열거에 92 ms 걸렸다.

### 쓸 API·crate·features

**방식 B — 권장 (`mf-encode`)**

`windows = "0.62"`의 features:

    "Win32_Foundation", "Win32_Graphics_Direct3D", "Win32_Graphics_Direct3D10", "Win32_Graphics_Direct3D11",
    "Win32_Graphics_Dxgi", "Win32_Graphics_Dxgi_Common", "Win32_Media_MediaFoundation",
    "Win32_System_Com", "Win32_System_ProcessStatus", "Win32_System_Threading"

**방식 A (`wc-encode`)**
- `windows-capture = "=2.0.1"`의 `encoder::VideoEncoder`
- 내부는 WinRT `MediaTranscoder` + `MediaStreamSource`이고, `SetHardwareAccelerationEnabled(true)`를 켠다.

### 최소 코드 (방식 B, 컴파일·실행 확인)

    MFStartup(MF_VERSION, MFSTARTUP_FULL)?;
    // D3D11 장치: BGRA_SUPPORT | VIDEO_SUPPORT, 그리고 ID3D10Multithread::SetMultithreadProtected(true)
    MFCreateDXGIDeviceManager(&mut token, &mut mgr)?; mgr.ResetDevice(&dev, token)?;
    MFCreateAttributes(&mut attrs, 4)?;
    attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;
    attrs.SetUnknown(&MF_SINK_WRITER_D3D_MANAGER, &mgr)?;
    let writer = MFCreateSinkWriterFromURL(&HSTRING::from(path), None, &attrs)?;
    // 출력: MFVideoFormat_H264 + MF_MT_AVG_BITRATE / FRAME_SIZE / FRAME_RATE / INTERLACE / PAR
    // 입력: MFVideoFormat_RGB32 (+ MF_MT_DEFAULT_STRIDE = W*4 → 위에서 아래로)
    let stream = writer.AddStream(&out_t)?; writer.SetInputMediaType(stream, &in_t, None)?; writer.BeginWriting()?;
    // 프레임마다:
    let buf = MFCreateDXGISurfaceBuffer(&ID3D11Texture2D::IID, &tex, 0, false)?;
    buf.SetCurrentLength(buf.cast::<IMF2DBuffer>()?.GetContiguousLength()?)?;
    let s = MFCreateSample()?; s.AddBuffer(&buf)?; s.SetSampleTime(t)?; s.SetSampleDuration(333_333)?;
    writer.WriteSample(stream, &s)?;
    // 끝: writer.Finalize()?
    // 고른 MFT 확인: writer.cast::<IMFSinkWriterEx>()?.GetTransformForStream(stream, i, Some(&mut cat), &mut tr)

실제로 끼워진 변환 사슬은 둘이었다.
1. video-processor — `d3d11_aware=1`, BGRA→NV12를 GPU에서 변환
2. encoder — `hardware_url=true`

### 실측 수치 (합성 1080p30 BGRA, 목표 8 Mbps)

| 경로 | 준비 | 최대 처리량 | 실시간 30 fps에서 CPU | 실시간 끝맺음 지연 | 실제 비트레이트 |
|---|---|---|---|---|---|
| **MF + D3D11 텍스처** | 390~445 ms | 341~456 fps | 10초 12% · **60초 9%**(한 코어 = 100%) | 10초 0.9 ms · **60초 2.9 ms** | **13~14 Mbps(초과)** |
| MF + CPU 버퍼(RGB32) | 355~675 ms | 347 fps | 72% | 1.2 ms | 8.2~8.7 Mbps |
| windows-capture `send_frame_buffer` | **0.55~1.35 s**(첫 실행은 **12.75 s**) | 59~60 fps | 23% | 2초 0.33 s · **10초 2.9 s(밀림)** | 6.8~8.2 Mbps |

- 60초 실시간(MF + 텍스처, 1800프레임): `WriteSample`은 최악 0.2 ms였다. 최대 작업 집합은 628 MB였는데, 합성 원본 30장(≈237 MB)과 iGPU 공유 메모리의 텍스처 30장이 포함된 값이다.
- 만든 MP4 10개를 Windows 셸 속성으로 모두 확인했다.
  - 길이(1·2·5·10·20초), 1920×1080, 30 fps, FourCC `H264`가 모두 맞았다.
  - 합성 영상 몇 개는 `spikes\out\`에 남겨 두었다. 기본 플레이어 재생 확인용이다.
- **의존성 무게**

  | 프로젝트 | crate 수 | exe 크기 |
  |---|---|---|
  | `mf-encode` | 20(windows 계열만) | 204 KB |
  | `wgc-raw` | 20 | 219 KB |
  | `wc-encode` | 38(rayon·crossbeam·parking_lot·thiserror·syn 등 추가) | 386 KB |

  - `wc-encode` 첫 빌드는 2분 34초 걸렸다. windows-capture가 `windows`를 feature 약 30개로 다시 빌드하기 때문이다.

### 함정

1. **windows-capture `VideoEncoder::new`가 0.5~1.3초 걸린다**(첫 실행 12.8초). F3 직후 바로 시작하려면 미리 만들어 두거나, 앞 프레임을 버퍼링해야 한다.
2. **windows-capture 실시간 입력은 10초에 2.9초가 밀렸다**(원인 미확인, MediaStreamSource 요청 방식으로 추정).
   - 채널이 무제한 `mpsc`이고 CPU 경로는 프레임마다 8 MB를 복사한다(`to_vec`).
   - 밀리면 메모리가 계속 는다. 10분 녹화에는 위험하다.
3. **MF D3D 경로는 `MF_MT_AVG_BITRATE`만으로는 목표 비트레이트를 지키지 않았다**(8 → 13~14 Mbps).
   - `ICodecAPI`(`CODECAPI_AVEncCommonRateControlMode`·`MeanBitRate`)로 직접 지정해야 한다. [확인 필요]
4. **MF D3D 경로에서 `WriteSample`은 바로 돌아온다**(비동기).
   - WGC 프레임 풀 텍스처를 그대로 넘기면 풀이 그 텍스처를 다시 쓴다.
   - 우리 텍스처 링(예: 8장)에 `CopyResource`한 뒤 넘겨야 한다.
5. 장치를 MF와 같이 쓰려면 `D3D11_CREATE_DEVICE_VIDEO_SUPPORT`와 `ID3D10Multithread::SetMultithreadProtected(true)`가 필요하다.
6. MF 준비에 0.35~0.7초가 든다 → 녹화 시작 전에 미리 만들어 둔다.

### 추천 구현 방식

1. `Recorder` 어댑터는 MF `IMFSinkWriter`를 직접 쓴다.
2. 텍스처 입력에 `MF_SINK_WRITER_D3D_MANAGER`와 하드웨어 변환을 켠다.
3. WGC와 **같은 D3D11 장치**를 쓴다. WGC 프레임 → 우리 텍스처 링에 복사 → `MFCreateDXGISurfaceBuffer` 순서다.
4. 범위 녹화는 `CopySubresourceRegion`으로 자른다. 홀수 크기는 짝수로 맞춘다.
5. 비트레이트는 `ICodecAPI`로 지정한다.
6. 시작할 때 하드웨어 MFT가 있는지 확인해(`MFTEnumEx` HARDWARE) 상태 섬·로그에 남긴다.
7. 아키텍처 문서의 "windows-capture `VideoEncoder`"는 MF 직접 사용으로 바꾸는 것을 검토한다.

### [확인 필요]

- 실제 WGC 프레임 + 10분 연속 녹화(이번에는 합성 60초까지만 했다)
- `ICodecAPI`로 비트레이트를 지정했을 때 지켜지는지
- 혼합 해상도에서 프레임 크기가 바뀔 때 처리
- 하드웨어 인코더가 없는 PC에서 소프트웨어 MFT로 넘어가는 경로

---

## 3. 5.1 Magnification API

### 결론: 컴파일되고, 숨긴 창으로 호출이 모두 성공한다

화면에 돋보기를 보이지 않고 확인했다. 숨긴 창과 DWM 가린 창 두 가지로만 실행했다.

### 쓸 API·crate·features

`windows-sys = "0.59"`(저장소 어댑터와 같은 판)의 features:

    "Win32_UI_Magnification", "Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_System_LibraryLoader",
    "Win32_Graphics_Gdi"   # MagImageScalingCallback 서명에 필요
    "Win32_Graphics_Dwm"   # 시험용 가림에만

- 함수: `MagInitialize`/`MagUninitialize`, 자식 창 클래스 `WC_MAGNIFIER`("Magnifier"), `MagSetWindowTransform`, `MagSetWindowSource`, `MagSetWindowFilterList`(EXCLUDE), `MagGetWindowFilterList`, `MagGetFullscreenTransform`
- 컴파일만 함: `MagSetFullscreenTransform`, `MagSetInputTransform`, `MagShowSystemCursor`
- DLL은 `magnification.dll`이다.

### 최소 코드 (컴파일·실행 확인, `magnify/src/main.rs`)

    MagInitialize();
    let host = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                               w!("InkSpikeMagHost"), w!(""), WS_POPUP, 0, 0, 800, 600, null_mut(), null_mut(), inst, null());
    let mag = CreateWindowExW(0, WC_MAGNIFIER, w!("MagnifierWindow"), WS_CHILD | WS_VISIBLE | MS_SHOWMAGNIFIEDCURSOR as u32,
                              0, 0, 800, 600, host, null_mut(), inst, null());
    let mut m = MAGTRANSFORM { v: [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0] };
    MagSetWindowTransform(mag, &mut m);
    MagSetWindowSource(mag, RECT { left: 100, top: 100, right: 500, bottom: 400 });
    let mut list = [island_hwnd /* …제외할 우리 창들 */];
    MagSetWindowFilterList(mag, MW_FILTERMODE_EXCLUDE, list.len() as i32, list.as_mut_ptr());
    // 끝: DestroyWindow(mag); DestroyWindow(host); MagUninitialize();

### 실측 수치

| 호출 | 결과 |
|---|---|
| `MagInitialize` | 1.0~1.2 ms |
| `CreateWindowExW(WC_MAGNIFIER)` | 첫 실행 **862 ms**, 이후 142~148 ms |
| `MagSetWindowTransform` | 성공, 다시 읽은 값이 같음 |
| `MagSetWindowSource` | **호출 1번이 약 16.6 ms 동안 막힌다.** 60회 = 995~999 ms. 숨긴 창이든 가린 창이든 같다 → 화면 주사율(60 Hz)에 묶여 동기로 그린다 |
| `MagSetWindowFilterList(EXCLUDE, [host])` | 성공, 다시 읽으면 개수 1 · mode 0 |

### 함정

1. **`MagSetWindowSource`가 프레임 하나만큼 막는다.**
   - 커서 따라가기 루프를 Tauri UI 스레드에서 돌리면 UI가 60 fps에 묶인다.
   - 돋보기 호스트 창은 **전용 스레드**(창을 만든 스레드 = 호출 스레드)에 둔다.
2. 첫 `WC_MAGNIFIER` 창 생성은 약 0.9초 → Alt+2 첫 진입 전에 미리 만들어 둔다(숨김 상태로).
3. `MW_FILTERMODE_INCLUDE`는 Windows 7부터 지원하지 않는다(문서). EXCLUDE만 쓴다.
   - 돋보기 창 자신은 자동으로 빠진다.
   - WDDM 그래픽 드라이버가 필요하다.
4. 문서에 따르면 `SetWindowDisplayAffinity`는 DWM이 합성할 때만 동작한다. 하지만 돋보기 원본에서 `WDA_EXCLUDEFROMCAPTURE` 창이 빠지는지는 문서에 없다 → 섬·지도는 필터 목록에 **명시적으로** 넣는다. [확인 필요]
5. 아래는 배경 지식이고 이번에 실측하지 않았다.
   - 32비트 프로세스(WOW64)에서는 Magnification API를 지원하지 않는다 → x64·arm64 빌드만 낸다.
   - `MagSetImageScalingCallback`은 DWM 합성에서 동작하지 않는다(폐기).
   - `MagSetInputTransform`은 UIAccess 권한이 필요하다. [확인 필요]
6. 4K 측정은 이 PC가 1080p 두 대라 하지 못했다.
   - 막히는 호출이 주사율에 묶여 있으므로 4K에서는 그리기 비용이 프레임 예산을 넘는지가 관건이다.
   - 07 계획 5.4(4K 30 fps 미달 시 WGC→GPU 2안)의 판단은 4K PC에서 해야 한다.

### 추천 구현 방식

1. 전용 스레드에서 호스트 창을 만든다. 스타일은 `WS_EX_LAYERED|TRANSPARENT|TOPMOST|TOOLWINDOW|NOACTIVATE`에 `SetLayeredWindowAttributes(255)`를 건다.
2. 그 위에 `WC_MAGNIFIER` 자식을 둔다.
3. 루프에서는 `MagSetWindowSource`가 vsync 대기 역할까지 하게 한다(따로 sleep하지 않음).
4. 필터 목록에는 우리 오버레이 창 전부를 넣는다(섬·후광·MiniMap).

### [확인 필요]

- 4K(또는 혼합 배율) PC에서 원본 1/2배율의 fps
- `WDA_EXCLUDEFROMCAPTURE` 창이 자동으로 빠지는지
- 영상 재생 중(S9) 끊김
- 가린 창 안에서 돋보기가 실제로 화소를 그리는지(이번에는 타이밍만 봤다)

---

## 4. D2/D3 MSIX — 도구와 서명 없는 포장

### 결론: 된다

- 이 PC에 Windows SDK 10.0.26100.0 도구가 있다.
- 손으로 쓴 매니페스트(데스크톱 full trust + `windows.startupTask`)로 **서명 없는 .msix 포장에 성공했다.**
- 설치·서명은 하지 않았다.

### 도구 위치 (`C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\`)

- `x64\makeappx.exe`(10.0.26100.7705) · `x64\signtool.exe` · `x64\makepri.exe` · `x64\mt.exe`
- x86·arm64 폴더에도 같은 도구가 있다.
- 더 오래된 SDK 폴더(14393~17134)에는 이 도구들이 없다.

### 매니페스트 (`msix\layout\AppxManifest.xml`, makeappx 의미 검사 통과)

    <Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
      xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
      xmlns:uap11="http://schemas.microsoft.com/appx/manifest/uap/windows10/11"
      xmlns:desktop="http://schemas.microsoft.com/appx/manifest/desktop/windows10"
      xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
      IgnorableNamespaces="uap uap11 desktop rescap">
      <Identity Name="kr.screenink.desktop" Publisher="CN=screen-ink spike" Version="0.1.0.0" ProcessorArchitecture="x64" />
      <Properties><DisplayName>screen-ink</DisplayName><PublisherDisplayName>screen-ink</PublisherDisplayName>
        <Logo>Assets\StoreLogo.png</Logo></Properties>
      <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" /></Dependencies>
      <Resources><Resource Language="ko-KR" /><Resource Language="en-US" /></Resources>
      <Applications>
        <Application Id="ScreenInk" Executable="screen-ink.exe" EntryPoint="Windows.FullTrustApplication">
          <uap:VisualElements DisplayName="screen-ink" Description="screen-ink" BackgroundColor="transparent"
            Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png" />
          <Extensions>
            <desktop:Extension Category="windows.startupTask" Executable="screen-ink.exe" EntryPoint="Windows.FullTrustApplication">
              <desktop:StartupTask TaskId="ScreenInkStartup" Enabled="false" DisplayName="screen-ink" />
            </desktop:Extension>
          </Extensions>
        </Application>
      </Applications>
      <Capabilities>
        <uap11:Capability Name="graphicsCaptureWithoutBorder" />
        <uap11:Capability Name="graphicsCaptureProgrammatic" />
        <rescap:Capability Name="runFullTrust" />
      </Capabilities>
    </Package>

포장 명령:

    makeappx pack /d D:\dev-cache\screen-ink\spikes\msix\layout /p D:\dev-cache\screen-ink\spikes\msix\out\screen-ink_0.1.0.0_x64.msix /o

### 실측 수치

- "Package creation succeeded" · **812 ms**
- `.msix` **10,843,231 B**(exe 18,580,992 B + 로고 5개가 압축됨)
- `/nv`를 쓰지 않았으므로 의미 검사가 켜져 있다.
  - 시험 삼아 `Square44x44Logo.png`를 빼 보니 즉시 거부했다. 오류는 `0x80080204 … declared … doesn't exist in the package`였다.
  - 기능 순서를 바꿔(`rescap` 먼저) 넣어도 통과했다 → 순서 제약은 없다.
- 학교 PC 정책을 **읽기만** 했다.

  | 키·패키지 | 값 |
  |---|---|
  | `HKLM\SOFTWARE\Policies\Microsoft\Windows\Appx` | `MdmHosts` 값만 있다(MDM 관리 PC로 보임) |
  | `…\Windows\AppInstaller`, `…\WindowsStore`(HKLM·HKCU) | 키 없음 |
  | 앱 설치 관리자 `Microsoft.DesktopAppInstaller` | 1.29.380.0 |
  | `Microsoft.WindowsStore` | 22610.1401.10.0 |

### 함정

1. 서명 없는 MSIX는 **설치가 되지 않는다**(11-distribution §6). 스토어 제출용으로만 쓴다. 로컬 설치 시험에는 자체 서명과 신뢰 등록이 필요하다(이번에는 금지라 하지 않음).
2. `Publisher="CN=…"`는 스토어판이면 Partner Center 값으로, 직접 배포판이면 인증서 주체와 **글자 그대로 같게** 바꿔야 한다.
3. `resources.pri`를 만들지 않았다. 문자열을 매니페스트에 바로 썼다. 스토어 제출 검사에서 경고나 거부가 나는지는 [확인 필요]. 필요하면 `makepri.exe`가 같은 폴더에 있다.
4. `desktop:StartupTask Enabled="false"`로 두고, 앱이 `StartupTask::RequestEnableAsync`로 켠다(§5 API). 처음부터 true로 해도 사용자가 설정에서 끌 수 있다.
5. `graphicsCaptureWithoutBorder`·`graphicsCaptureProgrammatic`은 `uap11` 네임스페이스로 검사를 통과했다. 이걸 선언해도 §1의 `RequestAccessAsync`가 따로 필요하다.

### 추천 구현 방식

1. 릴리스 스크립트(D9)에서 `layout\`을 만든다. 구성은 release exe, Assets(Tauri `icons\Square*Logo.png`·`StoreLogo.png`), 템플릿 매니페스트(버전·Publisher 치환)다.
2. `makeappx pack /o`로 포장한다(`/nv`는 쓰지 않는다).
3. SDK 경로는 `C:\Program Files (x86)\Windows Kits\10\bin\<최신>\x64\`에서 찾는다.

### [확인 필요]

- 개발 PC에서 자체 서명 설치 후 실행 시험(전역 키·WGC·UIA·`WDA_EXCLUDEFROMCAPTURE`가 패키지 안에서도 같은지)
- MDM이 CSP로 MSIX 설치를 막는지
- Partner Center 사전 검사(WACK) 결과

---

## 5. D10 업데이트·다시 켜기 API

### 결론: 모두 `windows` 0.62로 컴파일된다

- 패키지 없이 실행하면 판정 API가 아래 오류 코드로 빠르게 실패한다. "패키지로 실행 중인가" 분기는 확실하게 할 수 있다.
- 내려받기·설치 호출은 하지 않았다(컴파일만).

### 쓸 API·crate·features

| API | feature |
|---|---|
| `Windows.ApplicationModel.Package::Current` · `CheckUpdateAvailabilityAsync` · `GetAppInstallerInfo` | `"ApplicationModel"`(+`"Foundation"`) |
| `Windows.ApplicationModel.StartupTask::GetAsync` · `RequestEnableAsync` · `Disable` | `"ApplicationModel"` |
| `Windows.Services.Store.StoreContext::GetDefault` · `CanSilentlyDownloadStorePackageUpdates` · `GetAppAndOptionalStorePackageUpdatesAsync` · `TrySilentDownloadStorePackageUpdatesAsync` · `TrySilentDownloadAndInstallStorePackageUpdatesAsync` · `RequestDownloadAndInstallStorePackageUpdatesAsync` | `"Services_Store"`(+`"Foundation_Collections"` — `IVectorView`) |
| `Windows.Management.Deployment.PackageManager::AddPackageByAppInstallerFileAsync(&Uri, AddPackageByAppInstallerOptions::ForceTargetAppShutdown, None)` | `"Management_Deployment"`(+`"Foundation"` — `Uri`) |
| `RegisterApplicationRestart` · `UnregisterApplicationRestart` · `RESTART_NO_CRASH/NO_HANG/NO_REBOOT` | `"Win32_System_Recovery"` |
| `GetCurrentPackageFullName` | `"Win32_Storage_Packaging_Appx"` |
| `IInitializeWithWindow`(StoreContext 대화 상자를 창에 붙이기) | `"Win32_UI_Shell"` |

비동기는 `windows-future` 0.3의 `.join()`으로 기다린다(windows 0.62가 끌어온다).

### 최소 코드 (`updates/src/main.rs`)

    fn is_packaged() -> bool {               // 가장 싼 판정
        let mut len = 0u32;
        unsafe { GetCurrentPackageFullName(&mut len, None) } == ERROR_INSUFFICIENT_BUFFER   // 없으면 15700
    }
    let ctx = StoreContext::GetDefault()?;
    let ups = ctx.GetAppAndOptionalStorePackageUpdatesAsync()?.join()?;
    let dl  = ctx.TrySilentDownloadStorePackageUpdatesAsync(&ups)?.join()?;            // 미리 받기
    let ins = ctx.TrySilentDownloadAndInstallStorePackageUpdatesAsync(&ups)?.join()?;  // 적용
    let avail = Package::Current()?.CheckUpdateAvailabilityAsync()?.join()?;          // .appinstaller판
    let res = PackageManager::new()?.AddPackageByAppInstallerFileAsync(
        &Uri::CreateUri(h!("https://…/screen-ink.appinstaller"))?,
        AddPackageByAppInstallerOptions::ForceTargetAppShutdown, None)?.join()?;
    unsafe { RegisterApplicationRestart(&HSTRING::from("--restarted-after-update"),
                                        RESTART_NO_CRASH | RESTART_NO_HANG | RESTART_NO_REBOOT)? };

### 실측 — 패키지 없이 실행

| 호출 | 결과 |
|---|---|
| `GetCurrentPackageFullName` | rc = **15700** (`APPMODEL_ERROR_NO_PACKAGE`) |
| `Package::Current()` | **0x80073D54** "프로세스에 패키지 ID가 없습니다" |
| `StartupTask::GetAsync("ScreenInkStartup")` | **0x80070490**(`ERROR_NOT_FOUND`) · 0.5 ms — 0x80073D54가 **아니다** |
| `StoreContext::GetDefault()` | **성공**(78 ms) |
| `CanSilentlyDownloadStorePackageUpdates` | **true**(패키지 없어도 true — 판정에 쓰면 안 됨) |
| `GetAppAndOptionalStorePackageUpdatesAsync` | **0x803F6101** · 2.7 ms |
| `RegisterApplicationRestart` / `Unregister…` | 둘 다 성공(이 프로세스 한정 등록) |

전체 실행에 552 ms가 걸렸다.

### 함정

1. "스토어판인가"를 `StoreContext` 성공이나 `CanSilently…`로 판정하면 틀린다. 패키지 없이도 성공하고 true가 나온다.
   - 먼저 `GetCurrentPackageFullName`(또는 `Package::Current`)로 패키지인지 가른다.
   - 그다음 `Package::Current()?.SignatureKind()`(Store / Developer / Enterprise)로 스토어판과 직접 배포판을 가르는 것이 자연스럽다. [확인 필요]
2. `StartupTask` 오류 코드는 패키지 오류 코드와 다르다(0x80070490). 오류 코드 대신 앞의 패키지 판정으로 분기한다.
3. 데스크톱 앱에서 동의 창을 띄우는 Store API(`RequestDownloadAndInstall…`)를 쓰려면 `StoreContext`를 `IInitializeWithWindow::Initialize(hwnd)`로 우리 창에 붙여야 한다. 캐스트가 되는 것은 확인했다. silent 계열은 필요 없다.
4. `RegisterApplicationRestart`는 프로세스가 **60초 넘게** 실행되었을 때만 다시 켠다(문서상 조건, 실측하지 않음). 앱 시작 직후 1회 등록하면 된다.

### 추천 구현 방식

1. `Updater` 포트의 Windows 어댑터를 시작할 때 판정한다.
   1. `GetCurrentPackageFullName`으로 패키지인지 본다.
   2. 패키지면 `SignatureKind`로 Store판과 `.appinstaller`판을 가른다.
   3. 패키지가 아니면(NSIS·포터블) `Noop`이다.
2. Store판은 `GetAppAndOptional…`, `TrySilentDownload…`(미리 받기) 순서로 돌린다. 사용자가 [지금 업데이트]를 누르면 `TrySilentDownloadAndInstall…`을 부른다.
3. 직접 배포판은 `CheckUpdateAvailabilityAsync`로 확인한다. 누르면 `AddPackageByAppInstallerFileAsync(ForceTargetAppShutdown)`를 부른다.
4. 두 경로 모두 앱 시작 때 `RegisterApplicationRestart`를 등록해 둔다.

### [확인 필요]

- 0x803F6101의 정확한 의미(스토어 라이선스·식별 없음으로 추정)
- 실제 패키지(스토어 시험 비행·서명 `.appinstaller`)에서
  - `TrySilentDownload…`가 사용자 창 없이 끝나는지
  - `ForceTargetAppShutdown` 뒤 `RegisterApplicationRestart`로 다시 켜지는지
  - 트레이에 상주 중일 때 적용 시점
- 각 API의 최소 Windows 버전(`CheckUpdateAvailabilityAsync`·`AddPackageByAppInstallerFileAsync` 모두 Windows 10 1809 이전부터 있음으로 앎)

---

## 6. 3b-msaa — AccessibleObjectFromPoint / accLocation

### 결론: 된다

컴파일했고, 지금 커서 점에서 런타임 호출 1회(+따뜻한 10회)로 사각형을 받았다. 화면에는 아무것도 띄우지 않았다.

### 쓸 API·crate·features

`windows = "0.62"`의 features:

    "Win32_UI_Accessibility", "Win32_System_Variant", "Win32_System_Com", "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging", "Win32_UI_HiDpi", "Win32_Graphics_Gdi"

- 저장소 어댑터에는 이미 `Win32_UI_Accessibility`·`Win32_System_Com`이 있다. `Win32_System_Variant`만 더하면 된다.
- 함수: `AccessibleObjectFromPoint`, `IAccessible::accLocation`/`get_accRole`, `AccessibleObjectFromWindow(OBJID_WINDOW)`, `WindowFromAccessibleObject`

### 최소 코드 (`msaa/src/main.rs`)

    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
    CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    let mut acc: Option<IAccessible> = None;
    let mut child = VARIANT::default();                       // windows::Win32::System::Variant::VARIANT
    AccessibleObjectFromPoint(pt, &mut acc, &mut child)?;
    let acc = acc.unwrap();
    let (mut l, mut t, mut w, mut h) = (0, 0, 0, 0);
    acc.accLocation(&mut l, &mut t, &mut w, &mut h, &child)?;  // (left, top, width, height)
    let role = i32::try_from(&acc.get_accRole(&child)?).ok();  // VT_I4
    // 창 자체: AccessibleObjectFromWindow(hwnd, OBJID_WINDOW.0 as u32, &IAccessible::IID, &mut p) + VARIANT::from(0i32)

### 실측 수치

커서는 (3083, 76)에 있었다. 두 번째 모니터의 Chromium 계열 창이고, 이 창은 Per-Monitor 인식이다.

| 호출 | 결과 |
|---|---|
| `AccessibleObjectFromPoint` 첫 호출 | **13.65 ms** |
| `accLocation` | **0.05 ms** → (3030, 74, w 97, h 96) · role 16(`ROLE_SYSTEM_PANE`) · childId 0 |
| 따뜻한 10회 평균(From + Location) | **0.70 ms** |
| `OBJID_WINDOW` 사각형 | (3006, 58, 145×152) = `GetWindowRect`와 **정확히 같다**(물리 px) |
| 같은 점의 UIA `ElementFromPoint`(첫 호출) | 25.8 ms → (3030, 74)-(3127, 106) · ControlType 50011(MenuItem) |

**같은 점에서 MSAA는 더 바깥 요소(pane, 높이 96)를, UIA는 안쪽 메뉴 항목(높이 32)을 돌려주었다.** Chromium에서는 MSAA가 더 거칠다.

### 함정

1. `accName`에는 개인정보가 들어 있을 수 있다. 이번에는 읽지 않았다(역할·사각형만). 로그 규칙(R-LOG-1)과 맞추려면 이름을 로그에 남기지 않는다.
2. `accLocation`은 (l, t, **w, h**) 순서다. UIA는 (l, t, r, b)다.
3. 응답 없는 앱에서는 `AccessibleObjectFromPoint`가 오래 막힐 수 있다. 시간 제한이 없으므로 UIA 일꾼처럼 전용 스레드에서 부르고, 대상 창을 `SendMessageTimeout(WM_NULL, SMTO_ABORTIFHUNG, 50ms)`로 먼저 살핀다. 기존 `uia.rs`의 `responsive()`를 재사용하면 된다.
4. VARIANT는 0.62에서 `windows::Win32::System::Variant::VARIANT`(core 재수출)다. `VARIANT::from(i32)`와 `i32::try_from(&VARIANT)`로 바꾼다.

### 추천 구현 방식

1. MSAA는 UIA 색인이 비었거나 실패한 창에서만 대체로 쓴다(08 §4.2).
2. 커서 이동마다 `AccessibleObjectFromPoint`를 부르지 않는다. 0.7 ms지만 응답 없는 앱 위험이 있다.
3. 대신 색인 일꾼 스레드에서 점 질의를 받아 결과를 캐시한다.

### [확인 필요]

- 한글(HWP)·오래된 Win32 앱에서 MSAA가 UIA보다 나은 사례
- 멈춘 앱에서 막히는 시간

---

## 7. 3b-dpi — 남의 창 DPI 인식과 좌표

### 결론

- **DPI 인식 읽기는 된다.**
- **DPI 모르는 창의 실제 비교는 이 PC에서 할 수 없었다.** 모니터 두 대가 모두 100%이고, 보이는 최상위 창 7개가 전부 Per-Monitor 인식이었다.
- 배율을 바꾸는 것은 시스템 변경이라 하지 않았다.

### 쓸 API·features

`windows` 0.62 `"Win32_UI_HiDpi"`. 저장소의 windows-sys에도 같은 이름의 feature가 이미 있다.

| API | 반환 |
|---|---|
| `GetWindowDpiAwarenessContext(hwnd)`(Win10 1607+) | 다른 프로세스 창도 된다 |
| `GetAwarenessFromDpiAwarenessContext(ctx)` | `DPI_AWARENESS` 0 = unaware, 1 = system, 2 = per-monitor |
| `GetDpiFromDpiAwarenessContext(ctx)`(1803+) | system 인식 창이면 그 앱이 가정한 시스템 DPI. **Per-Monitor 문맥은 0을 돌려준다**(실측) |
| `GetDpiForWindow(hwnd)` | unaware 96 / system = 시스템 DPI / PM = 모니터 DPI |
| `GetWindowDpiHostingBehavior(hwnd)` | 혼합 호스팅(자식 창이 다른 인식일 때)인지 |
| `GetDpiForMonitor(MonitorFromWindow(..), MDT_EFFECTIVE_DPI)` | 모니터 DPI |
| 변환 `LogicalToPhysicalPointForPerMonitorDPI(hwnd, &mut pt)` / `PhysicalToLogicalPointForPerMonitorDPI` | 그 창의 인식 기준으로 좌표를 바꾼다 |

### 최소 코드 (`dpi/src/main.rs`)

    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
    let ctx = GetWindowDpiAwarenessContext(hwnd);
    let aw  = GetAwarenessFromDpiAwarenessContext(ctx).0;     // 0/1/2
    let sys = GetDpiFromDpiAwarenessContext(ctx);              // system이면 >0, PM이면 0
    let mut dx = 0; let mut dy = 0;
    GetDpiForMonitor(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), MDT_EFFECTIVE_DPI, &mut dx, &mut dy)?;
    let virtualized = (aw == 0 && dx != 96) || (aw == 1 && sys != dx);   // 좌표가 늘려 보이는 창

### 실측

- 모니터: (0, 0, 1920, 1080) 96 DPI 주 모니터 · (1920, 0, 3840, 1080) 96 DPI
- 보이는 최상위 창 7개가 모두 `aw=2`(Per-Monitor)였다. 모두 `ctxdpi=0`, `GetDpiForWindow=96`, `hosting=0`이다.
  - 앱: Aside 2 · WindowsTerminal · tidy_task 2 · Claude · Explorer(`Progman`)
- 어긋나는 창이 없어서 GetWindowRect / DWM 프레임 / UIA / MSAA 사각형 비교는 실행되지 않았다. 코드는 들어 있다: `dpi.exe --all`이면 PM 창으로도 비교한다.

### 문헌

1. **UIA는 물리 좌표다.**
   - Microsoft: "The UI Automation API does not use logical coordinates."
   - `BoundingRectangle`·`FromPoint`·`ClickablePoint`가 물리 좌표를 주고받는다.
   - 클라이언트가 DPI 인식이어야 맞는 값을 받는다.
   - 우리 프로세스는 PMv2이므로 UIA `BoundingRectangle`을 그대로 물리 px로 쓰면 된다.
2. **UIA 값이 맞는지는 제공자에 달렸다.**
   - Win32 표준 컨트롤은 UIA·oleacc의 프록시가 **클라이언트(우리) 프로세스에서** `GetWindowRect` 등으로 계산한다. 우리 인식(PMv2) 기준의 물리 값이 나온다.
   - 앱이 자체 제공자를 쓰면서 논리 좌표를 내보내면 그건 그 앱의 버그다. 예: Uno Platform 이슈에서 DIP를 그대로 내보낸 사례.
3. **MSAA(`accLocation`)의 DPI 동작은 Microsoft 문서에서 찾지 못했다.**
   - 추론: DPI 모르는 앱 안의 **자체 IAccessible 서버**는 자기 프로세스의 가상화된 좌표(논리)로 계산해 COM으로 그대로 넘긴다.
   - 그러면 우리에게 논리 좌표가 올 가능성이 높다.
   - 이번 실측에서 `OBJID_WINDOW`가 `GetWindowRect`와 같았던 것은 oleacc 시스템 프록시(클라이언트 쪽 계산) 경우다.
4. **혼합 배율에서는 전역 배율 하나로 나누면 틀린다**(Microsoft Q&A). 창마다, 모니터마다 계산한다.

### 함정

- `GetDpiFromDpiAwarenessContext`는 PM 문맥에서 0이다. 0을 "96"으로 취급하지 말 것.
- 창 하나 안에서도 자식 창의 인식이 다를 수 있다(`GetWindowDpiHostingBehavior` = MIXED). 루트 창 하나만 보고 판정하면 틀린다.

### 추천 구현 방식

1. UIA 사각형은 그대로 쓴다.
2. MSAA 대체 경로에서는 창마다 **자기 보정**을 한다.
   1. 그 창의 `OBJID_CLIENT` `accLocation`을 구한다.
   2. `GetClientRect`+`ClientToScreen` 값(우리 PMv2 기준 물리)과 비교한다.
   3. 두 값이 모니터DPI/96(system이면 모니터DPI/시스템DPI) 비율로 다르면, 그 서버의 모든 사각형에 `LogicalToPhysicalPointForPerMonitorDPI(hwnd)`를 적용한다.
   4. 같으면 그대로 쓴다.
   - `OBJID_CLIENT`는 자체 서버가 대개 직접 답하는 객체라 보정 기준으로 쓸 만하다. [확인 필요]

### [확인 필요]

배율 125% 또는 150% 모니터와 DPI 모르는 앱이 있는 PC에서 `dpi.exe`·`msaa.exe`를 다시 돌린다.
- DPI 모르는 앱 후보: 구버전 한글, 오래된 사내 프로그램, `SetThreadDpiAwarenessContext(UNAWARE)`로 만든 시험 창
- 확인할 것:
  - UIA `BoundingRectangle`이 물리 px인지
  - 자체 IAccessible 서버의 `accLocation`이 논리 px인지
  - `GetWindowRect`(PMv2 호출자) = 물리인지

---

## 부록 A. 출처

- [GraphicsCaptureSession.IsBorderRequired](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.isborderrequired) — 동의 필요, 거부 시 false 설정은 성공하지만 무시됨, Win10 2104(20348)+
- [GraphicsCaptureAccess.RequestAccessAsync](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscaptureaccess.requestaccessasync) — Borderless에는 `graphicsCaptureWithoutBorder` 기능 선언 필요
- [uap11:Capability](https://learn.microsoft.com/en-us/uwp/schemas/appxpackage/uapmanifestschema/element-uap11-capability) — `graphicsCaptureProgrammatic` / `graphicsCaptureWithoutBorder`
- [SetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity) — `WDA_EXCLUDEFROMCAPTURE`는 Win10 2004+, DWM 합성 때만 동작
- [MagSetWindowFilterList](https://learn.microsoft.com/en-us/windows/win32/api/magnification/nf-magnification-magsetwindowfilterlist) — INCLUDE는 Win7+ 미지원, WDDM 필요, 돋보기 창 자신은 자동 제외
- [UI Automation and Screen Scaling](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-and-screen-scaling) — UIA는 물리 좌표
- [Automation Element Property Identifiers](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-automation-element-propids) — BoundingRectangle은 물리 화면 좌표
- [Uno Platform #24753](https://github.com/unoplatform/uno/issues/24753) — 제공자가 DIP를 내보내면 어긋나는 사례
- [Microsoft Q&A: 혼합 DPI에서 요소 강조](https://learn.microsoft.com/en-us/answers/questions/6031018/how-can-a-windows-desktop-app-reliably-locate-and) — 전역 배율 하나로는 부족

## 부록 B. 이번에 하지 않은 것

- 보이는 창으로 하는 테두리 육안 확인, 돋보기 화면 표시, 4K 측정
- 배율을 바꾼 DPI 시험(시스템 변경이라 하지 않음)
- MSIX 서명·설치, `RequestAccessAsync`, `AddPackageByAppInstallerFileAsync`·Store 내려받기 실행
- 실제 WGC 프레임으로 하는 녹화 파일 저장(개인정보) — 인코더는 합성 영상으로만 시험했다
