# M6 실시간 확대 측정 절차 · 판단 기준 (관문 G6 · 계획 5.1 / 5.4)

> 일꾼은 실행하지 않는다(화면 전체를 덮는 돋보기가 뜬다). 메인 세션이 사용자에게 미리 묻고 한 번에 잰다.
> 스크립트: [`m6-zoom.ps1`](m6-zoom.ps1) · 결과: `bench/results/<날짜>-m6-zoom-<라벨>.md`

## 1. 무엇을 재나

| 측정 | 목표 | 근거 |
|---|---|---|
| **확대 fps** — 영상 재생 중(S9) 60초 | **1080p ≥ 30 fps**, 4K는 수치 기록 | 07 관문 G6, R-ZOOM-1 |
| 진입 시간 — `Alt+2` → 돋보기 첫 그림, 20회 | 목표 없음(참고: 다른 모드 p95 ≤ 150 ms) | 로그 `mode enter zoom [N ms]` |
| 눈 확인 — 확대 화면 안 섬·후광·위치 지도 | **2중으로 찍히지 않음(0)** | R-ZOOM-4, 07 5.5 |

## 2. 준비

1. 릴리스 빌드(`npm run app:build`)를 켠다. 기본 단축키(`Alt+2`) 그대로. 로그: `%LOCALAPPDATA%\kr.screenink.desktop\logs\screen-ink.log`.
2. [PresentMon](https://github.com/GameTechDev/PresentMon) 콘솔판(2.x 권장, 1.x면 `-LegacyFlags`). 관리자 권한 PowerShell이 필요할 수 있다.
3. 영상: 1080p 30/60 fps 로컬 파일(인터넷 없이) 또는 유튜브 1080p, **커서 모니터 전체 화면**으로 재생.
4. 4K: 학교 PC는 1080p 두 대라 4K가 없다 → 4K 모니터 PC, 또는 그래픽 드라이버 가상 고해상도(NVIDIA DSR · AMD VSR)로 3840×2160. 어느 쪽이었는지 결과에 적는다.

## 3. 절차

```powershell
# 진입 시간 20회 (Alt+2 켜기·끄기를 SendKeys로 — 등록된 전역 키라 screen-ink만 받는다)
powershell -ExecutionPolicy Bypass -File bench\m6-zoom.ps1 -Entry -Count 20 -Save

# fps 60초: 영상 재생 → 커서를 영상 위에 → 실행(-AutoZoom이면 Alt+2를 스크립트가 누름)
powershell -ExecutionPolicy Bypass -File bench\m6-zoom.ps1 -Seconds 60 -PresentMon C:\tools\PresentMon.exe -AutoZoom -Save -Label 1080p
# 4K 모니터(또는 DSR/VSR)에서 같은 것
powershell -ExecutionPolicy Bypass -File bench\m6-zoom.ps1 -Seconds 60 -PresentMon C:\tools\PresentMon.exe -AutoZoom -Save -Label 4k
```

fps는 세 군데에서 모은다.

| 출처 | 뜻 | G6 판정에 |
|---|---|---|
| PresentMon `screen-ink.exe` | 우리 프로세스의 Present | **있으면 이것** |
| 앱 로그 `zoom frames N in T s (avg F fps, max gap G ms)` | 돋보기 프레임 루프 — 한 바퀴 = `MagSetWindowSource` 한 번(주사율만큼 막히며 그 자리에서 다시 비춤, REPORT §3) | PresentMon이 0이면 이것 |
| PresentMon `dwm.exe` | 화면 합성 기준선(주사율) | 참고만 |

> Magnification API는 우리 프로세스에서 DXGI Present를 하지 않을 수 있다 → `screen-ink.exe`가 0이면 앱 로그 값을 쓰고,
> 의심되면 **눈 확인**을 더한다: 확대 화면 안에 초시계(예: 브라우저 밀리초 시계)를 띄우고 휴대폰 슬로모션(120/240 fps)으로 3초 찍어
> 화면에서 숫자가 바뀐 횟수를 센다.

## 4. 함께 볼 것(눈 · 한 번씩)

- [ ] 확대 화면 안에 섬·후광·위치 지도가 2중으로 안 보인다(필터 목록 R-ZOOM-4). 섬·HUD가 돋보기 **위**에 있다.
- [ ] 커서가 하나만 보인다(시스템 커서 숨김 + 확대 커서). 확대 커서까지 사라지면 `magnifier.rs` `HIDE_SYSTEM_CURSOR = false`.
- [ ] 다른 모니터로 커서를 옮기면 그 모니터에서는 커서가 보인다.
- [ ] 휠 = 배율(1.25 ~ 8), **Shift+휠은 아래 앱이 받는다**(스크롤). 다른 모니터의 휠은 그대로.
- [ ] 확대 중에도 아래 앱을 클릭·타이핑할 수 있다(클릭 통과). 클릭은 확대 커서가 가리키는 곳에 간다.
- [ ] 커서 따라가기: 작은 손떨림에 화면이 흔들리지 않고(10% 죽은 영역), 크게 움직이면 부드럽게 따라온다. `zoom.follow = instant`면 바로.
- [ ] 확대 중에도 방향키는 아래 앱이 받는다(PPT 슬라이드 넘기기). 확대 중 잡는 키는 Esc 하나.
- [ ] `Esc`·`Alt+2`로 끝, 끝난 뒤 Esc가 다른 앱에서 정상(임시 키 해제), 휠도 정상(훅 해제).
- [ ] 혼합 배율(100% + 150%) 보조 모니터에서 확대 위치가 커서와 맞는다.
- [ ] 첫 `Alt+2`가 앱 시작 직후에도 늦지 않다(돋보기 미리 만들기 — 로그 `magnifier prewarmed in N ms`).
- [ ] 잠금(`Win+L`) → 풀면 확대가 끝나 있고 커서·휠 정상.

## 5. 판단(07 §3 G6 → 5.4 결정 메모)

| 결과 | 결정 | 할 일 |
|---|---|---|
| 1080p ≥ 30 · 4K ≥ 30 | **Magnification API 확정** | 결과 파일만 남긴다 |
| 1080p ≥ 30 · 4K < 30 | **1차는 1080p 기준 마감**, 2안을 2차로 | `bench/results/<날짜>-m6-zoom.md`에 결정 메모: 수치·기기·영상, "2안(WGC `CreateForMonitor` → D3D11/wgpu 셰이더 확대) 2차 이관" |
| 1080p < 30 | **2안 시제품 바로 착수** | 원인 먼저 기록: 앱 로그 `max gap`(긴 끊김) · dwm 기준선 · CPU/GPU 사용률(작업 관리자) |
| 측정 불가(Present 0 + 로그 없음) | 보류 | 눈 확인(슬로모션)으로 대신 판정, 로그 줄이 안 나온 이유를 따로 본다 |

결정 메모에 쓸 것: 날짜 · 기기(CPU·GPU·드라이버는 모델 이름까지만) · 모니터 구성과 배율 · 영상(해상도·fps) · 세 출처의 fps · 진입 p50/p95 · 눈 확인 표 · 결정.
