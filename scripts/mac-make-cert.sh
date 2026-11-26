#!/bin/sh
# 맥 자체 서명 코드 서명 인증서를 한 번 만든다(M-sign, 무료 — Apple 개발자 등록 없이).
# 사람이 자기 터미널에서 실행한다. 키체인에 넣고 신뢰를 정할 때 맥 로그인 암호를 한 번(또는 두 번) 묻는다.
#
#   sh scripts/mac-make-cert.sh                 # 이름: Shookshok Self Sign
#   sh scripts/mac-make-cert.sh "다른 이름"
#
# 하는 일: 10년짜리 자체 서명 인증서(용도 = 코드 서명)와 개인 키를 만들어 로그인 키체인에 넣고,
# codesign이 묻지 않고 그 키를 쓰게 하고(-T /usr/bin/codesign), "코드 서명: 항상 신뢰"로 정한다.
# 임시 파일(개인 키 포함)은 끝나면 지운다. 개인 키는 이 맥 키체인에만 남는다.
set -eu

NAME="${1:-Shookshok Self Sign}"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"
OPENSSL=/usr/bin/openssl   # 맥 기본(LibreSSL) — Homebrew OpenSSL 3은 p12 형식이 달라 키체인이 못 읽을 수 있다

if security find-identity -p codesigning | grep -qF "\"$NAME\""; then
  echo "이미 있어요: $NAME"
  security find-identity -p codesigning | grep -F "\"$NAME\"" | sed -E 's/[0-9A-F]{40}/…/'
  exit 0
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

echo "1/3 인증서 만들기: $NAME"
"$OPENSSL" req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=$NAME" \
  -addext "basicConstraints=critical,CA:false" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=critical,codeSigning" \
  -keyout "$TMP/key.pem" -out "$TMP/cert.pem" >/dev/null 2>&1
PASS=$("$OPENSSL" rand -hex 16)
"$OPENSSL" pkcs12 -export -inkey "$TMP/key.pem" -in "$TMP/cert.pem" -name "$NAME" -out "$TMP/id.p12" -passout "pass:$PASS"

echo "2/3 로그인 키체인에 넣기(맥 암호를 물을 수 있어요)"
security import "$TMP/id.p12" -k "$KEYCHAIN" -P "$PASS" -T /usr/bin/codesign >/dev/null

echo "3/3 '코드 서명: 항상 신뢰'로 정하기(맥 암호 창이 떠요)"
security add-trusted-cert -r trustRoot -p codeSign -k "$KEYCHAIN" "$TMP/cert.pem"

echo
echo "끝났어요. 확인:"
security find-identity -v -p codesigning | sed -E 's/[0-9A-F]{40}/…/'
echo
echo "이제 빌드: APPLE_SIGNING_IDENTITY=\"$NAME\" sh scripts/mac-build-dmg.sh"
