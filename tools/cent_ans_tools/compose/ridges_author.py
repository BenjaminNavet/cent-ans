"""Retouches d'auteur de ``ridges.json`` (lot CC, C2 révision) : chaînes de montagnes et filtre des collines.

Les crêtes issues du squelette (C1) donnent des tubes isolés et des collines en boucles. Ce module :

- retire les crêtes ``hills`` fermées, courtes ou tortueuses (les zones de collines restent portées
  par la base du relief, sans arête) ;
- ajoute des chaînes ``massif`` d'auteur (waypoints lon/lat réels projetés par la déformation) :
  Pyrénées continues, Apennins, Dinarides, Balkans, Carpates en arc, Alpes scandinaves, Massif
  central, Taurus, Pontiques, Atlas, cordillères ibériques ;
- rattache à chaque chaîne d'auteur les cols de ``crossings.json`` proches.

Idempotent : les crêtes ajoutées portent ``"authored": true`` ; une fois amorcées (``chains_seeded``)
elles sont la source et ne sont plus jamais recalculées (l'éditeur de carte CE les modifie).
"""

from __future__ import annotations

import numpy as np
from shapely.geometry import LineString, Point

#: nom -> (hauteur m, largeur px de masse, waypoints lon/lat)
CHAINS: dict[str, tuple[float, float, list[tuple[float, float]]]] = {
    "Pyrénées": (
        3100,
        80,
        [
            (-1.9, 43.2),
            (-0.8, 42.85),
            (0.4, 42.7),
            (1.5, 42.6),
            (2.5, 42.45),
            (3.15, 42.4),
        ],
    ),
    "Cordillère cantabrique": (
        2300,
        55,
        [(-8.2, 43.0), (-6.5, 43.1), (-4.7, 43.05), (-2.8, 42.95)],
    ),
    "Sistema central": (
        2300,
        55,
        [(-7.0, 40.3), (-5.5, 40.4), (-4.0, 40.8), (-3.4, 40.9)],
    ),
    "Sierra Nevada": (3200, 50, [(-4.6, 36.9), (-3.4, 37.05), (-2.4, 37.1)]),
    "Apennins": (
        1900,
        50,
        [
            (8.7, 44.4),
            (10.3, 44.2),
            (11.5, 43.9),
            (12.6, 43.4),
            (13.4, 42.6),
            (14.2, 42.0),
            (15.3, 41.4),
            (15.9, 40.4),
            (16.3, 39.4),
            (16.2, 38.5),
        ],
    ),
    "Dinarides": (
        2000,
        60,
        [
            (13.9, 45.8),
            (15.2, 44.7),
            (16.6, 43.9),
            (18.0, 43.2),
            (19.3, 42.6),
            (20.5, 41.8),
            (21.0, 40.8),
            (21.6, 39.8),
            (22.1, 39.0),
        ],
    ),
    "Balkan": (2200, 55, [(22.4, 43.4), (24.3, 42.9), (26.3, 42.8), (28.0, 42.9)]),
    "Rhodope": (2600, 55, [(23.3, 42.3), (25.0, 41.7), (26.0, 41.4)]),
    "Carpates": (
        2000,
        65,
        [
            (18.8, 49.4),
            (20.4, 49.25),
            (22.3, 49.1),
            (23.6, 48.4),
            (24.9, 48.0),
            (25.5, 47.3),
            (25.7, 46.4),
            (25.8, 45.6),
            (24.8, 45.4),
            (23.4, 45.5),
            (22.3, 45.2),
            (22.0, 44.7),
        ],
    ),
    "Alpes scandinaves": (
        2000,
        70,
        [
            (6.0, 60.4),
            (7.5, 61.6),
            (9.0, 62.4),
            (11.0, 63.4),
            (13.5, 65.0),
            (15.5, 66.0),
        ],
    ),
    "Massif central": (1750, 75, [(2.6, 45.9), (3.2, 45.2), (3.0, 44.6), (3.5, 44.1)]),
    "Taurus": (
        3000,
        55,
        [
            (27.5, 37.2),
            (30.0, 37.0),
            (32.0, 36.8),
            (34.0, 37.0),
            (36.0, 37.4),
            (38.5, 38.0),
        ],
    ),
    "Pontiques": (
        2500,
        45,
        [(31.0, 41.1), (34.0, 41.3), (37.0, 40.8), (40.0, 40.6), (42.5, 40.7)],
    ),
    "Atlas": (
        2800,
        55,
        [
            (-8.0, 31.3),
            (-5.0, 32.0),
            (-3.5, 32.8),
            (-1.0, 33.5),
            (2.0, 34.4),
            (5.0, 35.8),
            (8.0, 36.3),
        ],
    ),
}
PASS_ATTACH_PX = 70.0
HILLS_MIN_LENGTH_PX = 140.0
HILLS_MIN_CHORD_RATIO = 0.85
HILLS_MAX_TURN_DEG = 80.0


def _shape(points: list) -> tuple[float, float, float]:
    """Longueur, rapport corde/longueur et virage total (degrés) d'une polyligne."""
    p = np.asarray(points, dtype=float)
    d = np.diff(p, axis=0)
    length = float(np.hypot(*d.T).sum())
    chord = float(np.hypot(*(p[-1] - p[0])))
    ang = np.arctan2(d[:, 1], d[:, 0])
    turn = np.abs((np.diff(ang) + np.pi) % (2 * np.pi) - np.pi).sum()
    return length, chord / max(length, 1e-6), float(np.degrees(turn))


def keep_hill(ridge: dict) -> bool:
    """Vrai si la colline est une arête nette (ni boucle, ni courte, ni tortueuse)."""
    length, ratio, turn = _shape(ridge["points"])
    return (
        length >= HILLS_MIN_LENGTH_PX
        and ratio >= HILLS_MIN_CHORD_RATIO
        and turn <= HILLS_MAX_TURN_DEG
    )


def seed_chains(doc: dict, cw, crossings: list[dict]) -> int:
    """Recopie ``CHAINS`` dans ``doc`` comme crêtes ``authored`` (une seule fois) ; renvoie le nombre."""
    passes = []
    for item in crossings:
        if item.get("type") == "pass":
            x, y = cw.forward(*item["lonlat"])
            passes.append(
                {"id": item["id"], "name": item["name"], "at": (float(x), float(y))}
            )
    ridges = doc["ridges"]
    for number, (name, (height, width, waypoints)) in enumerate(
        CHAINS.items(), start=1
    ):
        pts = np.array(
            [
                [float(a), float(b)]
                for a, b in (cw.forward(lon, lat) for lon, lat in waypoints)
            ]
        )
        line = LineString(pts)
        attached = []
        for p in passes:
            if line.distance(Point(p["at"])) <= PASS_ATTACH_PX:
                at = line.interpolate(line.project(Point(p["at"])))
                attached.append(
                    {
                        "id": p["id"],
                        "name": p["name"],
                        "at": [round(at.x, 1), round(at.y, 1)],
                        "width_px": 6,
                        "depth": 0.7,
                    }
                )
        ridges.append(
            {
                "id": f"chain_{number:02d}",
                "kind": "massif",
                "authored": True,
                "name": name,
                "points": np.round(pts, 1).tolist(),
                "width_px": float(width),
                "height_m": float(height),
                "passes": attached,
            }
        )
    return len(CHAINS)


def apply(doc: dict, cw, crossings: list[dict]) -> dict:
    """Applique les retouches d'auteur à ``doc`` (``ridges.json``) ; renvoie un rapport.

    Les crêtes ``authored`` font foi : elles ne sont JAMAIS recalculées ni écrasées (l'éditeur de
    carte les modifie). ``CHAINS`` ne sert qu'à l'amorçage, une seule fois (``chains_seeded``).
    Seules les collines générées (non ``authored``) passent le filtre de forme.
    """
    ridges = doc["ridges"]
    before = sum(r["kind"] == "hills" and not r.get("authored") for r in ridges)
    ridges = [
        r for r in ridges if r.get("authored") or r["kind"] != "hills" or keep_hill(r)
    ]
    dropped = before - sum(
        r["kind"] == "hills" and not r.get("authored") for r in ridges
    )
    doc["ridges"] = ridges
    seeded = 0
    if not doc.get("chains_seeded") and not any(r.get("authored") for r in ridges):
        seeded = seed_chains(doc, cw, crossings)
    if seeded or any(r.get("authored") for r in ridges):
        doc["chains_seeded"] = True
    return {"hills_dropped": dropped, "chains": seeded}
