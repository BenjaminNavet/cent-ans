"""Self-update of ``tools/launch.sh`` on the ``stable`` branch (ADR 0159).

Each test builds a tiny origin repository and a player clone holding the real launcher, a fake
Bevy client binary, then runs the launcher as a double-click would (ADR 0366: Bevy is the only
client).
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

LAUNCHER = Path(__file__).resolve().parents[1] / "launch.sh"
FAKE_CLIENT = "#!/bin/sh\nexit 0\n"

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
    """Player clone of ``origin`` on ``branch``, ready to launch without Rust."""
    clone = origin.parent / f"clone-{branch}"
    _git(origin.parent, "clone", "-q", "-b", branch, str(origin), str(clone))
    # Untracked, as in a real clone: the compiled client.
    binary = clone / "client" / "target" / "release" / "cent-ans"
    binary.parent.mkdir(parents=True)
    binary.write_text(FAKE_CLIENT)
    binary.chmod(0o755)
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
    client_script: str = FAKE_CLIENT,
) -> subprocess.CompletedProcess:
    """Runs the launcher of ``clone`` with a fake client, outside any CI opt-out."""
    binary = clone / "client" / "target" / "release" / "cent-ans"
    binary.write_text(client_script)
    binary.chmod(0o755)
    environment = {
        name: value
        for name, value in os.environ.items()
        if name not in ("CI", "CENT_ANS_NO_UPDATE", "CENT_ANS_RELIEF_DIR")
    }
    environment["HOME"] = str(clone.parent / "home")
    environment.update(extra_environment or {})
    return subprocess.run(
        [
            "bash",
            "tools/launch.sh",
            "--no-build",
            "--no-relief",
            "--no-models",
            *arguments,
        ],
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
    """Client arguments given after ``--`` reach the game started after the update."""
    clone = _clone(origin, "stable")
    _publish(origin)
    log = clone.parent / "client-arguments.log"
    logging_client = f'#!/bin/sh\necho "$@" >> "{log}"\n'

    result = _launch(clone, "--", "--state", "MainMenu", client_script=logging_client)

    assert result.returncode == 0, result.stderr
    assert "Mise à jour installée" in result.stdout
    assert log.read_text().splitlines()[-1].endswith("--state MainMenu")


def test_client_receives_the_repository_root(origin: Path) -> None:
    """The client is started with CENT_ANS_ROOT set to the clone (ADR 0362)."""
    clone = _clone(origin, "main")
    log = clone.parent / "bevy.log"
    script = f'#!/bin/sh\necho "root=$CENT_ANS_ROOT args=$@" >> "{log}"\n'

    result = _launch(clone, "--", "--state", "MainMenu", client_script=script)

    assert result.returncode == 0, result.stderr
    assert log.read_text().strip().endswith("args=--state MainMenu")
    assert clone.name in log.read_text()


def _relief_launch(clone: Path, store: Path) -> subprocess.CompletedProcess:
    """Launcher run with the composed-relief check on and a given pyramid store."""
    binary = clone / "client" / "target" / "release" / "cent-ans"
    binary.write_text(FAKE_CLIENT)
    binary.chmod(0o755)
    environment = {name: value for name, value in os.environ.items() if name != "CI"}
    environment["CENT_ANS_COMPOSED_PYRAMID"] = str(store)
    return subprocess.run(
        ["bash", "tools/launch.sh", "--no-build", "--no-models", "--no-update"],
        cwd=clone,
        env=environment,
        capture_output=True,
        text=True,
    )


def test_composed_pyramid_store_is_linked_when_missing(origin: Path) -> None:
    """No data/map/pyramid but a composed store: it is linked, never downloaded."""
    clone = _clone(origin, "main")
    (clone / "data" / "map").mkdir(parents=True)
    store = clone.parent / "store"
    store.mkdir()
    (store / "relief_pyramid.json").write_text("{}")

    result = _relief_launch(clone, store)

    assert result.returncode == 0, result.stderr
    assert (clone / "data" / "map" / "pyramid" / "relief_pyramid.json").is_file()
    assert (clone / "data" / "map" / "pyramid").is_symlink()


def test_missing_composed_pyramid_prints_the_generation_command(origin: Path) -> None:
    """Neither a pyramid nor a store: a clear message, the game still starts."""
    clone = _clone(origin, "main")
    (clone / "data" / "map").mkdir(parents=True)

    result = _relief_launch(clone, clone.parent / "absent-store")

    assert result.returncode == 0, result.stderr
    assert "compose-relief-real" in result.stdout
    assert "Lancement du jeu" in result.stdout


def test_old_real_pyramid_directory_is_left_alone(origin: Path) -> None:
    """A pyramid directory without manifest (old v3 package) is reported, never replaced."""
    clone = _clone(origin, "main")
    old = clone / "data" / "map" / "pyramid"
    old.mkdir(parents=True)
    (old / "package.json").write_text("{}")
    store = clone.parent / "store"
    store.mkdir()
    (store / "relief_pyramid.json").write_text("{}")

    result = _relief_launch(clone, store)

    assert (old / "package.json").exists()
    assert not old.is_symlink()
    assert "pyramid_v3_reel" in result.stdout


def test_dry_run_changes_nothing(origin: Path) -> None:
    """``--dry-run`` reports the plan and neither updates nor builds nor launches."""
    clone = _clone(origin, "stable")
    _publish(origin)
    before = _git(clone, "rev-parse", "HEAD")

    result = subprocess.run(
        ["bash", "tools/launch.sh", "--dry-run"],
        cwd=clone,
        env={name: value for name, value in os.environ.items() if name != "CI"},
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr
    assert "essai à blanc" in result.stdout
    assert _git(clone, "rev-parse", "HEAD") == before
