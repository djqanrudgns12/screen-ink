#!/bin/sh
# 맥 자체 서명 .app·.dmg 빌드(M-sign — 인수인계 2026-11-25 §2.5-4, 시안 I-mac §2.3, 결정: 자체 서명 무료 경로)
#
# 사용:  APPLE_SIGNING_IDENTITY="Shookshok Self Sign" sh scripts/mac-build-dmg.sh
#   신원(인증서)이 없으면 docs/handoff/mac-self-sign.md 1절대로 키체인 접근에서 한 번 만든다.
#
# 하는 일
#   1. 키체인에 그 코드 서명 신원이 있는지 본다(없으면 목록을 보여 주고 멈춤).
#   2. tauri build(app·dmg) — Tauri가 APPLE_SIGNING_IDENTITY로 서명한다.
#   3. 서명 검사 + 지정 요구 사항(DR)을 target/mac-dr.txt에 남기고, 지난번 것과 같은지 알려 준다.
#      DR이 같으면 macOS(TCC)는 같은 앱으로 보고 화면 기록·손쉬운 사용 권한을 이어 준다.
#      임시(ad-hoc) 서명은 DR이 빌드마다 바뀌어(cdhash) 업데이트마다 권한이 풀린다.
set -eu
cd "$(dirname "$0")/.."

ID="${APPLE_SIGNING_IDENTITY:-}"
list_identities() { security find-identity -p codesigning | sed -E 's/[0-9A-F]{40}/…/'; }
if [ -z "$ID" ]; then
  echo "APPLE_SIGNING_IDENTITY가 비어 있어요. 키체인의 코드 서명 신원:"
  list_identities
  echo "만드는 법: sh scripts/mac-make-cert.sh (또는 docs/handoff/mac-self-sign.md 1절)"
  exit 2
fi
if ! security find-identity -p codesigning | grep -qF "\"$ID\""; then
  echo "키체인에 코드 서명 신원 '$ID'이 없어요. 있는 것:"
  list_identities
  echo "만들기: sh scripts/mac-make-cert.sh"
  exit 2
fi
# 신뢰를 정하지 않은 인증서도 서명은 된다(받는 맥에서는 어차피 "그래도 열기"). 알려만 준다
if ! security find-identity -v -p codesigning | grep -qF "\"$ID\""; then
  echo "알림: '$ID'은 '항상 신뢰'가 아니에요(서명은 됩니다). docs/handoff/mac-self-sign.md 1절 5번"
fi
export APPLE_SIGNING_IDENTITY="$ID"

# cargo가 PATH에 없는 셸(앱 안 터미널 등)에서도 되게: rustup(Homebrew)·~/.cargo/bin에서 찾는다
if ! command -v cargo >/dev/null 2>&1; then
  for d in /opt/homebrew/opt/rustup/bin "$HOME/.cargo/bin" /usr/local/opt/rustup/bin; do
    if [ -x "$d/cargo" ]; then PATH="$d:$PATH"; export PATH; break; fi
  done
fi
command -v cargo >/dev/null 2>&1 || { echo "cargo를 찾지 못했어요(rustup 설치 확인: brew install rustup)"; exit 2; }

npm run app:build -- --bundles app,dmg

APP=$(ls -d target/release/bundle/macos/*.app 2>/dev/null | head -1)
DMG=$(ls target/release/bundle/dmg/*.dmg 2>/dev/null | head -1)
[ -n "$APP" ] || { echo "빌드한 .app을 찾지 못했어요(target/release/bundle/macos)"; exit 1; }

codesign --verify --deep --strict "$APP"
DR=$(codesign -dr - "$APP" 2>&1 | sed -n 's/^designated => //p')
echo "앱: $APP"
[ -n "$DMG" ] && echo "dmg: $DMG"
echo "지정 요구 사항: $DR"

PREV=target/mac-dr.txt
if [ -f "$PREV" ]; then
  if [ "$(cat "$PREV")" = "$DR" ]; then
    echo "지난 빌드와 DR이 같아요 → 덮어 설치해도 권한이 남아야 해요(확인: 설치 뒤 F1이 바로 되는지)"
  else
    echo "지난 빌드와 DR이 달라요 → 권한이 풀릴 수 있어요. 지난 것: $(cat "$PREV")"
  fi
fi
printf '%s' "$DR" > "$PREV"
