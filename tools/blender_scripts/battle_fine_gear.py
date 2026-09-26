"""Lot FG2: fine equipment of the battle figures (helmets, armour pieces, weapons, shields).

Every builder of ``GEAR`` replaces the V2 builder of the same name
(``battle_skinned_equipment`` / ``battle_skinned_weapons`` / ``battle_skinned_cavalry``) for
the fine figures of ``battle_fine_figures``. A builder takes a `Gear` (the V2 builder
context, the fitted body's landmarks, the garment BVH trees, the level of detail) and the
recipe's keyword arguments, and returns mesh objects in world space (bind pose) bound to the
rig's bones, with the coded materials of the export pipeline (``battle_skinned.material``).

Conventions shared with V2: two-handed weapons are built in the ``Prop`` frame
(``battle_skinned_weapons.prop_frame``: origin in the right fist, Y towards the tip), shields
on the left forearm, helmets rigid on ``Head``. Painted faces (``C_ARMS``) keep the heraldry
UV (0-1 over the face); every other face gets a per-piece unwrap (``unwrap``) for the baked
maps of FG3.
"""

GEAR = {}


class Gear:
    """What a fine builder needs: V2 context, body landmarks, garment BVHs, figure."""

    def __init__(self, ctx, lm, bvhs, fig_name, mounted):
        """Store the builder inputs (`ctx.level` is the level of detail)."""
        self.ctx = ctx
        self.lm = lm
        self.bvhs = bvhs
        self.fig = fig_name
        self.mounted = mounted

    @property
    def level(self):
        """Level of detail (0 full, 1 medium, 2 far)."""
        return self.ctx.level


def gear(name):
    """Register a fine builder under the V2 builder `name`."""

    def wrap(fn):
        GEAR[name] = fn
        return fn

    return wrap


def build(name, kwargs, g):
    """Fine pieces of the V2 item `name` (None when there is no fine builder)."""
    fn = GEAR.get(name)
    if fn is None:
        return None
    return fn(g, **kwargs)
