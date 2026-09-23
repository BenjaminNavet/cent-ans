"""Run Blender in background mode with a Python script."""

from __future__ import annotations

import subprocess
from pathlib import Path

BLENDER_BIN = Path("/opt/homebrew/bin/blender")
SCRIPTS_DIR = Path(__file__).resolve().parents[1] / "blender_scripts"
MODELS_DIR = Path(__file__).resolve().parents[2] / "game" / "assets" / "models"
MAX_TRIANGLES = 3000


class BlenderError(RuntimeError):
    """Raised when Blender exits with a non-zero status."""


def run_blender_script(
    script_path: Path | str, *args: str, timeout: float = 600
) -> str:
    """Invoke ``blender --background --python <script> -- <args>`` and return stdout.

    Arguments after ``--`` are ignored by Blender and available to the script
    through ``sys.argv[sys.argv.index("--") + 1:]``.
    """
    command = [
        str(BLENDER_BIN),
        "--background",
        "--python",
        str(script_path),
        "--",
        *args,
    ]
    completed = subprocess.run(
        command, capture_output=True, text=True, timeout=timeout, check=False
    )
    if completed.returncode != 0:
        raise BlenderError(
            f"Blender a échoué (code {completed.returncode}) :\n{completed.stdout}\n{completed.stderr}"
        )
    return completed.stdout


def smoke() -> str:
    """Run the bundled smoke script (cube -> temporary glTF) and return stdout."""
    return run_blender_script(SCRIPTS_DIR / "smoke.py")


def parse_model_counts(output: str) -> dict[str, int]:
    """Read the ``MODEL <name> <triangles>`` lines printed by ``models.py``."""
    counts: dict[str, int] = {}
    for line in output.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[0] == "MODEL":
            counts[parts[1]] = int(parts[2])
    return counts


def build_models(out_dir: Path | str = MODELS_DIR, *names: str) -> dict[str, int]:
    """Export the low-poly models to ``out_dir`` and return their triangle counts.

    Raises:
        BlenderError: if Blender fails, ``OK`` is missing, or a model has
            ``MAX_TRIANGLES`` triangles or more.
    """
    output = run_blender_script(SCRIPTS_DIR / "models.py", str(out_dir), *names)
    counts = parse_model_counts(output)
    if "OK" not in output.splitlines() or not counts:
        raise BlenderError(f"models.py n'a pas terminé :\n{output}")
    too_big = {name: count for name, count in counts.items() if count >= MAX_TRIANGLES}
    if too_big:
        raise BlenderError(f"Modèles trop lourds : {too_big}")
    return counts
