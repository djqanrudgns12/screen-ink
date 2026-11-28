# W25 — Windows 0.3.2 배포판 (P2 끝, `3-REL-032`)

- 보내기: 0.3.2 판 번호 커밋을 푸시한 뒤 `sh docs/plans/2026-11-28/briefs/send-win.sh W25-release032 standard`

---

[일감 W25-release032 — Windows 0.3.2 릴리스 산출물 만들기(커밋 없음)]
기준: origin/feat/foundation(판 0.3.2 커밋 이후). scripts/release-win.ps1·scripts/package-msix.ps1·scripts/package-portable.ps1.
할 일: 릴리스 빌드 → (1) NSIS 설치본, (2) 포터블 ZIP, (3) 무서명 MSIX(참고용), (4) SHA256SUMS.txt를 C:\agents\runs\W25-release032\upload\ 에 모은다. 서명 인증서가 없으니 서명하지 않는다. latest-win.json·.appinstaller는 만들되 올리지 않을 것이라고 보고에 적는다.
확인: 포터블 ZIP을 일감 폴더에 풀어 한 번 켜서 판 0.3.2·시작 기록 정상, F1로 범위를 확정한 뒤 안을 끌어 옮겨지는지 한 번(새 기능), 바로 끄기. NSIS는 실행하지 않고 만들어짐·크기만. 파일 이름에 판 0.3.2가 들어가는지.
커밋: 저장소 파일을 바꿀 필요가 없으면 커밋하지 않는다(스크립트가 고장 나 고친 경우만 커밋).
끝 보고: 각 파일 이름·크기·SHA-256, 실행 확인 결과. 사용자에게 보이는 보고와 모든 출력은 한국어로 쓴다. 맡은 범위는 묻지 않고 끝낸다.
