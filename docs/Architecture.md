# CodeWarp Architecture

CodeWarp는 Iced 프레임워크 기반의 Rust 네이티브 AI 코딩 데스크톱 앱입니다. Tauri/WebView 의존성 없이 단일 Cargo 프로젝트로 빌드됩니다.

## Core Architecture: Iced Elm Pattern

CodeWarp는 Iced의 Elm 아키텍처를 따릅니다:

```
State (App)  ←→  Message  ←→  update()  ←→  Task  ←→  view()
```

- **State**: `App` 구조체가 모든 애플리케이션 상태를 보유
- **Message**: `Message` enum이 모든 가능한 이벤트를 정의
- **update**: `App::update(message) -> Task<Message>`가 상태 전이를 처리
- **view**: `App::view()`가 현재 상태 기반 UI를 렌더링
- **Task**: 비동기 작업의 반환 타입 (`Task::none()`, `Task::perform()`, `Task::done()`, `Task::run()`)

## File Structure

```
src/
├── main.rs             # App struct, Message enum, module declarations, tests
├── bootstrap.rs        # window icon + embedded font setup
├── input.rs            # keyboard/window event routing
├── runtime_process/
│   └── mod.rs          # inference child process spawn/log/error helpers
├── update.rs           # App::update() dispatcher + async task wiring
├── view/
│   ├── mod.rs          # App::view() UI rendering shell
│   └── ui/mod.rs       # UI constants, spacing, style helpers
├── block/mod.rs        # chat block model, apply candidates, conversation helpers
├── model/mod.rs        # providers, model options, filters, inference engines, presets
├── util/mod.rs         # path/fuzzy/format/summarize helpers
├── palette.rs          # command palette state/items
├── session/mod.rs      # sessions, usage, favorites persistence
├── openrouter/mod.rs   # OpenRouter HTTP/SSE client, model listing, chat stream
├── tabby/mod.rs        # OpenAI-compatible endpoint client and chat stream
├── hf/mod.rs           # Hugging Face download stream, revision fallback handling
├── tools/mod.rs        # tool calls (read/write/glob/grep/run_command)
├── mcp/mod.rs          # stdio MCP client and tool definition aggregation
├── pty.rs              # PTY terminal (portable-pty based)
└── keystore/mod.rs     # OS credential manager persistence
```

## App State Composition

`App` 구조체는 `src/main.rs`에 정의되어 있으며, 상태 그룹으로 분리 가능:

- **Core UI**: `ui: UiState` (설정 패널, 명령 팔레트, 삭제 확인 등)
- **Model Filter**: `model_filter: ModelFilterState` (코딩/추론/일반 필터, 정렬 모드, 즐겨찾기)
- **MCP Input**: `mcp_input: McpInputState` (MCP 서버 이름/명령 입력)
- **Provider State**: `tabby_url_input`, `tabby_token_input`, `openai_compat_label`, `hf_token_input`
- **Inference State**: `inference_engine`, `inference_selected_model`, `inference_port_input`, `inference_binary_path`, `inference_generation`
- **Chat State**: `conversation`, `blocks`, `pending_tool_calls`, `pending_write_calls`, `streaming_block_id`, `stream_generation`, `compare_generation`, `generation_lookup_generation`
- **MCP Lifecycle State**: `mcp_abort_handle`, `mcp_request_generation`, `mcp_pending_results`, `mcp_pending_call_ids`, `mcp_tool_load_generations`
- **PTY Lifecycle State**: `pty_session`, `pty_generation`
- **Session State**: `current_session_id`, `current_session_title`, `inactive_sessions`
- **Model State**: `model_options`, `selected_model`, `selected_model_provider`, `usage`

## Async Runtime Model

CodeWarp는 Tokio 런타임을 사용하며, Iced의 `Task` 추상화와 통합됩니다:

- `Task::none()` — 상태만 변경, 비동기 작업 없음
- `Task::done(Message)` — 즉시 다른 Message를 디스패치
- `Task::perform(async_fn, msg_fn)` — 비동기 함수 실행 후 결과를 Message로 변환
- `Task::run(stream, msg_fn)` — 스트림에서 이벤트를 수신하여 Message로 변환

채팅 스트림은 `Task::run`으로 SSE 이벤트를 수신하고, 소유 assistant block ID와
stream generation을 포함한 `Message::ChatChunk`로 토큰을 전달합니다. retry가 같은
assistant block을 재사용하더라도 generation이 달라지므로, 중지·세션 전환·retry 뒤 늦게
도착한 이전 스트림 이벤트는 block ID 또는 generation이 현재 상태와 다르면 폐기합니다.

MCP tool call도 요청 generation을 함께 전달하며, 한 라운드의 모든 MCP 결과를 받은 뒤에만
다음 chat stream을 시작합니다. 중지·새 세션·창 닫기에서는 generation을 무효화하고
abort handle을 취소해 늦은 결과가 새 대화를 재개하지 못하게 합니다. 각 라운드의 tool-call ID는
비어 있거나 중복된 provider 값을 로컬 고유 ID로 정규화하고, 등록된 pending ID와 일치하는
결과만 소비합니다. 서버의 tools/list 응답도 서버별 generation과 존재 여부를 확인합니다.

OpenRouter 계정·사용량 조회도 각각 request generation을 포함합니다. API 키 변경, 새 메시지,
regenerate, compare, 중지 또는 세션 전환 뒤 도착한 이전 응답은 현재 상태를 덮어쓰지 않습니다.

Managed inference runtime 이벤트도 프로세스 generation을 포함합니다. 재시작 뒤 이전
프로세스의 로그·종료·자동 모델 페치 이벤트가 도착하면 현재 generation과 비교해 폐기하므로,
이전 프로세스가 새 서버의 PID·상태·모델 목록을 덮어쓸 수 없습니다.

Compare 응답과 PTY 출력·종료 이벤트도 각각 요청/세션 generation으로 구분합니다. 새 채팅에서
block ID가 재사용되거나 PTY를 재시작한 뒤 이전 셸 이벤트가 도착해도 현재 요청과 세대가
다르면 폐기합니다.

## Update Pipeline

`App::update()`는 `src/update.rs`에서 모든 Message를 라우팅합니다. 순수 UI/state 변경 arms는 helper 메서드로 분리되어 있습니다:

```
Message::ToggleFavorite => self.toggle_favorite(),

fn toggle_favorite(&mut self) -> Task<Message> {
    // ... state mutation ...
    Task::none()
}
```

비동기 작업은 두 개의 arm으로 분리됩니다:
1. 시작 arm: `Task::perform(async, Message::Result)`
2. 결과 arm: `Message::Result(r) => { /* handle */ }`

자세한 내용은 [UpdatePipeline.md](UpdatePipeline.md) 참조.

## Subsystem Interactions

- **OpenRouter**: 외부 API, API 키 필요, SSE 스트리밍, 모델 목록 fetch
- **Tabby/OpenAI-compat**: 로컬/원격 엔드포인트, 사용자 등록, 커스텀 라벨
- **HF Download**: HuggingFace에서 모델 다운로드, EXL2 프리셋 지원
- **MCP**: stdio 기반 MCP 서버 연결, 동적 tool 정의 로드
- **PTY**: portable-pty 기반 터미널 에뮬레이션
- **Session**: 로컬 JSON 파일 기반 세션 영속화
- **Keystore**: OS 크레덴셜 매니저에 API 키 저장

## Security Model

- API 키는 평문으로 디스크에 저장하지 않음 (OS Credential Manager 사용)
- 키는 코드/로그/git 어디에도 출력되지 않음
- `src/keystore/mod.rs`가 모든 크레덜셜 I/O를 캡슐화
