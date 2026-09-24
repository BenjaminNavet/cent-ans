"""Bone overrides that turn Quaternius actions into missing clips (lot V2).

Each function has the signature `pose(arm, t)` with `t` in [0, 1] over the clip and poses
bones on top of the evaluated action frame (rotations in the bone's local rest frame).
An optional attribute `frames` gives the clip length when it differs from the source.
"""


def pike_hold(arm, t):
    """Pike held forwards at waist height (placeholder: plain source action)."""


def pike_thrust(arm, t):
    """Pike push (placeholder)."""


pike_thrust.frames = 20


def bow_shoot(arm, t):
    """Longbow draw and loose (placeholder)."""


bow_shoot.frames = 36


def bow_rest(arm, t):
    """Longbow at rest (placeholder)."""


def crossbow_shoot(arm, t):
    """Crossbow aim, loose and span (placeholder)."""


crossbow_shoot.frames = 48


def crossbow_rest(arm, t):
    """Crossbow at rest (placeholder)."""
