#!/bin/sh
# 맥 릴리스 초안 만들기(R2-UPD-5): 서명 빌드 → .app.zip + SHA-256 → latest-mac.json → GitHub 릴리스 **초안**.
# 게시(초안 → 공개)는 사람이 GitHub에서 확인한 뒤 한다 — 공개하는 순간 모든 사용자의 앱이 하루 안에 받는다.
#
# 사용:  APPLE_SIGNING_IDENTITY="Shookshok Self Sign" sh scripts/release-mac.sh "바뀐 점 1" "바뀐 점 2" "바뀐 점 3"
#   판 번호는 apps/screen-ink/src-tauri/tauri.conf.json의 version을 먼저 올려 둔다(앱이 이 번호와 목록을 견준다).
#
# 받는 쪽(앱)은 ink-platform-macos/src/updater.rs:
#   https://github.com/djqanrudgns12/screen-ink/releases/latest/download/latest-mac.json 을 받아
#   url의 zip을 받고 sha256 확인 → 풀고 → 지금 앱과 같은 인증서 서명인지 codesign으로 확인 → 바꿔 끼우고 다시 연다.
set -eu
cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/.*"version": *"\([0-9][0-9.]*\)".*/\1/p' apps/screen-ink/src-tauri/tauri.conf.json | head -1)
[ -n "$VERSION" ] || { echo "tauri.conf.json에서 판 번호를 못 읽었어요"; exit 1; }
TAG="v$VERSION"
REPO="djqanrudgns12/screen-ink"

sh scripts/mac-build-dmg.sh

APP=$(ls -d target/release/bundle/macos/*.app | head -1)
DMG=$(ls target/release/bundle/dmg/*.dmg | head -1)
OUT=target/release/upload
rm -rf "$OUT"; mkdir -p "$OUT"
ZIP="$OUT/Shookshok_${VERSION}_aarch64.app.zip"
/usr/bin/ditto -c -k --keepParent "$APP" "$ZIP"
SHA=$(/usr/bin/shasum -a 256 "$ZIP" | awk '{print $1}')
cp "$DMG" "$OUT/"

# 바뀐 점(인자 최대 3개) → JSON 배열
NOTES=$(node -e 'console.log(JSON.stringify(process.argv.slice(1, 4)))' "$@")
cat > "$OUT/latest-mac.json" <<EOF
{
  "version": "$VERSION",
  "notes": $NOTES,
  "url": "https://github.com/$REPO/releases/download/$TAG/$(basename "$ZIP")",
  "sha256": "$SHA"
}
EOF

echo "올릴 파일:"; ls -la "$OUT"
if command -v gh >/dev/null 2>&1; then
  gh release create "$TAG" --repo "$REPO" --draft --title "슉쇽 $VERSION" --notes "$(printf '%s\n' "$@")" "$OUT"/*
  echo "초안을 만들었어요 → GitHub에서 확인 뒤 [Publish release]를 누르면 앱들이 하루 안에 받아요."
else
  echo "gh가 없어요 — $OUT 의 파일을 릴리스 $TAG 에 직접 올리고 게시하세요."
fi
