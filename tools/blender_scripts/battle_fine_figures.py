"""Lot FG1: fine figure recipes and builder (MakeHuman body on the fine ``human`` rig).

Every Quaternius recipe of ``battle_skinned_figures.FIGURES`` gets a fine counterpart: the
Quaternius body parts (the clothed modular men) are replaced by the fitted MakeHuman body
and garment shells (``OUTFITS``), the scripted equipment is rebuilt on the new body, faces
vary by variant, and the figure is exported to ``CAM1`` with colours and material codes.
"""


def build_all(manifest, only=None):
    """Build every fine figure (or those in `only`) and record them in `manifest`."""
    raise NotImplementedError


def check_poses(out):
    """Render the computed poses on the fitted body into `out` (visual check)."""
    raise NotImplementedError
