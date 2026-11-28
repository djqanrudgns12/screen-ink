# 새 장면(Hold-out) 인식 시험 사례

`bench/recog/cases/`(287점 기준선)와 **완전히 분리된 과적합 확인 전용** 사례 폴더입니다.

## 꼭 지킬 규칙

1. **규칙 고칠 때 보지 않는다.** `crates/ink-domain/src/recog.rs`나 `crates/ink-vision`의 규칙·점수·문턱값을 조정할 때 이 폴더의 사례·점별 결과를 열어 보고 맞추지 않습니다(시험 전용).
2. **287점 기준선과 섞지 않는다.** 이 폴더에는 `baseline.json`을 두지 않으며, `bench/recog/cases/baseline.json`에도 넣지 않습니다. CI(`.github/workflows/ci.yml`)의 기존 회귀 시험과 독립으로 운영합니다(추후 `7.3`에서 고정 여부를 따로 정합니다).
3. **개인정보 0.** 시험용 HTML(`bench/recog/pages/`)과 시험 앱(`rb04-win32` 등)으로만 뜨며, 창 제목·접근성 이름·실제 사용자 화면을 남기지 않습니다.

## 3차 관문 (PRD-3차 §2)

- 표본: 기존 287점에 쓰지 않은 화면·배치 **≥ 60점**
- 합격선: **Top-1 ≥ 80%** (늘려 잡음 85%)

## 판정 명령

```sh
# 전체 새 장면 재생 → bench/results/recog-holdout.csv (기준선 비교 없이 보고만)
cargo run -p ink-platform-fake --bin recog-replay -- --holdout

# 관문(Top-1 ≥ 80%) 판정
cargo run -p ink-platform-fake --bin recog-replay -- --holdout --min-top1 80

# CSV 저장 없이 콘솔 요약만
cargo run -p ink-platform-fake --bin recog-replay -- --holdout --no-csv

# 임의 폴더 지정
cargo run -p ink-platform-fake --bin recog-replay -- <사례 폴더> <CSV 경로>
```
