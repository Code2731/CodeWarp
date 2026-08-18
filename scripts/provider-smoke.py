#!/usr/bin/env python3
"""Verify an OpenAI-compatible provider's model list and streaming chat path."""

from __future__ import annotations

import argparse
import codecs
import json
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen


MODEL_KEYS = ("id", "name", "model", "model_name")


def normalize_base(endpoint: str) -> str:
    value = endpoint.strip().rstrip("/")
    if not value:
        raise ValueError("endpoint must not be empty")
    if not urlsplit(value).scheme:
        value = f"http://{value}"
    return value if value.endswith("/v1") else f"{value}/v1"


def headers(token: str) -> dict[str, str]:
    value = token.strip()
    if not value:
        return {}
    return {
        "Authorization": f"Bearer {value}",
        "x-api-key": value,
    }


def request_json(url: str, token: str, timeout: float) -> object:
    request = Request(url, headers=headers(token), method="GET")
    with urlopen(request, timeout=timeout) as response:
        return json.loads(response.read())


def model_ids(payload: object) -> list[str]:
    items = payload.get("data", payload) if isinstance(payload, dict) else payload
    if not isinstance(items, list):
        raise ValueError("/v1/models response has no data array")

    result: list[str] = []
    seen: set[str] = set()
    for item in items:
        if isinstance(item, str):
            value = item.strip()
        elif isinstance(item, dict):
            value = next(
                (str(item[key]).strip() for key in MODEL_KEYS if isinstance(item.get(key), str)),
                "",
            )
        else:
            value = ""
        if value and value not in seen:
            seen.add(value)
            result.append(value)
    return result


def stream_chat(url: str, model: str, prompt: str, token: str, timeout: float) -> tuple[str, int]:
    body = json.dumps(
        {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": True,
        }
    ).encode("utf-8")
    request = Request(
        url,
        data=body,
        headers={"Content-Type": "application/json", **headers(token)},
        method="POST",
    )
    decoder = codecs.getincrementaldecoder("utf-8")()
    buffer = ""
    output: list[str] = []
    done = False
    started = time.monotonic()
    with urlopen(request, timeout=timeout) as response:
        while not done:
            if time.monotonic() - started > timeout:
                raise TimeoutError("streaming response exceeded the smoke timeout")
            chunk = response.read(4096)
            if not chunk:
                break
            buffer += decoder.decode(chunk)
            while "\n" in buffer:
                line, buffer = buffer.split("\n", 1)
                payload = line.rstrip("\r").strip()
                if not payload.startswith("data:"):
                    continue
                payload = payload[5:].strip()
                if payload == "[DONE]":
                    done = True
                    break
                try:
                    event = json.loads(payload)
                except json.JSONDecodeError:
                    continue
                for choice in event.get("choices", []) if isinstance(event, dict) else []:
                    if not isinstance(choice, dict):
                        continue
                    delta = choice.get("delta")
                    value = delta.get("content") if isinstance(delta, dict) else choice.get("text")
                    if isinstance(value, str):
                        output.append(value)
        buffer += decoder.decode(b"", final=True)

    if not done:
        raise ValueError("streaming response ended before [DONE]")
    text = "".join(output)
    if not text.strip():
        raise ValueError("streaming response contained no text content")
    return text, len(output)


def format_error(error: BaseException) -> str:
    if isinstance(error, HTTPError):
        try:
            body = error.read().decode("utf-8", errors="replace").strip()
        except OSError:
            body = ""
        suffix = f": {body[:240]}" if body else ""
        return f"HTTP {error.code}{suffix}"
    if isinstance(error, URLError):
        return f"connection failed: {error.reason}"
    return str(error)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", default="http://127.0.0.1:11434")
    parser.add_argument("--model", default="")
    parser.add_argument("--token", default="")
    parser.add_argument("--prompt", default="Reply with exactly OK")
    parser.add_argument("--timeout-sec", type=float, default=120.0)
    args = parser.parse_args()

    try:
        base = normalize_base(args.endpoint)
        ids = model_ids(request_json(f"{base}/models", args.token, args.timeout_sec))
        if not ids:
            raise ValueError("/v1/models returned no model IDs")
        model = args.model.strip() or ids[0]
        if model not in ids:
            raise ValueError(f"requested model is not listed by /v1/models: {model}")
        text, chunk_count = stream_chat(
            f"{base}/chat/completions",
            model,
            args.prompt,
            args.token,
            args.timeout_sec,
        )
    except (OSError, TimeoutError, ValueError, json.JSONDecodeError) as error:
        print(f"provider smoke failed: {format_error(error)}", file=sys.stderr)
        return 1

    print(
        f"provider smoke passed: endpoint={base} models={len(ids)} "
        f"model={model} chunks={chunk_count} text_chars={len(text)} stream_done=true"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
