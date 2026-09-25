"""DA4 sound bank: era music (Wikimedia Commons) and battle instrument layers (Freesound CC0).

Reproducible pipeline, run with::

    uv run --project tools python -m cent_ans_tools.era_music

1. **Wikimedia Commons tracks** (``WIKIMEDIA_TRACKS``): each entry names a Commons file page.
   The page's licence is read from the API (``extmetadata.LicenseShortName``) and checked
   again here: only Public domain, CC0, CC BY or CC BY-SA are accepted (no NC, no ND). The
   original file is downloaded, converted to MP3 128 kbit/s (matching the existing
   ``wikimedia/`` bank) and loudness-normalised to about -16 LUFS with ffmpeg's ``loudnorm``.
2. **Battle instrument layers** (``BATTLE_LAYER_SOURCES``): short Freesound recordings, CC0
   only (checked the same way as ``audio_bank.verify_licence``), converted to loopable OGG
   Vorbis layers under ``game/assets/audio/music/battle_layers/``.
3. ``SOURCE.md`` is rewritten in both destination folders from the manifests below, so the
   credited list always matches what is on disk.

Network access required (Wikimedia Commons API + freesound.org). Downloads are cached under
``~/.cache/cent_ans/era_music`` so re-runs are cheap.
"""

from __future__ import annotations

import json
import re
import subprocess
import urllib.parse
from dataclasses import dataclass
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parents[2]
WIKIMEDIA_DIR = REPO_DIR / "game" / "assets" / "third_party" / "music" / "wikimedia"
BATTLE_LAYERS_DIR = REPO_DIR / "game" / "assets" / "audio" / "music" / "battle_layers"
CACHE_DIR = Path.home() / ".cache" / "cent_ans" / "era_music"
COMMONS_API = "https://commons.wikimedia.org/w/api.php"
ACCEPTED_LICENSES = {
    "Public domain",
    "CC0",
    "CC BY 3.0",
    "CC BY 4.0",
    "CC BY-SA 2.5",
    "CC BY-SA 3.0",
    "CC BY-SA 4.0",
}


@dataclass(frozen=True)
class WikimediaTrack:
    """One Commons audio file to add to the era-music bank."""

    file_title: str  # "File:...ogg" on Commons
    out_name: str  # without extension; written as <out_name>.mp3
    work: str
    performers: str
    culture: str  # france / england / burgundy / iberia / italy
    context: str  # campaign / court / war


WIKIMEDIA_TRACKS: list[WikimediaTrack] = [
    WikimediaTrack(
        "File:Machaut Douce Dame Jolie.ogg",
        "machaut_douce_dame_jolie",
        "Guillaume de Machaut — « Douce Dame Jolie » (virelai, XIVe s.)",
        "Réalisation MIDI, Tetraktys",
        "france",
        "campaign",
    ),
    WikimediaTrack(
        "File:Guillaume de Machaut - Riches d'amour et mandians d'amie.ogg",
        "machaut_riches_damour",
        "Guillaume de Machaut — « Riches d'amour et mandians d'amie » (XIVe s.)",
        "Réalisation MIDI, Tetraktys",
        "france",
        "campaign",
    ),
    WikimediaTrack(
        "File:Solage - Fumeux fume par fumée.ogg",
        "solage_fumeux_fume",
        "Solage — « Fumeux fume par fumée » (Ars subtilior, fin XIVe s.)",
        "Réalisation MIDI, Tetraktys",
        "france",
        "court",
    ),
    WikimediaTrack(
        "File:Landini - Ecco la primavera.ogg",
        "landini_ecco_la_primavera",
        "Francesco Landini — Ballata « Ecco la primavera » (XIVe s.)",
        "Réalisation MIDI, Tetraktys",
        "italy",
        "campaign",
    ),
    WikimediaTrack(
        "File:Landini - Si dolce non sono.ogg",
        "landini_si_dolce_non_sono",
        "Francesco Landini — « Si dolce non sono » (XIVe s.)",
        "Réalisation MIDI, Tetraktys",
        "italy",
        "court",
    ),
    WikimediaTrack(
        "File:Agincourt carol - Deo gracias 01.wav",
        "agincourt_carol_deo_gracias",
        "« Deo gracias Anglia » (Agincourt Carol, anonyme, XVe s.)",
        "Réalisation instrumentale (anonyme, Commons)",
        "england",
        "war",
    ),
    WikimediaTrack(
        "File:Sumer Is Icumen In (13th century English round).ogg",
        "sumer_is_icumen_in",
        "« Sumer is Icumen In » (rota anglaise, XIIIe s.)",
        "Brandtnight2000 (Commons)",
        "england",
        "campaign",
    ),
    WikimediaTrack(
        "File:Triste plaisir.ogg",
        "binchois_triste_plaisir",
        "Gilles Binchois — « Triste plaisir » (chanson de cour, XVe s.)",
        "Réalisation MIDI, Tetraktys",
        "burgundy",
        "court",
    ),
    WikimediaTrack(
        "File:Dueil angoisseux.ogg",
        "binchois_dueil_angoisseux",
        "Gilles Binchois — « Dueil angoisseux » (XVe s.)",
        "Réalisation MIDI, Tetraktys",
        "burgundy",
        "campaign",
    ),
    WikimediaTrack(
        "File:Dufay Ave Regina.ogg",
        "dufay_ave_regina",
        "Guillaume Dufay — « Ave Regina caelorum » (motet, XVe s.)",
        "Enregistrement Commons (CC0)",
        "burgundy",
        "court",
    ),
    WikimediaTrack(
        "File:Fundación Joaquín Díaz - ATO 00722 04 - Cantigas de Santa María.ogg",
        "cantigas_santa_maria",
        "Cantigas de Santa María (Alphonse X, XIIIe s.), tradition orale castillane",
        "Fundación Joaquín Díaz",
        "iberia",
        "campaign",
    ),
]


@dataclass(frozen=True)
class BattleLayer:
    """One Freesound CC0 recording turned into a looping battle-music layer."""

    sound_id: int
    out_name: str  # written as <out_name>.ogg
    note: str
    layer: str  # drums / straight_trumpet / bagpipe_drone / shawm


BATTLE_LAYER_SOURCES: list[BattleLayer] = [
    BattleLayer(274223, "battle_drum", "war drum loop", "drums"),
    BattleLayer(350428, "straight_trumpet_fanfare", "trumpet fanfare (stinger/critical)", "straight_trumpet"),
    BattleLayer(260864, "bagpipe_drone", "mittelalter sackpfeife (bagpipe) drone", "bagpipe_drone"),
]

# The shawm layer comes from Wikimedia Commons (CC BY-SA 3.0), not Freesound: it is a clean
# instrument demo rather than a battle field recording.
SHAWM_WIKIMEDIA_FILE = "File:Schalmei sound.ogg"
SHAWM_OUT_NAME = "shawm_note"


def _curl(url: str, out: Path | None = None) -> str:
    args = ["curl", "-sL", "-m", "60", "-A", "Mozilla/5.0"]
    if out is not None:
        out.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run([*args, "-o", str(out), url], check=True)
        return ""
    result = subprocess.run([*args, url], capture_output=True, text=True, check=True)
    return result.stdout


def _commons_imageinfo(file_title: str) -> dict:
    query = urllib.parse.urlencode(
        {
            "action": "query",
            "titles": file_title,
            "prop": "imageinfo",
            "iiprop": "extmetadata|url",
            "format": "json",
        }
    )
    data = json.loads(_curl(f"{COMMONS_API}?{query}"))
    pages = data["query"]["pages"]
    page = next(iter(pages.values()))
    if "imageinfo" not in page:
        raise RuntimeError(f"Commons: {file_title} introuvable ou pas de fichier")
    return page["imageinfo"][0]


def fetch_wikimedia(track: WikimediaTrack) -> Path:
    """Download and licence-check one Commons file (cached); returns the local original."""
    info = _commons_imageinfo(track.file_title)
    licence = info.get("extmetadata", {}).get("LicenseShortName", {}).get("value", "")
    if licence not in ACCEPTED_LICENSES:
        raise RuntimeError(f"{track.file_title}: licence refusee ({licence!r})")
    url = info["url"]
    suffix = Path(urllib.parse.urlparse(url).path).suffix or ".ogg"
    cached = CACHE_DIR / "wikimedia" / f"{track.out_name}{suffix}"
    if not cached.exists():
        _curl(url, cached)
    return cached


def fetch_freesound(sound_id: int) -> Path:
    """Download one Freesound preview after re-checking its licence page is CC0 only."""
    cache = CACHE_DIR / "freesound"
    page_path = cache / f"{sound_id}.html"
    if not page_path.exists():
        page_path.parent.mkdir(parents=True, exist_ok=True)
        page_path.write_text(_curl(f"https://freesound.org/s/{sound_id}/"), encoding="utf-8")
    page = page_path.read_text(encoding="utf-8")
    deeds = set(re.findall(r"creativecommons\.org/(?:licenses|publicdomain)/[a-z0-9.\-/]*", page))
    if not deeds or any("publicdomain/zero" not in deed for deed in deeds):
        raise RuntimeError(f"Freesound {sound_id}: licence n'est pas exclusivement CC0")
    preview_match = re.search(rf"https://cdn\.freesound\.org/previews/\d+/{sound_id}_\d+-hq\.mp3", page)
    if preview_match is None:
        raise RuntimeError(f"Freesound {sound_id}: aperçu introuvable")
    audio_path = cache / f"{sound_id}.mp3"
    if not audio_path.exists():
        _curl(preview_match.group(0), audio_path)
    return audio_path


def convert_music(src: Path, dst: Path, *, max_seconds: float | None = None) -> None:
    """Convert to MP3 128 kbit/s, ~-16 LUFS integrated loudness (ffmpeg loudnorm)."""
    dst.parent.mkdir(parents=True, exist_ok=True)
    cmd = ["ffmpeg", "-y", "-i", str(src)]
    if max_seconds is not None:
        cmd += ["-t", str(max_seconds)]
    cmd += ["-af", "loudnorm=I=-16:TP=-1.5:LRA=11", "-ar", "44100", "-b:a", "128k", str(dst)]
    subprocess.run(cmd, check=True, capture_output=True)


def convert_layer(src: Path, dst: Path, *, loop_seconds: float = 8.0) -> None:
    """Convert to a loopable stereo OGG Vorbis layer, trimmed and loudness-normalised."""
    dst.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        "ffmpeg",
        "-y",
        "-i",
        str(src),
        "-t",
        str(loop_seconds),
        "-af",
        "loudnorm=I=-16:TP=-1.5:LRA=11,afade=t=in:d=0.3,afade=t=out:st=" + str(loop_seconds - 0.5) + ":d=0.5",
        "-ac",
        "2",
        "-ar",
        "44100",
        "-c:a",
        "vorbis",
        "-strict",
        "-2",
        "-q:a",
        "4",
        str(dst),
    ]
    subprocess.run(cmd, check=True, capture_output=True)


def write_wikimedia_source_md() -> None:
    lines = [
        "# Wikimedia Commons — musique médiévale et Renaissance (instruments d'époque)",
        "",
        "Enregistrements sous licence libre, choisis pour leurs instruments d'époque (orgue "
        "positif, flûte et tambourin, luth, voix, viole de gambe) : pas de violons ni "
        "d'orchestre à cordes baroque.",
        "Licences compatibles avec les assets du projet (CC BY-SA 4.0, voir `LICENSE-ASSETS.md`).",
        "",
        "- **Récupéré le** : 2026-09-25, via l'API Commons (licence lue dans "
        "`extmetadata.LicenseShortName`)",
        "- **Traitement** : fichier d'origine réencodé en MP3 128 kbit/s (ffmpeg/libmp3lame), "
        "normalisé à -16 LUFS (`loudnorm`). Aucune modification musicale.",
        "- **Retirés le 2026-09-25** : six mouvements de Vivaldi et la Badinerie de Bach (cordes "
        "baroques, trop « Grand Siècle » pour la guerre de Cent Ans).",
        "",
        "| Fichier | Œuvre | Interprètes | Licence | Culture | Contexte | Source |",
        "|---|---|---|---|---|---|---|",
        "| `estampie_retrove_robertsbridge.mp3` | Estampie « Retrove », Robertsbridge Codex "
        "(Angleterre, début XIVe s.), orgue | Metzner (concert, v. 1990) | CC BY-SA 3.0 | "
        "england | war | https://commons.wikimedia.org/wiki/File:Estampie_Retrove_Robertsbridge.ogg |",
        "| `chominciamento_di_gioia.mp3` | Istampitta « Chominciamento di gioia », ms. Londres "
        "Add. 29987 (Italie, XIVe s.) | Ririkuku | CC BY-SA 4.0 | italy | campaign | "
        "https://commons.wikimedia.org/wiki/File:Chominciamento_de_Gioia1.ogg |",
        "| `dufay_se_la_face_ay_pale.mp3` | Guillaume Dufay — « Se la face ay pale » (v. 1430) "
        "| Ensemble Asteria | CC BY-SA 2.5 | burgundy | court | "
        "https://commons.wikimedia.org/wiki/File:Guillaume_Dufay_-_Se_La_Face_Ay_Pale.ogg |",
        "| `folia_ahigal_tamborilero.mp3` | Folía traditionnelle d'Ahigal (Cáceres), flûte à "
        "trois trous et tambourin | Loreto Galindo, tamborilero (Fundación Joaquín Díaz, 1988) "
        "| CC BY-SA 3.0 | iberia | campaign | "
        "https://commons.wikimedia.org/wiki/File:Fundaci%C3%B3n_Joaqu%C3%ADn_D%C3%ADaz_-_ATO_00330_01_-_Fol%C3%ADa.ogg |",
        "| `ortiz_recercada_primera.mp3` | Diego Ortiz — Recercada primera sobre tenores "
        "italianos, *Trattado de Glosas* (1553) | Phillip W. Serna, viole de gambe | "
        "CC BY-SA 4.0 | iberia | court | "
        "https://commons.wikimedia.org/wiki/File:Diego_Ortiz_(1510-1570)_-_Recercada_primera_sobre_tenores_italianos_from_Trattado_de_Glosas,_Libro_Secundo_(1553).ogg |",
        "| `ortiz_recercada_segunda.mp3` | Diego Ortiz — Recercada segunda sobre tenores "
        "italianos, *Trattado de Glosas* (1553) | Phillip W. Serna, viole de gambe | "
        "CC BY-SA 4.0 | iberia | court | "
        "https://commons.wikimedia.org/wiki/File:Diego_Ortiz_(1510-1570)_-_Recercada_segunda_sobre_tenores_italianos_from_Trattado_de_Glosas_(1553).ogg |",
    ]
    for track in WIKIMEDIA_TRACKS:
        info = _commons_imageinfo(track.file_title)
        licence = info.get("extmetadata", {}).get("LicenseShortName", {}).get("value", "")
        url = "https://commons.wikimedia.org/wiki/" + track.file_title.replace(" ", "_")
        lines.append(
            f"| `{track.out_name}.mp3` | {track.work} | {track.performers} | {licence} | "
            f"{track.culture} | {track.context} | {url} |"
        )
    lines.append(
        f"| `{SHAWM_OUT_NAME}.mp3` | Démonstration de chalemie (Schalmei) | Ajta (Commons) | "
        "CC BY-SA 3.0 | (couche de bataille) | battle | "
        "https://commons.wikimedia.org/wiki/File:Schalmei_sound.ogg |"
    )
    lines.append("")
    lines.append(
        "Attribution à afficher : « <Œuvre> », <Interprètes>, Wikimedia Commons, licence "
        "indiquée ci-dessus."
    )
    (WIKIMEDIA_DIR / "SOURCE.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_battle_layers_source_md() -> None:
    lines = [
        "# Couches instrumentales de bataille (DA4)",
        "",
        "Enregistrements courts, tirés en boucle, joués en fondu par "
        "`BattleMusicDirector` (`data/audio/battle_layers.json`) au-dessus du morceau de guerre "
        "de base : tambour, trompette droite (fanfare), bourdon de cornemuse. La chalemie vient "
        "de Wikimedia Commons (voir `game/assets/third_party/music/wikimedia/SOURCE.md`).",
        "",
        "- **Traitement** : aperçu Freesound (CC0, licence vérifiée page par page), coupé à 8 s, "
        "fondu d'entrée/sortie, normalisé à -16 LUFS, réencodé en OGG Vorbis mono "
        "(`tools/cent_ans_tools/era_music.py`).",
        "",
        "| Fichier | Son Freesound | Licence | Couche | Source |",
        "|---|---|---|---|---|",
    ]
    for layer in BATTLE_LAYER_SOURCES:
        lines.append(
            f"| `{layer.out_name}.ogg` | #{layer.sound_id} — {layer.note} | CC0 1.0 | "
            f"{layer.layer} | https://freesound.org/s/{layer.sound_id}/ |"
        )
    lines.append(
        f"| `{SHAWM_OUT_NAME}.ogg` | Schalmei sound (Ajta, Wikimedia Commons) | CC BY-SA 3.0 | "
        "shawm | https://commons.wikimedia.org/wiki/File:Schalmei_sound.ogg |"
    )
    lines.append("")
    (BATTLE_LAYERS_DIR / "SOURCE.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    failures: list[str] = []
    for track in WIKIMEDIA_TRACKS:
        dst = WIKIMEDIA_DIR / f"{track.out_name}.mp3"
        if dst.exists():
            continue
        print(f"[wikimedia] {track.file_title}")
        try:
            src = fetch_wikimedia(track)
            convert_music(src, dst)
        except Exception as exc:  # noqa: BLE001 - reported, not fatal for the whole run
            failures.append(f"{track.file_title}: {exc}")
            print(f"  echec: {exc}")

    shawm_track = WikimediaTrack(SHAWM_WIKIMEDIA_FILE, SHAWM_OUT_NAME, "", "", "", "battle")
    shawm_dst = WIKIMEDIA_DIR / f"{SHAWM_OUT_NAME}.mp3"
    if not shawm_dst.exists():
        print(f"[wikimedia] {SHAWM_WIKIMEDIA_FILE}")
        try:
            shawm_src = fetch_wikimedia(shawm_track)
            convert_music(shawm_src, shawm_dst, max_seconds=6.0)
            convert_layer(shawm_src, BATTLE_LAYERS_DIR / f"{SHAWM_OUT_NAME}.ogg", loop_seconds=6.0)
        except Exception as exc:  # noqa: BLE001
            failures.append(f"{SHAWM_WIKIMEDIA_FILE}: {exc}")
            print(f"  echec: {exc}")

    for layer in BATTLE_LAYER_SOURCES:
        dst = BATTLE_LAYERS_DIR / f"{layer.out_name}.ogg"
        if dst.exists():
            continue
        print(f"[freesound] {layer.sound_id} ({layer.layer})")
        try:
            src = fetch_freesound(layer.sound_id)
            convert_layer(src, dst)
        except Exception as exc:  # noqa: BLE001
            failures.append(f"freesound {layer.sound_id}: {exc}")
            print(f"  echec: {exc}")

    if all((WIKIMEDIA_DIR / f"{t.out_name}.mp3").exists() for t in WIKIMEDIA_TRACKS):
        write_wikimedia_source_md()
    if all((BATTLE_LAYERS_DIR / f"{layer.out_name}.ogg").exists() for layer in BATTLE_LAYER_SOURCES) and shawm_dst.exists():
        write_battle_layers_source_md()

    if failures:
        print("\nEchecs (relancer le script pour reessayer, cache/telechargements repris) :")
        for failure in failures:
            print(f"  - {failure}")
    else:
        print("Done.")


if __name__ == "__main__":
    main()
