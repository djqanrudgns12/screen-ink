# 슉쇽 UI 글꼴 후보 조사 · 견본 (2026-11-27)

`~/Desktop/Architecture/fonts/`(woff2 59개) 전수 분류와 공식 라이선스 확인, 실제 앱 문자열(한·영, 11~20px, 트레이 팝오버, 다크 아일랜드, 숫자·키캡) 렌더링 비교 결과다.
글꼴 파일은 라이선스·결정 확정 전이므로 저장소에 복사하지 않았고, 견본 이미지만 저장했다.

> **한눈에 보기**
> - 나란히 비교(기준선 3종 + 폴더 후보 5종): [compare.png](compare.png)
> - 트레이 팝오버 + 다크 아일랜드 실전 비교(6종): [tray-mock.png](tray-mock.png)
> - 개별 견본: [freesentation.png](freesentation.png) · [paperlogy.png](paperlogy.png) · [gmarket-sans.png](gmarket-sans.png) · [elice-dx-neolli.png](elice-dx-neolli.png) · [bookk-gothic.png](bookk-gothic.png) · [pretendard-ref.png](pretendard-ref.png)

---

## 1. 후보 · 기준선 비교표

실측 기준: Chromium(macOS, 2x DPR)에서 `file://`로 로컬 woff2 로드(Pretendard만 공식 jsDelivr CDN 웹 참고). 숫자 폭은 20px 기준 `0`~`9` 개별 실측.

| 구분 | 글꼴 | 폴더 보유 굵기 (파일 수) | woff2 크기 (파일당 / 합) | 라이선스 (공식 출처) | 한글·라틴 평 | 숫자·기호 평 (`00:12` `×2.2` `1920×1080` `⌥ F1`) | 견본 |
|---|---|---|---|---|---|---|---|
| **폴더 후보 1** | **Freesentation**<br>(프리젠테이션) | `400` `500` `600` `700` (4종)<br>*(공식 100~900 + VF)* | 837~871 KB<br>(합 3.34 MB; 4~600 2.55 MB) | **SIL OFL 1.1**<br>[공식](https://freesentation.blog/freesentation) · [전문](https://www.designptn.com/wp-content/uploads/2024/03/OFL-license.txt) | 본고딕 한글(11,172자 완비) + Roboto 라틴. 11~13px 또렷하고 세련됨. 살짝 좁은 자폭(20px `'한'` 15.0px)으로 정보 밀도 우수 | **기본 고정폭 숫자**(`10.88px` 전 숫자 동일). `tabular-nums` 없이도 시간·배율·해상도 세로 정렬 완벽. `⌥⌘⇧`는 OS 폴백 | [견본](freesentation.png) |
| **폴더 후보 2** | **Paperlogy**<br>(페이퍼로지) | `500` `600` `700` `800` `900` (5종)<br>*(**400 미보유**, 공식에는 100~900)* | 157~164 KB<br>(합 806 KB) | **SIL OFL 1.1**<br>[공식](https://freesentation.blog/paperlogyfont) · [전문](https://www.designptn.com/wp-content/uploads/2024/08/OFL-license.txt) | G마켓 산스 한글 + Montserrat 라틴. 기하학적이고 감각적이나 폴더에 `400`이 없어 본문(`500`)이 무거움. **한글 2,780자 제한**(`큑` 빈 글리프 공백) | **기본 고정폭 숫자**(`13.33px` 전 숫자 동일). 둥글고 또렷한 기하학 숫자가 다크 아일랜드에서 돋보임. `⌥⌘⇧`는 OS 폴백 | [견본](paperlogy.png) |
| **폴더 후보 3** | **Gmarket Sans**<br>(지마켓 산스) | `300` `500` `700` (3종)<br>*(공식도 3종, 400 없음)* | 459~596 KB<br>(합 1.54 MB) | **SIL OFL**<br>[공식](https://corp.gmarket.com/fonts/) *(버전·RFN 전문 일부 [미확인])* | 한글 11,172자 완비. 친근한 네모꼴 기하학 고딕이나 **`400` 부재**(`300`은 11~13px에서 얇고 흐림, `500`은 굵음). 맥 유틸리티 감성과 다소 거리 | **비례 숫자**(`9.04~13.86px`, `tnum` 미지원). `11:11`과 `88:59` 폭 차이 커서 별도 고정폭 폰트 필수 | [견본](gmarket-sans.png) |
| **폴더 참고 4** | **Elice DX Neolli**<br>(엘리스 DX널리체) | `300` `500` (2종)<br>*(공식 L/R/B 3종)* | 335 · 362 KB<br>(합 697 KB) | **원문 [미확인]**<br>[공식 브랜드](https://elice.io/ko/resources/brand) *(2차 사이트만 OFL 안내)* | `300`이 일반 `400` 근처 굵기라 13px 가독성 무난하나 폴더 2종뿐. **한글 2,780자 제한**(`큑` 토푸 박스 `[]` 노출) | **비례 숫자**(`11.20~15.64px`, `tnum` 미지원). 숫자 자폭 편차 큼 | [견본](elice-dx-neolli.png) |
| **폴더 참고 5** | **Bookk Gothic**<br>(부크크 고딕) | `300` `700` (2종)<br>*(공식도 2종뿐)* | 377 · 372 KB<br>(합 749 KB) | **자체 무료 약관**<br>[공식](https://bookk.co.kr/font) *(수정 금지·임베딩 [미확인])* | 한글 11,172자 완비, 단정하고 세련된 인상이나 **중간 굵기(400·500·600) 전무**해 `300`은 11~12px에서 지나치게 가늘고 `700`과 단차 과대 | **기본 고정폭 숫자**(`12.52px` 전 숫자 동일). 깔끔하나 굵기·라이선스 제약으로 UI 본문 탈락 | [견본](bookk-gothic.png) |
| **기준 · 현재** | **IBM Plex Sans KR**<br>+ IBM Plex Mono | `400` `500` `600`<br>+ Mono `400` `500` | Sans ~1.8–1.9 MB/굵기<br>Mono 32 KB (latin 2종) | **SIL OFL 1.1**<br>[GitHub](https://github.com/IBM/plex/blob/master/LICENSE.txt) | 정밀·중립적 기술 도구 인상, 한글 11,172자. 다만 단정하기보다 다소 딱딱하고 기본 줄높이 메트릭이 커 트레이 팝오버 세로가 길어짐 | Sans KR 자체도 기본 고정폭(`12.00px`), 현재는 Mono(`12.00px`)와 병용해 시간·키캡 정렬 안정적 | [비교](compare.png) |
| **기준 · OS** | **시스템 글꼴**<br>(macOS / Windows) | OS 가변/다중 굵기<br>(번들 0 KB) | 0 KB | OS 호출 무료<br>*(파일 추출·앱/PDF 임베딩 금지)* | macOS(`SF Pro + Apple SD Gothic Neo`)는 가장 자연스러운 맥 감성. Windows(`Segoe UI Variable + 맑은 고딕`)는 한글 맑은 고딕 인상 편차 큼 | macOS `SF Pro`는 `tabular-nums` 완벽 지원(`12.03px`) + `⌥⌘⇧` 기호 원생 지원 | [비교](compare.png) |
| **기준 · 웹 OFL** | **Pretendard**<br>(폴더 미보유) | 공식 100~900 (9종)<br>+ Variable | Static ~400 KB~1 MB<br>(Dynamic subset 분할 가능) | **SIL OFL 1.1**<br>[GitHub](https://github.com/orioncactus/pretendard/blob/main/LICENSE) | Apple SD Gothic Neo + Inter 기반으로 **맥 감성 + 크로스플랫폼 일관성** 모두 최상. 한·영 균형과 11~13px 선명도 최고 수준, 한글 11,172자 | `tabular-nums` 완벽 지원(`12.29px` 동일). `0`·`1` 형태 세련됨 | [견본](pretendard-ref.png) |

---

## 2. 라이선스 공식 출처 확인 요약

공식 페이지·원문 LICENSE 파일을 직접 열어 확인했다. 공식 원문에 명시되지 않은 항목은 `[미확인]`으로 표시한다.

| 글꼴 | 라이선스 유형 | 공식 출처 URL | 상업 사용 | 앱 번들·임베딩·재배포 | PDF 임베딩 | 표기·고지 의무 및 주의점 |
|---|---|---|---|---|---|---|
| **Freesentation** | SIL OFL 1.1 | [freesentation.blog/freesentation](https://freesentation.blog/freesentation)<br>[OFL-license.txt](https://www.designptn.com/wp-content/uploads/2024/03/OFL-license.txt) | 허용 | **허용** (단독 유료 판매·라이선스 변경 금지) | **허용** (OFL 1.1 §5) | 앱 번들 시 저작권 표시 + OFL 1.1 전문 포함 필수(§2). Reserved Font Name 없음 → 서브셋·최적화 자유 |
| **Paperlogy** | SIL OFL 1.1 | [freesentation.blog/paperlogyfont](https://freesentation.blog/paperlogyfont)<br>[OFL-license.txt](https://www.designptn.com/wp-content/uploads/2024/08/OFL-license.txt) | 허용 | **허용** (단독 유료 판매·라이선스 변경 금지) | **허용** (OFL 1.1 §5) | 앱 번들 시 저작권 표시 + OFL 1.1 전문 포함 필수(§2). 한글 2,780자 제한은 기술 제약 |
| **Gmarket Sans** | SIL OFL *(버전 미기재)* | [corp.gmarket.com/fonts/](https://corp.gmarket.com/fonts/) | 허용 | **허용** (공식: SIL OFL, 자유 수정·재배포 명시) | **허용** (SIL OFL 기준) | OFL §2 기준 저작권·전문 동봉 권장. **`[미확인]`**: 공식 웹페이지에 OFL 1.1 버전 명시 및 원문 LICENSE·Reserved Font Name 링크 없음 → 배포 ZIP 동봉 LICENSE 재확인 필요 |
| **Elice DX Neolli** /<br>**Digital Baeum** | `[미확인]`<br>*(2차 사이트만 OFL 안내)* | [elice.io/ko/resources/brand](https://elice.io/ko/resources/brand) | 무료 배포 언급 *(세부 조건 [미확인])* | **`[미확인]`** | **`[미확인]`** | 현행 공식 브랜드 페이지 본문에 라이선스 약관·전문 미게시 → 채택 전 다운로드 ZIP 동봉 LICENSE 또는 `contact@elice.io` 공식 확인 필수 |
| **Bookk Gothic** /<br>**Bookk Myungjo** | 자체 무료 약관 *(비-OFL)* | [bookk.co.kr/font](https://bookk.co.kr/font) | 허용 | **`[미확인]`** *(임베딩·재배포 조항 부재)* | **`[미확인]`** | ① **임의 수정·편집 금지**("배포되는 형태 그대로 사용") → TTF→WOFF2 변환·서브셋 적법성 `[미확인]`, ② 출처 표기 권장, ③ 사용 결과물이 `(주)부크크` 홍보에 활용될 수 있다는 조항 존재 |
| **Eulyoo1945** *(참고)* | 자체 무료 약관 | [eulyoo.co.kr/member/font.php](https://www.eulyoo.co.kr/member/font.php) | 인쇄·웹 디자인 등 허용 | **금지** ("임베이딩, 서버·디바이스 복사·배포 불가") | **금지** (임베딩 불가 명시) | 본문 사용 시 출처 표기 요구. 앱·PDF 임베딩 불가로 즉시 제외 |
| **RIDIBatang** *(참고)* | SIL OFL 1.1 | [ridicorp.com/ridibatang/](https://ridicorp.com/ridibatang/) | 허용 | **허용** | **허용** (OFL 1.1 §5) | 저작권·OFL 1.1 전문 포함 필수(어려울 시 출처 표기 권장). 명조 단일 굵기라 UI 본문 제외 |
| **Pretendard** *(기준)* | SIL OFL 1.1 | [github.com/orioncactus/pretendard/blob/main/LICENSE](https://github.com/orioncactus/pretendard/blob/main/LICENSE) | 허용 | **허용** | **허용** (OFL 1.1 §5) | 저작권·OFL 1.1 전문 포함 필수. RFN: `Pretendard`, `Source`, `Inter`, `M PLUS 1` (서브셋·수정 시 이름 규칙 준수 필요) |
| **IBM Plex Sans KR / Mono** *(현재)* | SIL OFL 1.1 | [github.com/IBM/plex/blob/master/LICENSE.txt](https://github.com/IBM/plex/blob/master/LICENSE.txt) | 허용 | **허용** | **허용** (OFL 1.1 §5) | 저작권·OFL 1.1 전문 포함 필수. RFN: `Plex` |

---

## 3. 추천 상위 3과 역할 배분

사용자 요청("세련되고 깔끔한 감성, 맥 감성, 한글+영어 UI")과 라이선스 안전성, 폴더 보유 여부를 종합해 **상위 3안**을 정리한다.

### 추천 1위 (폴더 내 1순위 · 바로 적용 가능): **Freesentation**
- **까닭**: 폴더 59개 파일 중 **400·500·600·700 4종 완비 + 한글 11,172자 + SIL OFL 1.1 공식 전문 확인 + 기본 고정폭 숫자**를 모두 통과하는 유일한 글꼴이다. `tray-mock.png`에서 보듯 현재 IBM Plex Sans KR보다 수직 리듬이 훨씬 단정하고, 11~13px 메뉴·보조 글에서 한·영 모두 또렷하다.
- **주의할 점**: 자폭이 살짝 좁아(PPT 최적화 특성) 정사각형 비율의 맥 기본체보다 세로로 단정한 인상을 준다. 400/500/600 3종 합이 약 2.55 MB이므로 앱에 넣을 때는 OFL 1.1 허용 범위 내에서 자주 쓰는 글자·유니코드 범위 분할(또는 공식 v1.006 2,780자 + 폴백 검토)을 고려하면 좋다.

### 추천 2위 (폴더 외 1순위 · "맥 감성" 최상): **Pretendard** *(폴더 외 오픈소스 기준선)*
- **까닭**: `compare.png`와 `tray-mock.png`에서 가장 자연스러운 **macOS 네이티브 감성(SF Pro + Apple SD Gothic Neo 비례)**을 Windows까지 그대로 재현한다. 영어 UI(`Region capture`, `Save folder`)의 Inter 기반 라틴 완성도가 Freesentation(Roboto 기반)보다 한 수 위이고, `tabular-nums`와 한글 11,172자, SIL OFL 1.1까지 흠이 없다.
- **주의할 점**: `~/Desktop/Architecture/fonts/`에는 없으므로 채택 시 `@fontsource`나 공식 GitHub 릴리스에서 별도 가져와야 한다. 폴더 내 글꼴로 한정하지 않는다면 실전 1순위 대안이다.

### 추천 3위 (제목·강조·브랜드 특화): **Paperlogy** *(본문용은 공식 `4Regular` 보강 조건부)*
- **까닭**: G마켓 산스 한글 + Montserrat 라틴 + 기본 고정폭 숫자 + 초경량(woff2 161 KB/굵기) + SIL OFL 1.1. 15~20px 제목, 다크 아일랜드, 소개 페이지·PDF 표지처럼 **세련되고 트렌디한 감성**을 내야 하는 자리에 가장 잘 어울린다.
- **주의할 점**: ① 폴더에 `4Regular`가 없어 `5Medium`을 본문에 쓰면 13px 트레이 메뉴가 지나치게 굵고 빽빽하다(`tray-mock.png` 상단 가운데). UI 본문에 쓰려면 공식 배포처에서 `4Regular`를 반드시 추가해야 한다. ② **한글 2,780자 제한**으로 범위 밖 음절(`큑` 등)이 시스템 폴백 대신 **빈 글리프(공백)**로 출력된다(`paperlogy.png` 확인). 앱 고정 UI 문구("슉쇽", "범위 캡처")는 모두 포함되지만, 사용자 폴더 경로(`~/Pictures/...`)처럼 임의 한글이 들어오는 영역에는 주의가 필요하다.

*(참고: 폴더 내 3+ 굵기 나머지 1종인 **Gmarket Sans**는 `400 Regular` 부재로 11~13px 본문이 너무 얇거나 굵고 `tnum`을 지원하지 않아 상위 3에서 제외했다.)*

### 역할별 추천 조합

| UI 역할 | 안 A — 폴더 내 최선 (즉시 전환) | 안 B — 맥 감성 최상 (폴더 외 OFL) | 안 C — 브랜드 개성 강조 |
|---|---|---|---|
| **UI 본문 · 보조** (11.5~13px) | `Freesentation` 400 / 500 | `Pretendard` 400 / 500 | `Freesentation` 400 / 500 |
| **제목 · 브랜드 · 섬 상태** (13.5~20px) | `Freesentation` 600 | `Pretendard` 600 | `Paperlogy` 600 / 700 *(고정 제목 한정)* |
| **숫자 · 시간 · 배율 · 키캡** (`00:12`, `×2.2`, `1920×1080`, `⌥ F1`) | **`IBM Plex Mono` 400/500 유지** *(32 KB뿐, `0`/`O` 명확)*<br>또는 `Freesentation` 단일 통일(기본 고정폭 숫자) | **`IBM Plex Mono` 400/500 유지**<br>또는 `Pretendard` + `tabular-nums` | `IBM Plex Mono` 400/500 유지 |

> **키캡 기호(`⌥`, `⌘`, `⇧`, `⌃`) 메모**: 로컬 후보 5종과 IBM Plex 모두 macOS 수정자 기호(`⌥⌘⇧⌃`)를 내장하지 않는다. CSS 글꼴 스택 끝에 `-apple-system, BlinkMacSystemFont, 'Segoe UI Symbol'`을 두면 `Apple Symbols`로 떨어져 가늘어지는 현상을 피하고 SF Pro의 또렷한 기호를 그대로 쓸 수 있다.

---

## 4. PDF 단축키 표용 추천

단축키 한 장(`CheatSheet`)을 배포용 PDF로 내보낼 때는 **① SIL OFL 1.1 PDF 임베딩 자유**, **② 8~10pt 고밀도 표 가독성**, **③ 키캡·숫자 열 정렬**이 핵심이다(OS 시스템 폰트인 SF Pro·맑은 고딕과 `Eulyoo1945`는 PDF 임베딩 불가, `BookkGothic`은 `[미확인]`).

1. **폴더 내 추천**: **표 본문 `Freesentation 400/600`** + **키캡 `IBM Plex Mono 500`** (+ 문서 상단 큰 제목에만 `Paperlogy 700` 선택 적용)
   - `Freesentation`은 자폭이 좁아 A4 한 장 3열 단축키 표에 설명을 넉넉히 넣어도 줄바꿈이 적고, SIL OFL 1.1이라 PDF 부분 임베딩·배포에 제약이 없다.
2. **폴더 외 추천**: **`Pretendard 400/600`** + **`IBM Plex Mono 500`** (SIL OFL 1.1).
3. **명조 참고**: 폴더 내 명조 중 `RIDIBatang`만 SIL OFL 1.1이지만 단일 굵기 바탕체이므로, 단축키 참조표처럼 빠른 스캔이 중요한 표 문서에는 산세리프+고정폭 조합이 적합하다.

---

## 5. 시스템 글꼴과 섞는 방안의 장단점

### 방안 ① 라틴/숫자는 시스템 글꼴 + 한글만 후보 글꼴 (`unicode-range` 혼합)
예: `font-family: -apple-system, 'Segoe UI Variable', 'Freesentation', sans-serif;` (또는 라틴 `SF Pro` + 한글 `Freesentation` 분리)
- **장점**: macOS에서 영문·숫자·`⌥⌘⇧` 기호가 SF Pro의 네이티브 광학 크기·힌팅으로 렌더링되고, 한글 서브셋만 번들해 용량을 줄일 수 있다.
- **단점 (권장하지 않음)**:
  - **한·영 혼합 줄의 어긋남**: `영역 캡처 Region capture`, `저장 폴더 ~/Pictures/슉쇽`처럼 한 줄에 한·영이 섞일 때 x-height·베이스라인·획 굵기·자폭이 서로 달라 이질감이 생긴다(특히 자폭이 좁은 `Freesentation`과 넓은 `SF Pro`를 섞으면 한글이 좁아 보임).
  - **OS별 이중 QA 부담**: Windows에서는 `Segoe UI Variable`과 후보 한글의 조합을 따로 검증해야 하고, `Paperlogy`(2,780자)처럼 글자가 빠지면 `맑은 고딕`이 불쑥 섞인다.

### 방안 ② OS별 분기 (macOS = `-apple-system`, Windows = 번들 후보)
- **장점**: macOS에서는 100% 네이티브 맥 감성(번들 로드 비용 0, SF Pro + Apple SD Gothic Neo), Windows에서는 `맑은 고딕`을 피하고 세련된 번들 폰트(`Pretendard` 또는 `Freesentation`)로 품격을 유지한다.
- **단점**: macOS와 Windows 스크린샷·문서·브랜드 인상이 달라지고, 레이아웃 미세 높이(`line-height`, `TrayPopover` 높이)를 두 OS에서 각각 확인해야 한다.

### 방안 ③ 전 OS 단일 번들 + 기호(`⌥⌘⇧⌃`)만 OS 폴백 *(가장 안전)*
- **장점**: 한·영 균형이 이미 조율된 단일 패밀리(`Freesentation` 또는 `Pretendard`)로 macOS·Windows·영어 UI·PDF까지 브랜드 통일성과 레이아웃 일관성을 지킨다. `10-design-system.md §3`의 "맑은 고딕 폴백 방지" 원칙에도 정확히 부합한다.
- **단점**: 앱 번들에 woff2 용량이 포함된다(서브셋/분할 시 3굵기 기준 약 0.8~2.5 MB).

---

## 6. 제외 글꼴 목록 (폴더 내 나머지 43개 woff2)

총 59개 woff2 = **후보·참고 5패밀리 16개** + **제외 43개**.

| 계열 / 글꼴 | 폴더 파일 수 | 제외 사유 (한 줄) |
|---|---|---|
| **배달의민족 BM 계열** (`BMDOHYEON`, `BMEULJIROTTF`, `BMJUA`, `BMYEONSUNG_ttf`) | 4 | 간판·붓글씨·포스터용 단일 굵기 디스플레이 서체로 11~13px UI 본문 부적합 |
| **쿠키런** (`CookieRun_Regular/Bold/Black`) | 3 | 게임 브랜드 개성이 강한 디스플레이 서체로 차분한 macOS 유틸리티 감성과 불일치 |
| **학교안심 · SchoolSafe 계열** (`Bombanghak`, `Bunpil`, `Butpen`×2, `Doldam`×2, `Eunhasu`, `Gaeulsopung`×2, `Ggooreogi`, `Kkokkoma`, `Malgeunnal`, `Monggeul`, `Tuho`, `SchoolSafeNadeuri`×2) | 16 | 손글씨·붓펜·분필·어린이 장식체 계열로 작은 UI 가독성과 맥 감성 모두 부적합 |
| **여기어때 잘난체** (`Jalnan2`×2, `JalnanGothic`×2) | 4 | 단일 극굵기(Heavy) 타이틀·배너용 디스플레이 서체로 본문·보조 글 불가 |
| **카페24 계열** (`Cafe24Danjunghae`, `Cafe24Dongdong`, `Cafe24Ssukssuk`) | 3 | 단일 굵기 손글씨·장식 디스플레이 서체로 UI 본문 부적합 |
| **메이플스토리** (`MaplestoryL/B`) | 2 | 게임 브랜딩용 둥근 디스플레이 서체, 2종뿐이고 차분한 도구 감성과 불일치 |
| **KCC 간판 · 도담도담** (`KCC-Ganpan`, `KCCDodamdodam`) | 2 | 단일 굵기 간판/손글씨 서체로 11~13px UI 본문 부적합 |
| **온글잎** (`Ownglyph2022`) | 1 | 단일 굵기 손글씨체로 UI 본문 부적합 |
| **JNaughty** (`JNaughtyL/M`) | 2 | 손글씨·캐주얼 장식체이며 확장 한글 누락(`뷁`), 상용 폰트 저작권 확인 불명확 |
| **엘리스 디지털배움체** (`EliceDigitalBaeumR`) | 1 | 폴더 내 단일 굵기(`R`), 공식에서도 제목용 서체로 소개하며 2,350자 제한(`뷁큑` 토푸), 라이선스 원문 `[미확인]` |
| **을유1945** (`Eulyoo1945-Regular/SemiBold`) | 2 | 본문용 명조체(2종, 2,750자)로 유틸리티 UI 부적합, **공식 약관상 앱·PDF 임베딩 명문 금지** |
| **부크크 명조** (`BookkMyungjo_Light/Bold`) | 2 | 출판용 명조체(2종, 중간 굵기 없음)로 UI 본문 부적합, 자체 약관(수정 금지·임베딩 `[미확인]`) |
| **리디바탕** (`RIDIBatang`) | 1 | 단일 굵기 전자책용 명조체로 11~13px UI 본문 부적합 *(SIL OFL 1.1이라 긴 글 PDF 참고만 가능)* |

---

## 7. [미확인] 항목 요약

- **Gmarket Sans**: 공식 페이지(`corp.gmarket.com/fonts/`)에 `'SIL Open Font License'` 및 자유 수정·재배포는 명시되어 있으나, OFL 버전(1.1 여부)과 원문 LICENSE·Reserved Font Name 조항은 웹페이지 본문에 미노출 → 채택 시 배포 ZIP 내부 LICENSE 파일 직접 확인 필요.
- **Elice DX Neolli / Elice Digital Baeum**: 2차 폰트 플랫폼(눈누·산돌구름)은 SIL OFL로 표기하나, 현행 공식 페이지(`elice.io/ko/resources/brand`) 본문에는 라이선스 약관·임베딩 조항이 직접 게시되어 있지 않음 → 채택 시 공식 배포 ZIP 동봉 LICENSE 또는 `contact@elice.io` 확인 필요.
- **Bookk Gothic / Bookk Myungjo**: 공식 페이지(`bookk.co.kr/font`) 원문에 앱 번들 임베딩·재배포·PDF 임베딩 허용 여부가 명시되지 않았고, "임의로 수정 및 편집 등을 할 수 없으며 배포되는 형태 그대로 사용" 조항 때문에 TTF→WOFF2 변환·서브셋 적법성 확인 불가 → 채택 비권장(사용 시 부크크 서면 확인 필요).
