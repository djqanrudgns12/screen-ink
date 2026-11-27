# Windows 작업 노드 — 맥 미니에서 Windows 일 맡기기 (2026-11-27 구축·운영 중)

목적: 슉쇽의 Windows 쪽(빌드·시험·실기·MSIX·업데이트)을 맥 미니의 총괄 세션이 **SSH로 Windows의 에이전트에게 맡기고, 결과를 가져와 합친다.**
맥 혼자서는 Windows 대상 교차 `cargo check`(`scripts/check-windows.sh`)까지만 된다. 실제 빌드·실행·화면 확인은 이 노드에서 한다.

## 1. 구조 (한눈에)

```
맥(총괄: 나누기·브리프·합치기·푸시·릴리스)
  └─ Tailscale + SSH (키 인증) ─▶ Windows "kyeonghun" (작업 노드)
        C:\agents\dispatch.ps1  접수: 브리프 저장 → worktree 만들기 → 예약 작업 등록
        C:\agents\worker.ps1    일꾼: **로그인된 화면 세션**에서 Codex/Claude CLI 비대화형 실행
        C:\dev\screen-ink       원본 저장소(공개 저장소 HTTPS 클론, feat/foundation)
        C:\dev\wt\<작업>        일감마다 따로 worktree · 브랜치 win/<작업>
        C:\dev\target           공유 빌드 폴더(CARGO_TARGET_DIR, cargo 잠금이 차례를 지킴)
        C:\agents\runs\<작업>   prompt.md · out.md(답) · log.txt · done.json
```

- **SSH가 끊겨도 일은 계속 돈다**(예약 작업). 화면 세션 안이라 에이전트가 앱을 띄우고 스크린숏·키 입력을 할 수 있다.
- **Windows는 GitHub 로그인이 없다**(gh 토큰 만료) → Windows 에이전트는 **커밋만** 하고, 맥이 `git fetch win`으로 가져와 검토·합친 뒤 GitHub에 푸시한다.

## 2. 이 노드의 사실

| 항목 | 값 |
|---|---|
| 접속 | `ssh win` — Tailscale `100.101.231.97`, 사용자 **`rudgnswh12`**(프로필 폴더 이름 `rudgn`과 다름), 키 `~/.ssh/id_ed25519_win` |
| 기계 | Windows 11 Pro · Ryzen 5 7500F 12스레드 · 32 GB · RTX 4060 · 남은 C: 약 288 GB |
| 모니터 | **주 4K 3840×2160 200%(DISPLAY2, 0,0)** + **FHD 1920×1080 100%(DISPLAY1, 3840,0)** → 혼합 배율·4K 확대 시험 가능 |
| 셸 | 기본 Windows PowerShell 5.1(PowerShell 7 없음) |
| 도구 | Git 2.53 · Node 24.14 · Rust 1.94.1 MSVC(맥은 1.100 — 필요하면 `rustup update stable`) · VS 2022/18 C++ 도구 · WebView2 158 |
| 에이전트 | **Codex**(ChatGPT 로그인, `%LOCALAPPDATA%\OpenAI\Codex\bin\<해시>\codex.exe` — 갱신되면 해시가 바뀌어 일꾼이 최신을 찾는다) · **Claude**(pro 계정 — 아껴 씀) · Antigravity는 IDE만 있고 CLI(`agy`) 없음 → Windows 일은 Codex 중심, 제미나이는 맥에서 |
| 한도 | 같은 계정이라 맥과 한도를 같이 쓴다(얻는 것은 기계 한 대 몫의 병렬과 Windows 본체 확인) |

## 3. 맥에서 쓰는 명령 (`~/.local/bin/win-agent`)

```bash
win-agent run <작업> [codex|claude] [등급] < 브리프.md   # 맡기고 끝날 때까지 기다림 → 브랜치 가져오기 → 답 출력(백그라운드로 돌린다)
win-agent wait <작업>      # 맥 쪽 기다림이 끊겼을 때 다시 기다리기
win-agent status           # 일감별 새 커밋 수·끝/진행
win-agent out <작업>       # 답(out.md)    · win-agent log <작업>  기록 끝 40줄
win-agent fetch <작업>     # Windows win/<작업> → 맥 refs/remotes/win/<작업>
```

등급 → 모델(브리지와 같은 표): codex `trivial·light`=gpt-6.1-terra, `standard·complex`=gpt-6.2-sol xhigh, **`deep`=gpt-6.2-astra high(기본)**, `critical`=astra xhigh. claude는 opus 5.5.
작업 이름은 영문·숫자·`._-` 40자 이내(예: `W5-mixeddpi`).

### 합치는 순서 (총괄)

1. `win-agent fetch <작업>` → `git log --oneline origin/feat/foundation..refs/remotes/win/<작업>`·`git diff`로 훑기(남의 범위·맥 코드 건드렸는지).
2. 최신 feat/foundation 위로: `git checkout -b tmp refs/remotes/win/<작업> && git rebase origin/feat/foundation`(충돌은 총괄이 푼다) → `git checkout feat/foundation && git merge --ff-only tmp`.
3. 맥 회귀: `scripts/check-windows.sh`(Windows 교차 검사) + 바뀐 범위 맥 시험(자원 보호 도구 아래) → 푸시.
4. 끝난 일감 정리: `ssh win "git -C C:\dev\screen-ink worktree remove --force C:\dev\wt\<작업>; git -C C:\dev\screen-ink branch -D win/<작업>; Unregister-ScheduledTask -TaskName agent-<작업> -Confirm:\$false"`, 맥 `git update-ref -d refs/remotes/win/<작업>`.

## 4. 화면 도우미 (Windows 에이전트·총괄 공용)

| 도구 | 하는 일 |
|---|---|
| `C:\agents\tools\shot.ps1 -Out <png>` | 모든 모니터 실제 픽셀 스크린숏. 캡처 제외 창(슉쇽 오버레이·섬)은 안 찍힌다. **사용자 화면이 담기니 확인 뒤 지운다** |
| `C:\agents\tools\keys.ps1 <키...>` | 실제 키보드처럼 가상 키(입력기를 거침). `F1`, `ctrl+shift+s`, `hangul d k s s u d space`, `w500`(기다림) |
| `C:\agents\tools\monitors.ps1` | 모니터 위치·크기·배율 |
| `C:\agents\gui.ps1 -Command "<명령>" -WaitSec N` | SSH(화면 없음)에서 화면 세션으로 명령 하나를 넘겨 출력 받기(총괄이 직접 확인할 때) |
| `C:\agents\status.ps1` | 일감 현황(`win-agent status`가 부름) |

## 5. 브리프 첫머리 (Windows 일감마다 그대로 붙인다)

```
[공통 규칙 — Windows 작업 노드, 꼭 지킴]
- 사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다(커밋 메시지·주석·문서 포함, 코드 식별자·명령·경로만 원문).
- 맡은 범위는 묻지 않고 끝낸다. 이 실행은 비대화형이라 질문해도 답이 오지 않는다.
- 작업 폴더는 지금 폴더(C:\dev\wt\<작업>, 브랜치 win/<작업>)다. 저장소의 다른 폴더(C:\dev\screen-ink, 다른 C:\dev\wt\*)는 고치지 않는다 — 다른 에이전트가 동시에 쓴다.
- 커밋만 하고 푸시하지 않는다(이 PC는 GitHub 로그인이 없다 — 맥의 총괄이 SSH로 가져가 합친다). 커밋 메시지 한국어, 끝에 빈 줄 뒤 `Co-Authored-By: Codex <noreply@openai.com>`. force·--no-verify 금지.
- 빌드 산출물은 공유 폴더 CARGO_TARGET_DIR=C:\dev\target(이미 설정됨). 다른 작업이 동시에 빌드하면 cargo 잠금 대기가 생기는 것은 정상이다.
- 앱을 띄울 때는 방금 빌드한 실행 파일을 C:\agents\runs\<작업>\app\ 로 복사해 **그 사본**을 실행한다(공유 target의 exe는 다른 일감의 빌드가 덮어쓴다). 성능 측정(진입 ms·fps·메모리)은 다른 일감이 빌드 중이면 값이 틀어지니 `C:\agents\status.ps1`로 혼자인지 보고 잰다.
- 이 실행은 로그인된 화면 세션 안이라 앱을 띄워 볼 수 있다. 화면 도우미: C:\agents\tools\shot.ps1 -Out <png>(사용자 화면이 담기니 확인 뒤 지운다), C:\agents\tools\keys.ps1 <키...>(실제 키 입력, 입력기를 거침), C:\agents\tools\monitors.ps1. 모니터: 주 4K 200% + 오른쪽 FHD 100%. 사용자의 다른 창·파일·앱은 건드리지 않는다. 띄운 앱·창은 끝나면 끈다.
- 비밀값·.env 없음(만들지 않음). 시스템 설정 변경·인증서 설치·MSIX 설치·드라이버 설치·관리자 권한 요구 동작 금지 — 필요하면 보고에 "사람 승인 필요"로 남긴다.
- 맥 쪽 동작(#[cfg(target_os = "macos")], crates/ink-platform-macos)은 바꾸지 않는다.
- 같은 실패가 3번이면 접근을 바꾸거나 그 조각을 "막힘"으로 남기고 나머지를 끝낸다. 통과시키려고 시험 기대값을 바꾸지 않는다.
- 끝 보고(마지막 메시지): 한 일, 커밋 해시, 실행한 검사와 결과(안 돌린 것은 "미실행"), 남은 것·사람 확인 필요. 짧게.
```

## 6. 착오를 막는 주의

- **Windows PowerShell 5.1은 BOM 없는 UTF-8 `.ps1`을 CP949로 읽어 한글이 깨지고 문법 오류가 난다** → Windows로 보내는 스크립트는 UTF-8 BOM(`printf '\xEF\xBB\xBF'; cat 파일`).
- **사용자에게 붙여 넣게 할 PowerShell 블록에 `*`·`_`를 넣지 않는다** — 채팅 표시가 서식 기호로 지워 `$_`가 `$.`, `'*S-1-…'`이 `'S-1-…'`가 됐다(2026-11-27). `Select-Object -ExpandProperty`, SID 개체 등으로 피한다.
- 예약 작업 계정은 `[Security.Principal.WindowsIdentity]::GetCurrent().Name`(SSH 세션의 `$env:USERDOMAIN`은 안 맞음, 오류 0x80070534).
- `.cargo\bin`의 rustup 대리 실행 파일이 심볼릭 링크면 SSH에서 "연결된 응용 프로그램 없음"으로 실패한다 → 하드 링크로 바꿔 둠(rustup 갱신 뒤 다시 생기면 같은 방법).
- 원격 데스크톱(RDP)으로 접속하면 콘솔 세션이 끊겨 화면 시험·스크린숏이 깨진다. 사람이 보려면 RustDesk·크롬 원격 데스크톱처럼 실제 화면을 그대로 비추는 것을 쓴다.
- **공유 빌드 폴더(C:\dev\target)의 최종 실행 파일은 worktree끼리 덮어쓴다** — 라이브러리 빌드는 공유해도 되지만, 앱을 띄울 때는 exe를 일감 폴더로 복사해 실행한다(브리프 첫머리에 들어 있음).
- **성능 측정 일감은 혼자 돌린다** — 다른 일감의 빌드·시험이 CPU를 쓰면 진입 ms·fps가 틀어진다. 측정 일감은 앞 일감이 끝난 뒤 단독으로.
- 같은 파일을 Windows 일감 둘이 동시에 크게 고치지 않게 범위를 나눈다(합칠 때 충돌은 총괄이 풀지만 비용이 든다).

## 7. 처음 설정 기록 (끝남)

- Windows 관리자 PowerShell: OpenSSH 서버 켜기·자동 시작, 기본 셸 = `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`(처음에 PowerShell 7로 지정했다가 그 셸이 없어 "not allowed because shell … does not exist"로 접속 거절됐음), 맥 공개 키를 `C:\ProgramData\ssh\administrators_authorized_keys`(관리자 계정) — 권한 SYSTEM·Administrators만·소유자 Administrators, `PasswordAuthentication no`, 방화벽 22번은 Tailscale 대역 `100.64.0.0/10`만, 잠자기 끄기, Tailscale `up --unattended`.
- 맥: `~/.ssh/config`의 `Host win kyeonghun`, 저장소 remote `win` = `win:C:/dev/screen-ink`(uploadpack `git upload-pack`).
- 그만 쓰려면: Windows `Stop-Service sshd; Set-Service sshd -StartupType Disabled` 후 키 파일에서 그 줄 삭제, 맥 `~/.ssh/id_ed25519_win*` 삭제.
