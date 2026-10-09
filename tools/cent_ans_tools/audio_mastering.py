"""Mesure de sonie et finitions audio (lot RX audio, ADR 0247).

- `track_gains` : gain par morceau (dB) pour ramener tous les morceaux de
  `data/audio/music.json` à une sonie intégrée commune (EBU R128, `ffmpeg ebur128`).
- `limit_peaks` : limiteur (re-encodage) des échantillons qui touchent 0 dBFS.
- `loop_crossfade` : fondu de boucle interne d'une ambiance (la fin se fond dans le début).

Usage : `uv run --project tools python -m cent_ans_tools.audio_mastering gains --apply`.
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
MUSIC_JSON = REPO / "data" / "audio" / "music.json"
GAME = REPO / "game"
MAX_BOOST_DB = 9.0
MAX_CUT_DB = -12.0
PEAK_CEILING_DBTP = -1.0
## Mono dupliqué en deux canaux sans atténuation (l'encodeur Vorbis natif exige 2 canaux).
DUPLICATE_MONO = "pan=stereo|c0=c0|c1=c0"


def _ogg_codec() -> list[str]:
    """Options d'encodage Vorbis : libvorbis si présent, sinon l'encodeur natif (stéréo, 128 k)."""
    encoders = subprocess.run(
        ["ffmpeg", "-hide_banner", "-encoders"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    if "libvorbis" in encoders:
        return ["-c:a", "libvorbis", "-q:a", "5"]
    # L'encodeur natif n'écrit que de la stéréo : le mono est dupliqué sans perte de niveau.
    return ["-c:a", "vorbis", "-strict", "-2", "-b:a", "128k"]


def _channels(path: Path) -> int:
    probe = subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-show_entries",
            "stream=channels",
            "-of",
            "csv=p=0",
            str(path),
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    return int(probe.stdout.split()[0])


def _mono_fix(path: Path, native_vorbis: bool) -> str:
    """Filtre de fin de chaîne : duplique un mono quand l'encodeur natif l'exige."""
    return DUPLICATE_MONO if native_vorbis and _channels(path) == 1 else "anull"


def _native_vorbis() -> bool:
    return _ogg_codec()[1] == "vorbis"


def measure(path: Path) -> tuple[float, float]:
    """Sonie intégrée (LUFS) et crête vraie (dBTP) d'un fichier audio."""
    result = subprocess.run(
        [
            "ffmpeg",
            "-hide_banner",
            "-nostats",
            "-i",
            str(path),
            "-af",
            "ebur128=peak=true",
            "-f",
            "null",
            "-",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    summary = result.stderr.rsplit("Summary:", 1)[-1]
    loudness = re.search(r"I:\s+(-?[\d.]+) LUFS", summary)
    peak = re.search(r"Peak:\s+(-?[\d.]+) dBFS", summary)
    if loudness is None or peak is None:
        raise ValueError(f"mesure impossible : {path}")
    return float(loudness.group(1)), float(peak.group(1))


def playlist_tracks(music: dict) -> list[str]:
    """Tous les morceaux des listes de lecture, sans doublon, dans l'ordre."""
    seen: dict[str, None] = {}
    for playlist in music["playlists"].values():
        for tier in ("primary", "fallback"):
            for track in playlist.get(tier, []):
                seen[track] = None
    return list(seen)


def gain_for(loudness: float, true_peak: float, target: float) -> float:
    """Gain (dB, arrondi à 0,5) qui amène `loudness` à `target` sans dépasser -1 dBTP."""
    gain = target - loudness
    gain = min(gain, PEAK_CEILING_DBTP - true_peak)
    gain = max(MAX_CUT_DB, min(MAX_BOOST_DB, gain))
    return round(gain * 2) / 2


def track_gains(
    music: dict, target: float | None = None
) -> tuple[float, dict[str, float]]:
    """Cible (LUFS) et gain de chaque morceau présent sur disque.

    Sans cible, la médiane des morceaux « primary » : le niveau global ne bouge pas, seuls les
    écarts sont résorbés.
    """
    measured: dict[str, tuple[float, float]] = {}
    for track in playlist_tracks(music):
        path = GAME / track
        if path.exists():
            measured[track] = measure(path)
    if target is None:
        primary = {t for p in music["playlists"].values() for t in p.get("primary", [])}
        target = round(
            statistics.median(measured[t][0] for t in primary if t in measured)
        )
    return target, {t: gain_for(*m, target) for t, m in measured.items()}


def limit_peaks(path: Path, ceiling_db: float = -1.0) -> None:
    """Re-encode `path` avec un limiteur à `ceiling_db` (dBFS), même format et même débit."""
    limit = 10 ** (ceiling_db / 20)
    temporary = path.with_name(path.stem + ".limited" + path.suffix)
    codec = ["-c:a", "pcm_s16le"] if path.suffix == ".wav" else _ogg_codec()
    subprocess.run(
        [
            "ffmpeg",
            "-y",
            "-loglevel",
            "error",
            "-i",
            str(path),
            "-af",
            f"alimiter=limit={limit:.4f}:level=disabled:attack=2:release=40,{_mono_fix(path, path.suffix != '.wav' and _native_vorbis())}",
            *codec,
            str(temporary),
        ],
        check=True,
    )
    temporary.replace(path)


def loop_crossfade(path: Path, seconds: float = 1.5) -> None:
    """Fond la fin de `path` dans son début (durée − `seconds`) : la boucle devient continue."""
    duration = float(
        subprocess.run(
            [
                "ffprobe",
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "csv=p=0",
                str(path),
            ],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    tail_start = duration - seconds
    graph = (
        f"[0]atrim=0:{seconds},asetpts=PTS-STARTPTS,afade=t=in:d={seconds}:curve=qsin[head];"
        f"[0]atrim={tail_start}:{duration},asetpts=PTS-STARTPTS,afade=t=out:d={seconds}:curve=qsin[tail];"
        f"[head][tail]amix=inputs=2:normalize=0:duration=longest[mix];"
        f"[0]atrim={seconds}:{tail_start},asetpts=PTS-STARTPTS[body];"
        f"[mix][body]concat=n=2:v=0:a=1,{_mono_fix(path, _native_vorbis())}[out]"
    )
    temporary = path.with_name(path.stem + ".loop" + path.suffix)
    subprocess.run(
        [
            "ffmpeg",
            "-y",
            "-loglevel",
            "error",
            "-i",
            str(path),
            "-filter_complex",
            graph,
            "-map",
            "[out]",
            *_ogg_codec(),
            str(temporary),
        ],
        check=True,
    )
    temporary.replace(path)


BANK_JSON = REPO / "data" / "audio" / "sound_bank.json"
VARIANT_BAND_DB = 3.0
VARIANT_MAX_DB = 6.0


def mean_level(path: Path) -> tuple[float, float]:
    """Niveau moyen et crête (dBFS, `volumedetect`) d'un fichier."""
    result = subprocess.run(
        [
            "ffmpeg",
            "-hide_banner",
            "-nostats",
            "-i",
            str(path),
            "-af",
            "volumedetect",
            "-f",
            "null",
            "-",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    mean = float(re.search(r"mean_volume: (-?[\d.]+) dB", result.stderr).group(1))
    peak = float(re.search(r"max_volume: (-?[\d.]+) dB", result.stderr).group(1))
    return mean, peak


def variant_gains(levels: dict[str, tuple[float, float]]) -> dict[str, float]:
    """Gain par variante qui ramène chacune à ± `VARIANT_BAND_DB` de la médiane du groupe.

    Les variantes déjà dans la bande ne bougent pas : l'écart naturel est gardé, seuls les
    extrêmes (inaudible / dominant) sont rognés. Résultat arrondi à 0,5 dB, borné à ± 6 dB et par
    la marge avant -1 dBFS de la crête (jamais d'écrêtage ajouté).
    """
    median = statistics.median(mean for mean, _ in levels.values())
    gains = {}
    for name, (level, peak) in levels.items():
        deviation = level - median
        if abs(deviation) <= VARIANT_BAND_DB:
            continue
        gain = -(
            deviation - VARIANT_BAND_DB
            if deviation > 0
            else deviation + VARIANT_BAND_DB
        )
        gain = max(-VARIANT_MAX_DB, min(VARIANT_MAX_DB, gain, -1.0 - peak))
        gain = (
            int(gain * 2) / 2
        )  # arrondi vers zéro : la marge de crête reste respectée
        if gain:
            gains[name] = gain
    return gains


def apply_variant_gains() -> int:
    """Écrit `file_gain_db` dans chaque événement de `sound_bank.json` (texte conservé)."""
    text = BANK_JSON.read_text(encoding="utf-8")
    bank = json.loads(text)
    count = 0
    for event, entry in bank["events"].items():
        files = entry.get("files", [])
        if len(files) < 3 or event in {"ui_click"}:
            continue
        levels = {
            name: mean_level(GAME / "assets" / "audio" / f"{name}.ogg")
            for name in files
            if (GAME / "assets" / "audio" / f"{name}.ogg").exists()
        }
        gains = variant_gains(levels) if len(levels) >= 3 else {}
        if not gains:
            continue
        pattern = re.compile(
            r'("' + re.escape(event) + r'": \{[^\n]*?"files": \[[^\]]*\])'
        )
        text, replaced = pattern.subn(
            lambda m, g=gains: (
                m.group(1) + ', "file_gain_db": ' + json.dumps(g, ensure_ascii=False)
            ),
            text,
            count=1,
        )
        count += replaced
    BANK_JSON.write_text(text, encoding="utf-8")
    json.loads(text)
    return count


def main() -> None:
    """Point d'entrée : `gains`, `variants`, `limit`, `loopfade`."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    gains = sub.add_parser("gains", help="calcule `track_gain_db` de music.json")
    gains.add_argument(
        "--target",
        type=float,
        default=None,
        help="cible en LUFS (défaut : médiane des primary)",
    )
    gains.add_argument(
        "--apply", action="store_true", help="écrit dans data/audio/music.json"
    )
    sub.add_parser(
        "variants",
        help="écrit `file_gain_db` (variantes d'un événement) dans sound_bank.json",
    )
    limit = sub.add_parser("limit", help="limiteur -1 dBFS sur des fichiers")
    limit.add_argument("files", nargs="+", type=Path)
    loop = sub.add_parser("loopfade", help="fondu de boucle interne")
    loop.add_argument("files", nargs="+", type=Path)
    loop.add_argument("--seconds", type=float, default=1.5)
    args = parser.parse_args()
    if args.command == "gains":
        music = json.loads(MUSIC_JSON.read_text(encoding="utf-8"))
        target, table = track_gains(music, args.target)
        print(
            f"cible {target} LUFS, {len(table)} morceaux, gain de {min(table.values())} à {max(table.values())} dB"
        )
        if args.apply:
            music["loudness_target_lufs"] = target
            music["track_gain_db"] = dict(sorted(table.items()))
            MUSIC_JSON.write_text(
                json.dumps(music, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
            )
    elif args.command == "variants":
        print(apply_variant_gains(), "événements mis à jour")
    elif args.command == "limit":
        for file in args.files:
            limit_peaks(file)
    else:
        for file in args.files:
            loop_crossfade(file, args.seconds)


if __name__ == "__main__":
    main()
