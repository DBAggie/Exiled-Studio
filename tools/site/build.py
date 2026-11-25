#!/usr/bin/env python3
"""The project site: the download page (docs/release/channels.md).

    build.py <releases.json> <out dir> --repo <owner/name> [--feeds <file>]

<releases.json> is GitHub's release list (`gh api --paginate --slurp
repos/<owner/name>/releases`: a list of pages, or one list). The page names
the newest published stable release (not a pre-release) and the newest
published experimental one (a pre-release), each with its installers, their
sizes, and their SHA-256 when GitHub gives one. Drafts are never shown.

--feeds writes, one per line, `<name> <url>` for the experimental release's
update and data feeds (latest.json, data-feed.json, data-feed.json.sig):
the experimental application reads them from the site under experimental/
(GitHub's `latest` never names a pre-release). pages.yml downloads them.

Standard library only; the page has no script and loads nothing from
elsewhere (ADR-011: no telemetry).
"""

import argparse
import html
import json
import shutil
import sys
from datetime import datetime
from pathlib import Path

SITE = Path(__file__).resolve().parents[2] / "site"

# (suffix, platform, kind, primary) in the order the page lists them.
PACKAGES = [
    ("-setup.exe", "Windows", "Installer (recommended)", True),
    (".msi", "Windows", "MSI installer, for all users", False),
    (".AppImage", "Linux", "AppImage", True),
    (".deb", "Linux", "Debian package", False),
]

FEEDS = ["latest.json", "data-feed.json", "data-feed.json.sig"]


def releases_of(raw):
    """The releases, flattened from `--slurp`'s pages, drafts left out."""
    flat = []
    for item in raw:
        flat.extend(item if isinstance(item, list) else [item])
    return [r for r in flat if not r.get("draft")]


def newest(releases, prerelease):
    chosen = [r for r in releases if bool(r.get("prerelease")) == prerelease]
    chosen.sort(key=lambda r: r.get("published_at") or "", reverse=True)
    return chosen[0] if chosen else None


def size_text(n):
    for unit in ("bytes", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n} {unit}" if unit == "bytes" else f"{n:.1f} {unit}"
        n /= 1024
    return ""


def date_text(stamp):
    if not stamp:
        return ""
    when = datetime.strptime(stamp, "%Y-%m-%dT%H:%M:%SZ")
    return f"{when.day} {when:%B %Y}"


def packages(release):
    """The release's installers, in PACKAGES order."""
    out = []
    for suffix, platform, kind, primary in PACKAGES:
        for asset in release.get("assets", []):
            name = asset.get("name", "")
            if name.endswith(suffix):
                digest = asset.get("digest") or ""
                out.append(
                    {
                        "name": name,
                        "url": asset.get("browser_download_url", ""),
                        "size": asset.get("size", 0),
                        "sha256": digest[7:] if digest.startswith("sha256:") else "",
                        "platform": platform,
                        "kind": kind,
                        "primary": primary,
                    }
                )
    return out


def feed_urls(release):
    by_name = {a.get("name"): a.get("browser_download_url") for a in release.get("assets", [])}
    return [(name, by_name[name]) for name in FEEDS if by_name.get(name)]


def section(channel, release, repo):
    e = html.escape
    title = "Stable" if channel == "stable" else "Experimental"
    blurb = {
        "stable": "The tested version. Most people want this one.",
        "experimental": (
            "Newer features from the experimental branch, tested less. It installs as a "
            "separate application, <strong>Exiled Studio Experimental</strong>, with its own "
            "settings, builds folder, and updates, and never replaces the stable one."
        ),
    }[channel]
    parts = [f'<section class="channel {channel}" aria-labelledby="{channel}-h">']
    parts.append(f'<h2 id="{channel}-h">{title}</h2>')
    parts.append(f'<p class="blurb">{blurb}</p>')
    if release is None:
        none = "No stable release yet." if channel == "stable" else "No experimental release yet."
        parts.append(
            f'<p class="none">{none} <a href="https://github.com/{e(repo)}/releases">'
            "All releases on GitHub</a>.</p></section>"
        )
        return "\n".join(parts)
    version = (release.get("tag_name") or "").removeprefix("v")
    parts.append(
        f'<p class="meta">Version <strong>{e(version)}</strong>'
        f" · {e(date_text(release.get('published_at')))}"
        f' · <a href="{e(release.get("html_url", ""))}">Release notes</a></p>'
    )
    found = packages(release)
    primary = [p for p in found if p["primary"]]
    if primary:
        parts.append('<p class="actions">')
        for p in primary:
            parts.append(
                f'<a class="button" href="{e(p["url"])}">Download for {e(p["platform"])}'
                f' <span>{e(p["kind"].split(" (")[0])}, {e(size_text(p["size"]))}</span></a>'
            )
        parts.append("</p>")
    if found:
        parts.append('<table><caption>All downloads</caption>')
        parts.append(
            '<thead><tr><th scope="col">Package</th><th scope="col">For</th>'
            '<th scope="col">Size</th><th scope="col">SHA-256</th></tr></thead><tbody>'
        )
        for p in found:
            sha = f'<code>{e(p["sha256"])}</code>' if p["sha256"] else "—"
            parts.append(
                f'<tr><td><a href="{e(p["url"])}">{e(p["name"])}</a></td>'
                f'<td>{e(p["platform"])}: {e(p["kind"])}</td>'
                f'<td class="num">{e(size_text(p["size"]))}</td><td class="sha">{sha}</td></tr>'
            )
        parts.append("</tbody></table>")
    else:
        parts.append('<p class="none">This release has no installers attached.</p>')
    parts.append("</section>")
    return "\n".join(parts)


def build(raw, out, repo, feeds=None):
    releases = releases_of(raw)
    stable = newest(releases, prerelease=False)
    experimental = newest(releases, prerelease=True)
    template = (SITE / "index.html").read_text(encoding="utf-8")
    page = (
        template.replace("{{repo}}", html.escape(repo))
        .replace("{{stable}}", section("stable", stable, repo))
        .replace("{{experimental}}", section("experimental", experimental, repo))
    )
    out.mkdir(parents=True, exist_ok=True)
    (out / "index.html").write_text(page, encoding="utf-8")
    shutil.copy(SITE / "style.css", out / "style.css")
    # GitHub Pages serves the files as they are (no Jekyll).
    (out / ".nojekyll").write_text("", encoding="utf-8")
    lines = feed_urls(experimental) if experimental else []
    if feeds is not None:
        feeds.write_text("".join(f"{n} {u}\n" for n, u in lines), encoding="utf-8")
    return stable, experimental, lines


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("releases", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--feeds", type=Path)
    args = parser.parse_args(argv)
    raw = json.loads(args.releases.read_text(encoding="utf-8"))
    stable, experimental, lines = build(raw, args.out, args.repo, args.feeds)
    for channel, r in (("stable", stable), ("experimental", experimental)):
        print(f"{channel}: {r['tag_name'] if r else 'none'}")
    print(f"experimental feeds: {len(lines)}")


if __name__ == "__main__":
    main(sys.argv[1:])
