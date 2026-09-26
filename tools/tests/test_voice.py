"""VO1 voices: data files match their schemas, corpus limits, synthesis jobs."""

import json
from decimal import Decimal
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from cent_ans_tools import voice_tts
from cent_ans_tools.budget import BudgetLedger

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


def test_main_records_real_spend_including_rejected_clips(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, budget_file: Path
) -> None:
    """A rejected (billed but unusable) clip is still recorded in docs/budget.md.

    Regression for the bug where only the manifest's recomputed total was checked
    against ``--cap`` and nothing was ever written to the budget ledger, so a billed
    refusal's cost silently vanished.
    """
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    monkeypatch.setattr(voice_tts, "VOICE_DIR", tmp_path / "voice")
    monkeypatch.setattr(voice_tts, "MANIFEST", tmp_path / "manifest.json")

    job_ok = voice_tts.Job("barks", "ok", "Texte correct", "voice_a", "instr")
    job_bad = voice_tts.Job("barks", "bad", "Texte refusé", "voice_a", "instr")
    monkeypatch.setattr(voice_tts, "all_jobs", lambda: [job_ok, job_bad])

    def fake_openrouter_checked(job, _api_key, _attempts):
        if job.path == "bad":
            raise voice_tts.RejectedClip("rejected", Decimal("0.02"))
        return Path("dummy.wav"), Decimal("0.01"), job.text

    monkeypatch.setattr(voice_tts, "openrouter_checked", fake_openrouter_checked)
    monkeypatch.setattr(voice_tts, "encode", lambda _raw, _job: 1.0)

    result = voice_tts.main(["--cap", "1", "--budget-path", str(budget_file)])
    assert result == 0
    ledger = BudgetLedger(budget_file)
    # 0.01 $ for the accepted clip + 0.02 $ billed on the rejected one, not lost.
    assert ledger.entries[-1].actual == Decimal("0.03")


def test_main_recheck_keeps_forgotten_cost_in_the_recorded_spend(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, budget_file: Path
) -> None:
    """Recheck's ``forget()`` drops a bad manifest entry.

    Its past cost must still be counted, not simply disappear once the entry is
    removed.
    """
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    voice_dir = tmp_path / "voice"
    manifest_path = tmp_path / "manifest.json"
    monkeypatch.setattr(voice_tts, "VOICE_DIR", voice_dir)
    monkeypatch.setattr(voice_tts, "MANIFEST", manifest_path)
    monkeypatch.setattr(voice_tts, "CACHE_DIR", tmp_path / "cache")

    job = voice_tts.Job("barks", "redo", "Texte à refaire", "voice_a", "instr")
    monkeypatch.setattr(voice_tts, "all_jobs", lambda: [job])

    # A previous run produced a clip whose transcript now fails the check.
    voice_dir.mkdir(parents=True)
    job.output.write_bytes(b"old-clip")
    voice_tts.save_manifest(
        {
            f"{job.path}.ogg": {
                "text": job.text,
                "voice": job.voice,
                "model": voice_tts.OPENROUTER_MODEL,
                "transcript": "grognement",  # a bad transcript: fails the check
                "seconds": 1.0,
                "cost_usd": 0.05,
            }
        }
    )

    def fake_openrouter_checked(_job, _api_key, _attempts):
        return Path("dummy.wav"), Decimal("0.01"), job.text

    monkeypatch.setattr(voice_tts, "openrouter_checked", fake_openrouter_checked)
    monkeypatch.setattr(voice_tts, "encode", lambda _raw, _job: 1.0)

    result = voice_tts.main(
        ["--recheck", "--cap", "1", "--budget-path", str(budget_file)]
    )
    assert result == 0
    ledger = BudgetLedger(budget_file)
    # 0.05 $ already spent on the forgotten clip + 0.01 $ for its redo.
    assert ledger.entries[-1].actual == Decimal("0.06")
