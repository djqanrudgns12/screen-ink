# 맥 통신 0 확인 (R-NET-2 맥 몫) — 2026-11-27 20:15 KST

- 판: 0.2.0 설치판(마이크 포함 재빌드, feat/foundation 9998d75 이전), 맥 미니, 모니터 2대.
- 상태: 앱을 켠 지 30분 넘음(켠 뒤 10분 업데이트 확인은 이미 지나감, 다음 확인은 하루 뒤) → 관측 동안은 업데이트 확인이 없는 대기 상태.
- 대상 프로세스: 슉쇽 본체 `screen-ink`(PID 72346)와 그 WebKit 네트워크 프로세스 `com.apple.WebKit.Networking`(PID 72353 — 슉쇽이 켜질 때 바로 뒤에 뜬 것). WebKit 화면(WebContent)·GPU 프로세스는 네트워크를 직접 쓰지 않는다.
- 방법: `lsof -nP -i TCP -a -p <pid>`(ESTABLISHED, 루프백 제외) 10초 간격 6회 = 60초, `lsof -nP -i UDP -a -p <pid>`(QUIC 포함) 1회.
- 결과: **바깥 TCP 0건, UDP 0건.**
- Windows는 같은 관측에서 WebView2 하위 프로세스(`msedgewebview2.exe`)가 2건 연결해 미통과 → W14에서 WebView2 백그라운드 통신을 끄는 중(docs/release/checklist-2.md R-NET-2).
- 남음: 업데이트 설정을 끈 상태(`update.auto` = false)로 앱을 새로 켜서 처음 10분(업데이트 확인 시점 포함) 동안 0건인지 — 0.3.0 릴리스 점검 때 함께.
