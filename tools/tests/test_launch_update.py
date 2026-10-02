"""Self-update of ``tools/launch.sh`` on the ``stable`` branch (ADR 0159).

Each test builds a tiny origin repository and a player clone holding the real launcher, a fake
core library and a fake Godot, then runs the launcher as a double-click would.
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

LAUNCHER = Path(__file__).resolve().parents[1] / "launch.sh"
FAKE_GODOT = '#!/bin/sh\n[ "$1" = "--version" ] && echo 4.7.2.stable.fake\nexit 0\n'

pytestmark = pytest.mark.skipif(
    sys.platform == "win32" or shutil.which("git") is None,
    reason="needs a POSIX shell and git",
)


def _git(repo: Path, *arguments: str) -> str:
    """Runs git in ``repo`` with a fixed identity and returns its output."""
    identity = ["-c", "user.name=test", "-c", "user.email=test@example.org"]
    completed = subprocess.run(
        ["git", *identity, *arguments],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def _commit(repo: Path, message: str) -> str:
    """Commits everything in ``repo`` and returns the new commit id."""
    _git(repo, "add", "-A")
    _git(repo, "commit", "-q", "-m", message)
    return _git(repo, "rev-parse", "HEAD")


@pytest.fixture
def origin(tmp_path: Path) -> Path:
    """Origin repository with one commit on ``main`` and ``stable``."""
    repo = tmp_path / "origin"
    (repo / "tools").mkdir(parents=True)
    _git(repo, "init", "-q", "-b", "main")
    shutil.copy(LAUNCHER, repo / "tools" / "launch.sh")
    (repo / "notes.txt").write_text("v1\n")
    _commit(repo, "v1")
    _git(repo, "branch", "stable")
    return repo


def _clone(origin: Path, branch: str) -> Path:
    """Player clone of ``origin`` on ``branch``, ready to launch without Rust nor Godot."""
    clone = origin.parent / f"clone-{branch}"
    _git(origin.parent, "clone", "-q", "-b", branch, str(origin), str(clone))
    # Untracked, as in a real clone: compiled core libraries and the import cache.
    library_dir = clone / "game" / "bin"
    library_dir.mkdir(parents=True)
    for name in ("libcent_ans.debug.dylib", "libcent_ans.debug.so"):
        (library_dir / name).write_text("")
    (clone / "game" / ".godot").mkdir()
    return clone


def _publish(origin: Path, branch: str = "stable") -> str:
    """Adds a commit to ``branch`` of ``origin`` that changes the notes and the launcher."""
    _git(origin, "switch", "-q", branch)
    (origin / "notes.txt").write_text("v2\n")
    with (origin / "tools" / "launch.sh").open("a") as launcher:
        launcher.write("# v2\n")
    return _commit(origin, "v2")


def _launch(
    clone: Path,
    *arguments: str,
    extra_environment: dict[str, str] | None = None,
    godot_script: str = FAKE_GODOT,
) -> subprocess.CompletedProcess:
    """Runs the launcher of ``clone`` with a fake Godot, outside any CI opt-out."""
    godot = clone.parent / "fake-godot"
    godot.write_text(godot_script)
    godot.chmod(0o755)
    environment = {
        name: value
        for name, value in os.environ.items()
        if name not in ("CI", "CENT_ANS_NO_UPDATE")
    }
    environment["GODOT"] = str(godot)
    environment.update(extra_environment or {})
    return subprocess.run(
        ["bash", "tools/launch.sh", "--no-build", "--no-relief", *arguments],
        cwd=clone,
        env=environment,
        capture_output=True,
        text=True,
    )


def test_stable_clone_installs_the_update_and_restarts(origin: Path) -> None:
    """A clone on ``stable`` fast-forwards, restarts on the new launcher and starts the game."""
    clone = _clone(origin, "stable")
    published = _publish(origin)

    result = _launch(clone)

    assert result.returncode == 0, result.stderr
    assert _git(clone, "rev-parse", "HEAD") == published
    assert "Mise à jour installée" in result.stdout
    assert result.stdout.count("Lancement du jeu") == 1


def test_up_to_date_clone_just_starts(origin: Path) -> None:
    """Nothing to install: one line says so and the game starts."""
    clone = _clone(origin, "stable")

    result = _launch(clone)

    assert result.returncode == 0, result.stderr
    assert "Le jeu est à jour" in result.stdout


def test_development_branch_is_never_touched(origin: Path) -> None:
    """A checkout on ``main`` is left alone even when ``main`` moved on origin."""
    clone = _clone(origin, "main")
    before = _git(clone, "rev-parse", "HEAD")
    _publish(origin, "main")

    result = _launch(clone)

    assert result.returncode == 0, result.stderr
    assert _git(clone, "rev-parse", "HEAD") == before
    assert "inactives" in result.stdout


@pytest.mark.parametrize(
    ("arguments", "environment"),
    [(["--no-update"], {}), ([], {"CENT_ANS_NO_UPDATE": "1"}), ([], {"CI": "true"})],
)
def test_opt_out_skips_the_update(
    origin: Path, arguments: list[str], environment: dict[str, str]
) -> None:
    """``--no-update``, ``CENT_ANS_NO_UPDATE`` and ``CI`` each skip the update."""
    clone = _clone(origin, "stable")
    before = _git(clone, "rev-parse", "HEAD")
    _publish(origin)

    result = _launch(clone, *arguments, extra_environment=environment)

    assert result.returncode == 0, result.stderr
    assert _git(clone, "rev-parse", "HEAD") == before


def test_local_change_in_an_updated_file_keeps_the_installed_version(
    origin: Path,
) -> None:
    """Git refuses to overwrite a locally edited file: nothing is lost, the game starts."""
    clone = _clone(origin, "stable")
    before = _git(clone, "rev-parse", "HEAD")
    (clone / "notes.txt").write_text("edited by the player\n")
    _publish(origin)

    result = _launch(clone)

    assert result.returncode == 0, result.stderr
    assert _git(clone, "rev-parse", "HEAD") == before
    assert (clone / "notes.txt").read_text() == "edited by the player\n"
    assert "Mise à jour non installée" in result.stdout
    assert "Lancement du jeu" in result.stdout


def test_unreachable_origin_keeps_the_installed_version(origin: Path) -> None:
    """Offline: the fetch fails, the installed version starts."""
    clone = _clone(origin, "stable")
    shutil.rmtree(origin)

    result = _launch(clone)

    assert result.returncode == 0, result.stderr
    assert "Mise à jour non vérifiée" in result.stdout
    assert "Lancement du jeu" in result.stdout


def test_arguments_survive_the_restart(origin: Path) -> None:
    """Godot arguments given after ``--`` reach the game started after the update."""
    clone = _clone(origin, "stable")
    _publish(origin)
    godot_log = clone.parent / "godot-arguments.log"
    logging_godot = FAKE_GODOT.replace("exit 0", f'echo "$@" >> "{godot_log}"\nexit 0')

    result = _launch(clone, "--", "--headless", godot_script=logging_godot)

    assert result.returncode == 0, result.stderr
    assert "Mise à jour installée" in result.stdout
    assert godot_log.read_text().splitlines()[-1].endswith("--headless")
