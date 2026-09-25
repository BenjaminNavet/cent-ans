"""UB1 interface sounds: short clips cut from the game's own free audio files.

Reproducible, offline pipeline (needs ``ffmpeg`` on the ``PATH``)::

    uv run --project tools python -m cent_ans_tools.ui_sounds

Every clip of ``game/assets/audio/ui/`` is a recipe (an ffmpeg filter graph) over files
already in the repository: the AU1 bank (``battle/``, CC0 Freesound recordings, see
``game/assets/audio/SOURCE.md``) and the procedural effects of ``sfx/`` (the project's own
synthesis). No new source, no network. The clips are 16-bit mono WAV at 44.1 kHz (the
ffmpeg build at hand has no libvorbis; the files are tiny). ``ui/SOURCE.md`` lists, per
clip, its source files and processing.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parents[2]
AUDIO_DIR = REPO_DIR / "game" / "assets" / "audio"
OUT_DIR = AUDIO_DIR / "ui"

# Leading silence removed from one-shot sources before cutting.
TRIM_START = "silenceremove=start_periods=1:start_threshold=-40dB"


@dataclass(frozen=True)
class Clip:
    """One interface clip: sources (relative to ``game/assets/audio``) and a filter graph."""

    name: str
    description: str
    sources: tuple[str, ...]
    graph: str
    note: str


CLIPS: tuple[Clip, ...] = (
    Clip(
        "order",
        "Ordre donné : un coup de tambour sec",
        ("battle/drum_1.ogg",),
        f"[0:a]{TRIM_START},atrim=0:0.45,afade=t=out:st=0.18:d=0.27,volume=0.9[out]",
        "first drum hit, 0.45 s, fade-out",
    ),
    Clip(
        "order_refused",
        "Ordre refusé : choc sourd et grave",
        ("battle/shield_bash_1.ogg",),
        f"[0:a]{TRIM_START},asetrate=44100*0.7,aresample=44100,lowpass=f=900,"
        "atrim=0:0.4,afade=t=out:st=0.15:d=0.25,volume=1.2[out]",
        "shield bash pitched down (×0.7), low-passed at 900 Hz, 0.4 s",
    ),
    Clip(
        "alert",
        "Alerte : un coup de cloche",
        ("battle/bell_toll_1.ogg",),
        f"[0:a]{TRIM_START},atrim=0:1.8,afade=t=out:st=0.7:d=1.1,volume=0.8[out]",
        "first bell stroke, 1.8 s, long fade-out",
    ),
    Clip(
        "letter",
        "Lettre reçue : parchemin déplié puis sceau de cire",
        ("sfx/page_turn.ogg", "battle/shield_bash_1.ogg"),
        "[0:a]asetrate=44100*0.92,aresample=44100[page];"
        f"[1:a]{TRIM_START},lowpass=f=600,atrim=0:0.18,afade=t=out:st=0.05:d=0.13,"
        "volume=0.5,adelay=430[seal];"
        "[page][seal]amix=inputs=2:normalize=0,atrim=0:0.8[out]",
        "page turn slowed (×0.92) + muffled shield tap as the wax seal, 0.8 s",
    ),
    Clip(
        "recruit",
        "Recrutement : roulement de tambour de levée",
        ("sfx/march_drum.ogg",),
        "[0:a]atrim=0:1.1,afade=t=out:st=0.7:d=0.4[out]",
        "march drum, 1.1 s, fade-out",
    ),
    Clip(
        "build",
        "Construction : deux coups de maillet sur le bois",
        ("battle/ram_hit_1.ogg",),
        f"[0:a]{TRIM_START},asetrate=44100*1.9,aresample=44100,atrim=0:0.3,"
        "afade=t=out:st=0.08:d=0.22,asplit[a][b];[b]adelay=240,volume=0.8[b2];"
        "[a][b2]amix=inputs=2:normalize=0,atrim=0:0.6[out]",
        "battering-ram hit pitched up (×1.9) to a mallet, twice, 0.6 s",
    ),
    Clip(
        "army_select",
        "Sélection d'une armée : piétinement de la troupe",
        ("battle/march_bed.ogg",),
        "[0:a]atrim=start=3:duration=1.0,asetpts=PTS-STARTPTS,"
        "afade=t=in:d=0.12,afade=t=out:st=0.55:d=0.45,volume=1.4[out]",
        "one second of marching feet, fade-in and fade-out",
    ),
    Clip(
        "card",
        "Clic sur une carte d'unité : tintement de métal",
        ("battle/sword_clash_1.ogg",),
        f"[0:a]{TRIM_START},highpass=f=900,atrim=0:0.14,afade=t=out:st=0.03:d=0.11,volume=0.45[out]",
        "attack of a sword clash, high-passed at 900 Hz, 0.14 s",
    ),
)


def build_clip(clip: Clip, out_dir: Path) -> Path:
    """Renders ``clip`` to ``<out_dir>/<name>.wav`` with ffmpeg."""
    out = out_dir / f"{clip.name}.wav"
    command = ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y"]
    for source in clip.sources:
        command += ["-i", str(AUDIO_DIR / source)]
    command += [
        "-filter_complex",
        clip.graph,
        "-map",
        "[out]",
        "-ac",
        "1",
        "-ar",
        "44100",
        "-c:a",
        "pcm_s16le",
        str(out),
    ]
    subprocess.run(command, check=True)
    return out


def source_table() -> str:
    """French ``SOURCE.md`` for the ``ui/`` folder."""
    lines = [
        "# Sons d'interface (UB1)",
        "",
        "Découpés hors ligne par `tools/cent_ans_tools/ui_sounds.py` dans des fichiers déjà présents :",
        "la banque AU1 (`battle/`, enregistrements Freesound CC0, voir `../SOURCE.md`) et les effets",
        "procéduraux du projet (`sfx/`). Licence : CC0 pour les dérivés des sons Freesound, celle du",
        "projet pour les dérivés de `sfx/`. WAV mono 16 bits, 44,1 kHz.",
        "",
        "| Fichier | Rôle | Sources | Traitement |",
        "|---|---|---|---|",
    ]
    for clip in CLIPS:
        sources = "<br>".join(f"`{source}`" for source in clip.sources)
        lines.append(
            f"| `ui/{clip.name}.wav` | {clip.description} | {sources} | {clip.note} |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    """Builds every clip (or the ones named) and rewrites ``ui/SOURCE.md``."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("clips", nargs="*", help="clip names (default: all)")
    args = parser.parse_args()
    if shutil.which("ffmpeg") is None:
        raise SystemExit("ffmpeg is required")
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for clip in CLIPS:
        if args.clips and clip.name not in args.clips:
            continue
        print(build_clip(clip, OUT_DIR).relative_to(REPO_DIR))
    (OUT_DIR / "SOURCE.md").write_text(source_table(), encoding="utf-8")


if __name__ == "__main__":
    main()
