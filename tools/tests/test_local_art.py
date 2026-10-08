"""Tests of the free local mflux backend (no model run: fake runner)."""

import io
import subprocess
from pathlib import Path

from PIL import Image

from cent_ans_tools import entry_art, local_art
from cent_ans_tools.portraits import PortraitJob


def _fake_runner(command: list[str]) -> None:
    output = Path(command[command.index("--output") + 1])
    Image.new("RGB", (local_art.WIDTH, local_art.HEIGHT), "navy").save(output)


def test_generate_writes_converted_miniature(tmp_path):
    """The raw mflux image is cropped and saved as the miniature."""
    job = PortraitJob("fac_test", "A realm", tmp_path / "fac_test.jpg")
    written, failed = local_art.generate([job], entry_art.convert, runner=_fake_runner)
    assert written == [job.out_path]
    assert failed == []
    image = Image.open(io.BytesIO(job.out_path.read_bytes()))
    assert image.size == (entry_art.ART_WIDTH, entry_art.ART_HEIGHT)


def test_failed_job_is_reported_and_batch_continues(tmp_path):
    """A failing job is reported without stopping the batch."""
    jobs = [
        PortraitJob("fac_bad", "A", tmp_path / "fac_bad.jpg"),
        PortraitJob("fac_good", "B", tmp_path / "fac_good.jpg"),
    ]

    def runner(command: list[str]) -> None:
        if "fac_bad" in command[command.index("--output") + 1]:
            raise subprocess.CalledProcessError(1, command)
        _fake_runner(command)

    written, failed = local_art.generate(jobs, entry_art.convert, runner=runner)
    assert written == [jobs[1].out_path]
    assert [job.character_id for job, _ in failed] == ["fac_bad"]


def test_command_uses_stable_seed_and_reference(tmp_path):
    """The seed is stable and the reference image is passed."""
    reference = tmp_path / "ref.png"
    job = PortraitJob("fac_x", "p", tmp_path / "x.jpg", reference=reference)
    command = local_art.command_for(job, tmp_path / "p.txt", tmp_path / "o.png")
    assert command[command.index("--seed") + 1] == str(local_art.seed_for(job))
    assert local_art.seed_for(job) == local_art.seed_for(job)
    assert command[command.index("--image-path") + 1] == str(reference)
