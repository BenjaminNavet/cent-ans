"""Pack / fetch of the « Cent Ans relief » hosting package (lot SZ7, ADR 0077).

No network: HTTP resume is exercised against a local ``http.server`` instance
that supports ``Range`` (``http.server.SimpleHTTPRequestHandler`` does).
"""

from __future__ import annotations

import http.server
import json
import threading
from pathlib import Path

import pytest

from cent_ans_tools.geo import relief_fetch, relief_pack


def _write_fake_manifests(
    map_dir: Path,
    stamp: str = "2026-01-01T00:00:00Z",
    bake_versions: dict | None = None,
) -> None:
    map_dir.mkdir(parents=True, exist_ok=True)
    (map_dir / "relief_pyramid.json").write_text(
        json.dumps(
            {
                "cache": {"generated_at": stamp},
                "bake_versions": bake_versions or {"tier1": 1, "tier2": 1, "tier3": 1},
            }
        )
    )
    (map_dir / "rivers_fine.json").write_text(json.dumps({"generated_at": stamp}))
    (map_dir / "fine_anchors.json").write_text(json.dumps({"generated_at": stamp}))


def _write_fake_pyramid(map_dir: Path) -> Path:
    pyramid = map_dir / "pyramid"
    (pyramid / "E1").mkdir(parents=True)
    (pyramid / "E1" / "0_0.png").write_bytes(b"p" * 6000)
    (pyramid / "E1" / "0_1.png").write_bytes(b"q" * 6000)
    (pyramid / "hydro_fine" / "E2").mkdir(parents=True)
    (pyramid / "hydro_fine" / "E2" / "3_4.bin").write_bytes(b"r" * 1500)
    return pyramid


def _write_hosting(path: Path, version: int = 1) -> None:
    path.write_text(
        json.dumps(
            {
                "package_name": "cent-ans-relief",
                "base_url": "http://unused/{version}/",
                "version": version,
                "manifest_name": "manifest.json",
                "bake": {
                    "pyramid_generated_at": None,
                    "rivers_generated_at": None,
                    "roads_generated_at": None,
                    "signature": None,
                },
            }
        )
    )


def _pack(tmp_path: Path, max_part_bytes: int = 4000) -> tuple[Path, Path, Path]:
    """A small fixture pyramid, packed with tiny parts to force a split."""
    map_dir = tmp_path / "map"
    _write_fake_manifests(map_dir)
    pyramid = _write_fake_pyramid(map_dir)
    hosting = tmp_path / "relief_hosting.json"
    _write_hosting(hosting)
    out = tmp_path / "pack"
    result = relief_pack.pack(
        out,
        map_dir=map_dir,
        pyramid_dir=pyramid,
        hosting_file=hosting,
        max_part_bytes=max_part_bytes,
    )
    assert len(result.parts) >= 2, "small max_part_bytes should force a split"
    return map_dir, hosting, out


# --------------------------------------------------------------------------- pack


def test_pack_splits_into_parts_with_matching_checksums(tmp_path: Path) -> None:
    """Each part's SHA-256 in the manifest matches the file on disk."""
    _, _, out = _pack(tmp_path)
    manifest = json.loads((out / "manifest.json").read_text())
    # tarfile's stream mode writes in its own fixed-size blocks (RECORDSIZE, 10240 B):
    # the split only checks the threshold between those writes, so a part can exceed
    # it by up to one block. Harmless at the real ~1.9 GiB scale.
    for entry in manifest["parts"]:
        path = out / entry["name"]
        assert path.stat().st_size == entry["bytes"]
        assert path.stat().st_size <= 4000 + 10240
    assert manifest["total_bytes"] == sum(e["bytes"] for e in manifest["parts"])
    # The real repo's CREDITS.md is used by default: its geo section is embedded.
    assert "Copernicus" in manifest["credits"]


def test_pack_bumps_hosting_version_only_after_a_rebake(tmp_path: Path) -> None:
    """First pack keeps v1; a later rebake (new generated_at) bumps the version."""
    map_dir, hosting, out = _pack(tmp_path)
    assert json.loads(hosting.read_text())["version"] == 1

    # Same cache: packing again must not bump the version.
    relief_pack.pack(
        tmp_path / "pack2",
        map_dir=map_dir,
        pyramid_dir=map_dir / "pyramid",
        hosting_file=hosting,
    )
    assert json.loads(hosting.read_text())["version"] == 1

    # Rebake (new timestamp): the next pack bumps the version.
    _write_fake_manifests(map_dir, stamp="2026-02-01T00:00:00Z")
    relief_pack.pack(
        tmp_path / "pack3",
        map_dir=map_dir,
        pyramid_dir=map_dir / "pyramid",
        hosting_file=hosting,
    )
    assert json.loads(hosting.read_text())["version"] == 2


def test_extract_credits_reads_only_the_geo_section(tmp_path: Path) -> None:
    """Only the ``## Données géographiques`` section is copied (not the whole file)."""
    credits = tmp_path / "CREDITS.md"
    credits.write_text(
        "# Crédits\n\n## Icônes\ntexte icônes\n\n"
        "## Données géographiques\ntexte relief\n\n## Données historiques\nautre"
    )
    text = relief_pack.extract_credits(credits)
    assert text.startswith("## Données géographiques")
    assert "texte relief" in text
    assert "texte icônes" not in text
    assert "Données historiques" not in text


def test_pack_bumps_version_when_a_pyramid_tier_is_rebaked(tmp_path: Path) -> None:
    """A bump in ``bake_versions`` (lot SZ2) is enough to trigger a new package version."""
    map_dir = tmp_path / "map"
    _write_fake_manifests(map_dir, bake_versions={"tier1": 1, "tier2": 1, "tier3": 1})
    pyramid = _write_fake_pyramid(map_dir)
    hosting = tmp_path / "relief_hosting.json"
    _write_hosting(hosting)
    relief_pack.pack(
        tmp_path / "pack1", map_dir=map_dir, pyramid_dir=pyramid, hosting_file=hosting
    )
    assert json.loads(hosting.read_text())["version"] == 1

    _write_fake_manifests(map_dir, bake_versions={"tier1": 1, "tier2": 1, "tier3": 2})
    relief_pack.pack(
        tmp_path / "pack2", map_dir=map_dir, pyramid_dir=pyramid, hosting_file=hosting
    )
    updated = json.loads(hosting.read_text())
    assert updated["version"] == 2
    assert updated["bake"]["pyramid_bake_versions"] == {
        "tier1": 1,
        "tier2": 1,
        "tier3": 2,
    }


def test_pack_refuses_when_disk_is_too_small(tmp_path: Path, monkeypatch) -> None:  # noqa: ANN001
    """A filesystem reporting too little free space aborts before writing anything."""
    map_dir = tmp_path / "map"
    _write_fake_manifests(map_dir)
    pyramid = _write_fake_pyramid(map_dir)
    hosting = tmp_path / "relief_hosting.json"
    _write_hosting(hosting)
    out = tmp_path / "pack"

    class _TinyDisk:
        free = 10  # bytes

    monkeypatch.setattr(relief_pack.shutil, "disk_usage", lambda _path: _TinyDisk())
    with pytest.raises(OSError, match="insuffisant"):
        relief_pack.pack(
            out, map_dir=map_dir, pyramid_dir=pyramid, hosting_file=hosting
        )
    assert not any(out.iterdir()) if out.exists() else True


# --------------------------------------------------------------------------- fetch (--from-dir)


def test_fetch_from_dir_reassembles_and_extracts(tmp_path: Path) -> None:
    """A local part set installs the pyramid tree atomically."""
    _, _, out = _pack(tmp_path)
    dest = tmp_path / "dest"
    result = relief_fetch.fetch(dest=dest, from_dir=out, log=lambda *_: None)
    assert result.pyramid_dir == dest / "pyramid"
    assert (dest / "pyramid" / "E1" / "0_0.png").read_bytes() == b"p" * 6000
    assert (dest / "pyramid" / "E1" / "0_1.png").read_bytes() == b"q" * 6000
    assert (
        dest / "pyramid" / "hydro_fine" / "E2" / "3_4.bin"
    ).read_bytes() == b"r" * 1500
    # No leftover staging directories.
    assert not any(p.name.startswith(".pyramid.") for p in dest.iterdir())


def test_fetch_from_dir_replaces_an_existing_install(tmp_path: Path) -> None:
    """Fetching again over an already-installed pyramid swaps it in cleanly."""
    _, _, out = _pack(tmp_path)
    dest = tmp_path / "dest"
    (dest / "pyramid" / "E1").mkdir(parents=True)
    (dest / "pyramid" / "E1" / "stale.png").write_bytes(b"old")
    relief_fetch.fetch(dest=dest, from_dir=out, log=lambda *_: None)
    assert not (dest / "pyramid" / "E1" / "stale.png").exists()
    assert (dest / "pyramid" / "E1" / "0_0.png").exists()


def test_fetch_rejects_a_bad_checksum(tmp_path: Path) -> None:
    """A tampered part is refused before any extraction happens."""
    _, _, out = _pack(tmp_path)
    manifest_path = out / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    tampered = out / manifest["parts"][0]["name"]
    tampered.write_bytes(b"corrupted" * 100)
    dest = tmp_path / "dest"
    with pytest.raises(ValueError, match="somme de contrôle invalide"):
        relief_fetch.fetch(dest=dest, from_dir=out, log=lambda *_: None)
    assert not dest.exists()


def test_fetch_rejects_a_missing_part(tmp_path: Path) -> None:
    """A part named in the manifest but absent from the folder is refused."""
    _, _, out = _pack(tmp_path)
    manifest = json.loads((out / "manifest.json").read_text())
    (out / manifest["parts"][-1]["name"]).unlink()
    dest = tmp_path / "dest"
    with pytest.raises(ValueError, match="manquante"):
        relief_fetch.fetch(dest=dest, from_dir=out, log=lambda *_: None)


# --------------------------------------------------------------------------- fetch (HTTP)


class _RangeServer(http.server.ThreadingHTTPServer):
    allow_reuse_address = True


@pytest.fixture
def http_server(tmp_path: Path):  # noqa: ANN201
    """Serve ``tmp_path / 'served'`` over HTTP with Range support, on localhost."""
    served = tmp_path / "served"
    served.mkdir()

    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):  # noqa: ANN002, ANN003, ANN204
            super().__init__(*args, directory=str(served), **kwargs)

        def log_message(self, *_args) -> None:  # noqa: ANN002
            pass

    server = _RangeServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield served, f"http://127.0.0.1:{server.server_port}/"
    finally:
        server.shutdown()
        thread.join(timeout=5)


def test_fetch_over_http_downloads_and_verifies(tmp_path: Path, http_server) -> None:  # noqa: ANN001
    """A full HTTP fetch (no prior partial file) downloads every part and installs it."""
    served, base_url = http_server
    _, hosting, out = _pack(tmp_path)
    for item in out.iterdir():
        (served / item.name).write_bytes(item.read_bytes())
    dest = tmp_path / "dest"
    result = relief_fetch.fetch(
        dest=dest, base_url=base_url, hosting_file=hosting, log=lambda *_: None
    )
    assert (dest / "pyramid" / "E1" / "0_0.png").read_bytes() == b"p" * 6000
    assert result.parts >= 2


def test_download_part_resumes_a_truncated_download(
    tmp_path: Path, http_server
) -> None:  # noqa: ANN001
    """A partial local file resumes with a Range request instead of restarting."""
    served, base_url = http_server
    content = b"0123456789" * 500
    (served / "big.bin").write_bytes(content)
    import hashlib

    expected = hashlib.sha256(content).hexdigest()
    dest = tmp_path / "big.bin"
    dest.write_bytes(content[:2000])  # a truncated previous attempt
    relief_fetch.download_part(
        base_url + "big.bin", dest, expected, log=lambda *_: None
    )
    assert dest.read_bytes() == content


def test_download_part_rejects_a_bad_checksum(tmp_path: Path, http_server) -> None:  # noqa: ANN001
    """A full download that does not match the expected SHA-256 is deleted and refused."""
    served, base_url = http_server
    (served / "small.bin").write_bytes(b"hello")
    dest = tmp_path / "small.bin"
    with pytest.raises(ValueError, match="somme de contrôle invalide"):
        relief_fetch.download_part(
            base_url + "small.bin", dest, "0" * 64, log=lambda *_: None
        )
    assert not dest.exists()


def test_manifest_rejects_traversal_part_names(tmp_path):
    """A manifest whose part names leave the parts folder is refused."""
    import json

    import pytest

    for bad in ("../evil", "/etc/passwd", "a/b", "..", "x\\y"):
        path = tmp_path / "manifest.json"
        path.write_text(
            json.dumps({"parts": [{"name": bad, "sha256": "0"}]}), encoding="utf-8"
        )
        with pytest.raises(ValueError):
            relief_fetch._read_manifest(path)
