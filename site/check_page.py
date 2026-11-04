"""Checks the Pages page before it ships (p6-pages acceptance checks; pages.yml runs it before each deploy).

Fails when: an HTML file does not parse into balanced tags; a local link, image or CSS url() does not resolve inside
the folder; a src or href leaves https://github.com/HaikAsatryan/wakeshot and the page itself; the page has a script,
an event handler attribute or an embedded frame; any text file holds an em dash or an emoji. With --final it also
fails while a spot still waits for another card (an HTML comment that starts with "waits for").

Run: python site/check_page.py [--final] site
Python 3.8 or later, standard library only.
"""

import html
import re
import sys
import xml.etree.ElementTree as ET
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

REPO = "https://github.com/HaikAsatryan/wakeshot"
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}
URL_ATTRS = {"href", "src", "poster", "action", "formaction", "cite", "data"}
BANNED_TAGS = {"script", "iframe", "frame", "object", "embed", "applet", "base"}
TEXT_SUFFIXES = {".html", ".css", ".svg", ".py", ".txt", ".md", ".xml", ".json"}
# The page must stay plain text: no em dash (Haik's writing rule), no emoji. The emoji set is the main emoji blocks
# plus the emoji presentation selector, the joiner and the keycap mark; a plain symbol such as (c) is allowed.
# Built from code points, so this file passes its own text check.
EM_DASH = chr(0x2014)
EMOJI_RANGES = [
    (0x1F000, 0x1FAFF), (0x2300, 0x23FF), (0x2600, 0x27BF), (0x2B00, 0x2BFF),
    (0x3030, 0x3030), (0x303D, 0x303D), (0x3297, 0x3297), (0x3299, 0x3299),
    (0xFE0F, 0xFE0F), (0x200D, 0x200D), (0x20E3, 0x20E3),
]
EMOJI = re.compile("[" + "".join(f"{chr(a)}-{chr(b)}" for a, b in EMOJI_RANGES) + "]")
WAITS = re.compile(r"^\s*waits for\b", re.IGNORECASE)


class Page(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.stack = []
        self.errors = []
        self.ids = set()
        self.urls = []
        self.waits = []
        self.doctype = False
        self.lang = False
        self.charset = False
        self.title = ""
        self.in_title = False

    def where(self):
        return f"line {self.getpos()[0]}"

    def handle_decl(self, decl):
        if decl.lower() == "doctype html":
            self.doctype = True

    def handle_starttag(self, tag, attrs):
        self.check_tag(tag, attrs)
        if tag not in VOID:
            self.stack.append((tag, self.where()))
        if tag == "title":
            self.in_title = True

    def handle_startendtag(self, tag, attrs):
        self.check_tag(tag, attrs)

    def check_tag(self, tag, attrs):
        if tag in BANNED_TAGS:
            self.errors.append(f"{self.where()}: <{tag}> is not allowed (no scripts, frames or embeds)")
        seen = set()
        for name, value in attrs:
            if name in seen:
                self.errors.append(f"{self.where()}: <{tag}> repeats the attribute {name}")
            seen.add(name)
            if name.startswith("on"):
                self.errors.append(f"{self.where()}: <{tag} {name}> is a script")
            if name == "id":
                if value in self.ids:
                    self.errors.append(f"{self.where()}: id {value!r} is used twice")
                self.ids.add(value)
            if name in URL_ATTRS and value is not None:
                self.urls.append((self.where(), tag, value))
            if name == "srcset" and value:
                for part in value.split(","):
                    if part.strip():
                        self.urls.append((self.where(), tag, part.split()[0]))
            if name == "style" and value:
                for url in css_urls(value):
                    self.urls.append((self.where(), tag, url))
        names = dict(attrs)
        if tag == "html" and names.get("lang"):
            self.lang = True
        if tag == "meta" and (names.get("charset") or "").lower() == "utf-8":
            self.charset = True
        if tag == "meta" and (names.get("http-equiv") or "").lower() == "refresh":
            self.errors.append(f"{self.where()}: a meta refresh is not allowed")

    def handle_endtag(self, tag):
        if tag in VOID:
            self.errors.append(f"{self.where()}: </{tag}> closes a void element")
            return
        if not self.stack:
            self.errors.append(f"{self.where()}: </{tag}> has no open tag")
            return
        open_tag, opened = self.stack.pop()
        if open_tag != tag:
            self.errors.append(f"{self.where()}: </{tag}> closes <{open_tag}> opened at {opened}")
        if tag == "title":
            self.in_title = False

    def handle_data(self, data):
        if self.in_title:
            self.title += data

    def handle_comment(self, data):
        if WAITS.match(data):
            self.waits.append(f"{self.where()}: {data.strip()[:100]}")


def css_urls(text):
    found = re.findall(r"""url\(\s*(['"]?)(.*?)\1\s*\)""", text)
    found = [u for _, u in found]
    found += re.findall(r"""@import\s+['"]([^'"]+)['"]""", text)
    return found


def check_url(root, base_file, url, ids):
    """Returns an error text, or None when the URL is allowed and resolves."""
    url = url.strip()
    if not url:
        return "empty URL"
    if url.startswith("//"):
        return f"{url}: protocol-relative URL to another host"
    parts = urlsplit(url)
    if parts.scheme:
        # The repo itself or a path under it; a look-alike such as .../wakeshot-evil does not pass.
        if url == REPO or any(url.startswith(REPO + c) for c in "/#?"):
            return None
        return f"{url}: outside {REPO} and the page"
    if not parts.path:
        if parts.fragment and parts.fragment not in ids:
            return f"{url}: no element with this id"
        return None
    target = (base_file.parent / unquote(parts.path)).resolve()
    try:
        target.relative_to(root)
    except ValueError:
        return f"{url}: leaves the page folder"
    if target.is_dir():
        target = target / "index.html"
    if not target.is_file():
        return f"{url}: file not found"
    return None


def check_text(path, text):
    errors = []
    plain = html.unescape(text) if path.suffix in (".html", ".svg") else text
    for number, line in enumerate(plain.splitlines(), 1):
        if EM_DASH in line:
            errors.append(f"{path.name} line {number}: em dash")
        for mark in EMOJI.findall(line):
            errors.append(f"{path.name} line {number}: emoji U+{ord(mark):04X}")
    return errors


def check_svg(root, path, errors):
    try:
        tree = ET.parse(path)
    except ET.ParseError as e:
        errors.append(f"{path.name}: does not parse as XML: {e}")
        return
    for node in tree.iter():
        tag = node.tag.split("}")[-1]
        if tag in ("script", "foreignObject"):
            errors.append(f"{path.name}: <{tag}> is not allowed")
        for name, value in node.attrib.items():
            local = name.split("}")[-1]
            if local.startswith("on"):
                errors.append(f"{path.name}: <{tag} {local}> is a script")
            if local in ("href", "src"):
                problem = check_url(root, path, value, set())
                if problem:
                    errors.append(f"{path.name}: {problem}")
        if tag == "style" and node.text:
            for url in css_urls(node.text):
                problem = check_url(root, path, url, set())
                if problem:
                    errors.append(f"{path.name}: {problem}")


def main(argv):
    final = "--final" in argv
    rest = [a for a in argv if a != "--final"]
    if len(rest) != 1:
        print(__doc__)
        return 2
    root = Path(rest[0]).resolve()
    if not (root / "index.html").is_file():
        print(f"{root}: no index.html")
        return 2

    errors, waits, used = [], [], set()
    files = sorted(p for p in root.rglob("*") if p.is_file())
    for path in files:
        rel = path.relative_to(root).as_posix()
        if path.suffix in TEXT_SUFFIXES:
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                errors.append(f"{rel}: not UTF-8")
                continue
            errors += check_text(path, text)
        if path.suffix == ".html":
            page = Page()
            page.feed(text)
            page.close()
            errors += [f"{rel} {e}" for e in page.errors]
            errors += [f"{rel}: <{tag}> opened at {at} is never closed" for tag, at in page.stack]
            if not page.doctype:
                errors.append(f"{rel}: no <!DOCTYPE html>")
            if not page.lang:
                errors.append(f"{rel}: <html> has no lang")
            if not page.charset:
                errors.append(f"{rel}: no <meta charset=\"utf-8\">")
            if not page.title.strip():
                errors.append(f"{rel}: no <title>")
            for at, tag, url in page.urls:
                problem = check_url(root, path, url, page.ids)
                if problem:
                    errors.append(f"{rel} {at}: <{tag}> {problem}")
                elif not urlsplit(url).scheme and urlsplit(url).path:
                    used.add((path.parent / unquote(urlsplit(url).path)).resolve())
            waits += [f"{rel} {w}" for w in page.waits]
        if path.suffix == ".css":
            for url in css_urls(text):
                problem = check_url(root, path, url, set())
                if problem:
                    errors.append(f"{rel}: url() {problem}")
                elif not urlsplit(url).scheme:
                    used.add((path.parent / unquote(urlsplit(url).path)).resolve())
            if "prefers-color-scheme: dark" not in text:
                errors.append(f"{rel}: no prefers-color-scheme: dark block")
        if path.suffix == ".svg":
            check_svg(root, path, errors)

    if final:
        errors += [f"still waits: {w}" for w in waits]

    unused = [p.relative_to(root).as_posix() for p in files
              if p not in used and p.name not in ("index.html", Path(__file__).name)]
    print(f"checked {len(files)} files in {root}")
    for w in waits if not final else []:
        print(f"waits: {w}")
    for u in unused:
        print(f"note: {u} is not linked from any page")
    for e in errors:
        print(f"error: {e}")
    print(f"{len(errors)} errors, {len(waits)} spots wait for other cards")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
