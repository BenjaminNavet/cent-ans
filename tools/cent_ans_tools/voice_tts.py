"""VO1 voices: unit barks, general's speeches and the advisor, synthesised with OpenAI TTS.

Reproducible pipeline, run with::

    uv run --project tools --with soundfile python -m cent_ans_tools.voice_tts --dry-run
    uv run --project tools --with soundfile python -m cent_ans_tools.voice_tts [--only barks] [--limit 5]

Shouted barks and war cries (VX) go to ElevenLabs v3 on fal.ai (``FAL_KEY``) and are
checked locally: add ``--with faster-whisper --with librosa`` (see ``voice_shout``).

1. **Jobs** are derived from the data files: ``data/voice/barks.json`` (one clip per
   line, voices rotated inside a language), ``data/voice/advisor.json`` (one clip per
   line, light room reverb) and ``data/speeches/battle_speeches.json`` crossed with
   ``data/voice/speech_voices.json`` (every sentence a faction's general can say, for
   every voice of that faction, plus the war cries of ``order_war_cry.json``).
   Sentences with a ``{general}`` placeholder are not voiced (subtitle only).
2. **Cache**: an existing output file is never regenerated. The raw API answer is also
   kept in ``~/.cache/cent_ans/tts`` (keyed by model, voice, instructions and text) so
   that re-encoding never calls the API twice.
3. **Cost**: ``--dry-run`` lists the pending jobs and estimates their cost; a real run
   stops before the running estimate would exceed ``--cap`` (3 $ by default, the VO1
   ceiling). The real cost of each clip (from its measured duration) is written to
   ``game/assets/audio/voice/manifest.json``.
4. **Post-processing** (ffmpeg): leading and trailing silence trimmed, light room reverb
   for the advisor, EBU R128 loudness normalisation to -16 LUFS, mono Ogg Vorbis.

Speech clips are named ``voice/speech/<voice>/<sha1(text)[:12]>.ogg`` so that the game
finds them from the subtitle text alone (``String.sha1_text()`` in GDScript).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import date
from decimal import Decimal
from pathlib import Path

from cent_ans_tools import budget, voice_shout
from cent_ans_tools.budget import DEFAULT_BUDGET_PATH, to_money

REPO_DIR = Path(__file__).resolve().parents[2]
DATA_DIR = REPO_DIR / "data"
VOICE_DIR = REPO_DIR / "game" / "assets" / "audio" / "voice"
MANIFEST = VOICE_DIR / "manifest.json"
CACHE_DIR = Path.home() / ".cache" / "cent_ans" / "tts"

API_URL = "https://api.openai.com/v1/audio/speech"
MODEL = "gpt-4o-mini-tts"
# OpenAI pricing (checked 2026-09-25, developers.openai.com/api/docs/pricing):
# gpt-4o-mini-tts, text input 0.60 $ / 1M tokens, audio output 12 $ / 1M tokens,
# i.e. about 0.015 $ per minute of speech (OpenAI's own per-minute estimate).
PRICE_PER_MINUTE = Decimal("0.015")
# OpenRouter backend (default): chat completions with audio output, streamed as pcm16.
# openai/gpt-audio-mini, per token: prompt 0.0000006, completion and audio output
# 0.0000024 $ (OpenRouter models API, 2026-09-25). The real cost of every clip is read
# from ``usage.cost``; the per-minute figure below only feeds the estimate.
OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"
OPENROUTER_MODEL = "openai/gpt-audio-mini"
PCM_RATE = 24000
READ_VERBATIM = (
    "Tu es un moteur de synthèse vocale, pas un personnage : tu ne converses pas et tu "
    "ne réponds jamais au texte. Ta seule sortie est la lecture à voix haute, mot pour "
    "mot, du texte placé entre les balises <texte> et </texte> du message de "
    "l'utilisateur, dans sa langue d'origine, sans traduire, sans ajouter ni retirer un "
    "seul mot, sans rien dire avant ni après, sans prolonger ni conclure le texte. Seule "
    "la voix change selon cette consigne "
    "d'interprétation : "
)
PRICE_PER_INPUT_TOKEN = Decimal("0.60") / Decimal(1_000_000)
DEFAULT_CAP = Decimal("3.00")
# Speaking rates used by the estimate (characters per second, deliberately slow so
# that the estimate errs on the expensive side).
CHARS_PER_SECOND = {"barks": 9.0, "speech": 10.0, "advisor": 12.0}
MIN_SECONDS = 1.2
TARGET_LUFS = -16.0
# Shouts and war cries sit above the spoken lines, as they would on a field.
SHOUT_LUFS = -13.0
SPEECH_HASH_LEN = 12


@dataclass(frozen=True)
class Job:
    """One clip to synthesise."""

    kind: str  # barks, speech or advisor
    path: str  # relative to VOICE_DIR, without extension
    text: str
    voice: str
    instructions: str
    reverb: bool = False
    # ElevenLabs takes (VX): none = OpenRouter reading, one = shouted bark, several = a
    # war cry chorus.
    shouts: tuple[voice_shout.Shout, ...] = ()

    @property
    def model(self) -> str:
        """Model that voices the clip (OpenRouter backend for readings)."""
        return voice_shout.MODEL if self.shouts else OPENROUTER_MODEL

    @property
    def output(self) -> Path:
        """Final Ogg file."""
        return VOICE_DIR / f"{self.path}.ogg"

    def cache_key(self, model: str = MODEL) -> str:
        """Key of the raw API answer of ``model`` in the local cache."""
        blob = "\n".join([model, self.voice, self.instructions, self.text])
        return hashlib.sha256(blob.encode("utf-8")).hexdigest()[:24]

    def estimated_seconds(self) -> float:
        """Expected speech duration."""
        return max(MIN_SECONDS, len(self.text) / CHARS_PER_SECOND[self.kind])

    def estimated_cost(self) -> Decimal:
        """Expected price of the API call."""
        if self.shouts:
            return sum((shout.cost() for shout in self.shouts), Decimal(0))
        return cost_for(self.estimated_seconds(), self.text, self.instructions)


def cost_for(seconds: float, text: str, instructions: str) -> Decimal:
    """Price of one request: audio minutes plus text tokens (4 characters a token)."""
    audio = PRICE_PER_MINUTE * Decimal(str(seconds)) / Decimal(60)
    tokens = Decimal(len(text) + len(instructions)) / Decimal(4)
    return audio + tokens * PRICE_PER_INPUT_TOKEN


def speech_file_id(text: str) -> str:
    """File name of a speech sentence (same as ``text.sha1_text().substr(0, 12)``)."""
    return hashlib.sha1(text.encode("utf-8")).hexdigest()[:SPEECH_HASH_LEN]


def _load(relative: str) -> dict:
    return json.loads((DATA_DIR / relative).read_text(encoding="utf-8"))


# --- Jobs ----------------------------------------------------------------------------


def bark_jobs(barks: dict) -> list[Job]:
    """One job per bark line; voices rotate inside a language and situation."""
    jobs = []
    for language, situations in barks["lines"].items():
        spec = barks["languages"][language]
        voices = spec["voices"]
        shout_voices = spec.get("shout_voices", [])
        for situation, lines in situations.items():
            tone = barks["situations"][situation]["tone"]
            tag = barks["situations"][situation].get("shout_tag", "")
            instructions = f"{spec['instructions']} Ton : {tone}."
            for index, line in enumerate(lines):
                voice = voices[index % len(voices)]
                shouts: tuple[voice_shout.Shout, ...] = ()
                if tag and shout_voices:
                    voice = shout_voices[index % len(shout_voices)]
                    shouts = (shout_for(line["text"], tag, voice, language, spec),)
                jobs.append(
                    Job(
                        "barks",
                        f"barks/{line['id']}",
                        line["text"],
                        voice,
                        instructions,
                        shouts=shouts,
                    )
                )
    return jobs


def shout_for(
    text: str, tag: str, voice: str, language: str, spec: dict
) -> voice_shout.Shout:
    """ElevenLabs request of ``text`` in bark language ``language`` (``spec``)."""
    code = spec.get("language_code", "")
    check = code or voice_shout.WHISPER_LANGUAGE.get(language, language)
    return voice_shout.Shout(text, tag, voice, code, check)


def advisor_jobs(advisor: dict) -> list[Job]:
    """One job per advisor line, with room reverb."""
    return [
        Job(
            "advisor",
            f"advisor/{line['id']}",
            line["text"],
            advisor["voice"],
            advisor["instructions"],
            reverb=True,
        )
        for line in advisor["lines"]
    ]


def speech_sentences(speeches: dict, faction: str) -> list[str]:
    """Every sentence the general of ``faction`` may say (placeholders excluded)."""

    def by_key(section: str) -> list[str]:
        table = speeches.get(section, {})
        return table.get(faction, table.get("default", []))

    sentences = list(by_key("openings"))
    for section in ("odds", "terrain", "weather"):
        for lines in speeches.get(section, {}).values():
            sentences.extend(lines)
    sentences.extend(by_key("closings"))
    return [s for s in dict.fromkeys(sentences) if "{" not in s]


def cry_chorus(text: str, faction: str, casting: dict) -> tuple[voice_shout.Shout, ...]:
    """ElevenLabs takes of the war cry ``text`` of ``faction`` (empty: no chorus)."""
    chorus = casting.get("cry_chorus")
    if not chorus:
        return ()
    code = chorus.get("language", {}).get(faction, chorus.get("default_language", "fr"))
    return tuple(
        voice_shout.Shout(text, chorus["tag"], voice, code, code)
        for voice in chorus["voices"]
    )


def speech_jobs(speeches: dict, casting: dict, cries: dict) -> list[Job]:
    """Speech sentences and war cries for every voice of every cast faction.

    Factions without their own casting use the ``default`` one: its voices say the
    default openings and closings, and the cries of every uncast faction.
    """
    jobs: dict[str, Job] = {}
    cast = casting["casting"]
    for faction, spec in cast.items():
        texts = speech_sentences(speeches, faction)
        # War cry text -> faction whose language it is in.
        if faction == "default":
            cry_texts = {speeches["default_cry"]: "default"}
            cry_texts.update({c: f for f, c in cries.items() if f not in cast})
        else:
            cry_texts = {cries.get(faction, speeches["default_cry"]): faction}
        for voice in spec["voices"]:
            for text in texts:
                job = Job(
                    "speech",
                    f"speech/{voice}/{speech_file_id(text)}",
                    text,
                    voice,
                    spec["instructions"],
                )
                jobs.setdefault(job.path, job)
            for text, cry_faction in cry_texts.items():
                job = Job(
                    "speech",
                    f"speech/{voice}/{speech_file_id(text)}",
                    text,
                    voice,
                    f"{spec['instructions']} {casting['cry_instructions']}",
                    shouts=cry_chorus(text, cry_faction, casting),
                )
                jobs.setdefault(job.path, job)
    return list(jobs.values())


def all_jobs() -> list[Job]:
    """Every clip described by the data files."""
    cries = _load("battle_orders/order_war_cry.json").get("labels_by_faction", {})
    return (
        bark_jobs(_load("voice/barks.json"))
        + advisor_jobs(_load("voice/advisor.json"))
        + speech_jobs(
            _load("speeches/battle_speeches.json"),
            _load("voice/speech_voices.json"),
            cries,
        )
    )


def pending(jobs: list[Job], manifest: dict | None = None) -> list[Job]:
    """Jobs whose output file does not exist yet, or whose clip was voiced by another
    model than the one the data now asks for (a reading now meant to be shouted)."""
    manifest = load_manifest() if manifest is None else manifest

    def stale(job: Job) -> bool:
        entry = manifest.get(f"{job.path}.ogg")
        return bool(job.shouts) and (entry is None or entry.get("model") != job.model)

    return [job for job in jobs if not job.output.exists() or stale(job)]


# --- Synthesis and encoding ---------------------------------------------------------


def synthesise(job: Job, api_key: str) -> Path:
    """Raw WAV answer of the API for ``job`` (from the cache when present)."""
    import httpx

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    raw = CACHE_DIR / f"{job.cache_key()}.wav"
    if raw.exists():
        return raw
    response = httpx.post(
        API_URL,
        headers={"Authorization": f"Bearer {api_key}"},
        json={
            "model": MODEL,
            "voice": job.voice,
            "input": job.text,
            "instructions": job.instructions,
            "response_format": "wav",
        },
        timeout=120.0,
    )
    if response.status_code != 200:
        raise RuntimeError(
            f"{job.path}: HTTP {response.status_code} {response.text[:300]}"
        )
    tmp = raw.with_suffix(".part")
    tmp.write_bytes(response.content)
    tmp.replace(raw)
    return raw


def plausible_seconds(text: str) -> tuple[float, float]:
    """Accepted duration range for ``text`` read aloud (catches added or missing words)."""
    return len(text) / 26.0, len(text) / 5.0 + 1.8


def _words(text: str) -> list[str]:
    cleaned = "".join(ch.lower() if ch.isalnum() else " " for ch in text)
    return cleaned.split()


def transcript_matches(text: str, transcript: str) -> bool:
    """The model's transcript says the text (most words, nothing much added).

    Stage directions or tags in the transcript (``(pause)``, ``*roar*``, ``</texte>``)
    may have been spoken aloud: rejected too.
    """
    if any(mark in transcript for mark in "<>*[]()"):
        return False
    expected, said = _words(text), _words(transcript)
    if not expected:
        return True
    common = sum(1 for word in expected if word in said)
    return common >= 0.6 * len(expected) and len(said) <= 1.3 * len(expected) + 2


def forget(job: Job) -> None:
    """Drop a clip (output, cached OpenRouter answer, manifest entry) to redo it."""
    key = job.cache_key(OPENROUTER_MODEL)
    for path in (job.output, CACHE_DIR / f"{key}.wav", CACHE_DIR / f"{key}.json"):
        path.unlink(missing_ok=True)
    manifest = load_manifest()
    if manifest.pop(f"{job.path}.ogg", None) is not None:
        save_manifest(manifest)


def _write_wav(path: Path, pcm: bytes) -> None:
    import wave

    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(PCM_RATE)
        out.writeframes(pcm)


def synthesise_openrouter(job: Job, api_key: str) -> tuple[Path, Decimal, str]:
    """Raw WAV of ``job`` from OpenRouter (streamed pcm16), its real cost and transcript.

    Audio output requires streaming on OpenRouter: the base64 pcm16 chunks of
    ``delta.audio.data`` are concatenated, ``delta.audio.transcript`` gives what was
    said, the last chunk carries ``usage.cost`` (kept with the cached answer). Raises
    ``RuntimeError`` on an HTTP error.
    """
    import base64

    import httpx

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    key = job.cache_key(OPENROUTER_MODEL)
    raw = CACHE_DIR / f"{key}.wav"
    meta = CACHE_DIR / f"{key}.json"
    if raw.exists() and meta.exists():
        cached = json.loads(meta.read_text("utf-8"))
        said = cached.get("transcript", "")
        cost = Decimal(str(cached.get("usage", {}).get("cost", 0)))
        return raw, cost, said
    body = {
        "model": OPENROUTER_MODEL,
        "modalities": ["text", "audio"],
        "audio": {"voice": job.voice, "format": "pcm16"},
        "stream": True,
        "usage": {"include": True},
        "messages": [
            {"role": "system", "content": READ_VERBATIM + job.instructions},
            # One text-only example of a verbatim reading: short, emotional lines
            # (charges, routs, war cries) otherwise tempt the model to improvise.
            {"role": "user", "content": "<texte>À moi ! Tenez ferme !</texte>"},
            {"role": "assistant", "content": "À moi ! Tenez ferme !"},
            {"role": "user", "content": f"<texte>{job.text}</texte>"},
        ],
    }
    pcm = bytearray()
    transcript: list[str] = []
    usage: dict = {}
    headers = {"Authorization": f"Bearer {api_key}", "X-Title": "Cent Ans VO1"}
    with httpx.stream(
        "POST", OPENROUTER_URL, headers=headers, json=body, timeout=180.0
    ) as response:
        if response.status_code != 200:
            response.read()
            raise RuntimeError(
                f"{job.path}: HTTP {response.status_code} {response.text[:300]}"
            )
        for line in response.iter_lines():
            if not line.startswith("data: ") or line == "data: [DONE]":
                continue
            chunk = json.loads(line[6:])
            if "error" in chunk:
                raise RuntimeError(f"{job.path}: {chunk['error']}")
            usage = chunk.get("usage") or usage
            for choice in chunk.get("choices", []):
                audio = (choice.get("delta") or {}).get("audio") or {}
                if audio.get("data"):
                    pcm.extend(base64.b64decode(audio["data"]))
                if audio.get("transcript"):
                    transcript.append(audio["transcript"])
    cost = Decimal(str(usage.get("cost", 0)))
    if not pcm:
        raise RejectedClip(f"{job.path}: no audio in the answer", cost)
    _write_wav(raw, bytes(pcm))
    said = "".join(transcript)
    meta.write_text(
        json.dumps({"transcript": said, "usage": usage}, ensure_ascii=False), "utf-8"
    )
    return raw, cost, said


class RejectedClip(ValueError):
    """A clip whose audio, duration or transcript does not match its text."""

    def __init__(self, message: str, cost: Decimal) -> None:
        """Keep the money spent on the rejected attempt."""
        super().__init__(message)
        self.cost = cost


def openrouter_checked(
    job: Job, api_key: str, attempts: int = 2
) -> tuple[Path, Decimal, str]:
    """OpenRouter clip whose duration and transcript fit the text (retried once)."""
    spent = Decimal(0)
    reason = ""
    for _attempt in range(attempts):
        try:
            raw, cost, said = synthesise_openrouter(job, api_key)
        except RejectedClip as error:
            spent += error.cost
            reason = str(error)
            continue
        spent += cost
        low, high = plausible_seconds(job.text)
        seconds = duration(raw)
        if low <= seconds <= high and transcript_matches(job.text, said):
            return raw, spent, said
        reason = f"{job.path}: {seconds:.1f} s (expected {low:.1f}-{high:.1f}), said {said!r}"
        key = job.cache_key(OPENROUTER_MODEL)
        (CACHE_DIR / f"{key}.wav").unlink(missing_ok=True)
        (CACHE_DIR / f"{key}.json").unlink(missing_ok=True)
    raise RejectedClip(reason, spent)


def shouted_clip(job: Job, api_key: str, attempts: int = 3) -> tuple[Path, Decimal, str]:
    """Checked ElevenLabs take of a shouted bark, or the chorus of a war cry (raw file,
    money spent, transcript heard). Raises ``RejectedShout`` when a take keeps failing."""
    spent = Decimal(0)
    raws, heard = [], ""
    for shout in job.shouts:
        try:
            raw, cost, said = voice_shout.checked(shout, api_key, CACHE_DIR, attempts)
        except voice_shout.RejectedShout as error:
            raise voice_shout.RejectedShout(str(error), spent + error.cost) from error
        spent += cost
        raws.append(raw)
        heard = heard or said
    if len(raws) == 1:
        return raws[0], spent, heard
    chorus = voice_shout.chorus_cache_path(list(job.shouts), CACHE_DIR)
    if not chorus.exists():
        voice_shout.mix_chorus(raws, chorus, seed=job.text)
    return chorus, spent, heard


def filter_chain(reverb: bool, shout: bool = False, chorus: bool = False) -> str:
    """Ffmpeg audio filters: trim silences, optional room reverb, loudness.

    Shouts get a battlefield treatment: low rumble cut, firm compression and a presence
    lift so that they cut through the melee; a chorus gets an open-air echo.
    """
    trim = "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0.05"
    filters = [trim, "areverse", trim, "areverse"]
    if reverb:
        # Small vaulted room: two short early reflections, low decay.
        filters.append("aecho=0.9:0.6:37|61:0.22|0.12")
    if chorus:
        # Open field: late, weak reflections from the woods and the far ranks.
        filters.append("aecho=0.85:0.7:140|260|410:0.22|0.14|0.08")
    if shout or chorus:
        filters += [
            "highpass=f=90",
            "acompressor=threshold=0.125:ratio=4:attack=4:release=90:makeup=2",
            "equalizer=f=2600:t=q:w=1.2:g=3",
        ]
    lufs = SHOUT_LUFS if shout or chorus else TARGET_LUFS
    filters.append(f"loudnorm=I={lufs}:TP=-1.5:LRA=11")
    filters.append("aresample=44100")
    return ",".join(filters)


def encode(raw: Path, job: Job) -> float:
    """Encode ``raw`` into the job's Ogg file; returns its duration in seconds."""
    job.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "out.wav"
        out = Path(tmp) / "out.ogg"
        subprocess.run(
            [
                "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
                "-i", str(raw),
                "-af", filter_chain(
                    job.reverb, shout=len(job.shouts) == 1, chorus=len(job.shouts) > 1
                ),
                "-ac", "1", "-c:a", "pcm_s16le",
                str(wav),
            ],
            check=True,
        )  # fmt: skip
        # Homebrew's ffmpeg has no libvorbis: Vorbis through libsndfile, as AU1 does.
        import soundfile

        samples, rate = soundfile.read(str(wav), dtype="float32")
        soundfile.write(str(out), samples, rate, format="OGG", subtype="VORBIS")
        shutil.move(out, job.output)
    return duration(job.output)


def duration(path: Path) -> float:
    """Duration of an audio file (ffprobe)."""
    result = subprocess.run(
        [
            "ffprobe", "-v", "error", "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1", str(path),
        ],
        check=True, capture_output=True, text=True,
    )  # fmt: skip
    return float(result.stdout.strip())


def load_manifest() -> dict:
    """Generated clips: relative path -> {text, voice, model, seconds, cost_usd}."""
    if MANIFEST.exists():
        return json.loads(MANIFEST.read_text(encoding="utf-8"))
    return {}


def save_manifest(manifest: dict) -> None:
    """Write the manifest, sorted, UTF-8."""
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(dict(sorted(manifest.items())), ensure_ascii=False, indent=1)
    MANIFEST.write_text(text + "\n", encoding="utf-8")


# --- Command line -------------------------------------------------------------------


def summarise(jobs: list[Job]) -> Decimal:
    """Print pending jobs per kind with their estimate; returns the total."""
    total = Decimal(0)
    for kind in ("barks", "advisor", "speech"):
        selected = [job for job in jobs if job.kind == kind]
        cost = sum((job.estimated_cost() for job in selected), Decimal(0))
        seconds = sum(job.estimated_seconds() for job in selected)
        total += cost
        print(
            f"{kind:8} {len(selected):4} clips  ~{seconds / 60:5.1f} min  ~{cost:.3f} $"
        )
    print(f"total    {len(jobs):4} clips  ~{total:.3f} $")
    return total


def main(argv: list[str] | None = None) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--dry-run", action="store_true", help="estimate only, no API call"
    )
    parser.add_argument(
        "--only", choices=["barks", "advisor", "speech"], action="append"
    )
    parser.add_argument("--limit", type=int, default=0, help="at most N clips")
    parser.add_argument(
        "--backend", choices=["openrouter", "openai"], default="openrouter"
    )
    parser.add_argument(
        "--recheck", action="store_true", help="redo clips failing the transcript check"
    )
    parser.add_argument(
        "--attempts", type=int, default=2, help="tries per clip (OpenRouter checks)"
    )
    parser.add_argument(
        "--cap", type=Decimal, default=DEFAULT_CAP, help="cost ceiling ($)"
    )
    parser.add_argument(
        "--budget-path",
        type=Path,
        default=DEFAULT_BUDGET_PATH,
        help="docs/budget.md ledger checked and recorded against",
    )
    args = parser.parse_args(argv)

    # Real cumulative spend of this whole invocation: money already spent on clips
    # ``--recheck`` is about to redo (``forgotten``, no longer in the manifest once
    # ``forget()`` drops them) plus every attempt made below, successful or rejected
    # (``run_cost``, via :class:`RejectedClip`'s billed-but-unusable cost). Neither
    # figure is retrievable from the manifest afterwards, so both are tracked here and
    # recorded to ``docs/budget.md`` in the ``finally`` block, not just on success.
    forgotten = Decimal(0)
    if args.recheck:
        # Redo generated clips whose recorded transcript fails today's checks.
        manifest = load_manifest()
        for job in all_jobs():
            entry = manifest.get(f"{job.path}.ogg")
            if job.shouts:  # ElevenLabs takes are checked by whisper when made
                continue
            if entry and not transcript_matches(job.text, entry.get("transcript", "")):
                print(f"recheck: redo {job.path} (said {entry.get('transcript')!r})")
                if not args.dry_run:
                    forgotten += Decimal(str(entry.get("cost_usd", 0)))
                    forget(job)
    jobs = pending(all_jobs())
    if args.only:
        jobs = [job for job in jobs if job.kind in args.only]
    if args.limit:
        jobs = jobs[: args.limit]
    manifest = load_manifest()
    spent = sum(
        (Decimal(str(e.get("cost_usd", 0))) for e in manifest.values()), Decimal(0)
    )
    print(f"already generated: {len(manifest)} clips, {spent:.3f} $ (measured)")
    estimate = summarise(jobs)
    if args.dry_run or not jobs:
        return 0
    key_name = (
        "OPENROUTER_API_KEY" if args.backend == "openrouter" else "OPENAI_API_KEY"
    )
    api_key = os.environ.get(key_name, "")
    fal_key = os.environ.get("FAL_KEY", "")
    for needed, name, value in (
        (any(not job.shouts for job in jobs), key_name, api_key),
        (any(job.shouts for job in jobs), "FAL_KEY", fal_key),
    ):
        if needed and not value:
            print(f"{name} missing", file=sys.stderr)
            return 2
    if spent + forgotten + estimate > args.cap:
        print(
            f"refused: {spent:.3f} + {forgotten:.3f} + {estimate:.3f} $ would exceed "
            f"the cap of {args.cap} $"
        )
        return 3
    if not budget.check(to_money(forgotten + estimate), args.budget_path):
        print("refused: le plafond global de docs/budget.md serait dépassé")
        return 3
    run_cost = Decimal(0)
    shout_cost = Decimal(0)  # part of run_cost billed by fal.ai
    result = 0
    try:
        for index, job in enumerate(jobs, 1):
            if spent + forgotten + run_cost + job.estimated_cost() > args.cap:
                print(f"stopped before the cap ({args.cap} $)")
                break
            model = OPENROUTER_MODEL if args.backend == "openrouter" else MODEL
            try:
                if job.shouts:
                    model = job.model
                    raw, cost, said = shouted_clip(job, fal_key, max(3, args.attempts))
                    shout_cost += cost
                elif args.backend == "openrouter":
                    raw, cost, said = openrouter_checked(job, api_key, args.attempts)
                else:
                    raw = synthesise(job, api_key)
                    # Billed audio is the raw answer, silences included.
                    cost = cost_for(duration(raw), job.text, job.instructions)
                    said = job.text
            except RuntimeError as error:  # refused key, quota, invalid voice...
                print(f"API error, stopping: {error}", file=sys.stderr)
                result = 4
                break
            except RejectedClip as error:  # implausible clip twice: skipped, not saved
                print(f"skipped: {error}", file=sys.stderr)
                run_cost += error.cost
                continue
            except voice_shout.RejectedShout as error:  # the older clip, if any, stays
                print(f"skipped: {error}", file=sys.stderr)
                run_cost += error.cost
                shout_cost += error.cost
                continue
            seconds = encode(raw, job)
            run_cost += cost
            manifest[f"{job.path}.ogg"] = {
                "text": job.text,
                "voice": job.voice,
                "model": model,
                "transcript": said,
                "seconds": round(seconds, 2),
                "cost_usd": float(round(cost, 5)),
            }
            save_manifest(manifest)
            print(f"[{index}/{len(jobs)}] {job.path} {seconds:.1f} s  {cost:.4f} $")
        print(f"this run: {run_cost:.3f} $ (estimated beforehand {estimate:.3f} $)")
    finally:
        shout_estimate = sum(
            (job.estimated_cost() for job in jobs if job.shouts), Decimal(0)
        )
        total_spent = to_money(forgotten + run_cost - shout_cost)
        if total_spent > 0:
            service = "OpenRouter" if args.backend == "openrouter" else "OpenAI"
            budget.add_entry(
                date.today().isoformat(),
                service,
                f"VO1 : synthèse voix ({len(jobs)} clips en attente)",
                to_money(estimate - shout_estimate),
                total_spent,
                path=args.budget_path,
            )
        if shout_cost > 0:
            budget.add_entry(
                date.today().isoformat(),
                "fal.ai",
                "VX : cris de bataille ElevenLabs v3 (répliques criées, chœurs des "
                "cris de guerre), contrôle whisper, langue et hauteur",
                to_money(shout_estimate),
                to_money(shout_cost),
                path=args.budget_path,
            )
    return result


if __name__ == "__main__":
    raise SystemExit(main())
