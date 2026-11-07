"""Draws social-card.png from social-card.html: 1280 x 640 at device scale 1, with headless Edge or Chrome.

social-card.png is the picture a shared link shows: the page's og:image, and GitHub's social preview for the repo
(set by hand in the repo's Settings). GitHub asks for a PNG, JPG or GIF under 1 MB, at least 640 x 320 and 1280 x 640
for best display. The script fails when the picture is not 1280 x 640 or not under 1 MB.

Run: python site/social_card.py [path to a Chromium browser]
Python 3.8 or later, standard library only. Kills only the browser it started, on a timeout.
"""

import os
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "social-card.html"
TARGET = HERE / "social-card.png"
WIDTH, HEIGHT = 1280, 640
LIMIT = 1024 * 1024
BROWSERS = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
]
ON_PATH = ["msedge", "microsoft-edge", "google-chrome", "chromium", "chromium-browser", "chrome"]


def find_browser(argv):
    if argv:
        return argv[0]
    for path in BROWSERS:
        if Path(path).is_file():
            return path
    for name in ON_PATH:
        found = shutil.which(name)
        if found:
            return found
    sys.exit("no Edge, Chrome or Chromium found; pass the browser's path")


def screenshot(browser, out):
    """Headless Chromium now and then exits before it writes the picture, so this tries three times."""
    for _ in range(3):
        profile = tempfile.mkdtemp(prefix="wakeshot-card-")
        args = [browser, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
                "--no-default-browser-check", f"--user-data-dir={profile}", "--force-device-scale-factor=1",
                # The page loads its font from its own folder, which a file:// page may only do with this.
                "--allow-file-access-from-files", f"--window-size={WIDTH},{HEIGHT}",
                "--virtual-time-budget=5000", f"--screenshot={out}", SOURCE.as_uri()]
        proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            proc.wait(120)
        except subprocess.TimeoutExpired:
            # On Windows the launcher starts the browser as its child: end the whole tree it started.
            if os.name == "nt":
                subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"], capture_output=True)
            else:
                proc.kill()
            proc.wait()
            sys.exit("the browser did not finish in 2 minutes")
        # On Windows the launcher can exit before the browser process it started has written the file.
        for _ in range(60):
            if out.is_file() and out.stat().st_size:
                break
            time.sleep(0.5)
        time.sleep(0.5)
        # The browser may hold its profile a moment longer; a leftover temp folder does no harm.
        shutil.rmtree(profile, ignore_errors=True)
        if out.is_file() and out.stat().st_size:
            return
    sys.exit("the browser wrote no picture")


def chunks(data):
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        sys.exit("the browser's picture is not a PNG")
    at = 8
    while at < len(data):
        length, kind = struct.unpack(">I4s", data[at:at + 8])
        yield kind, data[at + 8:at + 8 + length]
        at += 12 + length


def chunk(kind, body):
    return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))


def recompress(data):
    """The same pixels with the image data deflated at zlib's best level: Chromium writes PNGs fast, not small."""
    parts, image = [], b""
    for kind, body in chunks(data):
        if kind == b"IHDR":
            width, height = struct.unpack(">II", body[:8])
            if (width, height) != (WIDTH, HEIGHT):
                sys.exit(f"the picture is {width} x {height}, not {WIDTH} x {HEIGHT}")
        if kind == b"IDAT":
            image += body
            continue
        if kind == b"IEND":
            parts.append(chunk(b"IDAT", zlib.compress(zlib.decompress(image), 9)))
        parts.append(chunk(kind, body))
    return b"\x89PNG\r\n\x1a\n" + b"".join(parts)


def main(argv):
    browser = find_browser(argv)
    work = Path(tempfile.mkdtemp(prefix="wakeshot-card-"))
    try:
        shot = work / "card.png"
        screenshot(browser, shot)
        data = recompress(shot.read_bytes())
    finally:
        shutil.rmtree(work, ignore_errors=True)
    if len(data) >= LIMIT:
        sys.exit(f"the picture is {len(data):,} bytes, over GitHub's 1 MB")
    TARGET.write_bytes(data)
    print(f"wrote {TARGET} ({WIDTH} x {HEIGHT}, {len(data):,} bytes) with {browser}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
