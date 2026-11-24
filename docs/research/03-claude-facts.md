# Claude 직접 확인 사실 (2026-11-24)

## 배포·비용
- MS Store 개인 개발자 등록: 2025-09부터 전 세계 약 200개국 무료(신규 등록 흐름 기준, 신분증+셀카 인증). 회사 계정은 $99 1회.
  - 출처: https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/
- Store 제출 형식
  - MSIX: Store가 무료로 서명해 줌 → 비용 0.
  - EXE/MSI 그대로 등록: 게시자가 신뢰 CA 인증서로 직접 Authenticode 서명해야 함(내부 PE까지). 자체 서명 불가 → 인증서 비용 발생.
  - 출처: https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options , https://learn.microsoft.com/en-us/windows/apps/distribute-through-store/how-to-distribute-your-win32-app-through-microsoft-store
- Tauri 2는 EXE(NSIS)·MSI만 만들고 MSIX는 직접 안 만듦 → Microsoft winapp CLI 또는 MSIX Packaging Tool로 변환.
  - 출처: https://v2.tauri.app/distribute/microsoft-store/ , https://learn.microsoft.com/en-us/windows/apps/dev-tools/winapp-cli/guides/tauri
- Azure Artifact Signing(구 Trusted Signing, 월 약 $10): 개인은 미국·캐나다만 → 한국 개인은 사용 불가(조직은 미·캐·EU·영).
  - 출처: https://learn.microsoft.com/azure/artifact-signing/quickstart
  - 결론: Store 밖 exe는 무서명 → SmartScreen 경고. 무료로는 "Store(MSIX) 권장 + GitHub exe 보조"가 현실적.
- macOS: 공증(notarization)은 Apple Developer Program 연 $99 필요. 미공증 앱은 Sequoia부터 우클릭 열기 우회가 사라지고 "시스템 설정 → 개인정보 보호 및 보안 → 그래도 열기"로만 실행.
  - 출처: https://www.macrumors.com/2024/08/06/macos-sequoia-gatekeeper-security-change/
- macOS 함정 1: ad-hoc 서명 앱은 업데이트할 때마다 화면 기록 권한이 풀림(서명 정체성이 매번 바뀜). 무료로 가더라도 고정 자체 서명 인증서로 서명해야 권한이 유지됨. 일부 보고로는 Sequoia에서 ScreenCaptureKit이 Developer ID 서명을 요구[단일 보고].
  - 출처: https://developer.apple.com/forums/thread/795739 , https://github.com/CapSoftware/Cap/issues/1722
- macOS 함정 2: Sequoia는 화면 기록 앱에 주기적(월 단위) 재허용 알림을 띄움. 영구 해제 공식 방법 없음.
  - 출처: https://tidbits.com/2024/08/19/apple-reduces-excessive-sequoia-permission-requests-shifts-to-monthly

## Tauri 오버레이 관련
- `set_ignore_cursor_events`는 창 전체 단위 → "부분 클릭 통과"는 어렵다. 그래서 필기 레이어·툴바·핀을 각각 다른 창으로 나누는 구조가 필요.
  - 출처: https://github.com/tauri-apps/tauri/issues/11461 , https://dev.to/manasightgg/why-i-chose-tauri-v2-for-a-desktop-overlay-in-2026-597h
