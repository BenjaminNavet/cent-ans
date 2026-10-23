"""Cuisson des tableaux de textures du sol de bataille en KTX2 (lot ktx2-array).

Le client Bevy charge, pour un biome, une bande albedo (13 couches 1024 px), une bande de normales
(13 couches 256 px) et le grain micro (grille 2 x 4 de 1024 px). Il construit sinon les mips et
le tableau sur le CPU à chaque bataille. Ce module cuit chaque bande en un tableau KTX2
(couches empilées, mips générés, UASTC + zstd) dans un cache hors dépôt (``out/ktx2_cache/``,
ignoré par git, jamais commité) ; le client le transcode en BC7 (feature ``basis-universal``).

Encodeur : KTX-Software >= 4.4 (``ktx``). Pas de BC7 direct en 4.4 : UASTC transcodé par Bevy.
Couleurs sRGB ; normales linéaires. Les mips sont un filtre boîte 2 x 2 appliqué sur les octets
tels quels (espace gamma), comme ``mip_chain`` du client.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

from PIL import Image

from cent_ans_tools.paths import REPO_DIR as REPO

ASSETS = REPO / "game" / "assets"
CACHE_DIR = REPO / "out" / "ktx2_cache"
PACK_GLOB = "tx_battle_b*_pack.json"
MICRO_MANIFEST = REPO / "data" / "art" / "tx_micro_ground_pack.json"
KTX_CANDIDATES = (
    Path.home() / "dev" / "ktx-software" / "bin" / "ktx",
    Path("/usr/local/bin/ktx"),
)


@dataclass(frozen=True)
class Strip:
    """Une bande à cuire : chemin source (sous ``game/assets``), grille et espace colorimétrique."""

    source: str
    cols: int
    rows: int
    srgb: bool

    @property
    def layers(self) -> int:
        """Nombre de couches du tableau."""
        return self.cols * self.rows

    def output(self, cache: Path = CACHE_DIR) -> Path:
        """Chemin du KTX2 dans le cache (même chemin relatif, extension ``.ktx2``)."""
        return cache / Path(self.source).with_suffix(".ktx2")


def find_ktx() -> Path:
    """Localise l'outil ``ktx`` (variable ``KTX``, PATH, puis emplacements connus)."""
    for candidate in (os.environ.get("KTX"), shutil.which("ktx"), *KTX_CANDIDATES):
        if candidate and Path(candidate).is_file():
            return Path(candidate)
    raise FileNotFoundError(
        "outil `ktx` introuvable (KTX-Software 4.4, voir docs/wip/bevy-ktx2-array.md)"
    )


def strips_of_battle() -> list[Strip]:
    """Bandes du sol de bataille : albedo/normales de chaque biome, puis le grain micro."""
    strips: list[Strip] = []
    for pack in sorted((REPO / "data" / "art").glob(PACK_GLOB)):
        manifest = json.loads(pack.read_text(encoding="utf-8"))
        cols, rows = manifest["grid"]
        for key, srgb in (("albedo", True), ("normal", False)):
            if key in manifest:
                path = manifest[key].removeprefix("res://assets/")
                strips.append(Strip(path, cols, rows, srgb))
    micro = json.loads(MICRO_MANIFEST.read_text(encoding="utf-8"))
    cols, rows = micro["grid"]
    for key, srgb in (("albedo", True), ("normal", False)):
        strips.append(Strip(micro[key].removeprefix("res://assets/"), cols, rows, srgb))
    return strips


def split_layers(strip: Strip, folder: Path) -> list[Path]:
    """Découpe la bande en PNG de couches, dans l'ordre du client (``grid_to_strip``).

    Une grille ``cols x rows`` se lit ligne par ligne ; une bande verticale (1 colonne) se lit de haut en bas.
    """
    image = Image.open(ASSETS / strip.source).convert("RGBA")
    size = image.width // strip.cols
    if image.width % strip.cols or image.height != size * strip.rows:
        raise ValueError(
            f"{strip.source}: forme {image.size} inattendue pour la grille {strip.cols}x{strip.rows}"
        )
    paths = []
    for row in range(strip.rows):
        for col in range(strip.cols):
            layer = image.crop(
                (col * size, row * size, (col + 1) * size, (row + 1) * size)
            )
            path = folder / f"layer_{row * strip.cols + col:03d}.png"
            layer.save(path)
            paths.append(path)
    return paths


def bake(strip: Strip, ktx: Path, cache: Path = CACHE_DIR) -> Path:
    """Cuit une bande en tableau KTX2 UASTC + zstd avec mips ; renvoie le fichier produit."""
    out = strip.output(cache)
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ktx2_layers_") as tmp:
        layers = split_layers(strip, Path(tmp))
        # UNORM même pour l'albedo : le filtre boîte reste en espace gamma (comme `mip_chain`) ;
        # sRGB ou linéaire est choisi par le chargeur Bevy (`is_srgb`), pas par le fichier.
        command = [
            str(ktx), "create",
            "--format", "R8G8B8A8_UNORM",
            "--assign-tf", "linear",  # sinon ktx linéarise les PNG (sRGB par défaut) : octets altérés
            "--layers", str(strip.layers),
            "--generate-mipmap", "--mipmap-filter", "box",
            "--encode", "uastc", "--uastc-quality", "2",
            "--zstd", "19",
            *map(str, layers), str(out),
        ]  # fmt: skip
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(
                f"ktx create a échoué pour {strip.source} : {result.stderr.strip()}"
            )
    return out


def is_stale(strip: Strip, target: Path) -> bool:
    """Vrai si le tableau du cache manque ou date d'avant sa source ou d'avant cette recette."""
    if not target.is_file():
        return True
    baked_at = target.stat().st_mtime
    source = ASSETS / strip.source
    newest_input = max(
        source.stat().st_mtime if source.is_file() else 0.0,
        Path(__file__).stat().st_mtime,
    )
    return baked_at < newest_input


def bake_all(
    only: str | None = None, force: bool = False
) -> list[tuple[Strip, Path, int]]:
    """Cuit les bandes de bataille périmées du cache (toutes avec ``force``).

    Périmée = absente, plus ancienne que sa bande source, ou plus ancienne que ce script : un
    changement de texture ou de recette de cuisson est donc repris sans option.
    """
    ktx = find_ktx()
    done = []
    for strip in strips_of_battle():
        if only and only not in strip.source:
            continue
        target = strip.output()
        if not force and not is_stale(strip, target):
            continue
        path = bake(strip, ktx)
        done.append((strip, path, path.stat().st_size))
    return done
