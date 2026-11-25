#!/usr/bin/env python3
"""Tests of the download page builder (tools/site/build.py); CI's `site` job
runs them: python3 -m unittest discover -s tools/site -p 'test_*.py'."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build  # noqa: E402

REPO = "DBAggie/Exiled-Studio"


def asset(name, size=1000, digest=None):
    out = {
        "name": name,
        "size": size,
        "browser_download_url": f"https://github.com/{REPO}/releases/download/x/{name}",
    }
    if digest:
        out["digest"] = digest
    return out


def release(tag, published, prerelease=False, draft=False, assets=()):
    return {
        "tag_name": tag,
        "published_at": published,
        "prerelease": prerelease,
        "draft": draft,
        "html_url": f"https://github.com/{REPO}/releases/tag/{tag}",
        "assets": list(assets),
    }


def installers(product, version):
    return [
        asset(f"{product}_{version}_x64-setup.exe", 140_000_000, "sha256:" + "a" * 64),
        asset(f"{product}_{version}_x64-setup.exe.sig", 400),
        asset(f"{product}_{version}_x64_en-US.msi", 9_000_000),
        asset(f"{product.replace(' ', '-')}_{version}_x64-portable.zip", 8_000_000),
        asset(f"{product}_{version}_amd64.AppImage", 90_000_000),
        asset(f"{product}_{version}_amd64.deb", 7_000_000),
        asset("latest.json", 900),
        asset("data-feed.json", 300),
        asset("data-feed.json.sig", 200),
        asset("game-data.loombundle", 40_000_000),
    ]


class BuildTest(unittest.TestCase):
    def run_build(self, raw):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "site"
            feeds = Path(tmp) / "feeds.txt"
            stable, experimental, _ = build.build(raw, out, REPO, feeds)
            return (
                stable,
                experimental,
                (out / "index.html").read_text(encoding="utf-8"),
                feeds.read_text(encoding="utf-8"),
                sorted(p.name for p in out.iterdir()),
            )

    def test_newest_of_each_channel_without_drafts(self):
        raw = [
            # Two pages, as `gh api --paginate --slurp` gives them.
            [
                release("v0.2.0", "2026-12-01T10:00:00Z", assets=installers("Exiled Studio", "0.2.0")),
                release("v0.3.0", "2026-12-20T10:00:00Z", draft=True),
                release(
                    "v0.3.0-2",
                    "2026-12-15T10:00:00Z",
                    prerelease=True,
                    assets=installers("Exiled Studio Experimental", "0.3.0-2"),
                ),
            ],
            [
                release("v0.1.0", "2026-11-20T10:00:00Z"),
                release("v0.3.0-1", "2026-12-10T10:00:00Z", prerelease=True),
            ],
        ]
        stable, experimental, page, feeds, files = self.run_build(raw)
        self.assertEqual(stable["tag_name"], "v0.2.0")
        self.assertEqual(experimental["tag_name"], "v0.3.0-2")
        self.assertEqual(files, [".nojekyll", "index.html", "style.css"])
        self.assertIn("Version <strong>0.2.0</strong> · 1 December 2026", page)
        self.assertIn("Version <strong>0.3.0-2</strong>", page)
        self.assertNotIn("0.3.0-1", page)
        self.assertNotIn("v0.3.0<", page)
        # Installers only: no signatures, feeds, or packs.
        self.assertNotIn(".sig<", page)
        self.assertNotIn("latest.json", page)
        self.assertNotIn("loombundle", page)
        self.assertIn("Exiled Studio_0.2.0_x64-setup.exe", page)
        # A release that still has a portable zip does not offer it.
        self.assertNotIn("portable", page)
        self.assertIn("<code>" + "a" * 64 + "</code>", page)
        self.assertIn("133.5 MB", page)
        self.assertNotIn("{{", page)
        # The experimental feeds, for the site's experimental/ folder.
        self.assertEqual(
            [line.split(" ")[0] for line in feeds.splitlines()],
            ["latest.json", "data-feed.json", "data-feed.json.sig"],
        )
        self.assertTrue(all("/releases/download/" in line for line in feeds.splitlines()))

    def test_no_release_yet(self):
        stable, experimental, page, feeds, _ = self.run_build([])
        self.assertIsNone(stable)
        self.assertIsNone(experimental)
        self.assertIn("No stable release yet.", page)
        self.assertIn("No experimental release yet.", page)
        self.assertEqual(feeds, "")

    def test_text_is_escaped(self):
        raw = [release('v1.0.0"><script>', "2026-12-01T10:00:00Z")]
        _, _, page, _, _ = self.run_build(raw)
        self.assertNotIn("<script>", page)

    def test_the_command_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "releases.json"
            src.write_text(json.dumps([[]]), encoding="utf-8")
            build.main([str(src), str(Path(tmp) / "out"), "--repo", REPO])
            self.assertTrue((Path(tmp) / "out" / "index.html").exists())


if __name__ == "__main__":
    unittest.main()
