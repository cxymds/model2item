#!/usr/bin/env python3

import asyncio
import json
from pathlib import Path
import sys

try:
    import iterm2
except ImportError as exc:  # pragma: no cover - runtime dependency
    print(
        "Missing Python package 'iterm2'. Install it with: python3 -m pip install iterm2",
        file=sys.stderr,
    )
    raise SystemExit(1) from exc


PAYLOAD = json.load(sys.stdin)


async def main(connection: "iterm2.Connection") -> None:
    app = await iterm2.async_get_app(connection)
    action = PAYLOAD.get("action")

    if action == "list_sessions":
        sessions = []
        for window in app.terminal_windows:
            for tab in window.tabs:
                for session in tab.all_sessions:
                    sessions.append(
                        {
                            "session_id": session.session_id,
                            "window_id": window.window_id,
                            "window_title": getattr(window, "window_title", "") or "Window",
                            "tab_id": tab.tab_id,
                            "tab_title": getattr(tab, "title", "") or "Tab",
                            "session_title": getattr(session, "name", "") or "Session",
                        }
                    )
        json.dump({"sessions": sessions}, sys.stdout)
        return

    if action != "execute_prompt":
        raise RuntimeError(f"Unsupported action: {action}")

    session = app.get_session_by_id(PAYLOAD["session_id"])
    if session is None:
        raise RuntimeError(f"iTerm2 session not found: {PAYLOAD['session_id']}")

    command_text = PAYLOAD["command_text"].rstrip() + "\n"
    output_path = Path(PAYLOAD["output_path"])
    error_path = Path(PAYLOAD["error_path"])
    status_path = Path(PAYLOAD["status_path"])

    await session.async_send_text(command_text)

    await wait_for_status_file(status_path)

    status_code = status_path.read_text(encoding="utf-8").strip()
    output_text = read_text_file(output_path)
    error_text = read_text_file(error_path)

    cleanup_execution_files(output_path, error_path, status_path)

    if status_code != "0":
        failure_message = error_text or output_text or (
            f"claude command exited with status {status_code}"
        )
        raise RuntimeError(failure_message)

    json.dump({"output_text": output_text}, sys.stdout)


async def wait_for_status_file(status_path: Path, timeout_seconds: float = 600.0) -> None:
    elapsed = 0.0
    poll_interval = 0.25
    while elapsed < timeout_seconds:
        if status_path.exists():
            return
        await asyncio.sleep(poll_interval)
        elapsed += poll_interval

    raise RuntimeError(f"Timed out waiting for Claude command to finish: {status_path}")


def read_text_file(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8").strip()


def cleanup_execution_files(*paths: Path) -> None:
    for path in paths:
        try:
            path.unlink(missing_ok=True)
        except OSError:
            pass


if __name__ == "__main__":
    try:
        iterm2.run_until_complete(main)
    except Exception as exc:  # pragma: no cover - runtime bridge
        print(str(exc), file=sys.stderr)
        sys.exit(1)
