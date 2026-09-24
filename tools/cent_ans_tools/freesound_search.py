"""Lists CC0 Freesound candidates for a query (exploration helper of the AU1 sound bank).

Parses the public search page (no API token needed), filtered on the
"Creative Commons 0" licence, and prints id, duration, downloads, author and
title, most downloaded first. The licence of every retained sound is checked
again on its own page by ``audio_bank.verify_licence`` before download.

Usage: ``uv run --project tools python -m cent_ans_tools.freesound_search "sword clash" "horse neigh"``
"""

from __future__ import annotations

import re
import subprocess
import sys
import urllib.parse

SEARCH_URL = (
    "https://freesound.org/search/?q={query}&f=license%3A%22Creative+Commons+0%22"
)


def fetch(url: str) -> str:
    """Download a page as text with curl (follows redirects)."""
    result = subprocess.run(
        ["curl", "-sL", "-m", "40", "-A", "Mozilla/5.0", url],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout


def search(query: str) -> list[dict]:
    """Return the CC0 results of one search page, most downloaded first."""
    html = fetch(SEARCH_URL.format(query=urllib.parse.quote_plus(query)))
    results = []
    for block in re.finditer(r'class="bw-player"(.*?)tabindex', html, re.S):
        text = block.group(1)

        def attribute(name: str, text: str = text) -> str:
            match = re.search(name + r'="([^"]*)"', text)
            return match.group(1) if match else ""

        results.append(
            {
                "id": int(attribute("data-sound-id") or 0),
                "user": attribute("data-username"),
                "title": attribute("data-title"),
                "duration": float(attribute("data-duration") or 0.0),
                "downloads": int(attribute("data-num-downloads") or 0),
            }
        )
    results.sort(key=lambda entry: entry["downloads"], reverse=True)
    return results


def main(queries: list[str]) -> None:
    """Print the best candidates of each query."""
    for query in queries:
        print(f"## {query}")
        for entry in search(query)[:12]:
            print(
                f"  {entry['id']:>7} {entry['duration']:6.1f}s dl={entry['downloads']:6} "
                f"{entry['user']} | {entry['title']}"
            )


if __name__ == "__main__":
    main(sys.argv[1:])
