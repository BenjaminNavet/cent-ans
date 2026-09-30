"""VX shouted voices: battle barks and war cries synthesised with ElevenLabs v3 (fal.ai).

gpt-audio-mini reads a war cry at speaking pitch (about 130 Hz for « Montjoie !
Saint-Denis ! », the same as a calm « Monseigneur ? »): the barks of the fight
sounded limp. ElevenLabs v3 obeys emotion tags (``[shouting]``, ``[panicked]``…) and
really shouts (250-350 Hz). It gives no transcript, so every clip is checked locally:

* **words**: faster-whisper transcript close to the text (words or letters),
* **language**: whisper recognises the expected language,
* **shout**: median pitch (pYIN) high enough for a shouted male voice,
* **duration**: the plausible range of ``voice_tts.plausible_seconds``.

A war cry is a **chorus**: the cry of several voices, each laid twice with a small
pitch shift, random delay and gain, then an open-air echo — the cry "taken up by the
whole army" (``order_war_cry.json``).

Needs ``faster-whisper`` and ``librosa`` (``uv run --with``, see ``voice_tts``).
"""

from __future__ import annotations

import difflib
import hashlib
import json
import random
import subprocess
import tempfile
from dataclasses import dataclass
from decimal import Decimal
from functools import cache
from pathlib import Path

FAL_URL = "https://fal.run/fal-ai/elevenlabs/tts/eleven-v3"
MODEL = "fal-ai/elevenlabs/tts/eleven-v3"
# fal.ai pricing (checked 2026-09-30, api.fal.ai/v1/models/pricing): 0.10 $ / 1000
# characters of input text, tags included.
PRICE_PER_CHAR = Decimal("0.10") / Decimal(1000)
# Low stability = the most expressive delivery ("creative" in ElevenLabs v3).
STABILITY = 0.0
# Median pitch of a shouted adult male voice; spoken lines sit around 110-140 Hz.
MIN_SHOUT_F0_HZ = 180.0
MIN_LANGUAGE_PROBABILITY = 0.5
MIN_LETTER_RATIO = 0.72
# Whisper language of a bark language without an ElevenLabs code.
WHISPER_LANGUAGE = {"an": "fr", "sco": "en"}
CHORUS_LAYERS_PER_VOICE = 2
CHORUS_MAX_DELAY_S = 0.06
# Resampling shifts pitch and length together (a few % only, so the words stay put).
CHORUS_PITCH = (0.96, 1.04)
CHORUS_GAIN = (0.25, 0.4)
MIX_RATE = 44100


@dataclass(frozen=True)
class Shout:
    """One ElevenLabs request: spoken text, emotion tag, voice, language."""

    text: str
    tag: str
    voice: str
    language_code: str = ""
    check_language: str = ""

    @property
    def prompt(self) -> str:
        """Text sent to the model, emotion tag first."""
        return f"{self.tag} {self.text}".strip()

    def cache_key(self) -> str:
        """Key of the raw answer in the local cache."""
        blob = "\n".join(
            [MODEL, str(STABILITY), self.voice, self.language_code, self.prompt]
        )
        return hashlib.sha256(blob.encode("utf-8")).hexdigest()[:24]

    def cost(self) -> Decimal:
        """Price of one request."""
        return PRICE_PER_CHAR * len(self.prompt)


class RejectedShout(ValueError):
    """A shout that fails the checks (words, language, pitch, duration)."""

    def __init__(self, message: str, cost: Decimal) -> None:
        """Keep the money spent on the rejected attempts."""
        super().__init__(message)
        self.cost = cost


# --- Synthesis -----------------------------------------------------------------------


def synthesise(shout: Shout, api_key: str, cache_dir: Path) -> tuple[Path, Decimal]:
    """Raw MP3 of ``shout`` (from the cache when present) and what it cost."""
    import httpx

    cache_dir.mkdir(parents=True, exist_ok=True)
    raw = cache_dir / f"{shout.cache_key()}.mp3"
    if raw.exists():
        return raw, Decimal(0)
    body: dict = {"text": shout.prompt, "voice": shout.voice, "stability": STABILITY}
    if shout.language_code:
        body["language_code"] = shout.language_code
    headers = {"Authorization": f"Key {api_key}", "Content-Type": "application/json"}
    response = httpx.post(FAL_URL, headers=headers, json=body, timeout=180.0)
    if response.status_code != 200:
        raise RuntimeError(
            f"{shout.prompt}: HTTP {response.status_code} {response.text[:300]}"
        )
    url = response.json().get("audio", {}).get("url")
    if not url:
        raise RuntimeError(f"{shout.prompt}: no audio in {response.text[:300]}")
    audio = httpx.get(url, timeout=120.0)
    audio.raise_for_status()
    tmp = raw.with_suffix(".part")
    tmp.write_bytes(audio.content)
    tmp.replace(raw)
    return raw, shout.cost()


def forget(shout: Shout, cache_dir: Path) -> None:
    """Drop the cached answer of ``shout`` so that the next call draws a new take."""
    (cache_dir / f"{shout.cache_key()}.mp3").unlink(missing_ok=True)


# --- Checks --------------------------------------------------------------------------


@cache
def _whisper():
    from faster_whisper import WhisperModel

    return WhisperModel("small", compute_type="int8")


def _samples(path: Path, rate: int):
    import librosa

    return librosa.load(str(path), sr=rate, mono=True)[0]


def listen(path: Path, language: str) -> tuple[str, float]:
    """Whisper transcript of ``path`` and the probability that it is in ``language``.

    Language detection runs separately from the transcription forced to ``language``.
    """
    samples = _samples(path, 16000)
    model = _whisper()
    _, info = model.transcribe(samples, beam_size=1)
    probability = dict(info.all_language_probs or []).get(language, 0.0)
    segments, _ = model.transcribe(samples, language=language, beam_size=5)
    return " ".join(segment.text.strip() for segment in segments), probability


def median_pitch(path: Path) -> float:
    """Median F0 (Hz) of the voiced frames of ``path`` (pYIN), 0 when unvoiced."""
    import librosa
    import numpy

    samples = _samples(path, 22050)
    f0, _, _ = librosa.pyin(samples, fmin=70, fmax=600, sr=22050)
    voiced = f0[~numpy.isnan(f0)]
    return float(numpy.median(voiced)) if len(voiced) else 0.0


def _letters(text: str) -> str:
    return "".join(ch for ch in text.lower() if ch.isalpha())


def words_match(text: str, heard: str) -> bool:
    """``heard`` says ``text``: most of its words, or most of its letters.

    Homophones (« sauve qui peut » / « sauf qu'il peut ») and accented names pass.
    """
    from cent_ans_tools.voice_tts import transcript_matches

    if any(mark in heard for mark in "<>*[]()"):
        return False
    if transcript_matches(text, heard):
        return True
    ratio = difflib.SequenceMatcher(None, _letters(text), _letters(heard)).ratio()
    return ratio >= MIN_LETTER_RATIO


def check(shout: Shout, raw: Path, seconds: float) -> tuple[str, str]:
    """Transcript and rejection reason of a take ('' when the take is good)."""
    from cent_ans_tools.voice_tts import plausible_seconds

    low, high = plausible_seconds(shout.text)
    if not low <= seconds <= high + 1.0:
        return "", f"{seconds:.1f} s (expected {low:.1f}-{high + 1.0:.1f})"
    heard, probability = listen(raw, shout.check_language)
    if not words_match(shout.text, heard):
        return heard, f"heard {heard!r}"
    if probability < MIN_LANGUAGE_PROBABILITY:
        return heard, f"{shout.check_language} at {probability:.2f} only"
    pitch = median_pitch(raw)
    if pitch < MIN_SHOUT_F0_HZ:
        return heard, f"not shouted ({pitch:.0f} Hz)"
    return heard, ""


def checked(
    shout: Shout, api_key: str, cache_dir: Path, attempts: int = 3
) -> tuple[Path, Decimal, str]:
    """A take of ``shout`` that passes the checks (new takes drawn on failure)."""
    from cent_ans_tools.voice_tts import duration

    spent = Decimal(0)
    reason = ""
    for _attempt in range(attempts):
        raw, cost = synthesise(shout, api_key, cache_dir)
        spent += cost
        heard, reason = check(shout, raw, duration(raw))
        if not reason:
            return raw, spent, heard
        forget(shout, cache_dir)
    raise RejectedShout(f"{shout.voice} {shout.prompt!r}: {reason}", spent)


# --- Chorus --------------------------------------------------------------------------


def _decode_trimmed(raw: Path, out: Path) -> None:
    """Mono WAV of ``raw`` at MIX_RATE, leading and trailing silence cut."""
    trim = "silenceremove=start_periods=1:start_threshold=-45dB:start_silence=0.02"
    subprocess.run(
        [
            "ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(raw),
            "-af", f"{trim},areverse,{trim},areverse",
            "-ac", "1", "-ar", str(MIX_RATE), "-c:a", "pcm_s16le", str(out),
        ],
        check=True,
    )  # fmt: skip


def mix_chorus(raws: list[Path], out: Path, seed: str) -> None:
    """Chorus WAV of ``raws`` (deterministic for ``seed``).

    The first take leads at full gain. Every other take is time-stretched to the
    lead's length (pitch kept) so that the syllables fall together, then laid
    ``CHORUS_LAYERS_PER_VOICE`` times, each with its own small pitch shift, delay and
    lower gain: the ranks answer the leader without smearing the words.
    """
    import librosa
    import numpy
    import soundfile

    rng = random.Random(seed)
    takes = []
    with tempfile.TemporaryDirectory() as tmp:
        for index, raw in enumerate(raws):
            wav = Path(tmp) / f"{index}.wav"
            _decode_trimmed(raw, wav)
            samples, _ = soundfile.read(str(wav), dtype="float32")
            takes.append(samples / max(1e-6, float(numpy.abs(samples).max())))
    lead = takes[0]
    layers = [(0, lead)]
    for take in takes[1:]:
        aligned = librosa.effects.time_stretch(take, rate=len(take) / len(lead))
        for _layer in range(CHORUS_LAYERS_PER_VOICE):
            factor = rng.uniform(*CHORUS_PITCH)
            length = max(1, int(len(aligned) / factor))
            shifted = numpy.interp(
                numpy.arange(length) * factor, numpy.arange(len(aligned)), aligned
            )
            delay = int(rng.uniform(0.01, CHORUS_MAX_DELAY_S) * MIX_RATE)
            layers.append((delay, rng.uniform(*CHORUS_GAIN) * shifted))
    total = max(delay + len(layer) for delay, layer in layers)
    mix = numpy.zeros(total, dtype="float32")
    for delay, layer in layers:
        mix[delay : delay + len(layer)] += layer
    mix /= max(1e-6, float(numpy.abs(mix).max())) / 0.9
    soundfile.write(str(out), mix, MIX_RATE, subtype="PCM_16")


def chorus_cache_path(shouts: list[Shout], cache_dir: Path) -> Path:
    """Where the chorus of ``shouts`` is kept (one file per set of takes)."""
    blob = json.dumps([shout.cache_key() for shout in shouts])
    return cache_dir / f"chorus_{hashlib.sha256(blob.encode()).hexdigest()[:24]}.wav"
