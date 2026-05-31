# Provider Integration

CodeWarp의 멀티 프로바이더 아키텍처.

## Provider Model

CodeWarp는 두 가지 프로바이더를 동시에 활성화할 수 있습니다:

- **OpenRouter**: 외부 API 서비스, API 키 기반 인증
- **OpenAI-compat**: 로컬/원격 엔드포인트, 사용자 등록, 커스텀 라벨 (`[xLLM]`, `[Ollama]`, `[Tabby]` 등)

`LlmProvider` enum은 `src/main.rs`에 정의:
```rust
enum LlmProvider {
    OpenRouter,
    OpenAICompat,
}
```

## Model Selector Architecture

모델 선택기는 두 프로바이더의 모델을 통합 표시합니다:

- `model_options: Vec<ModelOption>` — 모든 프로바이더의 모델을 하나의 리스트로 통합
- `selected_model: Option<String>` — 선택된 모델 ID
- `selected_model_provider: Option<LlmProvider>` — 선택된 모델의 프로바이더

모델 표시 형식:
- `[OR][KO] gpt-4o  $5.00/$15.00` — OpenRouter, 한국어 친화
- `[xLLM] qwen2.5-coder-7b  free` — 자체 엔드포인트
- `[Ollama] qwen2.5-coder:7b  free` — 외부 Ollama

## Chat Stream Pipeline

채팅 메시지가 전송될 때의 프로바이더 라우팅:

```
User sends message
  → App::update(Message::Send)
    → determine provider from selected_model_provider
    → if OpenRouter: openrouter::chat_stream(conversation, model, key)
    → if OpenAI-compat: tabby::chat_stream(conversation, model, url, token)
    → Task::run(stream, Message::ChatChunk)
    → on each token: append to assistant block, update view
    → on done: finalize block, save session
```

## OpenRouter Integration (`src/openrouter.rs`)

- **Authentication**: API 키를 `keystore.rs`에서 읽음
- **Model Listing**: `GET /api/v1/models` 호출, 가격/태그 정보 파싱
- **Chat Stream**: SSE 기반 스트리밍, 토큰 단위로 `ChatEvent` 전달
- **Error Handling**: 401, 402, 404, 429, 5xx 등에 대한 한국어 에러 메시지

## OpenAI-Compat Integration (`src/tabby.rs`)

- **Registration**: 사용자가 라벨/URL/토큰을 직접 입력
- **Endpoint Validation**: `/v1/models` ping으로 연결 확인
- **Model Listing**: OpenAI 호환 `/v1/models` 엔드포인트 사용
- **Chat Stream**: OpenAI 호환 SSE 스트리밍
- **Runtime Management**: inference 서버 spawn/stop/health check

## Provider Health Indicators

- **Tabby status**: `tabby_status: Option<Result<(), String>>` — 엔드포인트 연결 상태
- **Retry logic**: `tabby_connect_retry_left`, `tabby_retry_generation` — 자동 재연결 시도
- **Visual indicator**: UI에서 연결됨/끊김/미시도 상태 표시 (● 아이콘)

## Error Humanization

각 프로바이더는 에러를 한국어로 변환하는 humanize 함수를 제공:
- `openrouter::humanize_error()` — OpenRouter 에러 변환
- `tabby::humanize_error()` — Tabby/OpenAI-compat 에러 변환

자세한 내용은 [ModelManager.md](ModelManager.md) 참조.
