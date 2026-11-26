# shookshok(슉쇽) 앱 아이콘 최종 시안 3종 (`21`~`23`)

2차 시안 `15-aperture-nib-mono.png`의 **6날 조리개 + 중앙 음각 만년필 펜촉 + 숨구멍 주황 점** 착상을 계승하면서, 사용자 피드백을 반영해 다음 사항을 리뉴얼한 최종 시안 3종입니다.

- **색상**: 청록 계열을 완전히 제거하고 **무채색(흰색·회색·먹색/흑연) 명암 + 주황(`#FF6A1A`) 포인트 한 곳(펜촉 숨구멍)**으로 절제 (`12`·`14`·`18` 톤).
- **형태**: 둥근 사각(squircle) 타일을 완전히 제거하고 **원형 조리개 원반(지름 880px, 1024×1024 정중앙)** 자체를 아이콘으로 구성. 원 바깥은 투명(알파 0)이며, 원 아래쪽에만 캔버스 내부에서 완전히 소멸하는 아주 옅은 바닥 그림자를 적용(가장자리 잘림 없음).
- **기하 통일**: 3종 모두 동일한 6날 조리개 곡률·내부 허브 비율·전체 높이 음각 펜촉 실루엣을 공유해 나란히 비교하기 쉽게 제작.
- **비교 미리보기**: [`preview.png`](./preview.png) — 흰 바탕(`#FFFFFF`) 1줄 + 짙은 바탕(`#1E1E1E`) 1줄, 각 칸 256px + 32px + 16px 병렬 배치.

---

## 스스로 꼽은 1순위 추천

**`22-aperture-black.png` (검은 조리개)**  
무광 먹색·흑연 6날 조리개에 위쪽에서만 은은하게 떨어지는 빛과 가느다란 은회색 외곽 베벨을 더해, 12·14·18이 가진 차분한 macOS 하드웨어 감성을 가장 완성도 높게 구현했습니다. 특히 32px·16px 트레이 크기로 줄였을 때 **짙은 원반 안의 흰 만년필 펜촉 + 주황 점** 대비가 가장 군더더기 없이 또렷하며, 흰 작업 표시줄과 짙은(`#1E1E1E`) 작업 표시줄 양쪽 모두에서 경계와 중심 실루엣이 안정적으로 읽힙니다.

---

## 시안 요약 및 소형 가독성 자기 평가

> [`preview.png`](./preview.png)의 흰 바탕(`#FFFFFF`)과 짙은 바탕(`#1E1E1E`) 실제 32px·16px 축소 렌더링을 직접 육안 확인한 평가입니다.

| 번호 | 파일명 | 한 줄 콘셉트 | 32px (라이트 / 다크) | 16px (라이트 / 다크) | 펜촉 음각·재생성 검수 메모 |
| :---: | --- | --- | :---: | :---: | --- |
| 21 | `21-aperture-white.png` | **흰 조리개**: 무광 백자 6날 조리개(`14` 질감) + 얇은 회색 외곽 베벨 + 먹색 음각 펜촉 + 주황 숨구멍 점 | 상 / 상 | 중 / 상 | 1차 채택; 뾰족한 먹색 음각 펜촉과 흰 슬릿·주황 점이 뚜렷함. 16px 순백 바탕에서는 외곽이 다소 부드러우나 회색 림과 중앙 먹색 펜촉으로 식별 가능 |
| 22 | `22-aperture-black.png` | **검은 조리개**: 무광 흑연·먹색 6날 조리개(상단 은은한 빛 + 얇은 회색 림) + 흰 음각 펜촉 + 주황 숨구멍 점 | 상 / 상 | 상 / 상 | 1차 채택; 라이트·다크 양쪽 32px·16px 모두에서 원형 윤곽과 흰 펜촉 실루엣이 가장 또렷하고 균형 잡힘 |
| 23 | `23-aperture-duo.png` | **흑백 교차**: `18` 스타일의 얇은 브러시드 은회색 링 + 흰·검정 3장씩 교차한 6날 조리개 + 흰 음각 펜촉 + 주황 숨구멍 점 | 상 / 상 | 상 / 상 | 2차 재생성(`v2`) 채택; 1차본의 뭉툭한 펜촉 끝과 날개 비율을 `21`·`22`와 동일한 조리개 기하 및 뾰족한 전체 높이 음각 펜촉으로 교정 |

---

## 규격 및 원 바깥 알파 0 검증 결과

`1024×1024` 32bpp RGBA PNG 정규화 직후 픽셀 단위 스크립트 검사로 확인한 결과입니다.

| 파일명 | 해상도 / 포맷 | 원반 지름(불투명 영역) | 4개 모서리 알파 | 1px 외곽 테두리 최대 알파 | 상단·좌우 원 바깥(`r>442`, `y≤512`) 최대 알파 | 하단 옅은 그림자 최대 알파 / 종료 `y` (캔버스 끝 1023) |
| --- | --- | :---: | :---: | :---: | :---: | :---: |
| `21-aperture-white.png` | `1024×1024` `Format32bppArgb` | `878×878` (`73..950`, 안티앨리어싱 포함 `880px`) | `0` | `0` (잘림 없음) | `0` | `36/255` (~14%) / `y=982` (하단 여유 41px `alpha=0`) |
| `22-aperture-black.png` | `1024×1024` `Format32bppArgb` | `878×878` (`73..950`, 안티앨리어싱 포함 `880px`) | `0` | `0` (잘림 없음) | `0` | `36/255` (~14%) / `y=982` (하단 여유 41px `alpha=0`) |
| `23-aperture-duo.png` | `1024×1024` `Format32bppArgb` | `878×878` (`73..950`, 안티앨리어싱 포함 `880px`) | `0` | `0` (잘림 없음) | `0` | `36/255` (~14%) / `y=982` (하단 여유 41px `alpha=0`) |

---

## 시안별 상세 및 실제 사용한 영어 생성 프롬프트

### 21. `21-aperture-white.png` — 흰 조리개
- **한 줄 콘셉트**: `14`의 무광 백자 감성을 6날 조리개 원반으로 옮기고, 흰 바탕에서도 경계가 살도록 얇은 쿨그레이 외곽 베벨을 두른 뒤 중앙에 먹색 음각 만년필 펜촉과 주황 숨구멍 점 하나를 배치한 시안.
- **사용 프롬프트** (참조 이미지: `round2/15-aperture-nib-mono.png`, `round2/14-aperture-swoosh.png`):
  > Standalone perfectly circular 6-blade swirling camera shutter aperture disc centered cleanly in a 1024x1024 frame (~86% diameter), NO rounded-square/squircle tile, placed on a completely flat, solid uniform medium-dark neutral slate-gray backdrop (#555A64) with ZERO cast shadow bleeding onto the backdrop. Keep the exact iconic structure of reference 15: a flush circular outer rim with 6 overlapping swirling shutter blades framing a central circular hub whose carved negative-space opening forms an unmistakable, sharp, upright classic fountain pen nib silhouette pointing straight up at 12 o'clock (clear diamond-pointed shoulders, crisp narrow vertical ink slit from top tip to center, and one small round breather hole). STRICTLY achromatic white/gray/black + ONE tiny orange dot (ZERO teal/blue/cyan): all 6 swirling blades and inner nib-framing collar sculpted in clean matte white porcelain / satin alabaster ceramic (#FAFAFC top-lit to #DDE0E6 soft cool-gray contact shadows) under one soft overhead studio light, with a delicate thin neutral light-gray outer bevel rim (#C6CBD3) so the white circle stays crisp even on a pure white taskbar. Central recessed negative-space fountain pen nib cavity is deep matte charcoal-ink black (#16181D) with a crisp pure white (#F8F9FA) vertical ink slit and ONE small satin vivid orange breather dot (#FF6A1A) as the sole color accent. Apple macOS industrial craft, no neon glow, no sparkles, no glossy plastic shine, no text or watermark.

### 22. `22-aperture-black.png` — 검은 조리개
- **한 줄 콘셉트**: 무광 먹색·아노다이즈드 흑연 6날 조리개 원반에 위쪽의 은은한 스튜디오 빛과 얇은 회색 림 하이라이트를 더하고, 중앙 음각 만년필 펜촉을 순백색으로 비워 주황 숨구멍 점을 강조한 시안.
- **사용 프롬프트** (참조 이미지: `round2/15-aperture-nib-mono.png`, `round2/12-giyeok-nieun-frame.png`):
  > Standalone perfectly circular 6-blade swirling camera shutter aperture disc centered cleanly in a 1024x1024 frame (~86% diameter), NO rounded-square/squircle tile, placed on a completely flat, solid uniform pure white backdrop (#FFFFFF) with ZERO cast shadow bleeding onto the backdrop. Keep the exact iconic structure of reference 15: a flush circular outer rim with 6 overlapping swirling shutter blades framing a central circular hub whose carved negative-space opening forms an unmistakable, sharp, upright classic fountain pen nib silhouette pointing straight up at 12 o'clock (clear diamond-pointed shoulders, crisp narrow vertical ink slit from top tip to center, and one small round breather hole). STRICTLY achromatic charcoal/graphite/white + ONE tiny orange dot (ZERO teal/blue/cyan): all 6 swirling blades and inner nib-framing collar sculpted in sleek matte charcoal / bead-blasted dark anodized graphite (#2E3138 softly lit upper surfaces to #13151A deep matte charcoal shadows) under one gentle overhead studio light, with a whisper-thin satin mid-gray hairline rim highlight (#565B66) around the circumference so the dark disc remains distinct on a dark #1E1E1E taskbar. Central recessed negative-space fountain pen nib cavity is luminous matte pure white (#F6F7F9) with a crisp deep matte charcoal (#15171C) vertical ink slit and ONE small satin vivid orange breather dot (#FF6A1A) as the sole color accent. Apple macOS industrial hardware craft, no neon glow, no sparkles, no glossy plastic shine, no text or watermark.

### 23. `23-aperture-duo.png` — 흑백 교차
- **한 줄 콘셉트**: `18`의 돋보기 테두리처럼 얇은 브러시드 은회색 금속 링 안에 무광 백자 날개 3장과 무광 흑연 날개 3장을 번갈아 배치하고, `22`와 동일한 중앙 흰 음각 펜촉과 주황 숨구멍 점을 결합한 시안.
- **사용 프롬프트 (`v2` 채택)** (참조 이미지: `raw_22_aperture_black`, `raw_21_aperture_white`, `round2/18-lens-stroke.png`):
  > Standalone perfectly circular 6-blade swirling aperture disc matching the exact sculpted 3D bevel geometry, inner charcoal circular hub proportion, and full-height pointed white negative-space upright fountain pen nib (+ vertical dark charcoal slit and single vivid orange breather dot #FF6A1A) of variant 22, wrapped cleanly in a sleek, slim precision-machined brushed satin silver-gray outer metal ring bezel (neutral silver-gray #D4D8DF to #888E98, inspired by reference 18's lens ring). Inside the silver-gray ring, the 6 swirling shutter blades strictly alternate 3 matte porcelain white blades (#F8F9FB with gentle cool-gray bevel shadows) and 3 matte charcoal-black anodized graphite blades (#23262C with gentle overhead bevel highlight) — white, black, white, black, white, black around the circle. Central inner circular collar is matte charcoal-graphite (#22252B), framing a razor-sharp pointed full-height pure white (#F8F9FA) negative-space fountain pen nib touching top and bottom of the inner circular hub, with a crisp dark charcoal vertical ink slit and ONE satin vivid orange breather dot (#FF6A1A). Strictly neutral achromatic white/gray/black + silver ring + single orange dot (ZERO teal/blue tint), flat medium neutral gray backdrop (#6B7280) with ZERO outer drop shadow and ZERO squircle box.
