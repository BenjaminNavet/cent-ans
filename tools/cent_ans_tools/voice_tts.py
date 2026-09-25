"""VO1 voices: unit barks, general's speeches and the advisor, synthesised with OpenAI TTS.

Reproducible pipeline, run with::

    uv run --project tools python -m cent_ans_tools.voice_tts --dry-run
    uv run --project tools python -m cent_ans_tools.voice_tts [--only barks] [--limit 5]

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
from decimal import Decimal
from pathlib import Path

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
PRICE_PER_INPUT_TOKEN = Decimal("0.60") / Decimal(1_000_000)
DEFAULT_CAP = Decimal("3.00")
# Speaking rates used by the estimate (characters per second, deliberately slow so
# that the estimate errs on the expensive side).
CHARS_PER_SECOND = {"barks": 9.0, "speech": 10.0, "advisor": 12.0}
MIN_SECONDS = 1.2
TARGET_LUFS = -16.0
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

    @property
    def output(self) -> Path:
        """Final Ogg file."""
        return VOICE_DIR / f"{self.path}.ogg"

    @property
    def cache_key(self) -> str:
        """Key of the raw API answer in the local cache."""
        blob = "\n".join([MODEL, self.voice, self.instructions, self.text])
        return hashlib.sha256(blob.encode("utf-8")).hexdigest()[:24]

    def estimated_seconds(self) -> float:
        """Expected speech duration."""
        return max(MIN_SECONDS, len(self.text) / CHARS_PER_SECOND[self.kind])

    def estimated_cost(self) -> Decimal:
        """Expected price of the API call."""
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
        for situation, lines in situations.items():
            tone = barks["situations"][situation]["tone"]
            instructions = f"{spec['instructions']} Ton : {tone}."
            for index, line in enumerate(lines):
                jobs.append(
                    Job(
                        "barks",
                        f"barks/{line['id']}",
                        line["text"],
                        voices[index % len(voices)],
                        instructions,
                    )
                )
    return jobs


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


def speech_jobs(speeches: dict, casting: dict, cries: dict) -> list[Job]:
    """Speech sentences and war cries for every voice of every cast faction.

    Factions without their own casting use the ``default`` one: its voices say the
    default openings and closings, and the cries of every uncast faction.
    """
    jobs: dict[str, Job] = {}
    cast = casting["casting"]
    for faction, spec in cast.items():
        texts = speech_sentences(speeches, faction)
        if faction == "default":
            cry_texts = [speeches["default_cry"]]
            cry_texts += [c for f, c in cries.items() if f not in cast]
        else:
            cry_texts = [cries.get(faction, speeches["default_cry"])]
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
            for text in dict.fromkeys(cry_texts):
                job = Job(
                    "speech",
                    f"speech/{voice}/{speech_file_id(text)}",
                    text,
                    voice,
                    f"{spec['instructions']} {casting['cry_instructions']}",
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


def pending(jobs: list[Job]) -> list[Job]:
    """Jobs whose output file does not exist yet (never regenerate)."""
    return [job for job in jobs if not job.output.exists()]


# --- Synthesis and encoding ---------------------------------------------------------


def synthesise(job: Job, api_key: str) -> Path:
    """Raw WAV answer of the API for ``job`` (from the cache when present)."""
    import httpx

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    raw = CACHE_DIR / f"{job.cache_key}.wav"
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
        raise RuntimeError(f"{job.path}: HTTP {response.status_code} {response.text[:300]}")
    tmp = raw.with_suffix(".part")
    tmp.write_bytes(response.content)
    tmp.replace(raw)
    return raw


def filter_chain(reverb: bool) -> str:
    """ffmpeg audio filters: trim silences, optional room reverb, loudness."""
    trim = "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0.05"
    filters = [trim, "areverse", trim, "areverse"]
    if reverb:
        # Small vaulted room: two short early reflections, low decay.
        filters.append("aecho=0.9:0.6:37|61:0.22|0.12")
    filters.append(f"loudnorm=I={TARGET_LUFS}:TP=-1.5:LRA=11")
    filters.append("aresample=44100")
    return ",".join(filters)


def encode(raw: Path, job: Job) -> float:
    """Encode ``raw`` into the job's Ogg file; returns its duration in seconds."""
    job.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "out.ogg"
        subprocess.run(
            [
                "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
                "-i", str(raw),
                "-af", filter_chain(job.reverb),
                "-ac", "1", "-c:a", "libvorbis", "-q:a", "4",
                str(out),
            ],
            check=True,
        )  # fmt: skip
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
        print(f"{kind:8} {len(selected):4} clips  ~{seconds / 60:5.1f} min  ~{cost:.3f} $")
    print(f"total    {len(jobs):4} clips  ~{total:.3f} $")
    return total


def main(argv: list[str] | None = None) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dry-run", action="store_true", help="estimate only, no API call")
    parser.add_argument("--only", choices=["barks", "advisor", "speech"], action="append")
    parser.add_argument("--limit", type=int, default=0, help="at most N clips")
    parser.add_argument("--cap", type=Decimal, default=DEFAULT_CAP, help="cost ceiling ($)")
    args = parser.parse_args(argv)

    jobs = pending(all_jobs())
    if args.only:
        jobs = [job for job in jobs if job.kind in args.only]
    if args.limit:
        jobs = jobs[: args.limit]
    manifest = load_manifest()
    spent = sum((Decimal(str(e.get("cost_usd", 0))) for e in manifest.values()), Decimal(0))
    print(f"already generated: {len(manifest)} clips, {spent:.3f} $ (measured)")
    estimate = summarise(jobs)
    if args.dry_run or not jobs:
        return 0
    api_key = os.environ.get("OPENAI_API_KEY", "")
    if not api_key:
        print("OPENAI_API_KEY missing", file=sys.stderr)
        return 2
    if spent + estimate > args.cap:
        print(f"refused: {spent:.3f} + {estimate:.3f} $ would exceed the cap of {args.cap} $")
        return 3
    run_cost = Decimal(0)
    for index, job in enumerate(jobs, 1):
        if spent + run_cost + job.estimated_cost() > args.cap:
            print(f"stopped before the cap ({args.cap} $)")
            break
        raw = synthesise(job, api_key)
        seconds = encode(raw, job)
        # Billed audio is the raw answer, silences included.
        cost = cost_for(duration(raw), job.text, job.instructions)
        run_cost += cost
        manifest[f"{job.path}.ogg"] = {
            "text": job.text,
            "voice": job.voice,
            "model": MODEL,
            "seconds": round(seconds, 2),
            "cost_usd": float(round(cost, 5)),
        }
        save_manifest(manifest)
        print(f"[{index}/{len(jobs)}] {job.path} {seconds:.1f} s  {cost:.4f} $")
    print(f"this run: {run_cost:.3f} $ (estimated beforehand {estimate:.3f} $)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
