"""Free local 2D stages of the GA3 chains (ADR 0190, lot B).

Shared by ``ga3_fal_figure.py`` and ``ga3_fal_decor.py`` (``--sheet-backend local`` and
``--cut-backend local``). No fal call, no entry in ``costs.json``:

* sheet / image: :func:`cent_ans_tools.local_art.render_image` (Z-Image Turbo, mflux), the
  source image being the img2img start;
* background removal: ``rembg`` (MIT; weights downloaded once into ``~/.rembg/models``, outside the
  repository).

Run a chain with the local 2D stages (only TRELLIS stays paid)::

    uv run --with rembg --with onnxruntime --with fal-client --with pillow --with numpy \
        python tools/experiments/ga3_fal_figure.py RAW_DIR --unit longbowman \
        --sheet-backend local --cut-backend local [--strength 0.45]
"""

import io
import re
from pathlib import Path

from PIL import Image

BACKENDS = ("fal", "local")
SHEET_ASPECT = "3:2"
SHEET_STRENGTH = 0.45
VIEW_STRENGTH = 0.55  # decor side/back views: more influence from the source
REMBG_MODEL = "isnet-general-use"
# The edit instruction is meaningless to a text-to-image model: it is dropped.
_EDIT_SENTENCE = re.compile(r"Edit this photographic[^.]*\.\s*")
_BACKGROUND = re.compile(
    r"plain uniform neutral mid-grey|plain mid-grey|neutral mid-grey"
)


def local_sheet_prompt(fal_prompt: str) -> str:
    """Descriptive prompt for the local model from the nano-banana edit prompt.

    Same man, strict A-pose, empty hands, front / left profile / back views, key colours kept;
    the background becomes plain white (easier to cut out with rembg).
    """
    body = _EDIT_SENTENCE.sub("", fal_prompt)
    body = _BACKGROUND.sub("plain pure white", body)
    return (
        "Photographic character reference sheet of one man, shown three times side by side "
        "on a plain pure white background: front view, left side view, back view. "
        "Strict A-pose in every view: arms straight and held 30 degrees away from the body, "
        "hands open and empty, nothing held, no bow, no weapon in hand. " + body
    ).strip()


def check_backend(name: str) -> str:
    """``name`` if it is a known backend, else ``ValueError``."""
    if name not in BACKENDS:
        raise ValueError(f"backend inconnu : {name}")
    return name


def render_local(
    prompt: str,
    output: Path,
    *,
    reference: Path | None = None,
    aspect_ratio: str | None = None,
    seed: int | None = None,
    strength: float | None = None,
) -> Path:
    """Write the local image to ``output`` unless it exists (cache), return the path."""
    if output.exists():
        return output
    from cent_ans_tools import local_art

    data = local_art.render_image(
        prompt,
        aspect_ratio=aspect_ratio,
        reference=reference.read_bytes() if reference is not None else None,
        seed=seed,
        strength=strength,
    )
    output.write_bytes(data)
    print(f"GA3 local image -> {output}")
    return output


def cut_local(source: Path, output: Path, remover=None) -> Path:
    """Background removal with rembg into ``output`` (RGBA PNG), cached.

    ``remover(image) -> image`` replaces rembg (tests).
    """
    if output.exists():
        return output
    image = Image.open(source).convert("RGB")
    if remover is None:
        from rembg import new_session, remove

        session = new_session(REMBG_MODEL)

        def remover(img):
            return remove(img, session=session)

    cut = remover(image)
    if not isinstance(cut, Image.Image):
        cut = Image.open(io.BytesIO(cut))
    cut.convert("RGBA").save(output)
    print(f"GA3 local cut-out -> {output}")
    return output
