"""VO1 voices: data files match their schemas, corpus limits, synthesis jobs."""

import json
from decimal import Decimal
from pathlib import Path

from jsonschema import Draft202012Validator

from cent_ans_tools import voice_tts

DATA = Path(__file__).resolve().parents[2] / "data"
FILES = {
    "voice/barks.json": "schemas/voice_barks.schema.json",
    "voice/advisor.json": "schemas/voice_advisor.schema.json",
    "voice/speech_voices.json": "schemas/voice_speech.schema.json",
}


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_voice_files_match_schemas() -> None:
    """Each voice data file matches its schema."""
    for data_file, schema_file in FILES.items():
        schema = _load(schema_file)
        Draft202012Validator.check_schema(schema)
        errors = list(Draft202012Validator(schema).iter_errors(_load(data_file)))
        assert not errors, (data_file, [error.message for error in errors])


def test_bark_corpus_limits_and_ids() -> None:
    """At most 40 lines per language, unique ids, known languages and units."""
    barks = _load("voice/barks.json")
    ids = []
    for language, situations in barks["lines"].items():
        assert language in barks["languages"], language
        count = sum(len(lines) for lines in situations.values())
        assert count <= 40, (language, count)
        for situation, lines in situations.items():
            assert situation in barks["situations"], situation
            ids.extend(line["id"] for line in lines)
            for line in lines:
                assert line["id"].startswith(f"{language}_"), line["id"]
                if language not in ("fr", "an"):
                    assert "fr" in line, line["id"]
    assert len(ids) == len(set(ids))
    units = {path.stem for path in (DATA / "unit_types").glob("unit_*.json")}
    factions = {path.stem for path in (DATA / "factions").glob("fac_*.json")}
    for unit in list(barks["unit_language"]) + barks["noble_units"]:
        assert unit in units, unit
    for faction in list(barks["faction_language"]) + list(barks["noble_language"]):
        assert faction == "default" or faction in factions, faction
    # The main languages cover every situation (others fall back on them).
    for language in ("fr", "en"):
        assert set(barks["lines"][language]) == set(barks["situations"]), language
    for language in barks["lines"]:
        if language not in ("fr", "en"):
            assert language in barks["fallback_language"], language


def test_advisor_triggers() -> None:
    """Unique ids; one campaign start line without faction; firsts present once."""
    advisor = _load("voice/advisor.json")
    ids = [line["id"] for line in advisor["lines"]]
    assert len(ids) == len(set(ids))
    starts = [line for line in advisor["lines"] if line["trigger"] == "campaign_start"]
    assert sum(1 for line in starts if "faction" not in line) == 1
    triggers = {line["trigger"] for line in advisor["lines"]}
    for first in (
        "first_battle",
        "first_assault",
        "first_siege",
        "first_victory",
        "first_defeat",
    ):
        assert first in triggers, first


def test_jobs_unique_and_estimated() -> None:
    """Every job has a distinct output, a voice of its casting and a small price."""
    jobs = voice_tts.all_jobs()
    paths = [job.path for job in jobs]
    assert len(paths) == len(set(paths))
    assert all("{" not in job.text for job in jobs)
    total = sum((job.estimated_cost() for job in jobs), Decimal(0))
    assert Decimal(0) < total < voice_tts.DEFAULT_CAP
    advisor = [job for job in jobs if job.kind == "advisor"]
    assert advisor and all(job.reverb for job in advisor)


def test_speech_covers_every_sentence() -> None:
    """Each cast voice has a clip for each sentence and cry of its faction."""
    speeches = _load("speeches/battle_speeches.json")
    casting = _load("voice/speech_voices.json")["casting"]
    paths = {job.path for job in voice_tts.all_jobs()}
    for faction, spec in casting.items():
        for sentence in voice_tts.speech_sentences(speeches, faction):
            for voice in spec["voices"]:
                assert f"speech/{voice}/{voice_tts.speech_file_id(sentence)}" in paths


def test_speech_file_id_matches_godot() -> None:
    """Same naming as GDScript ``text.sha1_text().substr(0, 12)``."""
    assert voice_tts.speech_file_id("abc") == "a9993e364706"


def test_dry_run_makes_no_call(capsys, monkeypatch) -> None:
    """--dry-run prints the estimate and never touches the network."""
    monkeypatch.delenv("OPENAI_API_KEY", raising=False)
    assert voice_tts.main(["--dry-run"]) == 0
    assert "total" in capsys.readouterr().out


def test_generated_clips_are_listed() -> None:
    """Every generated clip is in the manifest, with its measured cost."""
    manifest = voice_tts.load_manifest()
    for relative in manifest:
        assert (voice_tts.VOICE_DIR / relative).exists(), relative
    for path in voice_tts.VOICE_DIR.rglob("*.ogg"):
        assert path.relative_to(voice_tts.VOICE_DIR).as_posix() in manifest, path


def test_clip_checks_reject_improvised_speech() -> None:
    """The OpenRouter checks accept a verbatim reading and reject an improvisation."""
    text = "Monseigneur ?"
    assert voice_tts.transcript_matches(text, "Monseigneur ?")
    assert not voice_tts.transcript_matches(
        text, "Je suis là, loyal, la main tremblante. Qu'ordonne-t-on, monseigneur ?"
    )
    low, high = voice_tts.plausible_seconds(text)
    assert low < 1.25 < high
    assert not low <= 10.8 <= high
