"""Shared height field of the prototype's ground (Godot/glTF axes: Y up, camera looks toward -Z)."""

import math

SIZE_M = 80.0
RESOLUTION = 200
UV_TILE_M = 3.0


def height(x: float, z: float) -> float:
    """Return ground height in metres at (x, z): gentle rolls rising toward the back."""
    rise = max(0.0, -z - 6.0) * 0.06
    rolls = 0.35 * math.sin(x * 0.11 + 0.7) * math.cos(z * 0.09) + 0.18 * math.sin(x * 0.31 - z * 0.23)
    return rise + rolls
