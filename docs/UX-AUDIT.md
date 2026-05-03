# CodeWarp UX 감사 — 2026-05

방법: 코드 정찰(view 함수 전체) + 5개 핵심 흐름의 friction 분석 + 외부 reference(Warp / Cursor / Claude.ai / Linear / Raycast) 패턴 매핑.

> **한계**: 실제 화면 캡처 없이 코드만으로 분석. 시각 미감 / 폰트 렌더링 / 한국어 IME 동작 등은 직접 사용 후 보강 필요.

---

## 1. 흐름별 friction

### 흐름 ① 첫 실행 onboarding

| 단계 | 현 동작 | Friction |
|---|---|---|
| 앱 실행 | `has_key=false`면 Settings 자동 모달 | OK |
| OpenRouter 키 발급 | 텍스트로 URL 한 줄 안내 | **F1**: 가입/충전/모델 권장 절차 안내 0. 신규는 어디부터 시작할지 모름 |
| 첫 모델 선택 | 모델 페치 후 첫 항목 자동 선택 | **F2**: 어떤 모델 골라야 하는지 가이드 없음. `gpt-4o`/`claude-sonnet` 같은 권장 prefilter 없음 |
| Tabby 설정 (선택) | URL/token 입력 UI만 | **F3**: Tabby가 뭔지/어떻게 띄우는지 안내 0. `tabby serve --model X` 같은 예시 명령 없음 |
| 빈 채팅 화면 | "$ CodeWarp ready — 입력 후 Enter" | **F4**: 슬래시 명령, Plan/Build, 도구 사용법 안내 없음. 신규는 그냥 메시지만 보냄 |
| 충전 0 상태 | 메시지 전송 → 401/402 에러 | **F5**: 잔액은 status bar에 작게만 표시. 0 상태 경고 없음 |

### 흐름 ② 모델 선택

| 단계 | 현 동작 | Friction |
|---|---|---|
| 콤보박스 클릭 | 검색창 + 리스트 | OK |
| 태그 표시 | `[OR][KO] gpt-4o  $5.00/$15.00` | **F6**: prefix 4글자 + space + id + price가 한 줄에 우겨짐. 색상 위계 없음 (Display는 String 한정). 가독성 약함 |
| 필터 4종 | 코딩/추론/범용/⭐만 항상 노출 | **F7**: top bar 가로 공간 점유. 자주 안 쓰는 사용자에게 노이즈 |
| 정렬 | `sort_btn` 텍스트 | **F8**: 라벨이 모드만 표시 (e.g. `↑`). "정렬 기준" 명시 부족 |
| 즐겨찾기 | 별 버튼 1개 (선택 모델만) | **F9**: 리스트 안에서 즐겨찾기 모델 시각 구별 없음 (`★` prefix 등) |
| 컨텍스트 길이 | OpenRouterModel에 있지만 미표시 | **F10**: 긴 문서 다룰 때 필요한 정보 (`128k` 같은 짧은 라벨) 누락 |

### 흐름 ③ 메시지 전송 → 응답

| 단계 | 현 동작 | Friction |
|---|---|---|
| Enter / Send | conversation push + chat_stream 시작 | OK |
| **streaming 중** | 토큰 도착할 때마다 `markdown::parse` 재실행 | **F11**: **응답 중지 버튼 없음**. 무한 루프, 잘못 보낸 메시지, 긴 응답 끊을 방법 0. 큰 미싱 기능 |
| 모델 정보 | role label만 (`User` / `Assistant`) | **F12**: 응답한 모델 ID, timestamp, 비용 미표시 — 도구 라운드/모델 전환 시 추적 불가 |
| 에러 시각화 | error string이 status bar 한 줄 | **F13**: assistant 블록에 별도 시각 표시 없음. 빨간 border 같은 신호 부재 |
| 응답 중 스타일 | block 배경색만 | **F14**: spinner/pulsing dot 없음. 진행 중 시각 신호 약함 |
| 코드 블록 복사 | 전체 메시지 "복사" 버튼만 | **F15**: 응답 안 코드 블록별 복사/Apply 버튼 없음 (Cursor 패턴 부재) |
| 토큰별 markdown 재파싱 | `markdown::parse(&raw).collect()` 매 토큰 | **F16**: 긴 응답에서 누적 비용 큼. 실측 필요 |

### 흐름 ④ 도구 승인 (write_file / run_command)

| 단계 | 현 동작 | Friction |
|---|---|---|
| AI 도구 요청 | `view_inline_confirm`이 입력창 위 패널 | OK (모달보다 인라인이 자연스러움) |
| 카드 표시 | 아이콘 + path + bytes 또는 `$ command` | **F17**: write_file의 diff 인라인 미표시. (`view_write_confirm` 큰 모달은 dead_code) |
| 승인 단위 | "거부" / "✓ 모두 승인" | **F18**: 5개 중 1개만 거부 불가. 개별 승인 없음 |
| 실행 결과 | `tool_result` conversation에만 기록 | **F19**: run_command stdout/stderr 사용자가 어디서 보는지 불분명. assistant 다음 메시지 안에 텍스트로 묻혀 들어감 |
| 카드 5개+ | `max_height: 140px` 후 스크롤 | **F20**: 한눈에 안 보임 |

### 흐름 ⑤ Tabby 연결

| 단계 | 현 동작 | Friction |
|---|---|---|
| Settings 진입 | OpenRouter 섹션 + Tabby 섹션 평면 배치 | **F21**: 위계 없음. "필수 / 선택" 구분 불명. 두 provider가 동등하게 보여 혼란 |
| URL 저장 | 자동 fetch + 모델 셀렉터 추가 | OK |
| 연결 실패 | "Tabby 연결 실패: {error}" | **F22**: actionable 메시지 부족. "포트 8080에 응답 없음 — Tabby 서버가 실행 중인가요?" 같은 안내 없음 |
| 모델 받기 | 사용자가 별도로 `tabby download` 또는 `huggingface-cli` | **F23**: 모델 매니저 부재 (Phase 2-9 계획됨) |
| OR 잔액 0 → Tabby fallback | 사용자가 수동 모델 전환 | **F24**: 자동 fallback 없음 (선택적 기능) |

---

## 2. 글로벌 시각/구조 친화

### 위계 / 정보 밀도

- **G1. Right panel 미활용** — 280px 차지하면서 "Plan / Diff / History" placeholder만. 빈 공간이 가장 큰 시각 손실
- **G2. Sidebar 고정 220px** — 리사이즈 불가. 작은 화면 답답, 큰 화면 비효율
- **G3. Sidebar placeholder 다수** — "프로젝트: CodeWarp", "컨텍스트: 선택 안 됨" — 동작 없는 라벨이 화면 차지
- **G4. Top bar 정보 밀도 높음** — 필터 4 + 정렬 + 모델 picker(420px) + ⭐ + ⚙. 작은 화면에서 우측 잘림 가능

### 시각 강조

- **G5. CTA 약함** — Send 버튼이 일반 button과 동일. primary 색상 적용 안 됨
- **G6. 색상 위계 단조** — user (옅은 보라) / assistant (살짝 밝은 다크) 대비 약함
- **G7. Scrollbar 6px** — 너무 얇아 hit target 작음 (8~10px 권장)
- **G8. 빈 상태 illustration 없음** — 빈 채팅, 빈 사용량, 빈 right panel 모두 텍스트 한 줄

### 마이크로카피

- **G9. Send 버튼 단축키 안내 없음** — placeholder에 "Enter로 전송" 같은 hint 없음
- **G10. 명령 팔레트 hint 한 줄에 빽빽** — 단축키 5개가 한 줄. 스캔 어려움
- **G11. 세션 삭제 확인 없음** — `✕` 즉시 삭제. 실수 보호 없음
- **G12. 세션 자동 제목 없음** — 첫 user message 50자로 자동 명명하면 (빈 세션) placeholder 제거 가능

---

## 3. 외부 reference 패턴 매핑

| Reference | 핵심 패턴 | CodeWarp 현재 | 갭 |
|---|---|---|---|
| **Warp** | 블록 단위 (명령+출력 묶음), Workflows, AI Command Search | 블록 단위 ✅ | Workflows 부재. 블록 미감 차이 |
| **Cursor** | Composer (멀티 파일), Apply 버튼, @-mention 컨텍스트, Cmd+K inline edit | 도구 호출 루프 ✅ | Apply 부재 (큼). @-mention 부재 |
| **Claude.ai** | Projects, Artifacts (사이드바 분리), Conversation history | 멀티 세션 ✅ | Artifacts 부재 (right panel 미활용 자리) |
| **Linear** | 정밀한 spacing, 색상 위계(상태), 키보드 우선 | 키보드 단축키 ✅ | 색상 위계 약함 |
| **Raycast** | 팔레트 중심 진입, 즉시 미리보기 | 명령 팔레트 ✅ | 팔레트가 부차적 (top bar 동등) |

**CodeWarp 강점**: 멀티 provider (OpenRouter+Tabby), 한국어 친화 [KO] 태그 — 다른 곳에 거의 없음.
**약점**: 응답 중 인터랙션(stop/regenerate), 코드 적용(Apply), right panel 활용, onboarding.

---

## 4. 개선 전략 (우선순위)

### P0 — 사용성 차단 / 가시 효과 큼

- **P0-1. 응답 중지 버튼** (F11) — Send 버튼이 streaming 중엔 "Stop"으로 변경. `streaming_block_id` 활용해 chat_stream Task 취소
- **P0-2. 빈 채팅 onboarding** (F4) — 빈 상태 화면에 ① 예시 프롬프트 3개 ② 슬래시 명령 (`/plan`, `/build`) 안내 ③ Ctrl+K 팔레트 hint
- **P0-3. 세션 자동 제목** (G12) — 첫 user message 첫 30자로 자동 명명, 사용자가 수동 변경도 가능
- **P0-4. 인라인 confirm에 diff 인라인 토글** (F17) — 카드 클릭 시 펼쳐서 diff 표시 (`render_diff` 이미 존재)
- **P0-5. 세션 삭제 인라인 확인** (G11) — `✕` 클릭 시 `✓/✗` 두 버튼으로 변경

### P1 — 정밀화 / friction 줄임

- **P1-1. Tabby 연결 실패 actionable 메시지** (F22) — error 카테고리별 (Conn refused / 4xx / parse 실패) 다른 hint
- **P1-2. context_length 모델 라벨에 표시** (F10) — Display에 `128k` 추가
- **P1-3. 즐겨찾기 prefix `★`** (F9) — Display에 favorites HashSet 참조
- **P1-4. Sort 라벨 명시** (F8) — `↑` → `가격↑` / `가격↓` / `기본`
- **P1-5. assistant 블록 헤더에 모델 ID** (F12) — 메시지 전송 시 사용한 model을 Block에 저장
- **P1-6. 도구 실행 결과 시각 블록** (F19) — tool_result도 별도 블록으로 표시, 결과 collapse 가능
- **P1-7. 충전 0 / 401/402 시 경고 토스트** (F5) — status bar 강조 + Settings 열기 안내

### P2 — 구조 개선

- **P2-1. Right panel 활용** (G1) — "최근 도구 호출 로그" 또는 "현재 turn 진행 상태" 표시
- **P2-2. Sidebar resize** (G2) — Iced에서 drag handle 직접 구현 (PaneGrid 도입 검토)
- **P2-3. Settings 위계** (F21) — "AI Provider" 헤더 → "OpenRouter (필수 또는)" + "Tabby (필수 또는)" + 둘 중 하나 이상 필수 안내
- **P2-4. Welcome 화면** (F1, F3) — 첫 실행 시 Settings 모달 대신 provider 선택 카드 2개 (OpenRouter / Tabby) + 각 카드에 가입/설치 단계
- **P2-5. 코드 블록별 복사** (F15) — markdown viewer 확장. 코드 블록 hover에 복사 버튼

### P3 — 시각 / 미감

- **P3-1. CTA 강조** (G5) — Send 버튼에 primary 배경색 + 흰 텍스트
- **P3-2. Scrollbar 8~10px** (G7)
- **P3-3. user/assistant 색상 대비 강화** (G6) — user를 더 진한 보라, assistant를 더 밝은 톤
- **P3-4. 응답 중 spinner** (F14) — status bar 좌측에 ▶ 같은 pulsing dot
- **P3-5. 명령 팔레트 hint 카테고리화** (G10) — "기본 / 모드 / 모델" 등 그룹별 한 줄씩

### P4 — 큰 신규 기능 (별도 일정)

- **P4-1. 코드 블록 Apply** (Cursor 영감) — 파일에 직접 패치 적용
- **P4-2. @-mention 파일/심볼** (Cursor 영감) — 입력창에서 `@filename`으로 컨텍스트 추가
- **P4-3. 응답 regenerate / edit-and-resend**
- **P4-4. 모델 매니저** (Phase 2-9 계획)
- **P4-5. Drag & drop 파일 → 컨텍스트**
- **P4-6. PTY 터미널** (Phase 2-10 계획)

---

## 5. 권장 실행 순서

다음 한 PR 단위로 묶어 진행 권장:

1. **PR-A "응답 인터랙션"** — P0-1 (Stop) + P0-3 (자동 제목) + P0-5 (세션 삭제 확인) + P1-5 (블록 헤더 모델) — 1~2일
2. **PR-B "Onboarding"** — P0-2 (빈 화면) + P2-4 (Welcome) + F1/F3 안내 — 1일
3. **PR-C "Confirm UX"** — P0-4 (인라인 diff) + F18 (개별 승인) + P1-6 (도구 결과 블록) — 1~2일
4. **PR-D "모델 픽커 정밀화"** — P1-1~P1-4 (라벨/태그/정렬) + P3-1~P3-5 (시각 강조) — 1일
5. **PR-E "Right panel 활용"** — P2-1 (도구 호출 로그) — 0.5일

P4는 Phase 2-9/2-10과 함께 별도 일정.

---

## 6. 다음 단계 결정 필요

PM 측에서:
- A. 위 우선순위 그대로 갈지, 재배치 (예: PR-D를 먼저)
- B. PR 단위로 진행할지, 단일 작은 step씩 갈지
- C. 시각 미감 (P3) 결정 — 직접 화면 보면서 조정해야 정확함. 제가 추측만 가능
