# 수업 자료를 집어 바로 설명하는 데스크톱 도구

**첫 제품은 ‘문제 한 칸을 집어 답을 덮고, 교실 TV에 크게 띄워 풀이하는 도구’로 제안한다.** 교사는 사용 중인 PDF·슬라이드·웹 교재를 그대로 둔다. 한 문제를 캡처해 화면에 붙이고, 펜으로 풀고, 학생이 답할 때 원래 답을 공개한다. 이 한 장면 안에서 영역 인식, 확대, 필기, 핀, 가리기의 가치와 실패 지점을 모두 확인할 수 있다.

ZoomIt은 확대·판서를, Snipaste는 캡처·핀을, CleanShot X와 ShareX는 OCR을 포함한 캡처 후 작업을 제공한다. Classroomscreen에는 이미 타이머·뽑기·소음계·QR이 있다. 차별화 가설은 교사가 **수업 중에 자료를 옮기고 다시 편집하는 횟수**를 줄이면 이 도구를 계속 쓴다는 것이다. 아이캔 줌잇·노트·스크린의 판서·노트·화면 운영, PixPin의 캡처 흐름은 기획 배경으로 삼았다. 개별 제품의 미지원 여부나 독점 기능을 전제하지 않는다. [ZoomIt](https://learn.microsoft.com/en-us/sysinternals/downloads/zoomit) · [Snipaste](https://www.snipaste.com/) · [CleanShot X](https://cleanshot.com/features) · [ShareX](https://getsharex.com/) · [Classroomscreen](https://classroomscreen.com/widgets)

우선순위는 **P0 = MVP**, **P1 = 2차**, **P2 = 후보**다. 난이도 하는 한 화면 또는 한 라이브러리로 구현 가능한 수준, 중은 상태·입력·저장 경로 통합이 필요한 수준, 상은 두 운영체제의 실기기 검증이나 인식 품질 평가가 필요한 수준이다. 기능표의 난이도·효과와 비교표의 상대 평가는 모두 설계 단계 **[추정]**이다. 공식 문서 확인일은 2026-11-24다.

## 1. 기능 로드맵

| 범주 | MVP · P0 | 2차 · P1 | 후보 · P2 |
|---|---|---|---|
| 설명·확대 | **문제 한 칸 풀이**. 창 또는 드래그 영역을 정지 확대하고 펜·형광펜·지우개·실행 취소로 설명한다. 마치면 원래 앱으로 돌아간다. **중** | **움직이는 실물화상기 확대**. 동영상·시연의 한 영역을 따라가는 라이브 렌즈. 작은 화면에는 렌즈 위치를 고정한다. **상** | **두 시점 비교**. 같은 영역의 실험 전·후를 나란히 두고 교사가 차이를 표시한다. **중** |
| 캡처·핀 | 창 윤곽에 맞춰 캡처하고 바로 복사·PNG 저장·화면에 붙이기 중 하나를 선택한다. 핀 크기·위치·클릭 통과를 바꾸고 트레이에서 다시 잡는다. **중** | 복수 후보 경계에서 문제·그림을 골라 두 핀을 같은 높이로 정렬한다. 서로 다른 풀이 비교에 쓴다. **상** | 교사가 지정한 앱 창의 한 영역만 갱신하는 라이브 핀. 창이 닫히면 마지막 장면에 ‘멈춤’을 표시한다. **상** |
| 답 공개·개인정보 | **‘답 덮기’**는 핀에서 한 번 눌러 공개한다. **‘가리고 복사’**는 불투명 가림을 픽셀에 합성해 내보낸다. 두 기능의 이름·아이콘·저장 동작을 다르게 한다. **중** | 표에서 ‘성명’ 열을 한 번 지정하면 같은 양식의 다음 캡처에도 가림 후보를 제안한다. 매번 미리본다. **상** | 다음 문항으로 넘어가도 ‘문제 → 힌트 → 답’ 순서를 유지하는 공개 카드. **중** |
| 노트·보관함 | 교사가 ‘보관함에 담기’를 누른 핀만 로컬에 보관한다. 수업 종료 시 임시 핀과 저장한 자료의 개수를 보여 준다. **중** | 날짜·수업명별로 핀과 판서를 한 장씩 보관한다. 수업용 편집본과 학생용 평면 이미지를 각각 내보낸다. **중** | ‘지난 시간에 사용한 자료’에서 판서 획 순서를 되짚어 수업을 재개한다. **중** |
| 교실 위젯 | 문제 핀 옆에 **무음 타이머**를 붙인다. 후면 좌석에서도 남은 시간을 숫자와 진행 막대로 볼 수 있다. **하** | 신호등에 ‘집중·상의·활동’ 문구를 병기한다. 번호 뽑기는 중복 제외·라운드 초기화를 제공한다. 마이크 소음계는 기기별 기준선을 잡아 상대 음량만 표시한다. **신호등·뽑기 하 / 소음계 중** | 타이머 종료 후 힌트 공개를 교사가 미리 연결하고, 수업 중에는 한 번 눌러 보류한다. **중** |
| QR·색·도구 모음 | 입력한 링크·짧은 텍스트를 기기 안에서 QR로 만들어 핀으로 띄운다. 원문·도메인을 함께 표시한다. **하** | 캡처 속 QR 읽기, 확대 픽셀 색상 선택·HEX 복사, 표를 탭으로 구분해 복사하기를 도구 모음에 둔다. **QR·색 하 / 표 상** | 수업마다 자주 쓰는 QR·타이머·핀 배치를 한 세트로 저장한다. 새로 열 때 이전 반의 이름·임시 화면은 불러오지 않는다. **중** |

**MVP 사용 예** — 교사가 교재 창에서 ‘영역 캡처’를 누른다 → 문제를 집어 ‘화면에 붙이기’를 누른다 → 답을 덮고 1분 타이머를 시작한다 → 학생 풀이를 적고 답을 공개한다. 수업 후에는 보관할 핀만 고른다. TV가 화면 복제 모드라면 교사와 학생이 같은 화면을 본다는 점을 표시한다. 민감한 업무 화면은 TV에 먼저 띄우지 않고, 교사용 확장 화면에서 가린 결과만 내보내는 별도 흐름으로 다룬다.

## 2. 스마트 로직과 실패했을 때의 동작

| 기능 · 우선순위/난이도 | 구체적인 구현 | 교사가 판단할 지점 |
|---|---|---|
| 창 → UI 요소 → 콘텐츠 경계 · P0 창 중 / P1 요소·내용 상 | Windows는 커서 아래 최상위 창과 DWM 경계, 이어서 UI Automation `ElementFromPoint`의 사각형을 사용한다. macOS는 허용된 `SCWindow`와 AX의 `AXUIElementCopyElementAtPosition`·위치·크기를 쓴다. 문서 캔버스·스캔본은 정지 프레임의 커서 주변에서 OpenCV 여백 투영·윤곽선으로 후보를 만든다. 동일 사각형 제거 후 커서를 포함하는 후보를 크기순으로 묶는다. | 기본 후보 윤곽을 먼저 표시하고 휠이나 방향키로 한 단계 넓히거나 좁힌다. 요소 트리가 비거나 응답이 늦으면 창·수동 드래그를 즉시 쓴다. URI·문서 본문 전체를 상시 읽을 이유가 없다. [Windows 요소 조회](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomation-elementfrompoint) · [Apple 요소 조회](https://developer.apple.com/documentation/applicationservices/1462077-axuielementcopyelementatposition) |
| 텍스트·URL·QR · P0 QR 생성 하 / P1 분류 중 | 선택 픽셀만 작업 큐에 넣는다. ZXing-C++로 QR을 읽고, 글자가 있을 때만 로컬 OCR을 수행한다. 인식 문자열을 표준 URL 파서에 넣어 링크와 일반 텍스트를 구분한다. macOS는 Vision 지원 언어를 확인하고 Windows MSIX는 설치된 OCR 언어를 조회한다. 포터블은 Tesseract `kor+eng` 모델 경로를 제공한다. | 글은 ‘텍스트 복사’, URL은 ‘QR로 띄우기’, QR은 ‘내용 보기’를 제안한다. 웹 링크 열기는 주소를 보여 준 뒤 사용자가 선택한다. 닫힌 캡처의 작업 결과는 폐기한다. [ZXing-C++](https://github.com/zxing-cpp/zxing-cpp) · [Apple Vision](https://developer.apple.com/documentation/vision/vnrecognizetextrequest) · [Windows OCR의 패키지 조건](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr) · [Tesseract 언어 모델](https://github.com/tesseract-ocr/tessdata_fast) |
| 색상·표 · P1 하/상 | 색 추출은 클릭 지점의 색을 sRGB로 변환해 HEX를 표시한다. 단색 견본은 주변 색 분산으로 판별한다. 표는 OpenCV 수평·수직선과 OCR 단어 좌표로 행·열을 복원한다. 선 없는 표는 글자의 정렬과 반복 간격을 쓴다. | 작은 캡처라고 색으로 단정하지 않는다. 표는 셀 격자 미리보기 후 TSV를 복사한다. 병합 셀·오인식은 고치거나 이미지로 복사한다. |
| 상황별 툴바 · P0 하 / P1 추천 중 | 선택·판서·핀·텍스트 입력을 명시적인 상태로 관리한다. 선택 직후에는 ‘복사·붙이기·가리기’를 고정한다. 내용 분석이 끝나면 정해 둔 마지막 칸에 추천 하나를 추가한다. | 손가락 아래 버튼 순서는 그대로 유지한다. 나머지 동작은 ‘더 보기’에 두며 키보드 포커스도 유지한다. |
| 손 떨림·도형 · P0 보정 중 / P1 도형 중 | 시간·위치가 찍힌 펜 샘플에 속도 적응형 One Euro Filter를 적용한다. 빠른 획은 지연을 줄이고 느린 획은 흔들림을 줄인다. 획 끝을 잠깐 눌렀을 때만 직선 오차·닫힘·꼭짓점을 평가해 도형 후보를 겹쳐 보인다. | 작은 한글 획의 끝점과 점을 보존한다. 보정 강도를 ‘끔·약하게·강하게’로 두고 도형 변환은 한 단계로 취소한다. |
| 판서 자동 정리 · P1 중 / P2 문자화 상 | 시각과 거리로 연속 획을 묶고 교사가 선택한 묶음만 이동·정렬·크기 맞춤한다. 2차에는 펜 좌표를 보존한다. 이후 손글씨 인식은 인식본을 옆에 제시하고 교사가 채택한다. | 수식·한글 자모가 섞인 문제에서 잘못 묶이면 풀 수 있다. 원래 획과 정리한 배치를 함께 저장한다. |
| 발표·영상 감지 · P1 중 | Windows의 전경 창 변경 알림과 창/모니터 경계, macOS의 `NSWorkspace` 활성 앱·화면 경계·허용된 AX 전체 화면 속성을 결합한다. PPT 슬라이드 쇼와 브라우저 전체 화면은 ‘발표 가능성’으로 표시한다. | ‘발표 도구 띄우기’를 한 번 제안한다. 창 제목 하나로 영상 재생을 단정하지 않는다. 교사가 화면을 지정하거나 ‘이 앱에서는 묻지 않기’를 선택한다. |
| 다중 모니터 · P0 상 / P1 송출 상 | 디스플레이 ID·논리 사각형·픽셀 배율·회전을 분리한다. Windows는 Per-Monitor V2와 디스플레이별 WGC 프레임, macOS는 `NSScreen`과 `SCDisplay` 정보를 연결한다. 핀은 해당 화면의 정규화 위치와 원본 픽셀 영역을 함께 저장한다. | MVP는 한 화면 안에서 선택하고 TV 분리 시 핀을 남은 화면으로 회수한다. 2차에는 확장 화면에 **승인한 핀만 그리는 학생용 창**을 띄운다. 화면 복제에서는 교사 전용 미리보기를 제공할 수 없다. |
| 개인정보 가림 · P0 수동 중 / P1 제안 상 | ‘가리고 복사’의 기본 출발점은 정지 프레임이다. 로컬 OCR 상자, 전화·이메일 등의 형식, ‘성명·학번·보호자’ 열 제목을 조합한다. 그림 속 얼굴·자유 문장 속 이름은 수동 가림으로 보완한다. 전용 모드에서는 원본을 메모리에만 두고 OCR 원문·썸네일·클립보드 기록·최근 항목에 보내지 않는다. | 교사가 누락을 추가한 뒤 ‘가린 이미지 복사’를 누른다. 마스크 합성본에서 클립보드·저장·핀·보관함 썸네일을 모두 만든다. 편집 가능한 마스크층과 숨겨진 OCR 텍스트를 내보내지 않는다. ‘답 덮기’는 공유 전에 공개 가능성을 명시한다. |

## 3. 단축키와 입력

| 동작 | Windows 기본안 | macOS 기본안 | 범위 · 우선순위/난이도 |
|---|---|---|---|
| 도구 열기(리더 키) | Ctrl+Shift+1 | Control+Shift+1 | 전역 · P0 중. 손을 뗀 뒤 화면에 명령 팔레트와 다음 키를 표시한다. |
| 영역 캡처 | Ctrl+Shift+2 | Control+Shift+2 | 전역 · P0 중. 팔레트의 2, 트레이·메뉴 막대에서도 연다. |
| 판서 시작·종료 | Ctrl+Shift+3 | Control+Shift+3 | 전역 · P0 중. 팔레트의 3도 같은 동작이다. |
| 정지 확대 / 타이머 / QR | 리더 후 4 / 5 / 6 | 리더 후 4 / 5 / 6 | 팔레트 안 · P0 하. 숫자와 한글 명령명을 함께 표시한다. |
| 핀 / 복사 / 취소 | 선택 후 Enter / Ctrl+C / Esc | 선택 후 Return / ⌘C / Esc | 선택 안 · P0 하. Esc는 현재 단계만 닫고 편집·판서에서만 가로챈다. |
| 펜·지우개 / 실행 취소 | 판서 중 1·2 / Ctrl+Z | 판서 중 1·2 / ⌘Z | 판서 안 · P0 하. 텍스트 입력·한글 조합 중에는 도구 전환 키를 해제한다. |

- **등록과 충돌 · P0 중.** 위 조합은 초기 제안이다. 시작·저장 때 OS 전역 등록을 시도해 실패하면 해당 동작 옆에 ‘다른 조합 선택’을 띄운다. 자체 중복은 즉시 표시한다. 이미 사용 중인 앱의 이름이나 다른 앱 내부 단축키까지 열거할 수 있다고 가정하지 않는다. 입력기 전환에 쓰이는 한/영·오른쪽 Alt·Ctrl+Space·Caps Lock, macOS 화면 캡처 조합, Windows 로고 조합·F12는 추천에서 제외한다. 기존 조합은 새 조합 등록 성공 후 해제한다. [Windows RegisterHotKey](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey) · [Tauri 전역 단축키](https://v2.tauri.app/plugin/global-shortcut/)
- **프로필 · P1 하.** 교사는 팔레트 첫 줄에 판서·답 덮기·타이머, 발표자는 확대·레이저·일시 숨기기, 개발자는 영역·색·텍스트 복사를 둔다. 프로필을 바꿀 때 이미 익힌 전역 키는 유지하고 팔레트 즐겨찾기만 바꾼다.
- **장치 매핑 · P1 중.** 펜 측면 버튼을 누르는 동안 지우고 떼면 펜으로 돌아온다. 마우스 뒤로 버튼은 활성 핀에서 실행 취소, 전자칠판 두 손가락은 활성 판서면에서 이동·확대에 쓴다. Windows는 `WM_POINTER`와 펜 이력·버튼, macOS는 AppKit 태블릿 이벤트와 실제 드라이버 입력을 확인한다. 마우스로만 인식되는 칠판에는 화면 버튼을 제공한다. [Windows 포인터 이벤트](https://learn.microsoft.com/en-us/windows/win32/inputmsg/wm-pointerupdate) · [Qt의 태블릿·터치 구분](https://doc.qt.io/qt-6/qtabletevent.html)
- **설정 화면 · P0 중.** ‘동작 | 현재 키 | 전역/도구 안 | 바꾸기’ 한 표에서 키를 직접 눌러 등록한다. ‘지금 시험’은 눌렀을 때 실행될 동작을 보여 주고 끝난다. Esc·시간 초과·포커스 이탈 시 리더 모드를 닫아 원래 앱으로 돌려준다. 첫 실행에서 전역 키가 전부 실패해도 트레이·메뉴 막대로 수업을 진행할 수 있다.

## 4. 한국 교실에서 달라지는 설계

| 장면 | 제품 동작과 확인할 사례 | 우선순위/난이도 |
|---|---|---|
| 전자칠판에서 교사가 몸을 돌려 필기 | 도구막대를 좌우·하단에 붙이고 터치 표적은 초기 48 논리 픽셀로 잡는다. 펜이 접근한 기기에서는 손바닥을 무시하도록 입력 종류를 이용한다. 펜·손가락이 모두 마우스로 들어오면 ‘손가락으로도 쓰기’를 명시적으로 선택한다. | P0 중 |
| 교실 TV와 교사 노트북의 배율이 다름 | TV에서 실제 크기로 10초 미리보기. QR은 흰 여백과 대비를 유지하고 뒤쪽 자리에서 학생 휴대전화로 읽히는지 확인한다. 신호등·타이머는 색·글자·모양을 같이 사용한다. | P0 중 |
| 관리자 권한이 없는 PC, 오프라인 수업 | 사용자 폴더에서 실행하는 ZIP에 EXE·필수 라이브러리·언어 자료를 함께 넣는다. WebView2 설치 여부를 먼저 확인하고 차단된 환경에는 기관 승인 오프라인 런타임 포함판을 제공할지 용량을 보고 결정한다. USB·읽기 전용 폴더에서는 저장 가능한 사용자 폴더를 선택하게 한다. | P0 상 |
| 교과서 한글·영문·수식이 한 줄에 섞임 | 인쇄체 한글과 영문 인식을 함께 제공한다. 좁은 표, 받침, 흐린 스캔, 소수점·분수·단위의 실제 오독을 별도 집계한다. OCR 실패 시 원본 영역을 나란히 보여 주며 수식은 이미지로 유지할 수 있다. | P1 중 |
| 한글 글꼴과 입력기 | 기본 UI는 맑은 고딕·Apple SD Gothic Neo 등 시스템 글꼴로 표시한다. 판서 제목에만 선택형 나눔손글씨 펜을 두고 배포본에 해당 글꼴 라이선스를 동봉한다. 본문은 손글씨 서체 대신 읽기 좋은 고딕을 기본으로 둔다. 한글 조합 확정 Enter가 핀 생성으로 이어지지 않게 분리한다. [나눔손글씨 펜 배포 원본](https://github.com/google/fonts/tree/main/ofl/nanumpenscript) | P0 기본 하 / P1 선택 글꼴 하 |
| 나이스·K-에듀파인 자료를 설명 | 사용자 지정 ‘업무 자료’ 모드에서만 선택 캡처를 처리한다. 합성 학생 명부·결재 문서로 열 위치 변경, 스크롤, 동명이인, 서명 이미지 누락을 확인한다. 기관 프로그램이 캡처를 막으면 허용된 예시 이미지나 빈 칠판을 쓴다. 특정 학교 프로그램과의 호환은 미확인이다. | P0 수동 중 / P1 제안 상 |
| 한국어·영어 표시 | 기본 문구는 ‘화면에 붙이기 / Pin to screen’, ‘뒤 화면 누르기 / Click through’, ‘가린 이미지 복사 / Copy redacted image’. 명령은 짧은 동사, 상태는 ‘판서 중’, 권한 오류는 ‘화면 기록을 허용하면 영역을 캡처할 수 있어요’처럼 다음 행동을 담는다. | P0 하 |

## 5. 플랫폼 선택

**Tauri 2 + React·TypeScript 설정 화면 + 운영체제별 네이티브 캡처·필기 경로**를 권한다. 사용자가 익숙한 웹 기술은 설정·팔레트·보관함·번역에 쓴다. Rust는 핀·도구 상태와 저장 형식을 공유한다. Windows는 WGC → D3D11 텍스처 → DirectComposition/Direct2D, macOS는 ScreenCaptureKit → 픽셀 버퍼 → Metal/AppKit 창으로 연결한다. 프레임·펜 샘플은 네이티브 경로에 두고 웹 UI에는 도구 변경·선택 결과·작은 미리보기만 전달한다. 외부 서버나 상시 실행할 Next.js 서버는 필요하지 않다. [Tauri 웹뷰](https://v2.tauri.app/reference/webview-versions/) · [Windows 캡처](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture) · [ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit)

아래 성능·크기·개발 용이성은 **같은 기능을 각 구조로 구현했을 때의 [추정]**이다. 바이너리 크기는 설치 배포물 기준이며 OS 내장 런타임·OCR 모델 동봉 여부에 따라 달라진다. 메모리 비교는 동일한 픽셀 수와 핀 수로 해야 한다.

| 기준 | Tauri 2 + 네이티브 | Electron | Qt 6/C++ | .NET Avalonia | C#/WinUI + Swift/AppKit |
|---|---|---|---|---|---|
| 투명 오버레이 필기 지연 | 네이티브 창에서 펜 즉시 합성. 웹뷰 왕복 제거 | 캔버스 시제품은 빠름. JS·합성 지연이 크면 애드온 | Qt Quick 장면 그래프에 획 추가 | 사용자 그리기·Skia 경로 측정 | OS별 입력·합성 경로를 직접 제어 |
| 라이브 확대 | WGC/D3D·SCK/Metal 직접 연결 | 데스크톱 캡처 스트림 가능. 고성능 경로는 네이티브 추가 | 캡처 텍스처와 Qt 렌더러 연결 필요 | OS별 텍스처 연동 비용 큼 | 플랫폼별 GPU 텍스처 경로 설계에 유리 |
| 여러 핀의 메모리 | 네이티브 핀·텍스처 재사용 | 핀마다 BrowserWindow면 렌더러 비용 증가. 한 표면에 묶기 가능 | 한 장면 안에서 이미지 공유 가능 | 비트맵 수명·창별 표면 관리 필요 | 작은 창·공유 텍스처 수명 직접 제어 |
| 전역 단축키 | 공식 플러그인 | 내장 globalShortcut | OS 등록 또는 외부 라이브러리 | OS 상호운용 필요 | 각 OS 등록 API 직접 호출 |
| WGC·ScreenCaptureKit | Rust/WinRT와 Swift·ObjC 브리지 | 내장 고수준 API, 세부 제어 시 애드온 | 플랫폼 플러그인 | P/Invoke·플랫폼 바인딩 | 양쪽 직접 접근 |
| UIA·AX 영역 인식 | 네이티브 모듈 | 네이티브 모듈 | 플랫폼 코드 | 상호운용 모듈 | 양쪽 직접 접근 |
| 펜 압력 | WM_POINTER·AppKit | Pointer Events, 누락 장치는 네이티브 | QTabletEvent 제공 | PointerPointProperties.Pressure 활용·장치 확인 | 장치 이벤트·이력 직접 접근 |
| 기본 배포물 크기 | 시스템 웹뷰 사용 시 작음. 고정 런타임 포함 시 증가 | Chromium·Node 포함으로 큰 편 | Qt 라이브러리·플러그인 부담 | 자체 포함 .NET 배포 시 증가 | OS 프레임워크 의존도와 WinUI 런타임 포함 여부에 따라 차이 |
| TS 경험자·AI 협업 | UI는 높음. Rust·Swift 경계는 실기기 필요 | UI는 높음. 네이티브 애드온 포장 부담 | C++·QML 숙련 필요 | C#·XAML 경험이 있으면 유리 | 플랫폼별 UI와 회귀 검증을 각각 유지 |
| MSIX / DMG 제작 난이도 | 중 / 중. MSIX 추가 공정 | 중 / 중. 패키저·서명 작업 | 중 / 중. 플러그인·라이선스 점검 | 중 / 중. 런타임·앱 번들 점검 | 하~중 / 중. 표준 플랫폼 도구, 두 배포 체인 |

근거가 되는 프레임워크 문서. [Electron 프로세스 모델](https://www.electronjs.org/docs/latest/tutorial/process-model) · [Electron 단축키](https://www.electronjs.org/docs/latest/api/global-shortcut) · [Qt 장면 그래프](https://doc.qt.io/qt-6/qtquick-visualcanvas-scenegraph.html) · [Qt 태블릿 입력](https://doc.qt.io/qt-6/qtabletevent.html) · [Avalonia 포인터 속성](https://docs.avaloniaui.net/api/avalonia/input/pointerpointproperties)

**결정 순서 · P0 상.** 먼저 양쪽에서 ‘투명창 필기·클릭 통과 복구 → 정지 확대 → 핀 10개 → 창 닫기’를 시제품으로 측정한다. 네이티브 렌더링이 목표에 못 미치면 캡처·합성 경로부터 고친다. 학교 PC에서 WebView2가 설치·동봉 모두 불가능하고 이 환경이 필수 대상이면 Windows UI를 네이티브로 전환하는 대안을 택한다. 반대로 설치 크기보다 일정이 중요해지고 실측 지연이 허용 범위면 Electron 단일 렌더러 시제품도 선택지다. 학생용 내보내기 규칙과 노트 형식은 UI 선택과 독립시킨다.

## 6. 배포 경로와 비용

| 경로 | 교사에게 전달하는 것 | 개발자 비용·주의점 | 우선순위/난이도 |
|---|---|---|---|
| Windows 무료 공개판 | Microsoft Store의 MSIX. 사용자는 Store에서 설치·갱신 | 신규 개인은 공식 Store 개발자 시작 페이지의 무료 등록 흐름과 본인 확인을 거친다. Store 인증 후 Microsoft가 MSIX를 재서명해 별도 상용 서명 인증서 구매 비용이 없다. 기존 Partner Center 가입 화면에서 시작하면 구 등록 경로가 보일 수 있다. 학교에서 Store를 차단하는지는 별도 확인한다. [개인 무료 등록](https://learn.microsoft.com/en-us/windows/apps/publish/whats-new-individual-developer) · [Windows 서명 방식](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) | P0 포장 검토 중 / P1 출시 중 |
| Tauri의 MSIX 제작 | Windows 빌드 출력과 의존 파일, 패키지 manifest, 자산을 Windows SDK MakeAppx 등으로 묶는다. Store 예약 ID·게시자 일치와 패키징한 앱의 캡처·단축키를 확인한다. | Tauri 공식 Store 안내는 EXE/MSI 업로드 경로다. 이 방식은 개발자가 설치 파일에 Authenticode 서명해야 한다. 무료 MSIX 경로에는 별도 포장 작업을 배정한다. [Tauri 배포 안내](https://v2.tauri.app/distribute/microsoft-store/) · [Microsoft 수동 패키징](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion) | P0 중 |
| Windows 포터블 | ZIP을 풀어 EXE 실행. Store가 닫힌 교실의 두 번째 경로 | 미서명 시험판 배포 비용은 없지만 학교 정책에 따라 실행이 차단될 수 있다. 공개판 Authenticode 인증서는 공급자별 유료이며 서명 직후에도 SmartScreen 평판 경고가 가능하다. 학교 IT의 승인된 배포 경로도 선택지다. WebView2 없는 PC에 고정판을 동봉하면 패치 공급과 용량은 개발자 부담이다. [SmartScreen 평판](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation) · [WebView2 배포](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution) | P0 상 |
| macOS 공개판 | Developer ID Application으로 서명하고 Hardened Runtime 적용 → DMG 공증 제출 → 티켓 스테이플 → 새 Mac의 Gatekeeper로 확인 | Apple Developer Program은 일반적으로 연 99 USD 또는 현지 통화 상당액이다. 공증 제출은 멤버십에 포함된다. 적격 교육기관·비영리 법인은 면제를 신청할 수 있고 개인 교사 자격만으로 면제되지는 않는다. 무료 로컬 개발은 가능하다. 내려받은 미서명·미공증 앱은 Gatekeeper가 경고하거나 실행을 차단할 수 있다. [Apple Developer ID](https://developer.apple.com/developer-id/) · [공증](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) · [멤버십·면제 조건](https://developer.apple.com/help/account/membership/fee-waivers/) · [Gatekeeper 안내](https://support.apple.com/en-us/102445) | P0 상 |
| 업데이트·운영 | Store판은 Store 업데이트. DMG 직접 배포판은 서명된 Tauri 업데이트, 포터블은 새 ZIP 수동 교체를 기본으로 둔다. 수업 중 설치는 다음 종료 시점으로 미룬다. | Tauri 업데이트 서명키와 OS 코드 서명은 목적이 다르다. 직접 배포는 HTTPS 호스팅·대역폭·macOS 빌드 장비/CI 비용이 생길 수 있다. 기존 Mac과 무료 호스팅 한도 내에서 시작할 수 있고, QR·OCR·보관함은 로컬 처리로 서버 사용료를 줄인다. [Tauri 업데이트 서명](https://v2.tauri.app/plugin/updater/) | P0 수동 중 / P1 자동 중 |

## 7. 출시 전 판정할 위험과 목표

| 우선순위/난이도 | 위험 | 시제품에서 판정할 조건 |
|---|---|---|
| P0 상 | macOS 화면 기록·손쉬운 사용 권한, 전체 화면 Spaces | 캡처가 필요할 때 화면 기록을 요청하고, UI 요소 맞춤을 켤 때만 손쉬운 사용을 안내한다. 거절 시 각각 빈 칠판·QR, 또는 수동 드래그로 돌아간다. 권한 변경·앱 재시작 후 기존 핀 복구와 전체 화면 슬라이드 위 필기 여부를 본다. 마이크는 소음계를 켤 때만 요청하고 음성은 저장하지 않는다. [화면 캡처](https://developer.apple.com/documentation/screencapturekit) · [손쉬운 사용 신뢰 확인](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions) |
| P0 상 | HiDPI·회전·음수 원점·모니터 분리 | Windows 100%/150% 혼합과 Retina+일반 TV에서 화면 구석·경계의 같은 표식을 선택한다. 논리 좌표↔픽셀 왕복, 핫플러그와 좌우 배치 변경을 확인한다. [Apple 캡처 필터의 좌표·배율](https://developer.apple.com/documentation/screencapturekit/sccontentfilter) |
| P0 상 | 자기 창 무한 캡처·보호 영상·검은 프레임 | macOS는 캡처 필터에서 자체 창을 제외하고 Windows는 지원 버전의 자체 최상위 창에 WDA_EXCLUDEFROMCAPTURE 적용을 확인한다. 실패하면 정지 캡처 시 도구를 잠시 감추고, 라이브 렌즈는 중단한다. 검은 화면만으로 DRM을 단정할 수 없어 ‘이 화면을 가져올 수 없어요’와 이미지 열기를 제공한다. [SCContentFilter](https://developer.apple.com/documentation/screencapturekit/sccontentfilter) · [Windows 창 캡처 제외의 적용 범위](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity) |
| P0 상 | 학교 보안 프로그램·권한·오프라인 환경 | 표준 사용자로 시작해 펜·캡처·핀·종료까지 확인한다. 포터블 실행 허용 여부, Store 접근, WebView2 존재를 별도로 기록한다. 차단 시 학교 IT에 전달할 진단에는 앱 버전·기능명·OS 오류 코드만 담는다. |
| P0 상 | 학생 정보 재노출 | 가짜 학생 명부를 수동으로 가리고 외부 이미지 편집기에 붙여 넣어 마스크 아래 픽셀·숨은 텍스트가 없는지 확인한다. 클립보드 형식·썸네일·재실행 뒤 최근 항목도 같은 합성본을 사용하는지 본다. 인식 신뢰도는 교사 확인을 대신하지 않는다. |
| P0 상 / P1 라이브 상 | 필기 지연·핀 메모리·4K 확대 | **초기 합격 목표 [추정]** 60Hz 기준 실펜 접촉→가시 잉크 p95 50ms 이내(고속 촬영), 1080p 라이브 확대 p95 프레임 간격 33ms 이하. 핀 10개 전후 프로세스 합산 메모리와 GPU 사용량을 따로 측정한다. 4K RGBA 한 장은 약 31.6MiB라 원본·편집 복사·GPU 복제가 곱해진다. 핀 해제 후 반복 주기마다 메모리가 계속 늘면 출시를 보류한다. |

**PRD로 넘길 판단 기준 [추정]** — 교사 5명에게 새로운 문제 한 칸을 처음 보여 주고 도움 없이 ‘문제 붙이기 → 풀이 → 답 공개’를 시킨다. 4명 이상이 30초 안에 마치고, 업무 화면 예시에서 가린 정보가 모든 내보내기 경로에서 제거되면 다음 단계로 간다. 작업 시간이 길면 클릭이 가장 많이 발생한 단계부터 줄인다. 이 문서는 설계 제안이며 학교 장비 호환성·성능·사용성 목표는 실측 전이다.
