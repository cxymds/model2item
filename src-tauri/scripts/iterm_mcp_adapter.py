#!/usr/bin/env python3

import asyncio
import json
import sys

try:
    import iterm2
except ImportError as exc:  # pragma: no cover - runtime dependency
    print(
        "Missing Python package 'iterm2'. Install iTerm2 Python API support first.",
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

    prompt = PAYLOAD["prompt"].rstrip() + "\n"
    await session.async_send_text(prompt)
    await asyncio.sleep(2.0)

    output_text = f"Prompt dispatched to {PAYLOAD['session_id']}. No screen text captured yet."
    if hasattr(session, "async_get_screen_contents"):
        screen = await session.async_get_screen_contents()
        lines = [line.string for line in getattr(screen, "screen_contents", [])]
        screen_text = "\n".join(line.rstrip() for line in lines if line is not None).strip()
        if screen_text:
            output_text = screen_text

    json.dump({"output_text": output_text}, sys.stdout)


if __name__ == "__main__":
    try:
        iterm2.run_until_complete(main)
    except Exception as exc:  # pragma: no cover - runtime bridge
        print(str(exc), file=sys.stderr)
        sys.exit(1)
