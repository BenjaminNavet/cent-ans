"""DA7a: real, freely licensed performances of Ars nova / Trecento repertoire (France, Italy).

DA4 (ADR 0060) only found MIDI realisations of Machaut, Solage, Landini and Binchois on
Wikimedia Commons, so they were demoted to ``fallback``. This lot adds *performed* recordings:

- a 1963 concert tape digitised by the Swedish Performing Arts Agency (Musikverket, Svenskt
  visarkiv) and published on Commons as public domain: the Studio der frühen Musik (Thomas
  Binkley) in Stockholm, 23 October 1963. The tape is not split into tracks; each piece is cut
  out by its start/end time (found with ffmpeg ``silencedetect``, matched in order against the
  concert programme on the file page);
- a keyboard performance of a piece of the Faenza codex (CC BY 4.0, Francesco Ariis).

Reproducible pipeline, run with::

    uv run --project tools python -m cent_ans_tools.ars_nova

Each source's licence is read again from the Commons API (``extmetadata.LicenseShortName``,
same check as ``era_music.fetch_wikimedia``) before download. Pieces are converted to OGG
Vorbis (quality 5, stereo 44.1 kHz), loudness-normalised to about -16 LUFS like the DA4 bank,
with short fades and a gentle high-pass against tape rumble. ``SOURCE.md`` in the destination
folder is rewritten from the manifest below. Downloads are cached under
``~/.cache/cent_ans/era_music/ars_nova``.
"""

from __future__ import annotations

import hashlib
import secrets
import urllib.parse
from dataclasses import dataclass
from pathlib import Path

from cent_ans_tools.era_music import (
    ACCEPTED_LICENSES,
    CACHE_DIR,
    REPO_DIR,
    _commons_imageinfo,
    _curl,
    _run_ffmpeg_atomic,
)

ARS_NOVA_DIR = REPO_DIR / "game" / "assets" / "third_party" / "music" / "ars_nova"
ARS_NOVA_CACHE = CACHE_DIR / "ars_nova"


@dataclass(frozen=True)
class CommonsSource:
    """One Commons audio file (possibly a whole concert tape)."""

    file_title: str
    cache_name: str
    performers: str
    recorded: str
    licence: str  # as read on the file page (extmetadata.LicenseShortName)
    licence_note: str
    # Use Commons' own Vorbis transcode of the file instead of the original: the 1960s tapes
    # are 96 kHz / 24-bit WAVs of ~0.5 GB whose download kept stalling; the transcode is made
    # by Commons from the same file (same licence) and is ample for a mono 1963 tape.
    transcoded: bool = False


STUDIO_1963 = CommonsSource(
    "File:Studio des frühen Musik I - SMV - MMG7 0023.wav",
    "studio1_tc.ogg",
    "Studio der frühen Musik (Andrea von Ramm, Nigel Rogers, Sterling Jones, Thomas Binkley)",
    "concert, Musikhistoriska museet, Stockholm, 23 octobre 1963",
    "Public domain",
    "Domaine public (modèle {{PD-old}} ; enregistrement non publié de 1963, droits voisins "
    "suédois échus), numérisé et versé par Musikverket / Svenskt visarkiv",
    transcoded=True,
)
FAENZA_ARIIS = CommonsSource(
    "File:Bel fiore dança (keyboard).ogg",
    "bel_fiore.ogg",
    "Francesco Ariis, clavier",
    "2018",
    "CC BY 4.0",
    "CC BY 4.0 (œuvre de l'interprète, {{self|cc-by-4.0}})",
)


@dataclass(frozen=True)
class ArsNovaTrack:
    """One piece written as ``<out_name>.ogg``, cut from ``source`` if start/end are set."""

    out_name: str
    work: str
    region: str  # france / italy / burgundy / england
    source: CommonsSource
    start: float | None = None
    end: float | None = None


# Studio der frühen Musik tape (edited listening copy, no announcements): ten pieces separated by
# ~15 s of digital silence, in the order of the concert programme on the file page. Bounds come
# from ``silencedetect=noise=-40dB:d=1.5``. Not kept: "Hie vor do wir kynder waren" (German).
ARS_NOVA_TRACKS: list[ArsNovaTrack] = [
    ArsNovaTrack(
        "jacopo_fenice_fu",
        "Jacopo da Bologna — madrigal « Fenice fu » (Trecento, milieu XIVe s.)",
        "italy",
        STUDIO_1963,
        814.4,
        932.0,
    ),
    ArsNovaTrack(
        "saltarello_trecento",
        "Saltarello, anonyme italien (ms. Londres Add. 29987, XIVe s.)",
        "italy",
        STUDIO_1963,
        741.6,
        799.3,
    ),
    ArsNovaTrack(
        "faenza_bel_fiore_danca",
        "« Bel fiore dança », codex de Faenza (Italie, début XVe s.), clavier",
        "italy",
        FAENZA_ARIIS,
    ),
    ArsNovaTrack(
        "onques_ne_fut",
        "« Onques ne fut », chanson anonyme française (XIVe s.)",
        "france",
        STUDIO_1963,
        946.7,
        1116.6,
    ),
    ArsNovaTrack(
        "souvent_souspire",
        "« Souvent souspire », chanson de trouvère anonyme (v. 1300)",
        "france",
        STUDIO_1963,
        443.1,
        566.0,
    ),
    ArsNovaTrack(
        "pierrekin_chancon_fas",
        "Pierrekin de la Coupele — « Chancon fas non pas villaine » (trouvère, XIIIe s.)",
        "france",
        STUDIO_1963,
        1.3,
        270.8,
    ),
    ArsNovaTrack(
        "he_robinet",
        "« Hé Robinet », chanson anonyme (v. 1450)",
        "france",
        STUDIO_1963,
        1487.7,
        1575.2,
    ),
    ArsNovaTrack(
        "binchois_adieu_mamour",
        "Gilles Binchois — rondeau « Adieu m'amour » (XVe s.)",
        "burgundy",
        STUDIO_1963,
        1131.4,
        1290.6,
    ),
    ArsNovaTrack(
        "dufay_adieu_mamour",
        "Guillaume Dufay — rondeau « Adieu m'amour » (XVe s.)",
        "burgundy",
        STUDIO_1963,
        1305.6,
        1469.5,
    ),
    ArsNovaTrack(
        "bryd_one_brere",
        "« Bryd one brere », chanson anglaise anonyme (XIVe s.)",
        "england",
        STUDIO_1963,
        285.2,
        428.5,
    ),
]


def fetch_source(source: CommonsSource) -> Path:
    """Re-check the licence on the Commons file page, then download (cached)."""
    info = _commons_imageinfo(source.file_title)
    licence = info.get("extmetadata", {}).get("LicenseShortName", {}).get("value", "")
    if licence not in ACCEPTED_LICENSES:
        raise RuntimeError(f"{source.file_title}: licence refusee ({licence!r})")
    if licence != source.licence:
        raise RuntimeError(
            f"{source.file_title}: licence changee ({licence!r}, attendu {source.licence!r})"
        )
    cached = ARS_NOVA_CACHE / source.cache_name
    if not cached.exists():
        url = info["url"].split("?", 1)[0]
        if source.transcoded:
            url = transcoded_url(url)
        _curl(url, cached)
    return cached


def transcoded_url(original_url: str) -> str:
    """Commons' Vorbis transcode of an uploaded audio file (``.../transcoded/a/ab/F/F.ogg``)."""
    prefix = "https://upload.wikimedia.org/wikipedia/commons/"
    if not original_url.startswith(prefix):
        raise ValueError(f"URL Commons inattendue : {original_url}")
    hashed_path = original_url[len(prefix) :]
    name = hashed_path.rsplit("/", 1)[-1]
    return f"{prefix}transcoded/{hashed_path}/{name}.ogg"


def convert_piece(src: Path, dst: Path, start: float | None, end: float | None) -> None:
    """Cut, fade, high-pass and loudness-normalise to OGG Vorbis q5 (~-16 LUFS)."""
    cmd = ["ffmpeg", "-y"]
    if start is not None:
        cmd += ["-ss", f"{start:.2f}"]
    if end is not None:
        cmd += ["-to", f"{end:.2f}"]
    cmd += ["-i", str(src)]
    duration = (end - start) if (start is not None and end is not None) else None
    filters = ["highpass=f=40", "afade=t=in:d=0.8"]
    if duration is not None:
        filters.append(f"afade=t=out:st={max(duration - 2.0, 0):.2f}:d=2")
    filters.append("loudnorm=I=-16:TP=-1.5:LRA=11")
    cmd += [
        "-af",
        ",".join(filters),
        "-ac",
        "2",
        "-ar",
        "44100",
        "-c:a",
        "vorbis",
        "-strict",
        "-2",
        "-q:a",
        "5",
        "-f",
        "ogg",  # the atomic helper writes to "<dst>.part": the muxer can't be guessed
        str(dst),
    ]
    _run_ffmpeg_atomic(cmd, dst)


def _commons_url(file_title: str) -> str:
    return "https://commons.wikimedia.org/wiki/" + urllib.parse.quote(
        file_title.replace(" ", "_"), safe=":()_,'-."
    )


def _timecode(seconds: float) -> str:
    minutes, secs = divmod(int(round(seconds)), 60)
    return f"{minutes}:{secs:02d}"


def write_source_md() -> None:
    """Rewrite ``ars_nova/SOURCE.md`` from ``ARS_NOVA_TRACKS``."""
    lines = [
        "# Ars nova et Trecento — vrais enregistrements libres (DA7a)",
        "",
        "Interprétations réelles (voix, vièle, luth, flûte, orgue), jamais de rendu MIDI, qui "
        "remplacent en tête de playlist les réalisations MIDI de DA4 (restées en repli dans "
        "`../wikimedia/`). Licence relue sur la page Commons de chaque fichier via l'API "
        "(`extmetadata.LicenseShortName`) : domaine public ou CC BY uniquement.",
        "",
        "- **Récupéré le** : 2026-09-26 (`tools/cent_ans_tools/ars_nova.py`).",
        "- **Traitement** : pièce découpée dans la bande de concert (bornes ci-dessous ; le "
        "fichier d'origine n'est pas découpé en pistes, pièces séparées par ~15 s de silence et "
        "identifiées dans l'ordre du programme de la page Commons), filtre passe-haut 40 Hz, "
        "fondus, normalisation `loudnorm` à -16 LUFS, OGG Vorbis q5. Aucune modification "
        "musicale. Pour la bande de 1963, source = transcodage Vorbis produit par Commons à "
        "partir du WAV d'origine (96 kHz / 24 bits, ~0,5 Go, téléchargement instable).",
        "- **À vérifier à l'oreille** : l'attribution titre ↔ extrait suit l'ordre du "
        "programme (10 pièces, 10 plages) ; aucune écoute humaine pendant DA7a.",
        "",
        "| Fichier | Œuvre | Interprètes | Enregistrement | Extrait | Licence | Région | Source |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for track in ARS_NOVA_TRACKS:
        source = track.source
        excerpt = (
            f"{_timecode(track.start)}–{_timecode(track.end)}"
            if track.start is not None and track.end is not None
            else "intégral"
        )
        lines.append(
            f"| `{track.out_name}.ogg` | {track.work} | {source.performers} | "
            f"{source.recorded} | {excerpt} | {source.licence} | {track.region} | "
            f"{_commons_url(source.file_title)} |"
        )
    lines += ["", "## Licences des sources", ""]
    for source in dict.fromkeys(track.source for track in ARS_NOVA_TRACKS):
        lines.append(f"- {_commons_url(source.file_title)} : {source.licence_note}.")
    lines += [
        "",
        "Attribution à afficher : « <Œuvre> », <Interprètes>, Wikimedia Commons, licence "
        "indiquée ci-dessus.",
        "",
    ]
    ARS_NOVA_DIR.mkdir(parents=True, exist_ok=True)
    (ARS_NOVA_DIR / "SOURCE.md").write_text("\n".join(lines), encoding="utf-8")


GODOT_UID_ALPHABET = (
    "abcdefghijklmnopqrstuvwxyz012345678"  # ResourceUID::id_to_text (base 35)
)


def godot_uid(rng: secrets.SystemRandom | None = None) -> str:
    """A fresh ``uid://...`` string in Godot's text format (random positive 63-bit id)."""
    value = (rng or secrets.SystemRandom()).getrandbits(63) or 1
    text = ""
    while value:
        value, digit = divmod(value, len(GODOT_UID_ALPHABET))
        text = GODOT_UID_ALPHABET[digit] + text
    return "uid://" + text


def write_import_file(ogg: Path) -> None:
    """Write the Godot ``.import`` sidecar of a music ``.ogg`` (same params as the DA4 bank).

    Godot names the imported file ``<name>-<md5 of the res:// path>``; an existing sidecar is
    kept so its uid stays stable.
    """
    sidecar = ogg.with_name(ogg.name + ".import")
    if sidecar.exists():
        return
    res_path = "res://" + ogg.relative_to(REPO_DIR / "game").as_posix()
    digest = hashlib.md5(res_path.encode("utf-8")).hexdigest()  # noqa: S324 - Godot's naming
    imported = f"res://.godot/imported/{ogg.name}-{digest}.oggvorbisstr"
    sidecar.write_text(
        "[remap]\n\n"
        'importer="oggvorbisstr"\n'
        'type="AudioStreamOggVorbis"\n'
        f'uid="{godot_uid()}"\n'
        f'path="{imported}"\n\n'
        "[deps]\n\n"
        f'source_file="{res_path}"\n'
        f'dest_files=["{imported}"]\n\n'
        "[params]\n\n"
        "loop=false\n"
        "loop_offset=0\n"
        "bpm=0\n"
        "beat_count=0\n"
        "bar_beats=4\n",
        encoding="utf-8",
    )


def main() -> None:
    """Download, cut, convert and credit every DA7a track (idempotent)."""
    failures: list[str] = []
    sources: dict[CommonsSource, Path] = {}
    for track in ARS_NOVA_TRACKS:
        dst = ARS_NOVA_DIR / f"{track.out_name}.ogg"
        if dst.exists():
            continue
        print(f"[ars_nova] {track.out_name}")
        try:
            if track.source not in sources:
                sources[track.source] = fetch_source(track.source)
            convert_piece(sources[track.source], dst, track.start, track.end)
        except Exception as exc:  # noqa: BLE001 - reported, not fatal for the whole run
            failures.append(f"{track.out_name}: {exc}")
            print(f"  echec: {exc}")
    for track in ARS_NOVA_TRACKS:
        ogg = ARS_NOVA_DIR / f"{track.out_name}.ogg"
        if ogg.exists():
            write_import_file(ogg)
    write_source_md()
    if failures:
        print("\nEchecs (relancer pour reessayer) :")
        for failure in failures:
            print(f"  - {failure}")
    else:
        print("Done.")


if __name__ == "__main__":
    main()
