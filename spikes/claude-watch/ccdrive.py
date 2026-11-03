"""Drive an interactive `claude` session in a pseudo console, so live tests run without a person at the keyboard.

Needs `pip install pywinpty pyte` (any venv). Test helper for card p0-claude-live-tests, not product code.

  python ccdrive.py serve NAME --cwd DIR -- claude [args]   holds the console; run it in the background
  python ccdrive.py serve-sdk NAME --cwd DIR -- claude -p --input-format stream-json --output-format stream-json ...
                                                            SDK path; `send` text becomes a user message, a JSON
                                                            object is sent as is (control responses)
  python ccdrive.py send NAME TEXT                          types TEXT; {kenter} {esc} {up} {down} {tab} {ctrl-c} ...
                                                            Claude Code under ConPTY takes Enter only as {kenter}
  python ccdrive.py screen NAME                             prints the current screen
  python ccdrive.py wait NAME REGEX [SECONDS]               waits until the screen matches, then prints it

State lives in %CCDRIVE_STATE% (default %TEMP%\\ccdrive), one folder per NAME.
"""

import os
import re
import sys
import threading
import time
from pathlib import Path

KEYS = {
    "{enter}": "\r",
    "{kenter}": "\x1b[13u",
    "{esc}": "\x1b",
    "{up}": "\x1b[A",
    "{down}": "\x1b[B",
    "{right}": "\x1b[C",
    "{left}": "\x1b[D",
    "{tab}": "\t",
    "{ctrl-c}": "\x03",
    "{space}": " ",
    "{bs}": "\x7f",
}

# The test session must look like a fresh one started by a person. Inherited variables from the session that runs
# this script would mark it as a child session (no status file) and copy its entrypoint and messaging token.
DROP_ENV = re.compile(r"^(CLAUDE|CLAUDECODE$|AI_AGENT$)", re.IGNORECASE)


def state_dir(name):
    root = Path(os.environ.get("CCDRIVE_STATE") or Path(os.environ["TEMP"]) / "ccdrive")
    d = root / name
    d.mkdir(parents=True, exist_ok=True)
    return d


def write_atomic(path, text):
    tmp = path.with_suffix(".tmp")
    tmp.write_text(text, encoding="utf-8", newline="\n")
    # A reader holding the file open blocks the replace on Windows. Never let that end the session: closing the
    # pseudo console makes Claude Code exit.
    for _ in range(20):
        try:
            os.replace(tmp, path)
            return
        except PermissionError:
            time.sleep(0.05)


def serve(name, cwd, argv, cols=140, rows=45):
    import pyte
    from winpty import PtyProcess

    d = state_dir(name)
    inbox = d / "in"
    inbox.mkdir(exist_ok=True)
    for f in inbox.iterdir():
        f.unlink()
    env = {k: v for k, v in os.environ.items() if not DROP_ENV.match(k)}
    env["TERM"] = "xterm-256color"
    screen = pyte.Screen(cols, rows)
    stream = pyte.Stream(screen)
    lock = threading.Lock()
    proc = PtyProcess.spawn(argv, cwd=cwd, env=env, dimensions=(rows, cols))
    write_atomic(d / "pid.txt", str(proc.pid))
    (d / "exit.txt").unlink(missing_ok=True)

    def reader():
        while True:
            try:
                data = proc.read(65536)
            except EOFError:
                return
            if data:
                with lock:
                    stream.feed(data)

    threading.Thread(target=reader, daemon=True).start()
    while proc.isalive():
        for f in sorted(inbox.iterdir()):
            if f.suffix != ".txt":
                continue
            proc.write(f.read_text(encoding="utf-8"))
            f.unlink()
            time.sleep(0.3)
        with lock:
            text = "\n".join(line.rstrip() for line in screen.display)
        write_atomic(d / "screen.txt", text)
        time.sleep(0.25)
    write_atomic(d / "exit.txt", str(proc.exitstatus))


def serve_sdk(name, cwd, argv):
    """SDK path: `claude -p --input-format stream-json ...`, the way the VS Code extension and the Agent SDK run it.

    Each `send` becomes one user message. Output events are logged by type only, never their text.
    """
    import json
    import subprocess

    d = state_dir(name)
    inbox = d / "in"
    inbox.mkdir(exist_ok=True)
    for f in inbox.iterdir():
        f.unlink()
    env = {k: v for k, v in os.environ.items() if not DROP_ENV.match(k)}
    proc = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL, text=True, encoding="utf-8")
    write_atomic(d / "pid.txt", str(proc.pid))
    events = open(d / "events.txt", "a", encoding="utf-8")

    def reader():
        for raw in proc.stdout:
            try:
                ev = json.loads(raw)
            except ValueError:
                continue
            kind = "/".join(str(ev.get(k)) for k in ("type", "subtype") if ev.get(k))
            if ev.get("type") == "control_request":
                req = ev.get("request", {})
                kind += f" request_id={ev.get('request_id')} {req.get('subtype')} {req.get('tool_name', '')}"
                # Test prompts only: the input is needed to answer "allow" with updatedInput.
                if req.get("subtype") == "can_use_tool":
                    kind += " input=" + json.dumps(req.get("input"))
            if ev.get("type") == "assistant":
                items = ev.get("message", {}).get("content", [])
                kind += " " + ",".join(i.get("name") or i.get("type", "?") for i in items)
            events.write(f"{time.strftime('%H:%M:%S')} {kind}\n")
            events.flush()

    threading.Thread(target=reader, daemon=True).start()
    while proc.poll() is None:
        for f in sorted(inbox.iterdir()):
            if f.suffix != ".txt":
                continue
            text = f.read_text(encoding="utf-8")
            # A JSON object goes as is (control responses); anything else is a user message.
            if text.startswith("{"):
                line = text.strip()
            else:
                line = json.dumps({"type": "user", "message": {"role": "user", "content": text}})
            proc.stdin.write(line + "\n")
            proc.stdin.flush()
            f.unlink()
        time.sleep(0.25)
    write_atomic(d / "exit.txt", str(proc.returncode))


def encode(text):
    for token, seq in KEYS.items():
        text = text.replace(token, seq)
    return text


def send(name, text):
    inbox = state_dir(name) / "in"
    inbox.mkdir(exist_ok=True)
    tmp = inbox / f"{time.time_ns()}.part"
    tmp.write_text(encode(text), encoding="utf-8", newline="")
    os.replace(tmp, tmp.with_suffix(".txt"))


def screen_text(name):
    p = state_dir(name) / "screen.txt"
    return p.read_text(encoding="utf-8") if p.exists() else ""


def wait(name, pattern, seconds):
    rx = re.compile(pattern, re.MULTILINE)
    end = time.time() + seconds
    while time.time() < end:
        text = screen_text(name)
        if rx.search(text):
            print(text)
            return 0
        time.sleep(0.5)
    print(screen_text(name))
    print(f"--- timeout: no match for {pattern!r} ---")
    return 1


def main(args):
    cmd = args[0]
    if cmd == "serve":
        name = args[1]
        cwd = args[args.index("--cwd") + 1]
        argv = args[args.index("--") + 1:]
        serve(name, cwd, argv)
        return 0
    if cmd == "serve-sdk":
        name = args[1]
        cwd = args[args.index("--cwd") + 1]
        argv = args[args.index("--") + 1:]
        serve_sdk(name, cwd, argv)
        return 0
    if cmd == "send":
        send(args[1], args[2])
        return 0
    if cmd == "screen":
        print(screen_text(args[1]))
        return 0
    if cmd == "wait":
        return wait(args[1], args[2], float(args[3]) if len(args) > 3 else 60)
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
