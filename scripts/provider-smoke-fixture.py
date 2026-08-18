#!/usr/bin/env python3
"""Run provider-smoke.py against a deterministic local OpenAI-compatible fixture."""

from __future__ import annotations

import json
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


MODEL = "codewarp-fixture-model"
TOKEN = "codewarp-fixture-token"


class FixtureHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, format: str, *args: object) -> None:
        return

    def do_GET(self) -> None:  # noqa: N802
        if self.path != "/v1/models":
            self.send_error(404)
            return
        if not self._has_auth():
            self.send_error(401)
            return
        body = json.dumps({"object": "list", "data": [{"id": MODEL}]}).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/v1/chat/completions" or not self._has_auth():
            self.send_error(401)
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
            request = json.loads(self.rfile.read(length))
        except (ValueError, json.JSONDecodeError):
            self.send_error(400)
            return
        if request.get("model") != MODEL or request.get("stream") is not True:
            self.send_error(400)
            return

        frames = [
            'data: {"choices":[{"delta":{"content":"첫 응답 "}}]}\n\n',
            'data: {"choices":[{"delta":{"content":"😊"},"finish_reason":"stop"}]}\n\n',
            "data: [DONE]\n\n",
        ]
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()
        for frame in frames:
            payload = frame.encode("utf-8")
            for offset in range(0, len(payload), 2):
                self.wfile.write(payload[offset : offset + 2])
                self.wfile.flush()

    def _has_auth(self) -> bool:
        return (
            self.headers.get("Authorization") == f"Bearer {TOKEN}"
            and self.headers.get("x-api-key") == TOKEN
        )


def main() -> int:
    server = ThreadingHTTPServer(("127.0.0.1", 0), FixtureHandler)
    try:
        endpoint = f"http://127.0.0.1:{server.server_address[1]}"
        import threading

        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        smoke = Path(__file__).with_name("provider-smoke.py")
        result = subprocess.run(
            [
                sys.executable,
                str(smoke),
                "--endpoint",
                endpoint,
                "--model",
                MODEL,
                "--token",
                TOKEN,
                "--timeout-sec",
                "5",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            sys.stderr.write(result.stdout)
            sys.stderr.write(result.stderr)
            return result.returncode or 1
        print(result.stdout.strip())
        print("provider smoke fixture passed: auth, model list, UTF-8 SSE, and [DONE]")
        return 0
    finally:
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    raise SystemExit(main())
