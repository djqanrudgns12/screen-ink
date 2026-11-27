#!/usr/bin/env bash
# 맥에서 Windows 앱·플러그인·어댑터를 링크 없이 검사한다.
# 준비: rustup, clippy, Homebrew의 llvm, resource-guard
set -euo pipefail
cd "$(dirname "$0")/.."

if command -v brew >/dev/null; then
  prefix=$(brew --prefix)
  export PATH="$prefix/opt/rustup/bin:$prefix/opt/llvm/bin:$PATH"
fi
export PATH="$PATH:$HOME/.cargo/bin:$HOME/.local/bin"
target=x86_64-pc-windows-msvc

# Tauri 빌드 스크립트는 cargo check에서도 앱 아이콘과 버전 리소스를 컴파일한다.
command -v llvm-rc >/dev/null || { echo 'llvm-rc가 필요합니다. brew install llvm으로 설치하세요.' >&2; exit 1; }
export CC_x86_64_pc_windows_msvc="${CC_x86_64_pc_windows_msvc:-clang-cl}"
rustup target add "$target"

guarded_cargo() {
  if [[ ${RESOURCE_GUARD_ACTIVE:-0} == 1 ]]; then
    cargo "$@"
  else
    resource-guard run --profile build --wait-seconds 900 -- cargo "$@"
  fi
}
scope=(--locked --workspace --exclude ink-platform-macos --target "$target")
guarded_cargo check "${scope[@]}"
guarded_cargo clippy "${scope[@]}" --all-targets -- -D warnings
