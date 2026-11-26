# shookshok(슉쇽) 앱 아이콘 시안 10종

발표하는 교사용 화면 캡처·판서 트레이 앱 **shookshok(슉쇽)**(개발자: 찰떡쌤)의 앱 아이콘 시안 10종입니다.  
모든 시안은 **1024×1024 투명 배경 PNG(32bpp RGBA)**로 정규화했으며, `01`~`09`는 macOS/Windows 공통 스퀘어클 타일(824×824 안전 영역, 사방 100px 여백) 기준, `10`은 비정형 다이컷 스티커(최대 폭 880px 중앙 배치) 기준으로 맞췄습니다.

- 한눈에 비교: [`contact-sheet.png`](./contact-sheet.png) (5×2 격자)

---

## 시안 요약 및 소형 가독성 자기 평가

> 32×32 및 16×16 실제 축소 렌더링(라이트 `#F3F4F6`·다크 `#1E2024` 배경)을 확인한 뒤 평가했습니다.

| 번호 | 파일명 | 방향 / 한 줄 콘셉트 | 32px | 16px | 재생성·해석 메모 |
| --- | --- | --- | :---: | :---: | --- |
| 01 | `01-graphite.png` | **정밀 도구**: 기존 임시 아이콘을 무광 아노다이즈드 흑연 질감과 입체감 있는 주황 잉크 방울로 정제 | 상 | 상 | 2차 생성본(`v2`) 채택 |
| 02 | `02-jelly.png` | **말랑 젤리 3D**: 반투명 파스텔 민트-스카이 젤리 쿠션 위, 통통한 코랄 마커가 주황 곡선을 '슉' 긋는 모습 | 상 | 중 | 1차 단일 곡선 시안 채택, 824px 타일로 정규화 |
| 03 | `03-speed.png` | **슉 스피드**: 선명한 코발트-인디고 그라데이션 위 흰 조준틀을 대각선 주황·노랑 속도선이 날렵하게 관통 | 상 | 상 | 2차 생성본(`v2`) 채택 |
| 04 | `04-chaltteok.png` | **찰떡 패밀리**: `chaltteok-mascot.webp` 화풍의 분홍 찹쌀떡 캐릭터가 코코아 조준틀 안에서 형광펜을 든 모습(글자 없음) | 상 | 중 | 1차 채택, 16px에선 표정·펜 세부가 다소 단순화됨 |
| 05 | `05-stationery.png` | **문구점**: 따뜻한 미색 줄노트 종이 타일 위, 사각 메모 윤곽을 빨강 크레용 손그림 동그라미로 빠르게 표시 | 상 | 중 | 후처리에서 배경·하단 그림자 제거 후 824px로 정규화 |
| 06 | `06-chalkboard.png` | **칠판**: 얇은 원목 테두리의 진초록 칠판 타일 위, 흰·노랑 분필의 빠른 대각선 한 획과 가루 질감 | 상 | 상 | 외곽 그림자 제거 재생성본 채택 |
| 07 | `07-glass.png` | **유리 재질**: 미세한 무지개 굴절 테두리가 감도는 반투명 사파이어 젖빛 유리 타일과 음각 느낌의 흰 조준틀 | 상 | 중 | 조준틀을 둥근 닫힌 사각+중앙 점으로 해석; 유리 곡률(`n=3.75`)에 맞춰 투명 마스킹 |
| 08 | `08-hangul.png` | **한글 글자 로고**: 깊은 남색 타일 위 날렵한 흰색 `'슉'` 한 글자와 받침 `'ㄱ'`의 주황 붓·마커 한 획 포인트 | 상 | 중 | 한글 깨짐 방지를 위해 3차(`v3`)까지 재생성해 정확한 `'슉'` 확보 후 824px 정규화 |
| 09 | `09-shutter.png` | **셔터 조리개**: 깊은 청록 바탕 위 코랄·노랑·크림 6날 평면 기하 조리개가 돌며 중앙의 만년필 펜촉을 감쌈 | 상 | 중 | 순수 네거티브 스페이스 대신 중앙에 크림색 펜촉을 세운 2차(`v2`)로 소폭 변주(소형 시인성 우선) |
| 10 | `10-sticker.png` | **스티커**: 굵은 흰 다이컷 테두리 안에 기울어진 남색 화면 프레임을 코랄·시안 꼬리 별똥별이 '슉' 가로지름 | 상 | 중 | 초기 HTTP 429 한도 초과 후 재시도로 생성, 단색 슬레이트 배경을 투명 처리·디매팅 |

---

## 시안별 상세 및 실제 사용한 영어 생성 프롬프트

### 01. `01-graphite.png` — 정밀 도구
- **한 줄 콘셉트**: 현재 임시 아이콘(`ref/current-icon.png`)의 조형(흑연 타일 + 흰 조준틀 + 주황 잉크 방울)을 유지하면서 무광 금속 질감과 절제된 입체감으로 완성도를 끌어올린 프로 유틸리티 아이콘.
- **소형 가독성**: 32px **상** / 16px **상** — 어두운 흑연 타일과 고대비 흰 브래킷·주황 방울 덕분에 라이트·다크 작업표시줄과 트레이 모두에서 가장 안정적으로 읽힘.
- **참고 이미지**: `ref/current-icon.png`
- **사용 프롬프트**:
  > App icon design for a macOS and Windows precision screen-capture and annotation utility, refining the reference icon into a polished professional desktop icon. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile has a sleek matte anodized dark graphite surface (#222428 to #181A1E) with a very subtle precision-machined top rim highlight. Inside the safe center, four bold, geometrically crisp warm off-white rounded-corner viewfinder brackets frame a single smooth, vibrant amber-orange ink droplet (#FF8A1E to #FF6A00) with a subtle satin specular sheen and crisp soft depth. Ultra-clean, minimal Swiss industrial design aesthetic, razor-sharp silhouette clearly readable at 16px. No text, no numbers, no watermark, no neon glow, no sparkles, no extra objects.

### 02. `02-jelly.png` — 말랑 젤리 3D
- **한 줄 콘셉트**: 반투명 민트-스카이 젤리 쿠션 타일 위에 통통한 코랄 마커 펜이 윤기 있는 주황 곡선을 '슉' 하고 그리는 아기자기한 3D 점토·젤리 아이콘.
- **소형 가독성**: 32px **상** / 16px **중** — 32px에서는 펜과 곡선이 귀엽고 뚜렷하지만, 16px 라이트 배경에서는 밝은 파스텔 외곽 대비가 다소 부드러워짐.
- **사용 프롬프트**:
  > App icon design in a charming soft 3D translucent jelly and smooth matte clay style. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile is a soft, plump, translucent pastel mint-cream and sky-blue jelly cushion with gentle rounded 3D edges and soft studio lighting. Floating cleanly in the center is a single chubby, tactile pastel coral-pink and white marker pen tilted diagonally, actively drawing one bold, glossy translucent tangerine-yellow curved swoosh ribbon ('shook' motion trail) across the tile. Toy-like, clean, uncluttered minimalist 3D clay/jelly render, strong contrast and simple bold silhouette clearly readable at 16px. No text, no letters, no numbers, no sparkles, no clutter, no watermark.

### 03. `03-speed.png` — 슉 스피드
- **한 줄 콘셉트**: 짙은 코발트-인디고 그라데이션 바탕의 흰 캡처 조준틀을 강렬한 주황·노랑 기하 속도선이 대각선으로 가르며 '키 한 번에 슉'의 즉각적인 속도를 드러내는 아이콘.
- **소형 가독성**: 32px **상** / 16px **상** — 파랑 바탕, 흰 틀, 주황 대각선 사선의 보색 대비가 강해 16px에서도 속도감 있는 실루엣이 그대로 살아남.
- **사용 프롬프트**:
  > Modern bold graphic app icon emphasizing instant speed and fast screen capture. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile features a vivid electric cobalt-blue to deep royal-indigo gradient background. In the center, a crisp white bracketed capture selection square is dynamically sliced diagonally from bottom-left to top-right by a single thick, razor-sharp vibrant vermilion-orange and golden-yellow aerodynamic speed swoosh with clean geometric motion-blur trails. High-contrast vector-grade graphic design, ultra-clean kinetic composition, single bold silhouette readable even at 16px. No text, no numbers, no watermark, no messy lens flare, no neon fog, no extra icons.

### 04. `04-chaltteok.png` — 찰떡 패밀리
- **한 줄 콘셉트**: 찰떡쌤 마스코트(`ref/chaltteok-mascot.webp`)의 부드러운 분홍 찹쌀떡 캐릭터가 따뜻한 살구빛 타일의 코코아 조준틀 안에서 노란 마커를 들고 반갑게 바라보는 패밀리형 아이콘(글자 없음).
- **소형 가독성**: 32px **상** / 16px **중** — 32px에서는 표정과 펜까지 매력적으로 보이지만 16px에서는 분홍 떡 실루엣과 갈색 모서리만 남고 얼굴 세부가 뭉개짐.
- **참고 이미지**: `ref/chaltteok-mascot.webp`
- **사용 프롬프트**:
  > App icon design in the warm, soft-painted cute character illustration style of the reference pink mochi (rice-cake) mascot, with ZERO text or Korean letters anywhere. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile has a clean warm cream-apricot soft pastel background. Framed cleanly in the center by four crisp dark-cocoa rounded capture corner brackets, the adorable pink teardrop-shaped mochi character (matching the reference's soft pink shade, gently curved top tip, big expressive dark brown eyes, rosy cheeks, and happy smile) leans forward playfully holding a single chunky yellow-orange marker pen, ready to draw. Keep the composition super clean and uncluttered—no floating beans, no teacup, no ribbon clutter, no text whatsoever—so the pink mochi face and capture frame form a strong, adorable silhouette readable at 32px and 16px.

### 05. `05-stationery.png` — 문구점
- **한 줄 콘셉트**: 따뜻한 미색 줄노트 종이 위에 짙은 슬레이트 사각 카드 윤곽을 빨강 색연필·크레용 손그림 동그라미로 슥 표시한 아날로그 교실·문구 감성 아이콘.
- **소형 가독성**: 32px **상** / 16px **중** — 빨강 동그라미의 손맛은 32px에서 매력적이나, 16px 라이트 배경에서는 미색 타일 외곽과 얇은 사각 선이 다소 흐려짐.
- **사용 프롬프트**:
  > App icon design with a warm analog stationery and classroom notebook aesthetic. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely flat, solid uniform cool light-gray (#CBD5E1) background with no drop shadow so the tile edge is crisp. The tile looks like thick, warm cream-ivory textured sketch paper (#FAF4E8) with subtle soft paper grain and very faint warm horizontal notebook guide lines. Drawn boldly right in the center is a single expressive, imperfect hand-drawn loop circle in rich vermilion-red wax crayon and colored pencil texture, overlapping a minimal dark-slate rectangular note card outline, with one crisp angled pencil stroke tail showing quick marking motion. Charming tactile craft feel, minimal and uncluttered, strong red-on-cream contrast clearly recognizable at 16px. No text, no letters, no numbers, no watermark, no extra stationery props.

### 06. `06-chalkboard.png` — 칠판
- **한 줄 콘셉트**: 깔끔한 원목 베벨 테두리가 둘러진 진초록 칠판 타일 위에, 흰색과 노란 분필이 빠르게 그어 지나간 대각선 한 획과 절제된 분필 가루를 담은 교실 아이콘.
- **소형 가독성**: 32px **상** / 16px **상** — 짙은 녹색 칠판 위 밝은 대각선 분필 한 획이 단순하고 대비가 커서 16px에서도 칠판+판서 정체성이 즉시 보임.
- **사용 프롬프트**:
  > App icon design inspired by a classic school classroom chalkboard. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile is a rich, deep matte forest-green slate chalkboard (#1D4334 to #143226) framed by a subtle, sleek warm-oak wooden inner bevel edge. Across the center is one bold, swift, sweeping diagonal white-and-warm-yellow chalk swoosh stroke curving cleanly like a fast highlight mark, accompanied by restrained, crisp fine chalk dust texture along the stroke edge and two minimal corner chalk tick marks hinting at a screen frame. Clean, iconic, high-contrast classroom design, instantly legible at 16px. No text, no letters, no numbers, no erasers or cluttered props, no watermark.

### 07. `07-glass.png` — 유리 재질
- **한 줄 콘셉트**: 절제된 무지개 빛 굴절 베벨이 감도는 macOS 스타일 사파이어 젖빛 유리 타일 중앙에, 흰색 둥근 조준틀과 중심점이 새겨진 세련된 글래스모피즘 아이콘.
- **소형 가독성**: 32px **상** / 16px **중** — 32px 이상에서는 유리 질감과 굴절 테두리가 돋보이지만, 16px에서는 단순한 파란 사각 버튼처럼 보일 여지가 있음.
- **사용 프롬프트**:
  > App icon design in a refined modern macOS frosted glass (glassmorphism) aesthetic. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile is crafted from thick, luminous translucent frosted sapphire-ice glass (#DCE8F7 to #9BB8DF) with a crisp, subtle prismatic rainbow light-refraction sheen along its beveled glass rim—refined and tasteful, not gaudy. Deeply etched and frosted into the center of the glass is a bold, clean pure-white rounded viewfinder square frame with a crisp glowing-white precision center dot/stylus tip inside, casting a delicate internal glass refraction shadow for high contrast. Ultra-clean Apple industrial glass craftsmanship, strong bold geometry readable even at 16px. No text, no numbers, no watermark, no messy lens flares, no excessive neon glow.

### 08. `08-hangul.png` — 한글 글자 로고
- **한 줄 콘셉트**: 짙은 남색 타일 한가운데 한글 `'슉'` 한 글자를 단단한 타이포그래피로 세우고, 마지막 `'ㄱ'` 내리긋기 획을 날렵한 코랄 주황 붓·마커 터치로 처리한 한글 워드마크 아이콘.
- **소형 가독성**: 32px **상** / 16px **중** — 32px에서는 `'슉'`이 또렷하게 읽히며 브랜드 개성이 강함. 16px에서는 고유한 실루엣과 주황 포인트는 살아 있으나 자모 획(`ㅠ`의 두 세로획)은 픽셀 한계로 붙어 보임.
- **재생성 메모**: 1~2차에서 자모 균형을 보완해 3차(`v3`)에서 깨짐 없는 정확한 `'슉'`을 채택함.
- **사용 프롬프트**:
  > Minimalist modern Korean typographic app icon featuring ONE single bold Korean Hangul character "슉" (composed cleanly of top consonant ㅅ, middle vowel ㅠ with a horizontal bar and two short downward ticks, and bottom consonant ㄱ). A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile is a deep rich midnight-ink navy (#111827). Right in the center, the single Korean syllable "슉" is rendered in crisp pure white with a vibrant coral-orange dynamic brush-marker flick on the final stroke, styled like a bold, sleek, swift modern marker/calligraphy logo mark with slight forward kinetic slant. The Hangul character "슉" must be 100% accurately spelled, balanced, bold, and unmistakable—zero extra characters, zero broken strokes, zero English letters, zero watermark. Clean flat-vector graphic clarity, strong silhouette readable at 32px and 16px.

### 09. `09-shutter.png` — 셔터 조리개
- **한 줄 콘셉트**: 깊은 청록 바탕 위에 코랄·노랑·크림 3색의 6날 평면 기하 카메라 조리개가 회전하며, 중앙에 선명한 만년필 펜촉을 감싸 캡처와 판서를 한 기호로 묶은 아이콘.
- **소형 가독성**: 32px **상** / 16px **중** — 32px에서는 조리개와 펜촉이 모두 명확히 읽히나, 16px에서는 화려한 원형 조리개 패턴이 주가 되고 중앙 펜촉은 작은 점으로 축소됨.
- **해석 변주 메모**: 조리개 날개 사이 빈 공간만으로 펜촉 실루엣을 만드는 시도(3회)보다, 중앙 원 안에 독립된 크림색 펜촉을 배치한 `v2`가 32px 크기에서 훨씬 또렷해 이를 채택함.
- **사용 프롬프트**:
  > Bold flat geometric app icon combining screen capture and screen drawing in one clever negative-space mark. A centered macOS-style rounded square (squircle) tile filling about 82% of the 1024x1024 frame on a completely plain, flat pure solid white (#FFFFFF) background with no cast shadow bleeding onto the white background. The tile has a crisp deep-teal (#0E3B43) background. Filling the center is a bold, mathematically clean swirling 6-blade geometric camera shutter aperture rendered in vibrant flat blocks of coral-red (#FF5A36), warm marigold-yellow (#FFB703), and crisp cream-white (#FFF8EE), where the central aperture opening cleanly forms the unmistakable upright silhouette of a sharp stylus/fountain pen nib pointing upward. Mid-century modern Swiss flat graphic design, razor-sharp geometric edges, zero gradients or muddy textures, instantly recognizable silhouette at 16px. No text, no numbers, no watermark, no sparkles.

### 10. `10-sticker.png` — 스티커
- **한 줄 콘셉트**: 굵은 흰 다이컷 비닐 스티커 테두리 안에 기울어진 남색 화면 프레임을 코랄·시안 줄무늬 꼬리의 노란 별똥별이 '슉' 지나가는 발랄한 일러스트 아이콘.
- **소형 가독성**: 32px **상** / 16px **중** — 다크 배경(다크 모드 작업표시줄·트레이)에서는 흰 다이컷 테두리가 선명하게 돋보이지만, 라이트 배경 16px에서는 흰 테두리가 배경에 섞여 실루엣이 다소 작아 보임.
- **재생성 메모**: 첫 생성 요청이 일시적 API 한도(HTTP 429)로 중단되어 재시도해 생성함.
- **사용 프롬프트**:
  > Playful die-cut vinyl sticker style app icon (NOT a square tile—an organic die-cut silhouette) centered cleanly inside the square canvas with generous margin on a completely flat, solid uniform medium slate-gray (#64748B) background, with NO drop shadow on the background. The sticker features a crisp, thick, smooth pure-white die-cut outer border wrapping tightly around a lively flat-cel vector illustration: a cheerful golden-yellow comet/shooting star with a bold striped coral-and-cyan swooshing speed tail zipping swiftly diagonally across a tilted, minimal navy-blue screen frame. Crisp bold outlines, vibrant flat pop colors, charming playful illustration with zero clutter, strong silhouette readable at 32px and 16px. No text, no letters, no numbers, no tiny scattered stars outside the white die-cut border, no watermark.
