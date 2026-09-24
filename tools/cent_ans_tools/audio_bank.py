"""AU1 sound bank: free (CC0) recordings from Freesound, cut, mixed and encoded to OGG.

Reproducible pipeline, run with::

    uv run --project tools --with soundfile python -m cent_ans_tools.audio_bank

1. **Sources** (``SOURCES``): Freesound sounds chosen with ``freesound_search`` among
   the "Creative Commons 0" results. Each sound page is fetched and its licence is
   checked again, page by page (``verify_licence``): the page must link the CC0 deed
   and no other Creative Commons licence, otherwise the build stops. The public
   high-quality preview (MP3 128 kbit/s, same CC0 licence as the original; the
   original file needs a logged-in account) is downloaded to a local cache.
2. **Clips** (``CLIPS``): every game file is a recipe over the sources — cut on the
   loudest onsets (one-shots), loudest window turned into a seamless loop
   (beds and ambiences), or a mix of several CC0 sounds (war cries, trebuchet,
   town, distant battle...). Recipes are deterministic (fixed seeds).
3. **Encoding**: Ogg Vorbis through libsndfile (``soundfile``), mono for 3D sounds,
   stereo for 2D ambiences. ``SOURCE.md`` (per clip: sources, authors, URLs,
   licence, processing) is rewritten next to the files.

The cache lives in ``~/.cache/cent_ans/freesound`` (outside the repository).
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import re
import subprocess
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from scipy import signal

REPO_DIR = Path(__file__).resolve().parents[2]
AUDIO_DIR = REPO_DIR / "game" / "assets" / "audio"
CACHE_DIR = Path.home() / ".cache" / "cent_ans" / "freesound"
SAMPLE_RATE = 44100
CC0_DEED = "creativecommons.org/publicdomain/zero/1.0"
PAGE_URL = "https://freesound.org/s/{id}/"


# --- Sources ------------------------------------------------------------------------

# Freesound id -> short note on why it was chosen (the author and title come from the page).
SOURCES: dict[int, str] = {
    471095: "sword clash",
    364531: "sword clash (high quality)",
    326868: "sword clash",
    440069: "sword block combo",
    370203: "shield guard",
    182112: "shield / sword hits",
    205938: "arrow impact",
    521552: "arrow impact",
    534956: "arrow into wood",
    708223: "arrow hit",
    394004: "arrows fly by",
    675821: "five arrows whoosh by",
    789389: "arrow flyby",
    384910: "arrow flying",
    263675: "bow release",
    394179: "longbow release",
    384918: "bow release",
    384919: "crossbow firing",
    621352: "male war cry",
    866009: "battle cry scream",
    563011: "people screaming when charging into battle",
    577032: "male death sound",
    221544: "wounded man scream",
    610998: "male pain grunts",
    255322: "groans and screams",
    384401: "crowd / mob noise, voices only (Henry VI)",
    325548: "man screaming",
    149024: "horse whinny",
    347036: "horse whinny",
    269571: "horse neigh",
    437110: "horse whinny",
    539956: "war horn",
    175946: "horn",
    512490: "distant war horn",
    459876: "warrior tom drum",
    459875: "warrior bass drum",
    454855: "tolling bell",
    383192: "church bell",
    675970: "battering ram hits castle door",
    231438: "hemp rope creaks",
    479922: "catapult launch",
    513694: "heavy stone impact",
    703247: "big falling debris",
    703248: "falling debris",
    567249: "bricks / stones falling",
    389303: "rockfall",
    712918: "building collapse",
    187767: "cannon shot",
    404166: "background cannon shot",
    399656: "thunder blast",
    652690: "thunder strike",
    376646: "vikings in battle (swords, shields, men yelling)",
    175950: "sword battle",
    480675: "march on gravel road",
    527430: "six horses gallop",
    636178: "fire crackling, spitting, roaring",
    760241: "medium wind, constant",
    185070: "howling wind",
    157487: "rain, near, smooth",
    534910: "ocean waves",
    514550: "countryside ambience, spring",
    522299: "crickets at night",
    474342: "forest birds",
    424790: "crowded street at a medieval market",
    444900: "crowd murmuring",
}


@dataclass
class SourceInfo:
    """Metadata of a verified Freesound sound."""

    sound_id: int
    user: str
    title: str
    url: str
    preview: str
    licence: str = "CC0 1.0"


def _curl(url: str, output: Path | None = None) -> str:
    command = ["curl", "-sL", "--fail", "-m", "120", "-A", "Mozilla/5.0", url]
    if output is not None:
        command += ["-o", str(output)]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(f"download failed ({result.returncode}): {url}")
    return result.stdout


def verify_licence(page: str) -> bool:
    """True if the sound page links the CC0 deed and no other Creative Commons licence."""
    deeds = set(
        re.findall(r"creativecommons\.org/(?:licenses|publicdomain)/[a-z\-]+", page)
    )
    return deeds == {"creativecommons.org/publicdomain/zero"}


def parse_page(sound_id: int, page: str) -> SourceInfo:
    """Author, title and preview URL of a sound page."""
    title_match = re.search(r"<title>Freesound - (.*) by ([^<]*)</title>", page, re.S)
    preview_match = re.search(
        rf"https://cdn\.freesound\.org/previews/\d+/{sound_id}_\d+-hq\.mp3", page
    )
    url_match = re.search(rf'href="(/people/[^"/]+/sounds/{sound_id}/)"', page)
    if title_match is None or preview_match is None:
        raise RuntimeError(f"cannot parse Freesound page {sound_id}")
    user = html.unescape(title_match.group(2).strip())
    path = url_match.group(1) if url_match else f"/s/{sound_id}/"
    return SourceInfo(
        sound_id=sound_id,
        user=user,
        title=html.unescape(title_match.group(1).strip()),
        url="https://freesound.org" + path,
        preview=preview_match.group(0),
    )


def fetch_source(sound_id: int) -> tuple[SourceInfo, Path]:
    """Verify the licence of one sound and download its preview (cached)."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    page_path = CACHE_DIR / f"{sound_id}.html"
    if not page_path.exists():
        page_path.write_text(_curl(PAGE_URL.format(id=sound_id)), encoding="utf-8")
    page = page_path.read_text(encoding="utf-8")
    if not verify_licence(page):
        raise RuntimeError(f"Freesound {sound_id}: licence is not CC0, refusing it")
    info = parse_page(sound_id, page)
    audio_path = CACHE_DIR / f"{sound_id}.mp3"
    if not audio_path.exists():
        _curl(info.preview, audio_path)
    return info, audio_path


# --- Signal helpers -----------------------------------------------------------------


def load(path: Path) -> np.ndarray:
    """Decode a file to float32 stereo (n, 2) at 44.1 kHz."""
    import soundfile

    data, rate = soundfile.read(str(path), dtype="float32", always_2d=True)
    if data.shape[1] == 1:
        data = np.repeat(data, 2, axis=1)
    data = data[:, :2]
    if rate != SAMPLE_RATE:
        count = int(round(len(data) * SAMPLE_RATE / rate))
        data = signal.resample(data, count, axis=0).astype(np.float32)
    return data


def mono(x: np.ndarray) -> np.ndarray:
    """Average the channels (a 1-D array is returned as is)."""
    return x if x.ndim == 1 else x.mean(axis=1)


def stereo(x: np.ndarray) -> np.ndarray:
    """Duplicate a mono signal on two channels."""
    return np.stack([x, x], axis=1) if x.ndim == 1 else x


def seconds(x: np.ndarray) -> float:
    """Duration in seconds."""
    return len(x) / SAMPLE_RATE


def cut(x: np.ndarray, start: float, length: float | None = None) -> np.ndarray:
    """Samples from ``start`` (s) for ``length`` s (to the end if None)."""
    begin = int(start * SAMPLE_RATE)
    end = len(x) if length is None else begin + int(length * SAMPLE_RATE)
    return x[begin:end].copy()


def envelope(x: np.ndarray, window: float = 0.02) -> np.ndarray:
    """RMS envelope, one value per sample (moving window)."""
    size = max(1, int(window * SAMPLE_RATE))
    power = mono(x) ** 2
    return np.sqrt(np.convolve(power, np.ones(size) / size, mode="same"))


def trim(x: np.ndarray, threshold_db: float = -42.0) -> np.ndarray:
    """Drop leading and trailing samples quieter than ``threshold_db`` below the peak."""
    env = envelope(x, 0.01)
    if env.max() <= 0:
        return x
    loud = np.nonzero(env > env.max() * 10 ** (threshold_db / 20))[0]
    start = max(0, loud[0] - int(0.005 * SAMPLE_RATE))
    return x[start : loud[-1] + 1].copy()


def onsets(x: np.ndarray, count: int, min_gap: float = 0.3) -> list[int]:
    """Sample indices of the ``count`` strongest attacks, in time order."""
    env = envelope(x, 0.01)
    rise = np.maximum(np.diff(env, prepend=env[0]), 0)
    rise = np.convolve(rise, np.ones(64), mode="same")
    order = np.argsort(rise)[::-1]
    gap = int(min_gap * SAMPLE_RATE)
    chosen: list[int] = []
    for index in order:
        if all(abs(int(index) - other) > gap for other in chosen):
            chosen.append(int(index))
        if len(chosen) >= count:
            break
    return sorted(chosen)


def hit(
    x: np.ndarray, pick: int = 0, length: float = 1.0, count: int = 6
) -> np.ndarray:
    """One attack: the ``pick``-th strongest onset (time order), ``length`` s long."""
    marks = onsets(x, count)
    start = marks[min(pick, len(marks) - 1)]
    begin = max(0, start - int(0.012 * SAMPLE_RATE))
    segment = x[begin : begin + int(length * SAMPLE_RATE)].copy()
    return fade(trim(segment, -50.0), 0.003, min(0.25, seconds(segment) * 0.35))


def loudest_window(x: np.ndarray, length: float) -> np.ndarray:
    """The ``length`` s window with the highest energy (the whole signal if shorter)."""
    size = int(length * SAMPLE_RATE)
    if len(x) <= size:
        return x.copy()
    power = np.cumsum(np.concatenate([[0.0], mono(x).astype(np.float64) ** 2]))
    step = SAMPLE_RATE // 10
    starts = np.arange(0, len(x) - size, step)
    energy = power[starts + size] - power[starts]
    best = int(starts[int(np.argmax(energy))])
    return x[best : best + size].copy()


def fade(x: np.ndarray, fade_in: float, fade_out: float) -> np.ndarray:
    """Linear fade in and out (seconds)."""
    y = x.copy()
    n_in = min(len(y), int(fade_in * SAMPLE_RATE))
    n_out = min(len(y), int(fade_out * SAMPLE_RATE))
    shape = (-1, 1) if y.ndim == 2 else (-1,)
    if n_in:
        y[:n_in] *= np.linspace(0, 1, n_in).reshape(shape)
    if n_out:
        y[len(y) - n_out :] *= np.linspace(1, 0, n_out).reshape(shape)
    return y


def loopify(x: np.ndarray, crossfade: float = 1.5) -> np.ndarray:
    """Seamless loop: the tail is cross-faded (equal power) into the head."""
    n = int(crossfade * SAMPLE_RATE)
    n = min(n, len(x) // 3)
    body = x[: len(x) - n].copy()
    ramp = np.linspace(0, np.pi / 2, n)
    shape = (-1, 1) if x.ndim == 2 else (-1,)
    fade_in = np.sin(ramp).reshape(shape)
    fade_out = np.cos(ramp).reshape(shape)
    body[:n] = x[:n] * fade_in + x[len(x) - n :] * fade_out
    return body


def filtered(
    x: np.ndarray, kind: str, cutoff: float | list[float], order: int = 2
) -> np.ndarray:
    """Butterworth filter (``low``, ``high`` or ``band``) along time."""
    sos = signal.butter(order, cutoff, kind, fs=SAMPLE_RATE, output="sos")
    return signal.sosfilt(sos, x, axis=0).astype(np.float32)


def repitch(x: np.ndarray, factor: float) -> np.ndarray:
    """Resample by ``factor`` (> 1 = higher and shorter)."""
    count = max(1, int(len(x) / factor))
    positions = np.linspace(0, len(x) - 1, count)
    if x.ndim == 1:
        return np.interp(positions, np.arange(len(x)), x).astype(np.float32)
    return np.stack(
        [np.interp(positions, np.arange(len(x)), x[:, c]) for c in range(x.shape[1])],
        axis=1,
    ).astype(np.float32)


def mix(
    parts: list[tuple[np.ndarray, float, float]], length: float | None = None
) -> np.ndarray:
    """Sum of (signal, offset s, gain) ; mono unless a part is stereo."""
    is_stereo = any(part.ndim == 2 for part, _, _ in parts)
    end = max(int(offset * SAMPLE_RATE) + len(part) for part, offset, _ in parts)
    if length is not None:
        end = int(length * SAMPLE_RATE)
    out = np.zeros((end, 2) if is_stereo else end, dtype=np.float32)
    for part, offset, gain in parts:
        data = stereo(part) if is_stereo else part
        begin = int(offset * SAMPLE_RATE)
        if begin >= end:
            continue
        span = min(len(data), end - begin)
        out[begin : begin + span] += data[:span] * gain
    return out


def pan(x: np.ndarray, position: float) -> np.ndarray:
    """Equal-power pan of a mono signal (-1 left, +1 right)."""
    angle = (position + 1) * np.pi / 4
    return np.stack([x * np.cos(angle), x * np.sin(angle)], axis=1).astype(np.float32)


def peak_normalize(x: np.ndarray, peak_db: float = -1.0) -> np.ndarray:
    """Scale so the peak sits at ``peak_db`` dBFS."""
    peak = float(np.max(np.abs(x)))
    return x if peak <= 0 else (x * (10 ** (peak_db / 20) / peak)).astype(np.float32)


def rms_normalize(x: np.ndarray, rms_db: float, peak_db: float = -1.0) -> np.ndarray:
    """Scale to an RMS level (dBFS), then soft-clip the peaks under ``peak_db``."""
    rms = float(np.sqrt(np.mean(np.square(x))))
    if rms <= 0:
        return x
    y = x * (10 ** (rms_db / 20) / rms)
    ceiling = 10 ** (peak_db / 20)
    return (np.tanh(y / ceiling) * ceiling).astype(np.float32)


# --- Clips --------------------------------------------------------------------------


Sources = Callable[[int], np.ndarray]


@dataclass
class Clip:
    """One game file: ``recipe(sources)`` renders it from the cached sources."""

    name: str
    sources: list[int]
    recipe: Callable[[Sources], np.ndarray]
    loop: bool = False
    channels: int = 1
    note: str = ""
    level: str = "peak"  # "peak" (one-shots) or "rms" (loops)
    extra: dict = field(default_factory=dict)


def _hit(sound: int, pick: int = 0, length: float = 1.0, count: int = 6) -> Callable:
    return lambda get: hit(mono(get(sound)), pick, length, count)


def _take(
    sound: int, start: float = 0.0, length: float | None = None, fade_out: float = 0.2
) -> Callable:
    return lambda get: fade(trim(cut(mono(get(sound)), start, length)), 0.005, fade_out)


def _window(sound: int, length: float, fade_out: float = 0.3) -> Callable:
    return lambda get: fade(loudest_window(mono(get(sound)), length), 0.02, fade_out)


def _loop(
    sound: int, length: float, channels: int = 2, high_pass: float = 0.0
) -> Callable:
    def render(get: Sources) -> np.ndarray:
        x = get(sound)
        x = mono(x) if channels == 1 else x
        if high_pass:
            x = filtered(x, "high", high_pass)
        return loopify(loudest_window(x, length + 1.5), 1.5)

    return render


def _war_cry(seed: int) -> Callable:
    """A crowd shout: several CC0 cries, re-pitched and staggered, over the charge scream."""

    def render(get: Sources) -> np.ndarray:
        rng = np.random.default_rng(seed)
        crowd = loudest_window(mono(get(563011)), 4.5)
        voices = [
            trim(loudest_window(mono(get(621352)), 3.0)),
            trim(loudest_window(mono(get(866009)), 3.0)),
            trim(loudest_window(mono(get(325548)), 2.5)),
        ]
        parts = [(crowd, 0.0, 0.8)]
        for index in range(9):
            voice = voices[index % len(voices)]
            factor = float(rng.uniform(0.84, 1.1))
            parts.append(
                (
                    repitch(voice, factor),
                    float(rng.uniform(0.0, 0.6)),
                    float(rng.uniform(0.25, 0.5)),
                )
            )
        shout = mix(parts, length=4.5)
        shout = filtered(shout, "high", 90.0)
        return fade(shout, 0.05, 1.2)

    return render


def _rout_cry(get: Sources) -> np.ndarray:
    rng = np.random.default_rng(71)
    mob = loudest_window(mono(get(384401)), 4.0)
    screams = [hit(mono(get(325548)), pick, 1.6, 8) for pick in range(4)]
    parts = [(mob, 0.0, 0.7)]
    for index, scream in enumerate(screams):
        parts.append(
            (repitch(scream, float(rng.uniform(0.9, 1.08))), 0.3 + index * 0.7, 0.45)
        )
    return fade(mix(parts, length=4.0), 0.05, 1.0)


def _arrow_swarm(get: Sources) -> np.ndarray:
    """Volley whistle: many fly-bys, detuned and staggered (a cloud of arrows)."""
    rng = np.random.default_rng(29)
    flybys = [trim(mono(get(789389))), trim(mono(get(384910)))]
    parts = []
    for index in range(14):
        base = flybys[index % 2]
        parts.append(
            (
                repitch(base, float(rng.uniform(0.85, 1.2))),
                float(rng.uniform(0.0, 1.1)),
                float(rng.uniform(0.2, 0.45)),
            )
        )
    return fade(mix(parts, length=2.6), 0.02, 0.6)


def _war_drum(get: Sources) -> np.ndarray:
    """A war drum roll: tom and bass hits on a slow march pattern, rising at the end."""
    tom = trim(mono(get(459876)))
    bass = trim(mono(get(459875)))
    pattern = [
        (0.0, bass, 0.9),
        (0.5, tom, 0.6),
        (1.0, bass, 0.9),
        (1.5, tom, 0.6),
        (2.0, bass, 0.9),
        (2.33, tom, 0.55),
        (2.66, tom, 0.6),
        (3.0, bass, 1.0),
        (3.25, tom, 0.6),
        (3.5, tom, 0.7),
        (3.75, tom, 0.8),
    ]
    return fade(
        mix([(hit_, at, gain) for at, hit_, gain in pattern], length=5.2), 0.0, 0.8
    )


def _trebuchet(get: Sources) -> np.ndarray:
    """Trebuchet release: rope creak under tension, arm swing (catapult launch), whoosh."""
    creak = loudest_window(mono(get(231438)), 1.2)
    launch = hit(mono(get(479922)), 0, 2.2, 4)
    whoosh = filtered(trim(mono(get(789389))), "low", 900.0)
    whoosh = repitch(whoosh, 0.55)
    return fade(
        mix([(creak, 0.0, 0.7), (launch, 0.9, 1.0), (whoosh, 1.1, 0.6)], length=3.4),
        0.02,
        0.6,
    )


def _wall_collapse(get: Sources) -> np.ndarray:
    rock = loudest_window(mono(get(389303)), 6.0)
    bricks = trim(mono(get(567249)))
    debris = trim(mono(get(703248)))
    rumble = filtered(rock, "low", 400.0)
    return fade(
        mix(
            [
                (rock, 0.0, 0.7),
                (rumble, 0.0, 0.8),
                (bricks, 0.6, 0.6),
                (debris, 1.4, 0.6),
            ],
            length=6.0,
        ),
        0.01,
        1.5,
    )


def _town(get: Sources) -> np.ndarray:
    """Town murmur: the medieval market street, layered at offsets, over a crowd murmur."""
    market = stereo(trim(get(424790)))
    murmur = loudest_window(get(444900), 22.0)
    length = 30.0
    parts = [
        (filtered(murmur, "low", 3500.0), 0.0, 0.5),
        (filtered(murmur, "low", 3500.0), 20.0, 0.5),
    ]
    offset = 0.0
    while offset < length:
        parts.append((market, offset, 0.8))
        offset += seconds(market) - 1.0
    return loopify(fade(mix(parts, length=length + 1.5), 0.5, 0.5), 1.5)


def _battle_distant(get: Sources) -> np.ndarray:
    """Distant battle heard from high above: muffled melee and clamour, wide stereo."""
    melee = filtered(mono(loudest_window(get(376646), 32.0)), "low", 1400.0)
    clamor = filtered(mono(loudest_window(get(384401), 32.0)), "low", 1800.0)
    delayed = np.concatenate(
        [np.zeros(int(0.037 * SAMPLE_RATE), dtype=np.float32), clamor]
    )[: len(clamor)]
    left = melee * 0.8 + clamor * 0.6
    right = melee * 0.7 + delayed * 0.65
    return loopify(np.stack([left, right], axis=1).astype(np.float32), 2.0)


def _clips() -> list[Clip]:
    b = "battle/"
    a = "ambience/"
    clips = [
        Clip(b + "sword_clash_1", [471095], _hit(471095, 0, 1.2)),
        Clip(b + "sword_clash_2", [364531], _hit(364531, 0, 1.2)),
        Clip(b + "sword_clash_3", [326868], _hit(326868, 0, 1.0)),
        Clip(b + "sword_clash_4", [440069], _hit(440069, 0, 1.2)),
        Clip(b + "shield_bash_1", [370203], _hit(370203, 0, 0.8)),
        Clip(b + "shield_bash_2", [182112], _hit(182112, 0, 0.9)),
        Clip(b + "shield_bash_3", [182112], _hit(182112, 2, 0.9)),
        Clip(b + "arrow_impact_1", [205938], _hit(205938, 0, 0.6)),
        Clip(b + "arrow_impact_2", [521552], _hit(521552, 0, 0.4)),
        Clip(b + "arrow_impact_3", [534956], _hit(534956, 0, 0.9)),
        Clip(b + "arrow_impact_4", [708223], _hit(708223, 0, 0.9)),
        Clip(b + "arrow_whistle_1", [394004], _take(394004, 0.0, 3.5, 0.8)),
        Clip(b + "arrow_whistle_2", [675821], _take(675821, 0.0, 4.0, 0.8)),
        Clip(
            b + "arrow_whistle_3",
            [789389, 384910],
            _arrow_swarm,
            note="mix of 14 detuned fly-bys",
        ),
        Clip(b + "bow_release_1", [263675], _hit(263675, 0, 0.8)),
        Clip(b + "bow_release_2", [394179], _hit(394179, 0, 0.5)),
        Clip(b + "bow_release_3", [384918], _hit(384918, 0, 0.8)),
        Clip(b + "crossbow_release_1", [384919], _take(384919, 0.0, 0.6, 0.15)),
        Clip(b + "charge_cry_1", [621352], _window(621352, 3.5)),
        Clip(b + "charge_cry_2", [866009], _window(866009, 3.5)),
        Clip(b + "charge_cry_3", [563011], _window(563011, 4.0)),
        Clip(
            b + "war_cry_1",
            [563011, 621352, 866009, 325548],
            _war_cry(11),
            note="crowd shout mixed from 10 cries",
        ),
        Clip(
            b + "war_cry_2",
            [563011, 621352, 866009, 325548],
            _war_cry(23),
            note="crowd shout mixed from 10 cries",
        ),
        Clip(b + "death_groan_1", [577032], _take(577032)),
        Clip(b + "death_groan_2", [221544], _take(221544)),
        Clip(b + "death_groan_3", [610998], _hit(610998, 0, 1.2, 10)),
        Clip(b + "death_groan_4", [610998], _hit(610998, 4, 1.2, 10)),
        Clip(b + "death_groan_5", [610998], _hit(610998, 8, 1.2, 10)),
        Clip(b + "death_groan_6", [255322], _hit(255322, 1, 1.8, 8)),
        Clip(b + "death_groan_7", [255322], _hit(255322, 5, 1.8, 8)),
        Clip(b + "rout_cry_1", [384401], _window(384401, 4.0, 1.0)),
        Clip(
            b + "rout_cry_2", [384401, 325548], _rout_cry, note="mob noise + 4 screams"
        ),
        Clip(b + "horse_neigh_1", [149024], _take(149024, 0.0, 3.0, 0.5)),
        Clip(b + "horse_neigh_2", [347036], _window(347036, 3.0, 0.6)),
        Clip(b + "horse_neigh_3", [269571], _take(269571)),
        Clip(b + "horse_neigh_4", [437110], _window(437110, 2.5, 0.5)),
        Clip(b + "horn_1", [539956], _take(539956, 0.0, None, 0.6)),
        Clip(b + "horn_2", [175946], _take(175946, 0.0, None, 0.6)),
        Clip(b + "horn_3", [512490], _window(512490, 8.0, 2.0)),
        Clip(
            b + "drum_1", [459876, 459875], _war_drum, note="march pattern of 11 hits"
        ),
        Clip(b + "bell_toll_1", [454855], _hit(454855, 0, 5.0, 4)),
        Clip(b + "bell_toll_2", [383192], _take(383192, 0.0, None, 1.0)),
        Clip(b + "ram_hit_1", [675970], _hit(675970, 0, 1.8, 6)),
        Clip(b + "ram_hit_2", [675970], _hit(675970, 2, 1.8, 6)),
        Clip(b + "ram_hit_3", [675970], _hit(675970, 4, 1.8, 6)),
        Clip(
            b + "trebuchet_release_1",
            [231438, 479922, 789389],
            _trebuchet,
            note="rope creak + launch + low whoosh",
        ),
        Clip(b + "stone_impact_1", [513694], _take(513694)),
        Clip(b + "stone_impact_2", [703247], _take(703247, 0.0, 3.0, 0.8)),
        Clip(b + "stone_impact_3", [567249], _take(567249, 0.0, 3.0, 0.8)),
        Clip(b + "bombard_1", [187767], _hit(187767, 0, 4.0, 3)),
        Clip(b + "bombard_2", [404166], _hit(404166, 0, 3.5, 3)),
        Clip(b + "wall_collapse_1", [712918], _take(712918, 0.0, None, 0.8)),
        Clip(
            b + "wall_collapse_2",
            [389303, 567249, 703248],
            _wall_collapse,
            note="rockfall + bricks + debris",
        ),
        Clip(b + "thunder_1", [399656], _take(399656, 0.0, 10.0, 3.0)),
        Clip(b + "thunder_2", [652690], _take(652690, 0.0, 10.0, 3.0)),
        # Nappes 3D en boucle (mono).
        Clip(
            b + "melee_bed_1", [376646], _loop(376646, 40.0, 1), loop=True, level="rms"
        ),
        Clip(
            b + "melee_bed_2", [175950], _loop(175950, 30.0, 1), loop=True, level="rms"
        ),
        Clip(
            b + "clamor_bed", [384401], _loop(384401, 40.0, 1), loop=True, level="rms"
        ),
        Clip(
            b + "march_bed",
            [480675],
            _loop(480675, 25.0, 1, 60.0),
            loop=True,
            level="rms",
        ),
        Clip(
            b + "cavalry_bed", [527430], _loop(527430, 20.0, 1), loop=True, level="rms"
        ),
        Clip(b + "fire_bed", [636178], _loop(636178, 30.0, 1), loop=True, level="rms"),
        # Ambiances 2D (stéréo).
        Clip(
            a + "battle_distant",
            [376646, 384401],
            _battle_distant,
            loop=True,
            channels=2,
            level="rms",
            note="muffled melee + clamour",
        ),
        Clip(
            a + "wind",
            [760241],
            _loop(760241, 40.0),
            loop=True,
            channels=2,
            level="rms",
        ),
        Clip(
            a + "wind_strong",
            [185070],
            _loop(185070, 40.0),
            loop=True,
            channels=2,
            level="rms",
        ),
        Clip(
            a + "rain",
            [157487],
            _loop(157487, 30.0),
            loop=True,
            channels=2,
            level="rms",
        ),
        Clip(
            a + "sea", [534910], _loop(534910, 36.0), loop=True, channels=2, level="rms"
        ),
        Clip(
            a + "countryside",
            [514550],
            _loop(514550, 45.0, 2, 80.0),
            loop=True,
            channels=2,
            level="rms",
        ),
        Clip(
            a + "crickets",
            [522299],
            _loop(522299, 40.0),
            loop=True,
            channels=2,
            level="rms",
        ),
        Clip(
            a + "forest",
            [474342],
            _loop(474342, 45.0),
            loop=True,
            channels=2,
            level="rms",
        ),
        Clip(
            a + "town",
            [424790, 444900],
            _town,
            loop=True,
            channels=2,
            level="rms",
            note="market street layered over a crowd murmur",
        ),
    ]
    return clips


CLIPS = _clips()

# Target levels: one-shots peak-normalised, loops RMS-normalised (mixed in-game by the bank).
PEAK_DB = -1.0
LOOP_RMS_DB = -20.0
# libsndfile Vorbis: compression 0 = best quality, 1 = smallest.
COMPRESSION = {1: 0.62, 2: 0.6}


def render_clip(clip: Clip, get: Sources) -> np.ndarray:
    """Render one clip to float32 (mono (n,) or stereo (n, 2)), level-normalised."""
    x = clip.recipe(get)
    x = mono(x) if clip.channels == 1 else stereo(x)
    if clip.level == "rms":
        return rms_normalize(x, LOOP_RMS_DB, PEAK_DB)
    return peak_normalize(x, PEAK_DB)


def write_ogg(path: Path, samples: np.ndarray, channels: int) -> None:
    """Encode Ogg Vorbis with libsndfile."""
    import soundfile

    path.parent.mkdir(parents=True, exist_ok=True)
    soundfile.write(
        str(path),
        samples,
        SAMPLE_RATE,
        format="OGG",
        subtype="VORBIS",
        compression_level=COMPRESSION[channels],
    )


def source_markdown(clips: list[Clip], infos: dict[int, SourceInfo]) -> str:
    """Content of ``game/assets/audio/SOURCE.md`` (bank part)."""
    lines = [
        "# Banque sonore AU1 (`battle/`, `ambience/`)",
        "",
        "Générée par `tools/cent_ans_tools/audio_bank.py` (reproductible :",
        "`uv run --project tools --with soundfile python -m cent_ans_tools.audio_bank`).",
        "Sources : Freesound, licence **CC0 1.0** (domaine public) vérifiée page par page au",
        "téléchargement ; fichier téléchargé = aperçu haute qualité public (MP3 128 kbit/s) du son.",
        "Traitements : découpe sur attaques ou fenêtre la plus dense, fondus, bouclage en fondu",
        "enchaîné à puissance constante, filtres, transpositions, mélanges ; normalisation (crête",
        "−1 dBFS pour les sons ponctuels, RMS −20 dBFS pour les boucles) ; encodage Ogg Vorbis.",
        "Les effets `sfx/` et musiques `music/` d'origine restent de la synthèse procédurale",
        "(`tools/cent_ans_tools/audio.py`).",
        "",
        "| Fichier | Sources Freesound (auteur — titre) | Traitement |",
        "|---|---|---|",
    ]
    for clip in clips:
        refs = "<br>".join(
            f"[{info.sound_id}]({info.url}) {info.user} — {info.title}"
            for info in (infos[i] for i in clip.sources)
        )
        kind = "boucle" if clip.loop else "ponctuel"
        detail = f"{kind}, {'stéréo' if clip.channels == 2 else 'mono'}"
        if clip.note:
            detail += f" ; {clip.note}"
        lines.append(f"| `{clip.name}.ogg` | {refs} | {detail} |")
    lines.append("")
    return "\n".join(lines)


def build(only: list[str] | None = None) -> list[Path]:
    """Fetch, verify, render and encode every clip (or those named in ``only``)."""
    wanted = [
        c for c in CLIPS if not only or c.name in only or c.name.split("/")[-1] in only
    ]
    needed = sorted({s for clip in CLIPS for s in clip.sources})
    infos: dict[int, SourceInfo] = {}
    paths: dict[int, Path] = {}
    for sound_id in needed:
        infos[sound_id], paths[sound_id] = fetch_source(sound_id)
    cache: dict[int, np.ndarray] = {}

    def get(sound_id: int) -> np.ndarray:
        if sound_id not in cache:
            cache[sound_id] = load(paths[sound_id])
        return cache[sound_id]

    written = []
    for clip in wanted:
        samples = render_clip(clip, get)
        out = AUDIO_DIR / f"{clip.name}.ogg"
        write_ogg(out, samples, clip.channels)
        written.append(out)
        print(
            f"{clip.name}: {seconds(samples):.1f} s, {out.stat().st_size / 1024:.0f} Ko"
        )
    (AUDIO_DIR / "SOURCE.md").write_text(
        source_markdown(CLIPS, infos), encoding="utf-8"
    )
    manifest = {
        str(i): {
            "user": infos[i].user,
            "title": infos[i].title,
            "url": infos[i].url,
            "licence": infos[i].licence,
        }
        for i in needed
    }
    digest = hashlib.sha256(json.dumps(manifest, sort_keys=True).encode()).hexdigest()[
        :12
    ]
    print(f"{len(written)} clips, {len(needed)} CC0 sources (manifest {digest})")
    return written


def main() -> None:
    """Command line entry point."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "clips", nargs="*", help="only these clips (name or battle/name)"
    )
    args = parser.parse_args()
    build(args.clips or None)


if __name__ == "__main__":
    main()
