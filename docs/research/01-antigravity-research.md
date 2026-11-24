# 화면 필기·스마트 캡처·핀(Pin)·교실 도구 통합 데스크톱 앱 기획을 위한 딥 리서치 보고서

> **문서 목적**: '아이캔 줌잇' + 'Snipaste'를 핵심 축으로 삼고 '아이캔 노트'·'아이캔 스크린'·'Classroomscreen'의 핵심 역량을 흡수한 차세대 크로스플랫폼 데스크톱 앱(Windows `.exe`/포터블/Microsoft Store 무료 배포, macOS `.dmg`, 한국어·영어 지원)의 **PRD(제품 요구사항 정의서) 작성 전 단계 기초 리서치**
> **작성 기준일**: 2026-11-24

---

## 0. 문서 개요 및 조사 방법·한계

### 0.1 기획 배경 요약
- **타깃 사용자**: 한국어권 교사·강사·발표자(1차 핵심 페르소나) 및 영어권 교육자·개발자·디자이너·지식 노동자.
- **핵심 제품 비전**: 실시간 화면 확대·판서(`아이캔 줌잇`/`ZoomIt`), 스마트 UI 요소 캡처·화면 위 핀 고정(`Snipaste`), 오버레이·캔버스 판서(`아이캔 스크린`/`아이캔 노트`), 교실 운영 위젯(`Classroomscreen`), 한글 오프라인 OCR·QR 생성·유틸리티를 **하나의 가벼운 네이티브급 데스크톱 앱**으로 통합·초고도화.
- **배포 전략**: Windows 설치형(`.exe`)·무설치 포터블(`.zip`)·Microsoft Store 무료 배포 + macOS(`.dmg` Universal Binary).

### 0.2 1차 자료 확인 범위 및 조사 한계
1. **직접 추출·열람한 1차 자료**:
   - 아이캔 줌잇 단축키 설명서 PDF (`pdftotext`로 전문 추출 확인)
   - 이지쌤교실 네이버 블로그 원문 (`https://m.blog.naver.com/iz_ssaem/223318741562`)
   - 티스토리 Snipaste 사용기 원문 (`https://s-bug.tistory.com/4`)
   - GitHub REST API 직접 조회를 통한 오픈소스 저장소별 Stars·주력 언어·라이선스·최근 푸시일·`Snipaste/feedback` 최다 댓글 이슈 목록 확인
   - 웹 검색 교차 검증(Snipaste docs/feedback, Classroomscreen 공식 가이드, Microsoft PowerToys/Sysinternals 문서, Reddit `r/software`·`r/windows`·`r/macapps`·`r/Teachers`, 한국 교사 커뮤니티/블로그 사례)
2. **직접 확인하지 못한 항목 및 대체 조사 안내**:
   - **YouTube 영상 2건** (`https://www.youtube.com/watch?v=tWF0mWjMAF0` — 이지쌤교실, 759초 / `https://www.youtube.com/watch?v=1dCk2zBJ2jE` — 공부해서 남주는 선생님, 351초): 비브라우저·무자막 파싱 환경에서 영상 프레임 직접 시청 및 자동 생성 자막 XML 본문 추출이 불가하여(`CAPTION_TRACKS` 메타데이터만 확인되고 XML 응답 본문이 비어 있음), 영상 제목·채널·설명글(`shortDescription`)과 영상 설명에 링크된 원문 블로그(`https://m.blog.naver.com/iz_ssaem/223318741562`), 단축키 PDF, 관련 리뷰 자료로 대체 조사함.
   - **아이캔노트 공식 홈페이지** (`https://www.icannote.com/`): 직접 HTTP 요청 시 `Connection reset by peer`로 접속이 차단되어 검색 인덱스·공식 유튜브/블로그 후기 자료로 교차 확인함. 세부 미확인 사항은 본문 및 5절에 `[미확인]`으로 명시함.

---

## 1. 앱별 요약표 및 핵심 벤치마크 심층 분석

총 **24개 앱**을 5개 핵심 벤치마크 대상(아이캔 줌잇, 아이캔 노트, 아이캔 스크린, Classroomscreen, Snipaste)과 캡처 10종·필기/발표 9종으로 구분해 정리한다.

### 1.0 전체 24개 앱 종합 요약표

| # | 앱 이름 | 분류 | 제작자 / 배포처 | 가격 정책 | 지원 OS | 핵심 기능 5개 | 단축키 요약 | 장단점 |
|---|---|---|---|---|---|---|---|---|
| 1 | **아이캔 줌잇 (ICanZoomIt)** | 확대·판서 (국산) | 이상열 대표 / 아이캔노트 팀 (`cafe.naver.com/icannote`) | 완전 무료 (무설치 포터블) | Windows | ① 화면 확대·실시간 확대<br>② 한글 텍스트 입력 판서<br>③ 도형·점선 그리기 및 선택·지우개<br>④ 실행취소(`Ctrl+Z`)·다시실행(`Ctrl+Y`)<br>⑤ 타이머·화이트/블랙보드·저장/복사 | `Ctrl+1` 확대, `Ctrl+2` 판서, `Ctrl+3` 타이머, `Ctrl+4` 실시간 확대, `P`/`M`/`T`/`S`/`D`/`E`, `W`/`K`, `ESC`/우클릭 종료 | **장점**: 한글 입력 완벽, `Ctrl+Z`·점선·툴바 지원, 무설치<br>**단점**: Win 전용, 단축키 충돌, 미서명 SmartScreen 경고, 녹화·라이브 판서 미지원 |
| 2 | **아이캔 노트 (ICanNote)** | 디지털 판서 노트 | 이상열 대표 / 아이캔노트 팀 (`cafe.naver.com/icannote`) | 무료 | Windows | ① PDF·PPT·HWP·이미지 불러오기<br>② 페이지별 판서·저장 관리<br>③ 스마트 도형 보정·수학/과학 그래프<br>④ 커튼·스포트라이트·타이머<br>⑤ 강의 화면+음성 녹화(아이캔VOD 연동) | UI 툴바 중심 + 도구별 단축키(`[미확인]` 세부 전체표) | **장점**: 교재/학습지 수업 최적화, 한국 강사 니즈 반영<br>**단점**: Win 전용, 단순 오버레이 메모에는 무거움 |
| 3 | **아이캔 스크린 (ICanScreen)** | 화면 오버레이 판서 | 이상열 대표 / 아이캔노트 팀 (`cafe.naver.com/icannote`) | 무료 | Windows | ① 실행 중인 모든 화면 위 투명 판서<br>② 미니 툴바·마우스/판서 모드 전환<br>③ 펜·형광펜·도형·텍스트 입력<br>④ 커튼(가리기)·스포트라이트·캡처<br>⑤ 화면+음성 실시간 녹화 | 미니 툴바 아이콘 클릭 및 모드 전환 기반 | **장점**: 웹·영상 위 즉시 판서, 수업 흐름 유지<br>**단점**: Win 전용, Snipaste급 정밀 영역 캡처·핀 창 없음 |
| 4 | **Classroomscreen** | 교실 운영 대시보드 | Classroomscreen (웹 SaaS) | Freemium (26개 위젯 전부 무료 / Pro 연 $36) | 웹 브라우저 (전 OS) | ① 타이머·비주얼 타이머·스톱워치·시계<br>② 랜덤 이름 뽑기·모둠 만들기(블랙리스트)<br>③ 소음 측정기·신호등·작업 기호<br>④ QR 코드 생성기·투표(Poll)·점수판<br>⑤ 텍스트·그리기·PDF/영상/웹캠 임베드 | 웹 UI 클릭 중심 (단축키 체계 미약) | **장점**: 설치·학생 로그인 없이 즉시 사용, 후방 가독성 우수<br>**단점**: 오프라인 불가, 학교 방화벽 차단, 스냅 그리드 부재, Pro 저장 제한 |
| 5 | **Snipaste** | 스마트 캡처·핀 | Liule / Levie (`snipaste.com`) | 개인 무료 / Pro(상업·고급) 1대 $8.99·3대 $19.99 | Windows (포터블·설치·MS Store), macOS (Universal), Linux (AppImage) | ① UI 요소·창 자동 감지 캡처<br>② 픽셀 단위 미세 조정·히스토리 재생<br>③ 클립보드(이미지·텍스트·HTML·색상) 핀(`F3`)<br>④ 핀 창 투명도·회전·클릭 통과<br>⑤ 색상 추출기(`C`)·모자이크·주석 | `F1` 캡처, `F3` 핀, `Tab` 요소/창 전환, `W/A/S/D` 1px 이동, `C` 색상 복사, `<`/`>` 히스토리, `Ctrl+휠` 투명도, `Space` 주석 | **장점**: 극강의 속도와 UI 요소 스냅, 핀 창 UX 최상급<br>**단점**: 스크롤 캡처·화면 녹화 없음, OCR/QR 등 Pro 잠금, Mac 이질감 |
| 6 | **PixPin** | 스마트 캡처·핀·OCR | PixPin 팀 (`pixpin.com`) | 무료 + 일부 고급 기능 Pro | Windows, macOS | ① Snipaste식 UI 요소 감지·핀 창<br>② 무료 오프라인 OCR(핀 창에서도 추출)<br>③ 수직·수평 스크롤(긴 화면) 캡처<br>④ GIF·동영상 화면 녹화 + 주석<br>⑤ 번호 매기기(스텝)·모자이크·번역 | Snipaste와 유사한 단축키 체계(`F1`/`F3` 등 사용자 설정 가능) | **장점**: Snipaste의 빈틈(OCR·스크롤·GIF)을 모두 채움<br>**단점**: 비공개 소스 프라이버시 우려, 일부 기능 Pro 전환 불만 |
| 7 | **ShareX** | 캡처·녹화·자동화 | ShareX Team (GitHub) | 완전 무료 (오픈소스 GPL-3.0) | Windows | ① 영역·창·스크롤 캡처<br>② 80개 이상 호스팅·클라우드 자동 업로드<br>③ 화면 녹화(MP4/GIF) 및 후처리 워크플로우<br>④ OCR·QR 생성/인식·색상·눈금자<br>⑤ 강력한 내장 주석 편집기(Greenshot 기반) | `Ctrl+PrintScreen` 영역 등 전 기능 글로벌 단축키 지정 가능 | **장점**: 압도적 기능·자동화, 무료 오픈소스<br>**단점**: Win 전용, 설정이 너무 방대해 일반 교사·초보자 진입장벽 높음 |
| 8 | **Flameshot** | 캡처·즉시 주석 | Flameshot Org (GitHub) | 완전 무료 (오픈소스 GPL-3.0) | Windows, macOS, Linux | ① 영역 지정과 동시에 원형/사이드 주석 툴바<br>② 화살표·박스·흐림·텍스트·번호 카운터<br>③ 핀(Pin) 고정 창 기능<br>④ Imgur 업로드·클립보드·저장<br>⑤ CLI 스크립팅 및 단축키 커스텀 | `PrintScreen` / 커스텀 단축키, `Ctrl+C`/`Ctrl+S`, `Esc` | **장점**: 캡처-주석-복사 흐름이 매우 빠름, 크로스플랫폼<br>**단점**: UI 요소 자동 스냅 약함, Win 고DPI/멀티모니터 스케일링 버그, 녹화·OCR 없음 |
| 9 | **Greenshot** | 경량 캡처·주석 | Greenshot 팀 (GitHub) | Win 무료(GPL-3.0) / macOS App Store 유료 | Windows, macOS(별도) | ① 영역·창·전체화면·직전 영역 재캡처<br>② 내장 이미지 편집기(하이라이트·난독화/블러)<br>③ Office·Imgur·프린터·파일 직접 전송<br>④ 초경량 메모리 점유<br>⑤ 설정 가능한 동적 파일명 규칙 | `PrintScreen` 영역, `Alt+PrintScreen` 창, `Ctrl+PrintScreen` 전체, `Shift+PrintScreen` 직전 영역 | **장점**: 가볍고 기업/기관 보급에 검증됨<br>**단점**: Win 오픈소스 판은 업데이트 더딤·보안 이슈 언급, 핀/OCR/스크롤 미흡 |
| 10 | **ksnip** | 주석 특화 캡처 | Damir Porobic / ksnip (GitHub) | 완전 무료 (오픈소스 GPL-3.0) | Windows, macOS, Linux | ① kImageAnnotator 기반 벡터 주석·번호 스텝퍼<br>② 프레임 없는 핀(Pin) 창 고정<br>③ 지연 캡처·활성 창·직전 영역<br>④ 탭 기반 다중 캡처 편집<br>⑤ Tesseract OCR 플러그인(선택) | 글로벌 단축키 설정 가능 (OS별 상이) | **장점**: 크로스플랫폼 Qt, 번호 매기기·벡터 재편집 우수<br>**단점**: UI 요소 스냅·스크롤 캡처 약함, OCR 별도 플러그인 필요 |
| 11 | **Shottr** | macOS 초경량 캡처 | Max Chugunov (`shottr.cc`) | 무료(무제한) / 선택 일회성 $12 | macOS | ① 초고속 네이티브 캡처·스크롤 캡처<br>② 텍스트 지우기(Remove Text)·블러/픽셀화<br>③ 내장 오프라인 OCR(Apple Vision 기반 추정·고정밀)<br>④ 픽셀 눈금자·색상 피커·오버레이 핀<br>⑤ 스포트라이트·배경 패딩·스텝 번호 | `Cmd+Shift+1` 전체, `Cmd+Shift+2` 영역, `Cmd+Shift+A` 스크롤 등 | **장점**: 맥에서 압도적으로 빠르고 가벼움, 합리적 일회성 가격<br>**단점**: macOS 전용, 화면/GIF 녹화 미지원, 일부 기업 보안 차단 |
| 12 | **CleanShot X** | macOS 프리미엄 캡처·녹화 | MakeTheWeb (`cleanshot.com`) | $35(일회성+1년 업데이트) / 연 $19 갱신 / Pro 구독 | macOS | ① Quick Access 오버레이(우하단 임시 트레이·드래그)<br>② 스크롤 캡처·배경 꾸미기(Wallpaper/그림자)<br>③ 화면/GIF 녹화(웹캠·키입력 표시)<br>④ 고정밀 OCR·민감정보 자동 가리기<br>⑤ Pin 고정·CleanShot Cloud 링크 공유 | 커스텀 글로벌 단축키, `Cmd+Shift+3/4` 대체 가능 | **장점**: 맥 캡처·녹화 UX의 사실상 표준, 완성도 최상<br>**단점**: macOS 전용, 매년 업데이트 비용($19/년) 구독 피로감 |
| 13 | **Xnapper** | 캡처 미화(Prettify) | Xnapper | 유료 (일회성 라이선스 / 구독) | macOS, Windows | ① 자동 그라데이션 배경·모서리 둥글기·그림자<br>② SNS·문서용 비율 프리셋(16:9, 4:3 등)<br>③ 민감정보(이메일·API키) 자동 감지 및 블러<br>④ 내장 OCR·워터마크 삽입<br>⑤ 즉시 클립보드 복사 | 커스텀 글로벌 단축키 | **장점**: 발표 자료·SNS용 예쁜 스크린샷 제작 최단 시간<br>**단점**: 유료, 핀 창/실시간 화면 필기·교실 도구 없음 |
| 14 | **Snagit** | 기업·교육용 스위트 | TechSmith | 유료 연간 구독 (~$39~$63/년) | Windows, macOS | ① 파노라마·스크롤 캡처<br>② 스탬프·콜아웃·단계별 매뉴얼 템플릿(SUI 단순화)<br>③ 이미지 기반 설명 동영상 제작·녹화<br>④ 라이브러리(태그·검색) 관리<br>⑤ OCR·텍스트 바꾸기(Grab Text) | `PrintScreen` 통합 캡처 단축키 | **장점**: 매뉴얼·강의 교안 제작 기능 최강, 방대한 스탬프/템플릿<br>**단점**: 비싼 연간 구독 전환, 무겁고 교실 즉석 필기에는 부적합 |
| 15 | **알캡처 (ALCapture)** | 국내 대중 캡처 | 이스트소프트 (ESTsoft) | 개인 무료(광고) / AD-ZERO 월 구독 | Windows | ① 직접지정·창·단위영역·스크롤·지정사이즈 캡처<br>② 캡처 직후 간단 그리기(펜·도형·모자이크)<br>③ 최근 캡처 목록 관리<br>④ 직관적 한국어 UI<br>⑤ 알툴즈 제품군 연동 | `Ctrl+Shift+C` 등 모드별 단축키 | **장점**: 한국인에게 익숙한 UI, 스크롤/지정사이즈 편리<br>**단점**: Win 전용, 과도한 배너 광고·업데이트 시 광고설정 초기화 |
| 16 | **픽픽 (PicPick)** | 캡처·이미지 편집 | NGWIN (원더보이즈) | 한국어판 개인·기업 무료 / 영문판 상업 유료 | Windows | ① 전체·활성창·스크롤·영역·고정사각형 캡처<br>② 리본 UI 기반 고기능 이미지 편집기<br>③ 화면 색상 추출기·픽셀 눈금자·각도기·십자선<br>④ 화이트보드 기능<br>⑤ 자동 저장·워터마크·효과 필터 | `Shift+Ctrl+PrintScreen` 등 모드별 커스텀 단축키 | **장점**: 한국어판 무료, 눈금자·각도기·색상 추출 등 유틸 풍부<br>**단점**: Win 전용, DRM 오탐(Steam 등)으로 캡처 먹통 빈발, 무거움 |
| 17 | **Sysinternals ZoomIt (PowerToys 포함)** | 원조 확대·판서·녹화 | Microsoft (Mark Russinovich / PowerToys) | 완전 무료 (오픈소스 MIT) | Windows | ① 정지 확대(`Ctrl+1`) 및 LiveZoom(`Ctrl+4`)<br>② LiveDraw 실시간 화면 위 판서(`Ctrl+Shift+4`)<br>③ 도형·색상(6색)·하이라이터·텍스트(영문 위주)·화이트/블랙보드<br>④ MP4/GIF 화면 녹화(`Ctrl+5`) & 영역 스닙/OCR(`Ctrl+6`/`Ctrl+Alt+6`)<br>⑤ DemoMirror(보조 화면 미러링)·Snip Dictation·타이머(`Ctrl+3`) | `Ctrl+1` 확대, `Ctrl+2` 그리기, `Ctrl+3` 타이머, `Ctrl+4` LiveZoom, `Ctrl+Shift+4` LiveDraw, `Ctrl+5` 녹화, `Ctrl+6` 스닙, `Ctrl+Alt+6` OCR | **장점**: 초경량 네이티브 C++, 무설치/PowerToys 내장, 최근 녹화·OCR·LiveDraw까지 대폭 진화<br>**단점**: Win 전용, 툴바 부재로 단축키 암기 필수, 한글 폰트/선택·부분 지우개 UX 불편 |
| 18 | **Epic Pen** | 화면 오버레이 필기 | Tank Studios (`epicpen.com`) | Freemium (펜·형광펜 무료 / 도형·텍스트·화이트보드 Pro 구독) | Windows, macOS | ① 모든 앱·바탕화면 위 실시간 필기<br>② 펜·형광펜·지우개·클릭 통과 토글<br>③ (Pro) 직선·사각형·원·화살표·텍스트<br>④ (Pro) 화이트/블랙보드·사라지는 잉크(Fading ink)<br>⑤ 필압(와콤/태블릿) 지원·스크린샷 | 사용자 지정 글로벌 핫키(숨기기·펜·지우개 등) | **장점**: 오래된 표준 UI, 클릭 통과 직관적<br>**단점**: 도형·텍스트까지 구독 페이월 전환 불만 매우 큼, 간헐적 CPU 스파이크 |
| 19 | **gInk** | 경량 오픈소스 필기 | geovens (GitHub) | 완전 무료 (오픈소스 MIT) | Windows | ① 초경량 투명 오버레이 화면 필기<br>② 멀티 펜 슬롯(색상·굵기 프리셋)<br>③ 형광펜·지우개·실행취소<br>④ 마우스 포인터 모드(클릭 통과) 즉시 전환<br>⑤ 직관적 플로팅 미니 바 | 핫키로 잉크 모드/마우스 모드 전환 | **장점**: 매우 가볍고 빠름, MIT 라이선스<br>**단점**: Win 전용, 업데이트 뜸함, 도형·텍스트·고급 멀티모니터 처리 부족 |
| 20 | **ppInk** | 확장 오픈소스 필기 | pubpub-zz (GitHub, gInk 포크) | 완전 무료 (오픈소스 MIT) | Windows | ① gInk 기능 + 손도형·선·화살표·사각형·원·텍스트<br>② 넘버링/스탬프·자석 스냅<br>③ 마우스 커서 하이라이트(스포트라이트 링)<br>④ 실행취소/다시실행·화이트/블랙보드<br>⑤ 포터블 실행 및 클릭 통과 | 툴바 버튼 및 단축키 매핑 | **장점**: gInk의 한계(도형·텍스트·커서 강조)를 MIT로 해결<br>**단점**: Win 전용, 투박한 UI, Windows 기본 압축해제 시 설정 깨짐 이슈 보고 |
| 21 | **Presentify** | macOS 발표 필기 | Ram Patra (`presentifyapp.com`) | 유료 일회성 (Mac App Store) / Setapp 포함 | macOS | ① 화면 위 실시간 그리기·그라데이션 색상<br>② 도형·텍스트·화이트보드 모드<br>③ 커서 하이라이트(헤일로)·스포트라이트<br>④ 화면 확대(Zoom)<br>⑤ iPad Sidecar + Apple Pencil 완벽 지원 | 메뉴바 + 글로벌 단축키 전환 | **장점**: 맥 네이티브 디자인 우수, 아이패드 애플펜슬 연동 강력<br>**단점**: macOS 전용, 일부 배포채널(Setapp 등)에서 확대 시 간헐적 랙 보고 |
| 22 | **DemoPro** | macOS 초간편 발표 | DemoPro (Mac App Store) | 저가 일회성 (~$3.99) | macOS | ① 키 홀드/단축키 기반 즉시 판서<br>② 커서 링·스포트라이트·레이저 포인터<br>③ 자동 페이딩(사라지는) 주석<br>④ 내장 발표 카운트다운 타이머<br>⑤ 도형(선·화살표·사각형·원)·텍스트 | 단축키를 누른 상태로 트랙패드 드래그 (옵션 변경 가능) | **장점**: 가격 저렴, 발표 집중형(타이머+페이딩 잉크)<br>**단점**: macOS 전용, 기본 '키 누른 채 드래그' 방식이 손 피로 유발, 녹화 없음 |
| 23 | **ScreenBrush** | macOS 발표 브러시 | ScreenBrush (Mac App Store) | Freemium (고급 기능 인앱 결제) | macOS | ① 풀스크린·일반 창 위 실시간 판서<br>② Ghost 모드(일시 클릭 통과)<br>③ 손전등(Flashlight) 스포트라이트·확대<br>④ 스냅샷 저장(PNG/PSD/PDF 내보내기)<br>⑤ 화면 정지(Freeze) + 화이트보드 | 글로벌 단축키 + 메뉴바 제어 | **장점**: 손전등 효과·PSD/PDF 내보내기 등 발표 특화<br>**단점**: macOS 전용, 무료/유료 경계 혼란, 빠른 앱 전환(Cmd+Tab) 시 프레임 끊김 보고 |
| 24 | **Annotate (Screen Annotation)** | Windows 화면 주석 | Annotate (Microsoft Store) | 무료 | Windows | ① 자유선·형광펜·화살표·다이어그램 아이콘 라이브러리<br>② Smart Draw(손그림 도형 자동 보정)<br>③ 픽셀 눈금자·번호 배지·스포트라이트 헤일로<br>④ 화면 돋보기(Magnifier)<br>⑤ 화면 블러/가리기 오버레이 | 시스템 트레이 + 단축키 + 컨텍스트 메뉴 | **장점**: 무료인데도 Smart Draw·아이콘·눈금자 등 기능 방대<br>**단점**: Win 중심, 단축키/메뉴 학습곡선 높음, 블러가 보안 마스킹(암호화) 아님 |

---

### 1.1 아이캔 줌잇 (ICanZoomIt) 심층 분석

1. **제작자·배포 방식·가격·OS**:
   - **제작자**: 현직 수학 강사 겸 개발자 **이상열 대표**(아이캔노트·스마트플립러닝연구회)와 아이캔노트 팀이 원조 Microsoft Sysinternals ZoomIt의 한국어·수업 현장 한계를 보완하여 개발.
   - **배포처·방식**: 공식 네이버 카페([https://cafe.naver.com/icannote](https://cafe.naver.com/icannote)) 자료실을 통해 회원가입 없이 무료 다운로드 가능하며, 설치 과정이 필요 없는 **무설치(Portable) `.exe`** 형태로 배포됨(v0.72대 등 패치형 업데이트 지속).
   - **가격 / OS**: 완전 무료 / **Windows 전용** (macOS·Linux 미지원).
   - **버전 이력**: 초기 화면 확대+한글 타이핑+도형 점선+`Ctrl+Z` 지원에서 출발해 실시간 확대(`Ctrl+4`), 클립보드 복사(`Ctrl+C`), 이미지 저장(`Ctrl+S`), 펜 굵기 조절, 플로팅 툴바, 형광펜(`M`) 등이 패치로 추가됨. *(전체 버전별 세부 릴리즈 노트 및 빌드 번호 이력은 카페 비공개/게시글 단위 분산으로 `[미확인]`)*

2. **UI 구조**:
   - **시스템 트레이 상주**: 실행 시 작업표시줄 우측 하단 트레이 아이콘 생성 → 우클릭으로 **[사용자 설정]**(단축키·펜 기본 설정 변경) 및 종료 제어.
   - **플로팅 툴바 + 단축키 이원화**: 원조 ZoomIt이 툴바 없이 100% 단축키 암기에 의존하는 것과 달리, 화면에 펜·형광펜·도형·지우개 등을 마우스/전자칠판 터치로 고를 수 있는 **툴바(툴박스)**를 함께 제공함(이지쌤교실 블로그 확인).
   - **모드 전환 구조**: 확대(`Ctrl+1`), 판서(`Ctrl+2`), 타이머(`Ctrl+3`), 실시간 확대(`Ctrl+4`) 4대 모드로 진입하며 `ESC` 또는 화면이동 상태 마우스 우클릭으로 빠져나옴.

3. **기본 단축키 전체 표** *(공식 설명서 PDF `___ .pdf` 1차 추출 기준 + 블로그 `M` 형광펜 교차 확인)*:

| 구분 | 단축키 | 동작 내용 | 비고 |
|---|---|---|---|
| **실행 모드** | `Ctrl + 1` | 화면 확대 모드 진입 | 종료: 화면이동 상태에서 우클릭 또는 `ESC` |
| | `Ctrl + 2` | 판서(그리기) 모드 진입 | 종료: 화면이동 상태에서 우클릭 또는 `ESC` |
| | `Ctrl + 3` | 타이머 실행 | 종료: 마우스 우클릭 또는 `ESC` |
| | `Ctrl + 4` | 실시간 확대 모드 진입/종료 | **화면 확대/축소만 지원** (판서 불가) |
| **확대 / 굵기** | `Up` / `Down` (또는 `마우스 휠`) | 화면 확대 / 축소 | 확대/판서 모드 |
| | `Ctrl + Up` / `Ctrl + Down` (또는 `Ctrl + 휠`) | 펜 굵기 크게 / 작게 | 실시간 확대 모드에서는 화면 확대/축소로 동작 |
| **도형·기능** | `Space bar` | 마우스 커서를 화면 센터로 이동 | 확대·판서 모드 |
| | `Tab` | 원 그리기 | 드래그 중 적용 |
| | `Ctrl` (누른 채 드래그) | 사각형 그리기 | `+ Alt` 추가 시 **점선 사각형** |
| | `Shift` (누른 채 드래그) | 직선 그리기 | `+ Alt` 추가 시 **점선 직선** |
| | `Shift + Ctrl` (누른 채 드래그) | 화살표 그리기 | `+ Alt` 추가 시 **점선 화살표** |
| | `Ctrl + Z` / `Ctrl + Y` | 실행 취소 / 다시 실행 | 원조 ZoomIt과 차별화되는 핵심 개선점 |
| | `Ctrl + C` / `Ctrl + S` | 클립보드로 복사 / 이미지 파일로 저장 | |
| | `W` / `K` | 화이트보드 / 블랙보드 전환 | 즉석 빈 칠판 판서 |
| **판서 도구** | `P` | 펜(자유선) | |
| | `M` | 형광펜(마커) | PDF 표에는 미기재, 리뷰/블로그에서 확인 |
| | `S` | 판서 내용 선택 툴 (이동·조절) | 원조 ZoomIt에 없는 개체 선택 기능 |
| | `T` | 텍스트 입력 (한글 완벽 지원) | 입력 중 `Ctrl + T`로 폰트 설정 가능 |
| | `D` | 지우개 (선 또는 도형 단위 삭제) | |
| | `E` | 전체 삭제 | |
| **타이머** | `Up` / `Down` | 1분 단위 시간 증가 / 감소 | |
| | `Left` / `Right` | 10초 단위 시간 증가 / 감소 | |
| | `Ctrl + Up` / `Ctrl + Down` (또는 `Ctrl + 휠`) | 표시 시간 글자 크기 확대 / 축소 | 교실 뒤쪽 가독성 확보 |
| **색상** | `R` / `Y` / `G` / `O` / `B` / `L` | Red / Yellow / Green / Orange / Blue / Lime | 확대·판서·타이머 공통 적용 |

4. **교실 활용 방식 및 원조 Sysinternals ZoomIt과의 비교**:
   - **교실 활용**: PDF·웹 학습지의 문제 한 문항을 `Ctrl+1`로 확대한 뒤 `M`(형광펜)·`P`(펜)로 풀이하고, `T`로 한글 핵심 개념을 타이핑하며, 모둠 활동 시 `Ctrl+3`으로 대형 타이머를 띄움.
   - **원조 ZoomIt(특히 2025~2026 PowerToys 통합 최신판)과의 차이**:

| 비교 항목 | 아이캔 줌잇 (ICanZoomIt) | 원조 Sysinternals ZoomIt (PowerToys 최신 포함) |
|---|---|---|
| **한글 텍스트 입력** | 원활함 (`Ctrl+T` 폰트 설정) | 영문 중심, 한글 입력 조합/깨짐 오류 빈발 |
| **플로팅 툴바** | 제공 (전자칠판·초보자 마우스 조작 가능) | 없음 (100% 단축키 기반) |
| **점선 도형 / 선택·부분 지우개** | `+Alt` 점선 도형, `S` 개체 선택, `D` 획 단위 지우개 | 점선 도형 없음, 개체 선택 이동 없음, `E` 전체 삭제/`Ctrl+Z` 위주 |
| **실시간 화면 위 판서 (LiveDraw)** | 미지원 (정지 화면 기반 판서) | **지원** (`Ctrl+Shift+4` LiveDraw) |
| **화면 녹화 (MP4/GIF)** | 미지원 | **지원** (`Ctrl+5`, 영역 `Ctrl+Shift+5`, 창 `Ctrl+Alt+5`) |
| **영역 스닙 & OCR & 보조기능** | 전체/확대화면 `Ctrl+C`/`Ctrl+S`만 지원 | 영역 스닙(`Ctrl+6`), **OCR(`Ctrl+Alt+6`)**, DemoMirror, Snip Dictation |
| **배포·보안 신뢰도** | 무설치 포터블(미서명 시 SmartScreen 경고) | Microsoft 공식 서명 + PowerToys 내장(MIT 오픈소스 C++) |

---

### 1.2 아이캔 노트 (ICanNote) · 아이캔 스크린 (ICanScreen) 비교 분석

- **동일 제작자 계열 확인**: 아이캔 줌잇·아이캔 노트·아이캔 스크린 모두 **이상열 대표(아이캔 시리즈)**가 개발·무료 배포하는 동일 제품군(`cafe.naver.com/icannote` / `icannote.com`)이다.
- **세 제품 간 역할 분담과 차별점**:

| 구분 | 아이캔 노트 (ICanNote) | 아이캔 스크린 (ICanScreen) | 아이캔 줌잇 (ICanZoomIt) |
|---|---|---|---|
| **핵심 개념** | **전용 캔버스·노트 공간** | **윈도우 전체 위 투명 오버레이 레이어** | **단축키 기반 순간 확대·즉석 판서** |
| **수업 자료 처리** | PDF, HWP, PPT, 이미지 불러오기 및 페이지 관리·저장 | 자료 불러오기 없음(현재 화면·브라우저·영상 위에 직접 필기) | 현재 화면 캡처 정지 후 확대/필기 |
| **특화 도구** | 스마트 도형 보정, 수학·과학 그래프/좌표, 페이지 추가/배경 반전 | 미니 툴바 커스텀, 마우스↔판서 즉시 전환 | `Ctrl+1~4` 초고속 호출, 화면 확대, 대형 타이머 |
| **교실 집중·녹화** | 커튼(가림막), 스포트라이트, 화면+마이크 녹화, 아이캔VOD 연동 | 커튼, 스포트라이트, 화면 캡처, 화면+마이크 녹화 | 화이트/블랙보드(`W`/`K`), 확대/축소 집중 |
| **신규 앱 흡수 포인트** | **스마트 도형 보정·수학/좌표 도구·커튼(가림막)·스포트라이트·교재/PDF 판서 레이어** | **바탕화면/앱 위 투명 오버레이 + 클릭 통과(마우스 전환) + 도킹 가능한 미니 툴바** | **0.1초 단축키 진입(`Ctrl+1~4`)·한글 타이핑·점선 도형·개체 선택/지우개** |

---

### 1.3 Classroomscreen 심층 분석

1. **26개 전체 위젯 목록** *(공식 위젯 목록 기준)*:
   - **시간 관리 (6종)**: `Timer`(카운트다운+알람), `Visual timer`(원형 축소 시각 타이머), `Stopwatch`(랩 측정 스톱워치), `Clock`(디지털/아날로그 시계), `Timetable`(수업 일정·체크리스트·단계별 타이머), `Event countdown`(행사 D-Day 카운트다운).
   - **교실·행동 관리 (6종)**: `Traffic light`(신호등: 빨강/노랑/초록), `Sound level`(마이크 기반 교실 소음 측정기), `Work symbols`(침묵·속삭임·짝과 상의·모둠 협력 아이콘), `Randomizer`(랜덤 이름 뽑기/스피너), `Group maker`(자동 모둠 편성 + 특정 학생 분리 블랙리스트), `Dice`(1~3개 3D 주사위·동전·난수).
   - **상호작용·평가 (6종)**: `Poll`(실시간 원격/터치 투표), `QR Code`(즉석 URL QR 변환), `Scoreboard`(팀/개인 점수판), `Draw`(화면 위 그리기), `Text`(리치 텍스트·포스트잇 공지), `Stickers`(드래그 앤 드롭 스티커·자석 단어·게임 말).
   - **미디어·콘텐츠 (8종)**: `Background`(테마/커스텀 배경), `Calendar`(일간/월간 달력), `Image`(이미지/GIF 드래그 표시), `PDF`(학습지 PDF 페이지 넘김 뷰어), `Video`(YouTube·Vimeo·로컬 영상 재생), `Webcam`(실물화상기/웹캠 라이브 피드), `Embed`(외부 웹앱 임베드), `Hyperlink`(클릭 가능한 외부 링크 버튼).
   - **부가 공통 바**: `Annotation bar`(주석 바), 원격 투표(Remote poll voting).

2. **Free(Basic) vs Pro($36/년, 월 $3 연간 청구) 비교**:
   - **무료 범위의 파격성**: **26개 위젯 전부 + 주석 바 + 원격 투표 + 템플릿 라이브러리**가 무료(광고 없음). 명단 저장은 **최대 3개**, 이벤트 카운트다운은 **1개**까지 무료.
   - **Pro 유료 범위**: 화면 덱 저장·발행(무료는 일회용), 작업공간/폴더 정리, 학생 명단 **최대 100개(또는 무제한)** 저장, 커스텀 배경·캐러셀 업로드, Pro 배경 라이브러리, 위젯 커스텀 테마, 카운트다운 무제한 저장. (기관/학교 볼륨 라이선스 별도)

3. **UX 강점 및 한계**:
   - **강점**: 학생 회원가입 불필요, 수십 개 브라우저 탭(타이머+뽑기+QR+소음계)을 1화면으로 통합, 교실 뒤에서도 읽히는 고대비·대형 폰트 UI, `Group Maker`의 **학생 분리(블랙리스트) 비밀 설정**.
   - **한계**: **인터넷 상시 연결 필수(오프라인 불가)**, 미국·한국 학교 방화벽/유해사이트 필터 차단 사례(`r/Teachers`), 위젯 간 스냅/정렬 그리드 부재, 이름 뽑기 박스의 과도한 여백, `Poll` 중복 투표 어뷰징, `Sound level`의 시각 경고 외 강제력 부재, 동료 교사와 화면 링크 공유/협업 미흡.

---

### 1.4 Snipaste 심층 분석

1. **스마트 영역 감지(Auto-detection) 메커니즘**:
   - `F1` 진입 시 픽셀 분석(CV)이 아니라 **OS 접근성/UI 자동화 트리 API**를 호출해 마우스 커서 `(X, Y)` 아래 UI 컨트롤의 정확한 바운딩 박스를 마이크로초 단위로 조회함.
   - **Windows**: `UIAutomationCore.dll`(`IUIAutomation::ElementFromPoint` → `CurrentBoundingRectangle`) + 구형 Win32 앱용 `oleacc.dll`(MSAA `AccessibleObjectFromPoint`) 폴백.
   - **macOS**: Accessibility API(`AXUIElementCreateSystemWide()` → `AXUIElementCopyElementAtPosition` → `kAXPositionAttribute`/`kAXSizeAttribute`).
   - **조작 디테일**: `Tab` 키로 창 단위 감지↔세부 UI 요소 감지를 전환하고, 마우스 휠로 UI 계층(자식↔부모 컨테이너)을 오르내리며 영역을 확장·축소 가능. 커스텀 렌더링 앱(Electron 일부/게임)에서는 내부 요소 대신 외곽 창만 잡히는 한계가 있음.

2. **정밀 제어·히스토리·색상 선택기**:
   - **픽셀 정밀 제어**: `W/A/S/D`로 커서 1px 이동, 방향키로 선택 영역 경계 미세 조절, `Ctrl+A` 전체화면 선택, `R` 직전 캡처 영역 복원.
   - **히스토리 재생**: 캡처 모드 상태에서 `<` / `>` 키로 과거 성공 캡처 영역·이미지를 즉시 호출.
   - **색상 선택기**: 돋보기 활성화 중 `C` 키로 픽셀 색상 복사(RGB/HEX). 클립보드에 `#RRGGBB` 또는 RGB 값이 있을 때 `F3`을 누르면 **색상 견본 카드**로 화면에 핀 고정.

3. **핀(붙이기, Pin) 창 조작 및 다형식 클립보드 파싱**:
   - `F3`으로 클립보드 내용(이미지, 파일, **일반 텍스트, HTML 리치 텍스트, 색상 코드**)을 최상위(Always-on-top) 테두리 없는 플로팅 창으로 띄움.
   - **조작법**: 드래그 이동, 휠/코너 드래그로 확대·축소, 회전·좌우/상하 반전, **`Ctrl + 마우스 휠`로 개별 창 투명도(Opacity) 조절**, **마우스 클릭 통과(Click-through)**(글로벌 단축키 토글 시 창을 통과해 뒷배경 앱 클릭 가능), 그룹별 일괄 숨김/표시, `Space` 키로 핀 창 위 즉시 주석 편집.

4. **주석 도구 & Pro(유료) 범위**:
   - **무료(개인용)**: 사각형, 타원, 직선, 화살표, 자유선(연필), 형광펜(마커), 모자이크/가우시안 블러(휠로 강도·굵기 조절), 텍스트.
   - **Pro 유료 범위** (개인 외 상업용 필수 + 고급 기능, 데스크톱 1대 $8.99 / 3대 $19.99, Win/Mac/Linux 공용):
     - **OCR(텍스트 추출)** 및 **바코드/QR 코드 디코딩**
     - **Super-snip**(글로벌 단축키 없이 마우스/키 조합 즉시 캡처)
     - **핀 창 고급 제어**: 우클릭 메뉴에서 주석 직접 토글, 자유 선택(Free-select) 주석, `Esc`/`Shift+Esc` 동작 커스텀, **`F5` 핀 창 콘텐츠 즉시/자동 새로고침**
     - **고급 주석**: **번호 매기기(Number markings)**, **둥근 사각형(Rounded rectangles)**
     - **CLI·상세 커스텀**: 명령줄 제어 확장(화이트보드 커스텀 색상, 그룹 일괄 삭제, 자동저장 건너뛰기), 돋보기·트레이 마우스 액션 상세 설정.

5. **지원 OS, macOS 차이점, 구현 기술**:
   - **구현 기술**: **C++ + Qt 프레임워크** 기반 네이티브 앱(경량 메모리, 다중 모니터·HiDPI 처리).
   - **macOS 버전 차이점**: Windows는 포터블·설치형·MS Store를 지원하지만 macOS는 Mac App Store 없이 공식 사이트 **Universal Binary(`.dmg`, Apple Silicon+Intel)** 직접 배포만 제공. macOS 손쉬운 사용(Accessibility)·화면 기록 권한 필수이며, macOS 전용 sRGB 색공간 기본값 보정 및 macOS Sequoia/Apple Silicon에서 로그인 후 자동 실행 이슈(`Snipaste/feedback #3511`) 등 OS 특화 이슈가 존재. 또한 단축키(`Ctrl`→`Cmd`) 및 룩앤필이 Windows 이식 앱처럼 이질적이라는 평이 있음.

---

## 2. 기능 전수 목록 및 범주별 기능 매트릭스

**범례**: `O` = 기본 무료/내장 지원 · `P` = 유료/Pro 기능 · `△` = 부분 지원/플러그인/제한적 · `X` = 미지원

> 비교 대표 앱 열(10개 열로 압축하되 본문 요약표 24개 앱의 핵심 기능을 모두 포괄):
> **IZ**=아이캔줌잇 · **IN/S**=아이캔노트/스크린 · **ZI**=ZoomIt(PowerToys 최신) · **SP**=Snipaste · **PP**=PixPin · **SX**=ShareX · **FS**=Flameshot · **SC**=Shottr/CleanShot X · **CS**=Classroomscreen · **EP/pI**=Epic Pen(Pro)/ppInk

### 2.1 ① 화면 필기 (Live Overlay & Canvas Drawing)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| 정지 화면 위 필기 | O | O | O | O(캡처후) | O(캡처후) | O(캡처후) | O(캡처후) | O(캡처후) | O(캔버스) | O |
| **라이브 화면 위 실시간 필기(LiveDraw)** | X | O(스크린) | O | X | X | X | X | X | X | O |
| **클릭 통과(Click-through)↔판서 전환** | X | O(스크린) | O | O(핀창) | O(핀창) | X | X | X | X | O |
| 자유선 펜 & 굵기 휠 조절 | O | O | O | O | O | O | O | O | O | O |
| 형광펜(반투명 마커) | O | O | O | O | O | O | O | O | X | O |
| 직선·사각형·원·화살표 | O | O | O | O | O | O | O | O | X | P(EP)/O(pI) |
| **점선 도형/선/화살표 (`+Alt`)** | O | △ | X | X | X | O | X | X | X | X |
| 둥근 사각형 / 번호 스탬프(1,2,3) | X | X | X | P | O | O | O | O | X | O(pI) |
| **스마트 도형 보정(손그림→정형)** | X | O(노트) | X | X | X | X | X | X | X | X(Annotate O) |
| **한글 텍스트 타이핑 & 폰트 설정** | O | O | △(깨짐) | O | O | O | O | O | O | P(EP)/O(pI) |
| **개체 선택·이동·크기 조절(`S`)** | O | O | X | O(핀주석) | O | O | O | O | X | X |
| 실행 취소(`Ctrl+Z`) / 다시 실행(`Ctrl+Y`) | O | O | O(`Ctrl+Z`) | O | O | O | O | O | O | O |
| 획 단위 지우개(`D`) / 전체 지우기(`E`) | O | O | X(`E`만) | O | O | O | O | O | O | O |
| 화이트보드(`W`) / 블랙보드(`K`) 전환 | O | O | O | P(CLI) | X | X | X | X | O | P(EP)/O(pI) |
| 자동 사라지는 잉크(Fading Ink) | X | X | X | X | X | X | X | X | X | P(EP) |
| 필압(와콤·애플펜슬·Windows Ink) 지원 | X | O | X | X | X | X | X | X | X | O(EP) |
| 플로팅 미니 툴바 + 단축키 병행 | O | O | X(단축키만) | O | O | O | O | O | O | O |

> **초고도화 포인트**: 아이캔줌잇의 `점선 도형·개체 선택·획 지우개·한글 타이핑` + 아이캔스크린의 `라이브 오버레이·클릭 통과` + 아이캔노트의 `스마트 도형 보정` + Epic Pen/DemoPro의 `사라지는 잉크`를 단일 엔진으로 결합.

### 2.2 ② 화면 확대 (Zoom & Spotlight)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| 정지 화면 확대 (`Ctrl+1`) + 휠 배율 | O | O(노트) | O | X | X | X | X | X | X | X |
| **실시간 라이브 확대 (`Ctrl+4`)** | O | X | O | X | X | X | X | X | X | X |
| 확대 상태에서 즉시 필기 | O | O | O | X | X | X | X | X | X | X |
| **커서 중심 스포트라이트 / 돋보기** | X | O | X | O(돋보기) | O(돋보기) | O(돋보기) | X | O(Shottr) | X | O(pI 링) |
| **커튼(상하좌우 가림막 단계 공개)** | X | O | X | X | X | X | X | X | X | X |
| 보조 모니터 화면 미러링(DemoMirror) | X | X | O | X | X | X | X | X | X | X |

> **초고도화 포인트**: 정지 확대(`Ctrl+1`)와 라이브 확대(`Ctrl+4`)를 모두 지원하되, 라이브 확대 상태에서도 판서가 가능하도록 고도화하고 교실용 `커튼(가림막)`·`스포트라이트`를 내장.

### 2.3 ③ 스마트 캡처 (Capture & Recording)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| 자유 영역 드래그 캡처 | X | O | O(`Ctrl+6`) | O | O | O | O | O | X | △(EP) |
| **스마트 창·UI 요소 자동 감지(UIA/AX)** | X | X | X | O | O | O | X | △(창) | X | X |
| 휠로 UI 요소 부모/자식 계층 탐색 | X | X | X | O | O | X | X | X | X | X |
| 픽셀 미세 이동(`W/A/S/D`·화살표) + 돋보기 | X | X | X | O | O | O | X | O | X | X |
| **스크롤(긴 웹/문서) 수직·수평 캡처** | X | X | X | X | O | O | X | O | X | X |
| 직전 영역 재캡처(`R`) / 캡처 히스토리(`<`/`>`) | X | X | X | O | O | △ | X | O | X | X |
| 지연(타이머) 캡처 | X | X | X | X | X | O | O | O | X | X |
| 다중 캡처 병합 / 임시 트레이(Quick Access) | X | X | X | X | X | X | X | O(CSX) | X | X |
| **화면 녹화(MP4) / GIF 생성** | X | O(녹화) | O(`Ctrl+5`) | X | O | O | X | O(CSX) | X | X |

> **초고도화 포인트**: Snipaste의 `UI 요소 자동 스냅 + 히스토리`에 PixPin/Shottr의 `스크롤 캡처`, CleanShot X의 `Quick Access 임시 트레이`, ZoomIt의 `MP4/GIF 녹화`를 무료로 통합.

### 2.4 ④ 핀 / 붙이기 (Pin to Screen)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| 캡처 영역 즉시 최상위 핀 창 띄우기 | X | X | X | O | O | O | O | O | X | X |
| 클립보드 이미지 → 핀 창 (`F3`) | X | X | X | O | O | O | X | O | X | X |
| **텍스트·HTML → 핀 창 렌더링** | X | X | X | O | O | X | X | X | X | X |
| **색상 코드(HEX/RGB) → 색상 카드 핀** | X | X | X | O | O | X | X | X | X | X |
| 핀 창 확대·축소·회전·좌우/상하 반전 | X | X | X | O | O | X | O(크기) | O | X | X |
| **개별 핀 창 투명도 조절 (`Ctrl+휠`)** | X | X | X | O | O | X | O | O | X | X |
| **핀 창 클릭 통과(Click-through)** | X | X | X | O | O | X | X | X | X | X |
| **핀 창 위 직접 주석(`Space`) & 재편집** | X | X | X | P(일부O) | O | X | X | X | X | X |
| 핀 창 그룹 관리(일괄 숨김/표시/삭제) | X | X | X | O | O | X | X | X | X | X |
| 핀 창 내 즉시 OCR 텍스트 추출 / 번역 | X | X | X | P | O | X | X | O | X | X |
| 핀 창 영역 라이브 새로고침(`F5`) | X | X | X | P | X | X | X | X | X | X |

> **초고도화 포인트**: 교사가 문제지 한 문제나 정답표·수업 타이머·텍스트 공지를 `F3`으로 떼어내 화면 위에 고정하고 투명도 조절·클릭 통과·핀 위 판서까지 할 수 있는 핵심 킬러 워크플로우 구현.

### 2.5 ⑤ 이미지 편집·보안 가리기 (Editing & Redaction)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| 모자이크 / 가우시안 블러 | X | X | X | O | O | O | O | O | X | X |
| **텍스트/민감정보 자동 감지 가리기** | X | X | X | X | X | X | X | O(CSX/Shottr) | X | X |
| 크롭(자르기)·리사이즈·회전 | X | X | X | O | O | O | X | O | X | X |
| **배경 패딩·그라데이션·그림자(Prettify)** | X | X | X | X | X | X | X | O(CSX/Xnapper) | X | X |
| 워터마크 삽입 | X | X | X | X | X | O | X | O | X | X |
| 비파괴 벡터 주석 재편집 | X | O(노트) | X | △(재편집요청) | O | O(편집기) | X | O | X | X |

### 2.6 ⑥ OCR · QR 코드 (Text & Code Intelligence)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| **오프라인 한글·영문 고정밀 OCR** | X | X | △(WinOCR) | P | O | △(WinOCR) | X | O(Mac) | X | X |
| **QR 코드·바코드 스캔(디코딩)** | X | X | X | P | O | O | X | X | X | X |
| **QR 코드 생성기 (URL/텍스트 → 핀·공유)** | X | X | X | X | X | O | X | X | O | X |
| 텍스트 음성 받아쓰기(Snip Dictation) | X | X | X | X | X | X | X | X | X | X |

> **초고도화 포인트**: QR **생성**(Classroomscreen 기능) + QR **디코딩**(Snipaste Pro) + **오프라인 한글 고정밀 OCR**(Apple Vision / RapidOCR ONNX)을 무료 기본 탑재.

### 2.7 ⑦ 교실 도구 (Classroom Management Widgets)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| 타이머 / 스톱워치 (`Ctrl+3`) | O | O | O | X | X | X | X | X | O | X |
| **시각적 타이머(원형/바 형태)** | X | X | X | X | X | X | X | X | O | X |
| **랜덤 이름 뽑기 / 모둠 편성(블랙리스트)** | X | X | X | X | X | X | X | X | O | X |
| **교실 소음 측정기(마이크 dB)** | X | X | X | X | X | X | X | X | O | X |
| **신호등 / 작업 기호(침묵·모둠 등)** | X | X | X | X | X | X | X | X | O | X |
| **주사위 / 점수판 / 수업 시간표** | X | X | X | X | X | X | X | X | O | X |
| **즉석 투표(Poll) / 스티커 / 실물화상기(웹캠)** | X | X | X | X | X | X | X | X | O | X |
| 학습지 PDF 페이지 뷰어 + 위젯 동시 배치 | X | O(PDF) | X | X | X | X | X | X | O | X |

> **초고도화 포인트**: 웹 기반 Classroomscreen과 달리 **100% 오프라인 데스크톱 위젯 + 플로팅 핀 창**으로 제공하여 학교 방화벽·인터넷 장애와 무관하게 수업 중 어떤 앱(PPT·PDF·한글) 위에도 타이머·신호등·뽑기·QR을 띄워둘 수 있게 함.

### 2.8 ⑧ 유틸리티 (Desktop Utilities)

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| **화면 색상 추출기 (HEX/RGB/HSV)** | X | X | X | O | O | O | X | O(Shottr) | X | X |
| **화면 픽셀 눈금자 / 각도기 / 십자선** | X | X | X | △(크기) | X | O | X | O(Shottr) | X | X(PicPick O) |
| **로컬 클립보드 히스토리 관리자** | X | X | X | △(이미지) | △ | △ | X | X | X | X |
| 이미지 합치기 / PDF·JPG 일괄 내보내기 | X | O(노트) | X | X | X | O | X | X | X | X |

### 2.9 ⑨ 설정 · 배포 · OS 호환성

| 세부 기능 | IZ | IN/S | ZI | SP | PP | SX | FS | SC | CS | EP/pI |
|---|---|---|---|---|---|---|---|---|---|---|
| Windows `.exe` 설치 / **무설치 포터블(`.zip`)** | O(포터블) | O | O(포터블) | O(둘다) | O | O(둘다) | O | X | X(웹) | O |
| **Microsoft Store 무료 배포** | X | X | O(PowerToys) | O | X | O | X | X | X | X |
| **macOS `.dmg` (Apple Silicon + Intel)** | X | X | X | O | O | X | O | O | X(웹) | O(EP) |
| 한국어 + 영어 완벽 UI 지원 | △(한글만) | △(한글만) | △(영어위주) | O | △(중/영) | O | O | △(영문) | O | O |
| Per-Monitor DPI(4K 전자칠판) 대응 | △ | △ | O | O | O | O | △ | O(Mac) | O(웹) | △ |
| 완전 무광고 & 오프라인 100% 동작 | O | O | O | O | △ | O | O | O | X | O |

---

## 3. 사용자 불만 · 미충족 요구 상위 15개 및 한국 특화 포인트

### 3.1 사용자 불만 · 미충족 요구 상위 15개 (출처 포함)

| 순위 | 불만 / 미충족 요구 | 영향받는 주요 앱 | 사용자 목소리 및 확인된 출처 | 신규 앱 해결 방향 |
|---|---|---|---|---|
| **1** | **스크롤(긴 화면) 캡처 부재** | Snipaste, Flameshot, ZoomIt, 아이캔줌잇 | `Snipaste/feedback #19`는 **댓글 347개·리액션 83개**로 역대 최다 요청 이슈이며, 웹페이지·PDF 전체를 못 찍어 QQ/WeChat·PixPin으로 이탈한다는 불만 다수. | 수직·수평 자동 스크롤 캡처 및 수동 파노라마 스티칭 기본 무료 탑재 |
| **2** | **무료·고정밀 오프라인 한글 OCR 부재** | Snipaste, PixPin, PowerToys Text Extractor | `Snipaste/feedback #188`(69댓글) 최다 요청 중 하나였으나 Pro로 제한; PixPin도 일부 Pro 전환; PowerToys Text Extractor(Win OCR)는 한글 정확도 70~80%대로 받침·작은 폰트 오인식 빈발(Reddit/한국 블로그). | macOS Apple Vision + Windows RapidOCR(PaddleOCR 한국어 ONNX) 오프라인 무료 내장 |
| **3** | **캡처+핀 도구와 실시간 화면 필기·확대 도구의 분절** | ZoomIt + Snipaste + Classroomscreen | Reddit(`r/software`, `r/Teachers`)에서 교사·발표자들이 "화면 위 필기 + 캡처 후 핀 고정 + 교실 타이머"를 위해 앱 3~4개를 동시에 띄우고 단축키가 꼬이는 불편 호소. | 확대·라이브 판서·캡처·핀·교실 위젯을 단일 앱·통합 단축키 체계로 통합 |
| **4** | **구독 피로감 및 핵심 기능 페이월** | Epic Pen, CleanShot X, Snagit, 알캡처, Classroomscreen | Epic Pen은 기본 도형·텍스트까지 월/연 구독화(Reddit/Trustpilot 불만), CleanShot X는 연 $19 업데이트 갱신 피로(`r/macapps`), Snagit은 구독 전환, 알캡처는 AD-ZERO 유료화 유도. | 핵심 캡처·핀·판서·OCR·교실 도구를 **영구 무료**로 제공 (MS Store 무료 배포) |
| **5** | **Windows·macOS 크로스플랫폼 패리티 부족** | 아이캔 시리즈, ZoomIt, ShareX, 픽픽, 알캡처, Shottr, CleanShot X | 아이캔·ZoomIt·ShareX·픽픽은 Windows 전용, Shottr·CleanShot X·Presentify·DemoPro는 macOS 전용. Snipaste는 Mac에서 Windows 이식 앱 특유의 이질감·Sequoia 자동실행 버그(`#3511`). | Windows(`.exe`/MS Store) + macOS(`.dmg` Universal) 동시 1급 지원, OS 네이티브 단축키 프리셋(`Ctrl`↔`Cmd`) |
| **6** | **멀티모니터·4K HiDPI 배율(150~200%) 좌표 밀림** | Flameshot, 아이캔줌잇, Snipaste | 교탁 PC(FHD 100%) + 교실 4K 전자칠판(150~200%) 복제/확장 시 캡처 영역 어긋남·트레이 아이콘 흐림·터치 좌표 밀림(`Snipaste/feedback` DPI 이슈, Reddit `Flameshot` 불만). | Per-Monitor DPI Awareness v2 및 모니터별 독립 스케일링 매핑 구현 |
| **7** | **글로벌 단축키 충돌 및 불편한 입력 방식** | 아이캔줌잇, Snipaste, DemoPro | 아이캔줌잇 `Ctrl+1/2`는 브라우저 탭·오피스 단축키와 충돌(블로그·인디스쿨); Snipaste `F1/F3` 충돌(`#818`, 29댓글); DemoPro는 키를 누른 채 드래그해야 해 손이 피로함(App Store 리뷰). | 전역 단축키 충돌 감지·프로필별 프리셋(줌잇형/Snipaste형/커스텀) + 토글/홀드 모드 선택 |
| **8** | **광고·번들 팝업 및 DRM 오탐 캡처 차단** | 알캡처, 픽픽 | 알캡처는 캡처마다 우하단 광고 노출 및 업데이트 시 광고 비활성화 설정 초기화(티스토리 다수); 픽픽은 Steam 등 실행 중 DRM 오탐으로 캡처 먹통(나무위키/블로그). | 100% 무광고·무번들 정책, 표준 OS 캡처 API(DXGI/ScreenCaptureKit) 사용 |
| **9** | **학교·기업 PC 보안 차단(관리자 권한·SmartScreen·방화벽)** | 아이캔줌잇, MS Store 앱, Classroomscreen | 교육청 GPO로 MS Store 차단(인디스쿨), 미서명 포터블 `.exe` 실행 시 SmartScreen 차단, 교내 백신(V3/알약) 격리, 학교 웹 필터가 `classroomscreen.com` 차단(`r/Teachers`). | 코드 서명된 **무설치 포터블 + 설치형 + MS Store** 3종 배포 및 100% 오프라인 구동 |
| **10** | **교실 대시보드의 오프라인 불가 & 위젯 레이아웃 경직** | Classroomscreen | 인터넷 끊기면 수업 중단, 위젯 간 스냅 그리드 부재, 이름 뽑기 박스가 텍스트 대비 너무 큼(SoftwareFinder, G2 리뷰). | 로컬 저장(학생 명단·화면 프리셋) + 위젯 스냅 정렬 + 화면 위 핀처럼 띄우는 오버레이 위젯 |
| **11** | **캡처 후 비파괴 2차 재편집 불가 & 바탕화면 파일 난립** | OS 기본 캡처 도구, Snipaste, Flameshot | `Snipaste/feedback #1126`(35댓글) — 이미 그린 주석을 나중에 다시 수정할 수 없음; macOS/Windows 기본 도구는 바탕화면에 PNG를 수십 개씩 쌓아둠(`r/macapps`). | 벡터 레이어 기반 비파괴 재편집 + CleanShot X식 임시 플로팅 트레이(자동 정리) |
| **12** | **원조 ZoomIt 툴바 부재 vs 아이캔줌잇 라이브 판서/녹화 부재** | Sysinternals ZoomIt, 아이캔줌잇 | 원조 ZoomIt은 툴바 없이 단축키 암기 필수+한글 타이핑/개체선택 불편(이지쌤교실 블로그); 아이캔줌잇은 정지 화면에 갇혀 스크롤·동영상 조작 중 필기 및 녹화가 안 됨. | 플로팅 툴바 + 단축키 동시 지원, LiveDraw(실시간 클릭 통과 판서) + MP4/GIF 녹화 통합 |
| **13** | **과도한 복잡도 vs 지나친 단순함의 UX 간극** | ShareX, Annotate, gInk | Reddit `r/software`에서 ShareX는 설정 메뉴가 너무 많아 초보자 포기, Annotate는 단축키·컨텍스트 메뉴 학습곡선 과다, 반면 gInk/OS 툴은 도형·텍스트·OCR이 부족하다는 평가. | 1클릭 기본 모드(심플) + 접이식 고급 패널(파워유저/교사 도구) 2단 UI |
| **14** | **클립보드·히스토리 민감정보 노출 및 가짜 블러 보안 우려** | PixPin, Annotate, 클라우드 자동업로드 툴 | 비공개 소스 앱의 클라우드/텔레메트리 우려, 비밀번호·계좌번호 캡처 히스토리 잔류 위험, Annotate 블러가 암호화 마스킹이 아니라는 경고(MS Store 설명). | 100% 로컬 처리(외부 전송 0), 시크릿/자동 삭제 타이머, 픽셀 영구 파괴(Irreversible) 블러·모자이크 |
| **15** | **다중 캡처 병합·워터마크·교실 투표 어뷰징/소음계 실효성** | Snipaste, Classroomscreen | `Snipaste/feedback #45`(병합)·`#454`(워터마크) 장기 미해결; Classroomscreen `r/Teachers`에서 학생 투표 중복 어뷰징 및 소음 측정기 무시 문제 지적. | 다중 이미지 병합, 워터마크 프리셋, 소음 임계치 초과 시 경고음/시각 경고 트리거 |

---

### 3.2 한국 사용자 · 교사 환경 특화 포인트

1. **한국 교사 커뮤니티(인디스쿨·아이캔노트 카페·교사 유튜브) 핵심 니즈**:
   - **원클릭 수업 자료 제작 흐름**: 인디스쿨 인기 교사 제작 툴(`CutPen` 등) 사례에서 확인되듯, 교사들은 *영역 지정 → 즉시 박스/화살표/모자이크(학생 이름·주민번호 가리기) → `Enter` 한 번으로 클립보드 복사 후 한글(HWP)·PPT·밴드/하이클래스 붙여넣기*의 **초단축 워크플로우**를 최우선으로 꼽음.
   - **수업 중 3대 결합 요구**: ① 학습지/PDF/교과서 웹뷰어 확대 + 형광펜 + 한글 텍스트 설명 타이핑(아이캔 줌잇/노트 영역) ② 학생 활동용 대형 타이머 + 모둠/발표자 랜덤 뽑기 + QR 코드(Classroomscreen 영역) ③ 문제 일부분만 오려 화면 한쪽에 띄워두고 판서하기(Snipaste 핀 영역).
2. **학교 PC 보안·권한 환경 제약 대응**:
   - **관리자 권한 부재 & MS Store 차단**: 교육청/시도 교육기관 그룹 정책(GPO)으로 MS Store 접근과 시스템 영역 설치(`Program Files`)가 막힌 교실 PC가 많음 → **단일 폴더 무설치 포터블(`.zip`/`.exe`)**과 **사용자 영역 설치(Per-User Installer)**, **MS Store(MSIX)** 세 가지를 모두 제공해야 함.
   - **SmartScreen 및 교내 백신(V3·알약) 오탐 대응**: 아이캔 줌잇도 미서명 빌드 시 SmartScreen 경고가 진입장벽으로 지적됨. 정식 **코드 서명(Trusted Signing / EV 인증서)** 적용과 정적/동적 백신 화이트리스트 제출, UPX 패킹 미사용이 필수.
   - **망분리/오프라인 교실**: 인터넷이 불안정하거나 Classroomscreen 같은 해외 웹서비스가 교육청 방화벽에 막히는 환경에서도 **100% 로컬 오프라인**으로 모든 위젯(타이머·뽑기·QR 생성·OCR)이 돌아가야 함.
3. **교실 하드웨어(전자칠판·대형 TV·교탁 PC) 대응**:
   - **4K UHD(3840×2160) + 고배율(150~200%)**: 대형 전자칠판과 교탁 모니터(FHD 100%)를 **복제/확장 듀얼**로 연결할 때 창 크기나 펜 좌표가 밀리는 문제가 잦음 → Per-Monitor DPI v2 필수.
   - **터치·전자펜(Windows Ink) 최적화**: 단축키(`Ctrl+1` 등)를 누르기 힘든 전자칠판 앞 서서 진행하는 수업에서는 **화면 모서리 도킹 플로팅 툴바 + 터치 제스처**가 필수.
4. **한글 OCR 정확도 전략**:
   - 학습지·옛날 PDF·교과서 캡처 후 한글(HWP) 문서로 옮길 때 Windows 내장 OCR(PowerToys Text Extractor)은 한글 정확도(약 70~80%, 받침·작은 폰트 취약)로 실무 만족도가 낮음.
   - 따라서 **macOS는 Apple Vision(`VNRecognizeTextRequest`, 한국어 최상급)**, **Windows는 RapidOCR 기반 PaddleOCR PP-OCRv4/v5 한국어 ONNX 모델(95%+ 정확도, Apache-2.0)**을 온디바이스 번들링하는 전략이 가장 효과적임.

---

## 4. 기술 스택 · 오픈소스 참고 목록

### 4.1 GitHub 오픈소스 저장소 참고 목록 *(2026-11-24 GitHub API 직접 조회 기준)*

> ⚠️ **라이선스 핵심 주의사항**: 상용/무료 폐쇄형(또는 MS Store 독자 배포)으로 출시하려면 **`GPL-3.0`**(`Flameshot`, `ShareX`, `Greenshot`, `ksnip`, `eSearch`, `OpenBoard`) 및 **`Snow Apps` 커스텀 라이선스**(`snow-shot`) 코드는 **절대 직접 복사·링크하지 말고**, UI/아키텍처·기능 레퍼런스로만 참고해야 함. 코드·크레이트 재사용은 **`MIT` / `Apache-2.0` / `LGPL-3.0(동적 링크)`** 계열로 제한해야 함.

| 저장소 (GitHub URL) | Stars | 주력 언어/스택 | 라이선스 | 참고할 핵심 모듈 / 구현 포인트 | 상용·스토어 활용 주의점 |
|---|---|---|---|---|---|
| `https://github.com/ShareX/ShareX` | 41,344 | C# (.NET WinForms/WPF) | GPL-3.0 | Windows 캡처 모드 전체(스크롤·창·영역), 눈금자·색상피커·QR·단축키 엔진 | GPL-3.0 (아이디어·UX 참고만) |
| `https://github.com/flameshot-org/flameshot` | 31,704 | C++ (Qt) | GPL-3.0 | 캡처 직후 인라인 원형/사이드 주석 툴바 UX, 크로스플랫폼 빌드 구조 | GPL-3.0 (코드 차용 불가) |
| `https://github.com/greenshot/greenshot` | 5,288 | C# (.NET WinForms) | GPL-3.0 | 경량 캡처 + 내장 편집기 구조, Windows UI 캡처 처리 | GPL-3.0 (코드 차용 불가) |
| `https://github.com/ksnip/ksnip` | 3,387 | C++ (Qt5/Qt6) | GPL-3.0 | 프레임 없는 핀 창, 탭 편집기, Tesseract OCR 플러그인 연동 구조 | GPL-3.0 (코드 차용 불가) |
| `https://github.com/ksnip/kImageAnnotator` | 95 | C++ (Qt) | **LGPL-3.0** | 벡터 주석(화살표·번호 스텝퍼·블러·선택 재편집) 전용 모듈 | LGPL-3.0 (동적 링크 시 주의) |
| `https://github.com/microsoft/PowerToys` (ZoomIt 모듈) | 141,346 | C++ / WinRT Interop | **MIT** | `src/modules/ZoomIt/ZoomIt` — 네이티브 C++ 정지/라이브 줌·LiveDraw·녹화·DemoMirror | **MIT (코드·패턴 적극 참고 가능)**, Win 전용 |
| `https://github.com/geovens/gInk` | 2,626 | C# (.NET WinForms) | **MIT** | 초경량 투명 오버레이·멀티 펜 슬롯·클릭 통과 전환 아키텍처 | **MIT (참고·차용 가능)**, Win 전용 |
| `https://github.com/pubpub-zz/ppInk` | 1,327 | C# (.NET WinForms) | **MIT** | gInk 확장(도형·텍스트·스냅·커서 하이라이트 링·스탬프) 구현 | **MIT (참고·차용 가능)**, Win 전용 |
| `https://github.com/xland/ScreenCapture` | 1,596 | C++ (ThorVG) | **MIT** | 단일 실행 파일 초경량 캡처·벡터 주석(ThorVG 기반)·핀 구현 | **MIT (참고·차용 가능)** |
| `https://github.com/nashaofu/xcap` | 1,042 | Rust | **Apache-2.0** | Win/macOS/Linux(X11·Wayland) 크로스플랫폼 화면·창 캡처 크레이트 | **Apache-2.0 (직접 의존성 채택 유력)** |
| `https://github.com/mg-chao/snow-shot` | 7,413 | C++ / Rust / Python | **Other (Snow Apps Licensing)** | 캡처·핀·스크롤·오프라인 OCR·녹화 올인원 아키텍처 참고 | 커스텀 라이선스 (코드 차용 금지) |
| `https://github.com/xushengfeng/eSearch` | 7,476 | TypeScript (Electron) | GPL-3.0 | 만향 스크롤 캡처·핀·오프라인 OCR·화면 번역 UI 플로우 | GPL-3.0 (UX 참고만) |
| `https://github.com/dynobo/normcap` | 2,778 | Python (PySide Qt) | GPL-3.0(확인필요:`Other`표기) | 영역 지정 → 즉시 오프라인 OCR 클립보드 복사 UX 파이프라인 | 라이선스 불명확(`Other`), UX만 참고 |
| `https://github.com/OpenBoard-org/OpenBoard` | 3,112 | C++ (Qt) | GPL-3.0 | 교실용 전자칠판·PDF 판서·교사 유틸리티(자·각도기·커튼) UX | GPL-3.0 (교실 UX 참고만) |
| `https://github.com/RapidAI/RapidOCR` | — | C++ / Python / ONNX | **Apache-2.0** | PaddleOCR 모델의 경량 ONNX 변환·추론 파이프라인(Python/OpenCV 불필요) | **Apache-2.0 (NOTICE 표기 후 번들 가능)** |
| `https://github.com/PaddlePaddle/PaddleOCR` | — | Python / C++ | **Apache-2.0** | `PP-OCRv4` / `PP-OCRv5` 한국어 검출(DBNet)·인식 모델 가중치 | **Apache-2.0 (NOTICE 표기 후 번들 가능)** |
| `https://github.com/pykeio/ort` | — | Rust | **MIT / Apache-2.0** | Rust용 ONNX Runtime 바인딩(Win DirectML / macOS CoreML 가속) | **MIT/Apache-2.0 (직접 채택 유력)** |

*(참고: Tauri 2 기반 경량 캡처·핀 오픈소스 레퍼런스로 `tstone-1/screenpick`, `hzguz/prin`, `codehakase/snipp` 등도 존재하나 API 직접 조회 대상 외 검색 결과로만 확인됨)*

---

### 4.2 핵심 기술 구현 메커니즘 및 프레임워크 비교

1. **플랫폼별 네이티브 핵심 API 매핑**:

| 핵심 기능 | Windows 구현 API | macOS 구현 API | 주의·함정 사항 |
|---|---|---|---|
| **고속 화면 캡처 / 실시간 확대** | DXGI Desktop Duplication / `Windows.Graphics.Capture` (WGC) | `ScreenCaptureKit` (`SCStream` / `SCScreenshotManager`) | macOS 12.3+ ScreenCaptureKit 권장, 화면 기록 권한 필수; Win WGC로 DRM 오탐 및 다중 모니터 DPI 안정화 |
| **스마트 창·UI 요소 자동 감지** | UI Automation (`IUIAutomation::ElementFromPoint` + `CurrentBoundingRectangle`) + MSAA(`oleacc.dll`) 폴백 | Accessibility API (`AXUIElementCreateSystemWide` → `AXUIElementCopyElementAtPosition`) | macOS 손쉬운 사용(Accessibility) 권한 필요; Electron/게임 등 커스텀 렌더러는 외곽 창만 잡힘 |
| **투명 오버레이 & 클릭 통과** | 계층 창(`WS_EX_LAYERED \| WS_EX_TRANSPARENT \| WS_EX_TOPMOST`) + Direct2D/Skia | `NSWindow` (`ignoresMouseEvents = true`, `level = .screenSaver`, `collectionBehavior = .canJoinAllSpaces`) | 다중 모니터·전체화면 스페이스(macOS Space) 위 오버레이 유지 처리 필수 |
| **오프라인 한글 OCR** | `RapidOCR` + `PaddleOCR PP-OCRv4/v5` 한국어 ONNX (`ort` + DirectML/CPU) | **Apple Vision** (`VNRecognizeTextRequest`, `recognitionLanguages = ["ko-KR", "en-US"]`) | Windows 내장 OCR은 한글 정확도 저하로 보조/미사용 권장; 모델 ONNX 번들 용량 관리 필요 |
| **화면 녹화 (MP4/GIF)** | WGC + Media Foundation(하드웨어 H.264) / FFmpeg | `ScreenCaptureKit` + `AVAssetWriter` / GIF 인코더 | 확대/판서 오버레이 레이어까지 프레임에 함께 합성(Composite)해야 함 |

2. **데스크톱 프레임워크 3종 비교 (기획 앱 적합성)**:

| 평가 기준 | ① **Tauri 2 (Rust + WebView/Canvas) + OS 네이티브 브리지** | ② **Qt 6 (C++)** *(Snipaste·Flameshot 방식)* | ③ **Electron (TS/Chromium)** *(eSearch 방식)* |
|---|---|---|---|
| **설치/포터블 용량** | **매우 작음 (~10~25MB + OCR 모델)** (Win WebView2 기본 탑재) | 중간 (~30~60MB, DLL 동봉) | 큼 (~100~200MB 이상) |
| **상시 대기 메모리/속도** | **가벼움** (백엔드 Rust 네이티브, UI 창만 WebView 지연 생성 가능) | **최고 경량·최고 응답속도** | 무거움 (저사양 학교 PC 부담) |
| **교실 위젯(Classroomscreen) 구현** | **매우 빠름** (타이머·뽑기·QR·투표·다국어 UI를 React/Svelte로 즉시 구축) | 느림 (QML/Widgets로 리치 위젯 제작 공수 큼) | 빠름 |
| **캡처/오버레이/확대 정밀도** | Rust(`xcap`·`windows-rs`·`objc2`) + 투명 네이티브 캔버스(또는 WebGL/Canvas2D) 분리 설계 필요 | 단일 C++ 레이어에서 OS 창·픽셀 직접 제어로 가장 매끄러움 | OS 창·저지연 오버레이 제어에 네이티브 애드온 필수 |
| **라이선스 / Store 배포** | **MIT / Apache-2.0** (상용·MS Store 완전 자유) | 오픈소스 버전은 **LGPL/GPL** 제약 (정적 링크·포터블 단일 exe 시 상용 라이선스 구매 필요) | MIT (배포 자유로우나 용량 큼) |
| **권장 결론** | **1순위 추천**: Rust 백엔드(캡처·UIA/AX·확대·LiveDraw·ONNX OCR) + Tauri/네이티브 경량 창(교실 위젯·설정·핀 트레이) 하이브리드 | 순수 C++ 팀이고 LGPL 동적 링크 준수(또는 Qt 상용 라이선스) 시 대안 | 저사양 교실 PC·무설치 포터블 배포 목표와 상충 |

---

## 5. 출처 URL 목록 및 `[미확인]` 항목

### 5.1 직접 확인·검증한 출처 URL 목록

#### A. 아이캔 줌잇 · 아이캔 노트 · 아이캔 스크린
- 아이캔 줌잇 단축키 설명서 PDF (전문 추출): `https://padlet-uploads.storage.googleapis.com/204219175/888b79ed29e77745b45d362f995c77ef/___.pdf`
- 이지쌤교실 YouTube 영상 (메타데이터·설명글 확인): `https://www.youtube.com/watch?v=tWF0mWjMAF0`
- 이지쌤교실 네이버 블로그 원문 (단축키·툴바·ZoomIt 비교): `https://m.blog.naver.com/iz_ssaem/223318741562`
- 아이캔 시리즈 공식 배포 카페: `https://cafe.naver.com/icannote`
- 아이캔 시리즈 공식 도메인 (접속 리셋 발생): `https://www.icannote.com/`

#### B. Classroomscreen
- Classroomscreen 공식 사이트 & 위젯/가격 안내: `https://classroomscreen.com/`
- 리뷰·비교 출처: G2 (`https://www.g2.com/`), SoftwareFinder (`https://softwarefinder.com/`), Reddit `r/Teachers` (`https://www.reddit.com/r/Teachers/`)

#### C. Snipaste & PixPin
- Snipaste 공식 사이트 & 문서: `https://www.snipaste.com/`, `https://docs.snipaste.com/key-bindings`, `https://docs.snipaste.com/pro`, `https://docs.snipaste.com/troubleshooting`
- Snipaste 공식 피드백·이슈 트래커(GitHub API 직접 조회): `https://github.com/Snipaste/feedback` (`#19`, `#45`, `#188`, `#282`, `#454`, `#818`, `#1112`, `#1126`, `#3511` 등)
- 공부해서 남주는 선생님 YouTube 영상 (메타데이터·설명글 확인): `https://www.youtube.com/watch?v=1dCk2zBJ2jE`
- 티스토리 Snipaste 사용법 원문: `https://s-bug.tistory.com/4`
- PixPin 공식 사이트 및 Homebrew Cask: `https://pixpin.com/`, `https://formulae.brew.sh/`

#### D. 경쟁 앱 · 오픈소스 저장소 · 기술 문서
- GitHub 저장소 (모두 GitHub API로 직접 조회 완료):
  - `https://github.com/ShareX/ShareX`
  - `https://github.com/flameshot-org/flameshot`
  - `https://github.com/greenshot/greenshot`
  - `https://github.com/ksnip/ksnip`
  - `https://github.com/ksnip/kImageAnnotator`
  - `https://github.com/microsoft/PowerToys` (ZoomIt 모듈 포함)
  - `https://github.com/geovens/gInk`
  - `https://github.com/pubpub-zz/ppInk`
  - `https://github.com/xland/ScreenCapture`
  - `https://github.com/nashaofu/xcap`
  - `https://github.com/mg-chao/snow-shot`
  - `https://github.com/xushengfeng/eSearch`
  - `https://github.com/dynobo/normcap`
  - `https://github.com/OpenBoard-org/OpenBoard`
  - `https://github.com/RapidAI/RapidOCR`
  - `https://github.com/PaddlePaddle/PaddleOCR`
  - `https://github.com/pykeio/ort`
- 경쟁 상용/프리웨어 공식 사이트:
  - Shottr (`https://shottr.cc/`), CleanShot X (`https://cleanshot.com/`), Epic Pen (`https://epicpen.com/`), Presentify (`https://presentifyapp.com/`)
- 커뮤니티·기술 참고: Reddit (`r/software`, `r/windows`, `r/macapps`, `r/Teachers`), Microsoft UI Automation 공식 문서(`IUIAutomation`), Apple Accessibility(`AXUIElement`)

---

### 5.2 `[미확인]` 항목 목록

1. **`[미확인]` YouTube 영상 2건의 실제 화면 시연·음성 발화 전체 내용** (`tWF0mWjMAF0`, `1dCk2zBJ2jE`): 비브라우저 환경 제약 및 YouTube timedtext 응답 미반환으로 영상 내 구두 팁·시연 화면 프레임은 직접 확인하지 못했으며, 영상 설명란의 네이버 블로그 원문(`iz_ssaem/223318741562`)과 공식 단축키 PDF로 대체함.
2. **`[미확인]` 아이캔 줌잇 전체 버전별 상세 체인지로그 및 [사용자 설정] 대화상자의 모든 세부 체크박스 항목**: 공식 네이버 카페 내부 게시글 및 `.exe` 바이너리 실행 화면 미열람.
3. **`[미확인]` 아이캔 노트 · 아이캔 스크린의 전체 단축키 표 및 공식 사이트(`icannote.com`) 최신 안내문**: `https://www.icannote.com/` 접속 시 `Connection reset by peer` 오류로 원문 직접 열람 불가.
4. **`[미확인]` `docs.snipaste.com`의 세부 본문 전체 텍스트**: 해당 문서 사이트가 SPA(클라이언트 사이드 JS 렌더링) 구조여서 단순 HTTP 요청 시 내비게이션 헤더만 반환되었으며, `Snipaste/feedback` GitHub Wiki·이슈 및 검색 요약으로 교차 확인함.
5. **`[미확인]` 국내 교사 커뮤니티 `indischool.com` 내부 로그인 전용 게시글 원문**: 회원 인증이 필요한 인디스쿨 내부 스레드는 직접 열람하지 못했고, 외부 공개 교사 블로그(`dorms.school`, `iz_ssaem` 등) 및 웹 검색으로 간접 확인함.
