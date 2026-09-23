"""Run Blender in background mode with a Python script."""

from __future__ import annotations

import subprocess
from pathlib import Path

BLENDER_BIN = Path("/opt/homebrew/bin/blender")
SCRIPTS_DIR = Path(__file__).resolve().parents[1] / "blender_scripts"


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
