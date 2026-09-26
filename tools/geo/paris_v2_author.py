"""One-off authoring of data/landmarks_v2/paris.json (lot VH5) from ALPAGE geometry + hand facts.

Kept for review and reproducibility: every wall, gate, monument footprint, district and the
medieval Seine are taken from ALPAGE "Paris en 1380" (ODbL, © ALPAGE : P. Rouet) through the
feature ids (``fid``) of its land-use layer; dates, heights and certainty are hand facts
(docs/research/vh-sources.md). Running it OVERWRITES paris.json; afterwards the file is maintained
by hand and ``cent-ans geo landmarks --city paris`` regenerates the ALPAGE streets and parcels.

    uv run --project tools python tools/geo/paris_v2_author.py
    uv run --project tools cent-ans geo landmarks --city paris
"""

# ruff: noqa: D103

import math
import sys
from pathlib import Path

import numpy as np
from shapely.geometry import LineString, Point, Polygon
from shapely.ops import unary_union

from cent_ans_tools.geo import alpage, landmarks_v2

REPO = Path(__file__).resolve().parents[2]
ORIGIN = [3760563, 2889108]  # Notre-Dame (ALPAGE footprint centroid)
EXTENT = 2300.0
CLIP = Point(0, 0).buffer(EXTENT - 60.0)

usage = {r["fid"]: r for r in alpage.read_layer("1380_usages_sol")}
ilots = alpage.read_layer("1380_ilots")
G = {fid: alpage.to_local(r["geom"], ORIGIN).buffer(0) for fid, r in usage.items()}


def rnd(p):
    return [round(float(p[0])), round(float(p[1]))]


def centroid(*fids):
    g = unary_union([G[f] for f in fids])
    c = g.centroid
    return np.array([c.x, c.y])


def axis(fid, eastward=True):
    """Centre, angle (deg, v2), long side, short side of the ALPAGE footprint."""
    g = G[fid]
    m = list(g.minimum_rotated_rectangle.exterior.coords)
    a = math.dist(m[0], m[1])
    b = math.dist(m[1], m[2])
    p, q = (m[0], m[1]) if a >= b else (m[1], m[2])
    ang = math.degrees(math.atan2(q[1] - p[1], q[0] - p[0]))
    if eastward and not -90.0 <= ang <= 90.0:
        ang = ang + 180.0 if ang < 0 else ang - 180.0
    c = g.centroid
    return [round(c.x), round(c.y)], round(ang), max(a, b), min(a, b)


def centerline(fid, step=15.0):
    """Centre line of a thin wall polygon: boundary points binned along its long axis."""
    g = G[fid]
    m = list(g.minimum_rotated_rectangle.exterior.coords)
    a = math.dist(m[0], m[1])
    b = math.dist(m[1], m[2])
    if a >= b:
        p0 = (np.array(m[0]) + np.array(m[3])) / 2
        p1 = (np.array(m[1]) + np.array(m[2])) / 2
    else:
        p0 = (np.array(m[1]) + np.array(m[0])) / 2
        p1 = (np.array(m[2]) + np.array(m[3])) / 2
    u = (p1 - p0) / np.linalg.norm(p1 - p0)
    v = np.array([-u[1], u[0]])
    length = float(np.linalg.norm(p1 - p0))
    pts = []
    for poly in getattr(g, "geoms", [g]):
        ring = LineString(poly.exterior.coords)
        for s in np.arange(0.0, ring.length, 1.0):
            pts.append(np.array(ring.interpolate(s).coords[0]))
    pts = np.array(pts)
    su = (pts - p0) @ u
    sv = (pts - p0) @ v
    n = max(1, int(round(length / step)))
    out = []
    for k in range(n + 1):
        t = length * k / n
        sel = np.abs(su - t) < max(step * 0.6, 3.0)
        off = float(np.mean(sv[sel])) if sel.any() else 0.0
        out.append(p0 + u * t + v * off)
    return out


def chain(items):
    """Wall polyline from ordered items: ("pt", x, y) | ("wall", fid) | ("gate", name, kind, fids)."""
    pts = []
    gates = []
    for it in items:
        if it[0] == "pt":
            pts.append(np.array(it[1:], dtype=float))
        elif it[0] == "wall":
            line = centerline(it[1])
            if pts and np.linalg.norm(line[-1] - pts[-1]) < np.linalg.norm(
                line[0] - pts[-1]
            ):
                line = line[::-1]
            pts.extend(line)
        else:
            c = centroid(*it[3])
            pts.append(c)
            gates.append({"name": it[1], "at": rnd(c), "kind": it[2]})
    # Drop points too close to the previous one.
    clean = [pts[0]]
    for p in pts[1:]:
        if np.linalg.norm(p - clean[-1]) > 4.0:
            clean.append(p)
    line = LineString(clean).simplify(1.5)
    return [rnd(p) for p in line.coords], gates


# --- Walls --------------------------------------------------------------------------------------

pa_right, pa_right_gates = chain(
    [
        ("pt", *centroid(441)),
        ("wall", 65),
        ("wall", 436),
        ("wall", 429),
        ("wall", 442),
        ("gate", "Porte Saint-Honoré (dite aux Aveugles)", "gate", (439, 470)),
        ("wall", 468),
        ("gate", "Poterne Coquillière", "postern", (471, 472)),
        ("wall", 475),
        ("gate", "Porte Montmartre", "gate", (19, 474)),
        ("wall", 497),
        ("gate", "Porte de la Comtesse d'Artois", "postern", (495,)),
        ("wall", 490),
        ("gate", "Porte Saint-Denis", "gate", (171, 491)),
        ("wall", 568),
        ("wall", 570),
        ("gate", "Porte Saint-Martin", "gate", (167, 566)),
        ("wall", 1020),
        ("gate", "Poterne Nicolas-Huidelon", "postern", (1022,)),
        ("wall", 1025),
        ("gate", "Porte du Temple (porte de Braque)", "gate", (199, 1023)),
        ("wall", 1029),
        ("gate", "Poterne du Chaume", "postern", (207, 1027)),
        ("wall", 1033),
        ("gate", "Porte Barbette", "gate", (985, 1031)),
        ("wall", 1035),
        ("wall", 1037),
        ("wall", 1038),
        ("wall", 1039),
        ("gate", "Porte Baudoyer (Saint-Antoine)", "gate", (206, 980)),
        ("wall", 982),
        ("gate", "Archet Saint-Paul", "postern", (979, 984)),
        ("wall", 989),
        ("gate", "Poterne des Béguines (des Barrés)", "postern", (993,)),
        ("wall", 994),
        ("pt", *centroid(239)),
    ]
)
pa_left, pa_left_gates = chain(
    [
        ("pt", *centroid(786)),
        ("wall", 787),
        ("wall", 793),
        ("wall", 790),
        ("wall", 791),
        ("wall", 727),
        ("gate", "Porte de Buci", "gate", (784, 785)),
        ("wall", 730),
        ("gate", "Porte Saint-Germain (des Cordeliers)", "gate", (725, 729)),
        ("wall", 722),
        ("wall", 723),
        ("wall", 724),
        ("gate", "Porte Saint-Michel (d'Enfer)", "gate", (950, 951)),
        ("wall", 761),
        ("wall", 747),
        ("gate", "Porte Saint-Jacques", "gate", (759, 854)),
        ("wall", 855),
        ("gate", "Porte Papale (Sainte-Geneviève)", "gate", (347, 857)),
        ("wall", 861),
        ("gate", "Porte Bordelle (Saint-Marcel)", "gate", (361, 858)),
        ("wall", 877),
        ("wall", 878),
        ("gate", "Porte Saint-Victor", "gate", (360, 875)),
        ("wall", 894),
        ("wall", 876),
        ("pt", *centroid(890)),
    ]
)
cv_items = [
    ("pt", *centroid(440)),
    ("wall", 1537),
    ("gate", "Porte Saint-Honoré", "gate", (455,)),
    ("wall", 443),
    ("gate", "Porte Montmartre", "gate", (456,)),
    ("wall", 444),
    ("gate", "Porte Saint-Denis", "gate", (457,)),
    ("wall", 445),
    ("gate", "Porte Saint-Martin", "gate", (1018,)),
    ("wall", 446),
    ("gate", "Porte du Temple", "gate", (1017,)),
    ("wall", 447),
    ("wall", 448),
    ("wall", 449),
    ("wall", 450),
    ("gate", "Porte Saint-Antoine", "gate", (1014,)),
    ("wall", 451),
    ("pt", *centroid(453)),
    ("wall", 452),
    ("wall", 454),
    ("pt", *centroid(239)),
]
cv, cv_gates = chain(cv_items)

walls = [
    {
        "id": "philippe_auguste_droite",
        "name": "Enceinte de Philippe Auguste (rive droite, 1190-1209)",
        "points": pa_right,
        "closed": False,
        "height_m": 9.0,
        "thickness_m": 3.0,
        "tower_spacing_m": 62.0,
        "tower_radius_m": 3.2,
        "tower_height_m": 15.0,
        "ditch_m": 0.0,
        "gates": pa_right_gates,
        "note": "Tracé, portes et poternes : ALPAGE, Paris en 1380 (P. Rouet), couche des usages du sol (murs, portes, tours). Du Louvre (tour du Coin) à la Seine en amont (tour Barbeau). Sans fossé en 1340 : les fossés sont creusés en 1356-1358 (Étienne Marcel). Tours : ≈ 15 m, espacées d'≈ 60 m (77 tours pour 5 100 m).",
    },
    {
        "id": "philippe_auguste_gauche",
        "name": "Enceinte de Philippe Auguste (rive gauche, vers 1200-1215)",
        "points": pa_left,
        "closed": False,
        "height_m": 9.0,
        "thickness_m": 3.0,
        "tower_spacing_m": 62.0,
        "tower_radius_m": 3.2,
        "tower_height_m": 15.0,
        "ditch_m": 0.0,
        "gates": pa_left_gates,
        "note": "Tracé et portes : ALPAGE (1380). De la tour de Nesle au château de la Tournelle. Fossés de 1356-1358 non représentés en 1340.",
    },
    {
        "id": "charles_v_levee",
        "name": "Enceinte de Charles V (fossés et levée de terre, 1356-1364)",
        "points": cv,
        "closed": False,
        "height_m": 5.0,
        "thickness_m": 8.0,
        "tower_spacing_m": 0.0,
        "tower_radius_m": 0.0,
        "tower_height_m": 0.0,
        "ditch_m": 24.0,
        "from_year": 1356,
        "until_year": 1364,
        "gates": [dict(g, height_m=10) for g in cv_gates],
        "note": "Tracé ALPAGE (1380). Fossés et levée de terre entrepris sous Étienne Marcel (1356-1358) ; gabarit restitué (hypothèse).",
    },
    {
        "id": "charles_v",
        "name": "Enceinte de Charles V (rive droite, maçonnée 1365-1383)",
        "points": cv,
        "closed": False,
        "height_m": 10.0,
        "thickness_m": 4.0,
        "tower_spacing_m": 110.0,
        "tower_radius_m": 4.5,
        "tower_height_m": 16.0,
        "ditch_m": 28.0,
        "from_year": 1365,
        "gates": [dict(g, height_m=18) for g in cv_gates],
        "note": "Tracé ALPAGE (1380), de la tour de Bois (Seine en aval) à la tour de Billy puis le long de la Seine jusqu'à la tour Barbeau. Maçonnerie de 1365 à ≈ 1383 : la date d'apparition retenue (1365) est celle du début des travaux ; gabarits de tours et de fossés restitués.",
    },
]

# --- Districts ---------------------------------------------------------------------------------


def region_blocks(region, inside=None, outside=None):
    polys = []
    for r in ilots:
        if r["REGION"] != region:
            continue
        g = alpage.to_local(r["geom"], ORIGIN).buffer(0)
        c = g.representative_point()
        if inside is not None and not inside.contains(c):
            continue
        if outside is not None and outside.contains(c):
            continue
        polys.append(g)
    return polys


def as_district_polygon(polys, grow=7.0, simplify=4.0):
    u = unary_union([p.buffer(grow, join_style=2) for p in polys]).buffer(
        -grow * 0.5, join_style=2
    )
    u = u.intersection(CLIP)
    parts = sorted(getattr(u, "geoms", [u]), key=lambda g: -g.area)
    return parts


def ring_of(poly, simplify=4.0):
    p = poly.simplify(simplify)
    return [rnd(q) for q in list(p.exterior.coords)[:-1]]


# Right bank inside Philippe Auguste: wall line closed well to the south.
pa_r = Polygon(pa_right + [[pa_right[-1][0], -700], [pa_right[0][0], -700]]).buffer(0)
pa_l = Polygon(pa_left + [[pa_left[-1][0], 1100], [pa_left[0][0], 1100]]).buffer(0)
cv_poly = Polygon(cv + [[-1158, 600]]).buffer(0)

ville = as_district_polygon(
    region_blocks("RD", inside=pa_r) + region_blocks("PONT", inside=pa_r)
)[0]
univ = as_district_polygon(region_blocks("RG", inside=pa_l))[0]
cite = as_district_polygon(region_blocks("CITE"))[0]

districts = [
    {
        "id": "cite",
        "name": "La Cité",
        "zone": "intra",
        "density": 0.95,
        "polygon": ring_of(cite),
        "houses": {"townhouse": 0.55, "stonehouse": 0.25, "timber": 0.2},
    },
    {
        "id": "ville",
        "name": "La Ville (rive droite, dans l'enceinte de Philippe Auguste)",
        "zone": "intra",
        "density": 0.93,
        "polygon": ring_of(ville),
        "houses": {"townhouse": 0.55, "timber": 0.3, "stonehouse": 0.15},
    },
    {
        "id": "universite",
        "name": "L'Université (rive gauche, dans l'enceinte de Philippe Auguste)",
        "zone": "intra",
        "density": 0.85,
        "polygon": ring_of(univ),
        "houses": {"townhouse": 0.45, "stonehouse": 0.3, "timber": 0.25},
    },
]

# Faubourgs: ALPAGE built zones (1380) outside Philippe Auguste's walls.
built = []
for fid, r in usage.items():
    if r["LEGENDE"] != "Zones baties, clos" or r["REGION"] not in ("RD", "RG"):
        continue
    g = G[fid]
    c = g.representative_point()
    if pa_r.contains(c) and r["REGION"] == "RD":
        continue
    if pa_l.contains(c) and r["REGION"] == "RG":
        continue
    built.append(g)
names = [
    ((-1150, 150), "Bourg Saint-Germain-des-Prés"),
    ((-1250, 700), "Bourg Saint-Germain (vers le Pré-aux-Clercs)"),
    ((-900, 1250), "Faubourg Saint-Honoré"),
    ((-350, 1500), "Faubourg Montmartre"),
    ((250, 1500), "Faubourgs Saint-Denis et Saint-Martin"),
    ((900, 1300), "Culture du Temple"),
    ((900, 500), "Marais et Saint-Paul (entre les deux enceintes)"),
    ((1700, -300), "Faubourg Saint-Antoine"),
    ((300, -900), "Saint-Victor"),
    ((0, -1700), "Bourg Saint-Marcel"),
    ((-700, -1100), "Faubourg Saint-Jacques"),
    ((-350, -1300), "Faubourg Saint-Médard"),
]
parts = as_district_polygon(built, grow=6.0)
k = 0
for part in parts:
    if part.area < 2500.0:
        continue
    c = part.representative_point()
    name = min(names, key=lambda n: math.dist(n[0], (c.x, c.y)))[1]
    between = cv_poly.contains(c)
    k += 1
    districts.append(
        {
            "id": f"faubourg_{k:02d}",
            "name": name,
            "zone": "faubourg",
            "density": 0.55 if between else 0.45,
            "polygon": ring_of(part, 5.0),
        }
    )
print(
    "districts",
    len(districts),
    sum(len(d["polygon"]) for d in districts),
    file=sys.stderr,
)

# --- Open spaces -------------------------------------------------------------------------------


def space(sid, name, kind, fids=None, poly=None, **kw):
    if fids is not None:
        g = unary_union([G[f] for f in fids]).buffer(0)
        g = max(getattr(g, "geoms", [g]), key=lambda x: x.area)
        poly = ring_of(g, 2.0)
    d = {"id": sid, "name": name, "kind": kind}
    d.update(kw)
    d["polygon"] = poly
    return d


def field_at(x, y, box=None):
    for fid, r in usage.items():
        if r["LEGENDE"] == "Champs et cultures, terrains vagues" and G[fid].contains(
            Point(x, y)
        ):
            g = G[fid]
            if box is not None:
                g = g.intersection(Polygon(box))
                g = max(getattr(g, "geoms", [g]), key=lambda q: q.area)
            return ring_of(g, 4.0)
    raise KeyError((x, y))


nd_c, nd_ang, _, _ = axis(271)
u = np.array([math.cos(math.radians(nd_ang)), math.sin(math.radians(nd_ang))])
vv = np.array([-u[1], u[0]])
west = np.array(nd_c) - u * 64.0
pc = west - u * 22.0
parvis = [
    rnd(pc + u * a + vv * b) for a, b in ((-20, -22), (20, -22), (20, 22), (-20, 22))
]

cemeteries = [
    (fid, r["TOPONYME"]) for fid, r in usage.items() if r["LEGENDE"] == "Cimetieres"
]
open_spaces = [
    {
        "id": "parvis_notre_dame",
        "name": "Parvis Notre-Dame",
        "kind": "parvis",
        "polygon": parvis,
    },
    {
        "id": "greve",
        "name": "Place et port de Grève",
        "kind": "strand",
        "polygon": [
            [165, 300],
            [212, 340],
            [255, 334],
            [300, 300],
            [330, 252],
            [285, 230],
            [205, 268],
        ],
    },
    space("champeaux", "Les Champeaux (marché des Halles)", "market", fids=[29]),
    space("jardin_du_roi", "Jardin du roi (palais de la Cité)", "garden", fids=[508]),
    space("cour_du_mai", "Cour du Mai (palais de la Cité)", "square", fids=[542]),
    {
        "id": "ile_notre_dame",
        "name": "Île Notre-Dame (prés)",
        "kind": "meadow",
        "polygon": ring_of(G[362], 4.0),
    },
    {
        "id": "ile_aux_vaches",
        "name": "Île aux Vaches (pâtures)",
        "kind": "meadow",
        "polygon": ring_of(G[363], 4.0),
    },
    {
        "id": "ile_louviers",
        "name": "Île Louviers (île aux Javiaux)",
        "kind": "meadow",
        "polygon": ring_of(G[364], 4.0),
    },
    {
        "id": "ile_du_patriarche",
        "name": "Îlots en aval de la Cité (île du Patriarche, île aux Juifs)",
        "kind": "meadow",
        "polygon": ring_of(G[389], 2.0),
    },
    {
        "id": "petit_pre_aux_clercs",
        "name": "Petit Pré-aux-Clercs",
        "kind": "meadow",
        "polygon": ring_of(G[1430], 4.0),
    },
    {
        "id": "grand_pre_aux_clercs",
        "name": "Grand Pré-aux-Clercs",
        "kind": "meadow",
        "polygon": field_at(
            -1400, 650, box=[(-2100, 200), (-1150, 200), (-1150, 1000), (-2100, 1000)]
        ),
    },
]
for fid, name in cemeteries:
    c = G[fid].representative_point()
    if Point(c.x, c.y).distance(Point(0, 0)) > EXTENT - 80:
        continue
    open_spaces.append(
        space(
            f"cimetiere_{fid}",
            name.replace("Cimetiere", "Cimetière").replace("Saints", "Saints-"),
            "cemetery",
            fids=[fid],
        )
    )

# --- Monuments ---------------------------------------------------------------------------------

monuments = []


def church(
    mid,
    name,
    fid,
    cert="attested",
    desc=None,
    length=None,
    width=None,
    height=None,
    tower=None,
    apse="flat",
    model="church",
    **dates,
):
    at, ang, L, W = axis(fid)
    L = length or L
    W = width or min(W, L * 0.45)
    H = height or float(np.clip(8 + L * 0.12, 9, 19))
    params = {
        "length_m": round(L),
        "width_m": round(W),
        "height_m": round(H),
        "apse": apse,
    }
    if tower is None and L >= 35:
        tower = {"at": "west", "size": 7, "height": round(H + 14)}
    if tower:
        params["tower"] = tower
    m = {
        "id": mid,
        "name": name,
        "model": model,
        "at": at,
        "angle_deg": ang,
        "clear_m": 5,
        "certainty": cert,
        "params": params,
    }
    m.update(dates)
    if desc:
        m["description"] = desc
    monuments.append(m)


def boxy(
    mid,
    name,
    fid,
    model,
    cert="probable",
    desc=None,
    eastward=False,
    params=None,
    **dates,
):
    at, ang, L, W = axis(fid, eastward=eastward)
    p = {"length_m": round(L), "width_m": round(W)}
    p.update(params or {})
    m = {
        "id": mid,
        "name": name,
        "model": model,
        "at": at,
        "angle_deg": ang,
        "clear_m": 4,
        "certainty": cert,
        "params": p,
    }
    m.update(dates)
    if desc:
        m["description"] = desc
    monuments.append(m)


def tower(
    mid, name, fid, radius, height, cert="attested", desc=None, model="tower", **dates
):
    at = rnd(centroid(fid))
    m = {
        "id": mid,
        "name": name,
        "model": model,
        "at": at,
        "angle_deg": 0,
        "clear_m": 3,
        "certainty": cert,
        "params": {"radius_m": radius, "height": height},
    }
    m.update(dates)
    if desc:
        m["description"] = desc
    monuments.append(m)


def ring_local(fid, at, ang, simplify=4.0):
    g = G[fid]
    g = max(getattr(g, "geoms", [g]), key=lambda q: q.area).simplify(simplify)
    a = math.radians(ang)
    ux, uy = math.cos(a), math.sin(a)
    out = []
    for x, y in list(g.exterior.coords)[:-1]:
        dx, dy = x - at[0], y - at[1]
        out.append([round(dx * ux + dy * uy), round(-dx * uy + dy * ux)])
    return out


# Notre-Dame.
monuments.append(
    {
        "id": "notre_dame",
        "name": "Cathédrale Notre-Dame",
        "model": "gothic_cathedral",
        "at": nd_c,
        "angle_deg": nd_ang,
        "clear_m": 10,
        "certainty": "attested",
        "params": {
            "length_m": 128,
            "width_m": 48,
            "nave_width_m": 12,
            "vault_m": 33,
            "ridge_m": 43,
            "aisle_m": 20,
            "transept_m": 48,
            "transept_width_m": 14,
            "transept_at": 0.47,
            "apse": "round",
            "chapels": True,
            "west_towers": [
                {"side": "north", "size": 15, "height": 69, "top": "flat"},
                {"side": "south", "size": 15, "height": 69, "top": "flat"},
            ],
            "crossing": {"size": 6, "height": 45, "spire_m": 33},
        },
        "description": "Chantier de 1163 ; chœur 1163-1182, nef 1182-1190, façade 1190-1225 ; tours de 69 m ; chapelles et derniers travaux vers 1345. Flèche de la croisée posée vers 1250 (hauteur restituée, ≈ 78 m). Emprise : ALPAGE (128 m).",
    }
)
church(
    "sainte_chapelle",
    "Sainte-Chapelle",
    514,
    length=36,
    width=17,
    height=30,
    apse="round",
    tower={"at": "crossing", "size": 3, "height": 42, "spire_m": 33},
    desc="Chapelle palatine consacrée en 1248 : 36 × 17 m, 42,5 m au faîte ; flèche de bois (la première, remplacée vers 1383).",
)
boxy(
    "palais_grand_salle",
    "Palais de la Cité : Grand-Salle (Philippe le Bel)",
    525,
    "royal_palace",
    cert="attested",
    params={"height_m": 18, "length_m": 70, "width_m": 27},
    desc="Grand-Salle à deux nefs reconstruite par Philippe le Bel (1301-1315), ≈ 70 × 27 m.",
)
boxy(
    "palais_logis",
    "Palais de la Cité : logis du roi",
    539,
    "royal_palace",
    params={"height_m": 16, "ranges": [[-30, 22, 40, 10, 12, 0]]},
)
boxy(
    "palais_conciergerie",
    "Palais de la Cité : salles basses et galerie sur la Seine",
    544,
    "royal_palace",
    params={"height_m": 16, "width_m": 16},
)
tower(
    "tour_cesar",
    "Tour de César (tour criminelle)",
    545,
    5.0,
    30.0,
    desc="Tours rondes du quai de l'Horloge (Philippe le Bel).",
)
tower("tour_argent", "Tour d'Argent (tour civile)", 546, 5.0, 30.0)
tower(
    "tour_bonbec",
    "Tour Bonbec (tour des Réformateurs)",
    541,
    5.0,
    26.0,
    desc="Tour de Saint Louis (XIIIe s.).",
)
monuments.append(
    {
        "id": "tour_horloge",
        "name": "Tour de l'Horloge",
        "model": "belfry",
        "at": rnd(centroid(543)),
        "angle_deg": axis(543, False)[1],
        "clear_m": 2,
        "certainty": "attested",
        "from_year": 1350,
        "params": {"size": 10, "height": 47, "top": "pyramid"},
        "description": "Construite 1350-1353 (Jean le Bon), horloge en 1371 : absente en 1340.",
    }
)
boxy(
    "hotel_dieu",
    "Hôtel-Dieu",
    599,
    "enclosure",
    params={"height_m": 14, "ranges": True},
    desc="Salles des malades le long du bras sud de la Seine, au sud du parvis ; emprise ALPAGE.",
)
boxy(
    "eveche",
    "Palais de l'évêque",
    613,
    "royal_palace",
    params={"height_m": 16, "width_m": 14, "ranges": [[20, 20, 30, 10, 12, 90]]},
)
# Châtelets.
at, ang, L, W = axis(551, False)
monuments.append(
    {
        "id": "grand_chatelet",
        "name": "Grand Châtelet",
        "model": "castle",
        "at": rnd(centroid(551, 462)),
        "angle_deg": ang,
        "clear_m": 4,
        "certainty": "attested",
        "params": {
            "ring": [[-22, -18], [22, -18], [22, 18], [-22, 18]],
            "height_m": 14,
            "thickness_m": 3,
            "tower_radius_m": 5,
            "tower_height_m": 22,
            "halls": [[0, 0, 26, 12, 14, 0]],
        },
        "description": "Forteresse de la tête du Grand-Pont (pierre sous Louis VI, vers 1130), siège de la prévôté ; rue Saint-Denis sous sa voûte. Gabarit restitué sur l'emprise ALPAGE.",
    }
)
monuments.append(
    {
        "id": "petit_chatelet",
        "name": "Petit Châtelet",
        "model": "castle",
        "at": rnd(centroid(59, 590)),
        "angle_deg": axis(59, False)[1],
        "clear_m": 3,
        "certainty": "attested",
        "params": {
            "ring": [[-10, -12], [10, -12], [10, 12], [-10, 12]],
            "height_m": 13,
            "thickness_m": 2.5,
            "tower_radius_m": 4,
            "tower_height_m": 18,
        },
        "description": "Porte fortifiée de la tête sud du Petit-Pont (vers 1130 ; reconstruit après la crue de 1296).",
    }
)
boxy("fort_l_eveque", "For-l'Évêque", 460, "royal_palace", params={"height_m": 14})
# Louvre.
lat, lang, _, _ = axis(433, False)
ring = [[-39, -36], [39, -36], [39, 36], [-39, 36]]
monuments.append(
    {
        "id": "louvre",
        "name": "Château du Louvre (Philippe Auguste)",
        "model": "castle",
        "at": rnd(centroid(433, 1665)),
        "angle_deg": lang,
        "clear_m": 12,
        "certainty": "attested",
        "until_year": 1363,
        "params": {
            "ring": ring,
            "height_m": 12,
            "thickness_m": 3,
            "tower_radius_m": 4.5,
            "tower_height_m": 20,
            "ditch_m": 12,
            "keep": {"at": [0, 0], "radius_m": 7.5, "height_m": 31},
            "halls": [[-26, 0, 60, 10, 12, 90], [0, 26, 60, 9, 11, 0]],
        },
        "description": "Forteresse de 1190-1202 : enceinte ≈ 78 × 72 m flanquée de tours, fossés ; Grosse Tour de 15 m de diamètre et ≈ 31 m. Emprise ALPAGE.",
    }
)
monuments.append(
    {
        "id": "louvre_charles_v",
        "name": "Château du Louvre (Charles V)",
        "model": "castle",
        "at": rnd(centroid(433, 1665)),
        "angle_deg": lang,
        "clear_m": 12,
        "certainty": "probable",
        "from_year": 1364,
        "params": {
            "ring": ring,
            "height_m": 16,
            "thickness_m": 3,
            "tower_radius_m": 5,
            "tower_height_m": 30,
            "ditch_m": 12,
            "keep": {"at": [0, 0], "radius_m": 7.5, "height_m": 31},
            "halls": [
                [-26, 0, 66, 12, 20, 90],
                [0, 26, 66, 12, 20, 0],
                [0, -26, 66, 11, 18, 0],
            ],
        },
        "description": "Résidence aménagée par Raymond du Temple pour Charles V (1364-1380) : logis surélevés, grand escalier ; gabarit restitué.",
    }
)
tower(
    "tour_de_nesle",
    "Tour de Nesle",
    786,
    5.0,
    25.0,
    desc="Tête de l'enceinte de la rive gauche sur la Seine, face à la tour du Coin (chaîne tendue la nuit).",
)
tower("tour_du_coin", "Tour du Coin", 441, 4.0, 20.0)
tower("tour_barbeau", "Tour Barbeau", 239, 4.5, 20.0)
boxy(
    "tournelle",
    "Château de la Tournelle",
    891,
    "keep",
    params={"radius_m": 6, "height": 22},
    desc="Tête de l'enceinte de la rive gauche en amont.",
)
tower("tour_de_bois", "Tour de Bois", 440, 5.0, 18.0, cert="probable", from_year=1365)
tower("tour_de_billy", "Tour de Billy", 453, 6.0, 22.0, cert="probable", from_year=1365)
monuments.append(
    {
        "id": "bastille",
        "name": "Bastille Saint-Antoine",
        "model": "castle",
        "at": rnd(centroid(1163)),
        "angle_deg": axis(1163, False)[1],
        "clear_m": 10,
        "certainty": "attested",
        "from_year": 1370,
        "params": {
            "ring": [[-33, -17], [0, -20], [33, -17], [33, 17], [0, 20], [-33, 17]],
            "height_m": 22,
            "thickness_m": 3,
            "tower_radius_m": 7.5,
            "tower_height_m": 24,
            "ditch_m": 25,
        },
        "description": "Commencée en 1370, achevée vers 1382 ; huit tours de 24 m. Absente en 1340.",
    }
)
# Halles.
for i, fid in enumerate((24, 25, 28)):
    boxy(
        f"halle_{i + 1}",
        f"Halles des Champeaux ({i + 1})",
        fid,
        "hall",
        params={"height_m": 13},
        desc="Halles couvertes de Philippe Auguste (1183), agrandies sous Saint Louis (1269) ; emprises ALPAGE."
        if i == 0
        else None,
    )
# Temple.
tat, tang, _, _ = axis(1124, False)
monuments.append(
    {
        "id": "temple_enclos",
        "name": "Enclos du Temple",
        "model": "castle",
        "at": tat,
        "angle_deg": tang,
        "clear_m": 4,
        "certainty": "attested",
        "params": {
            "ring": ring_local(1124, tat, tang, 6.0),
            "height_m": 9,
            "thickness_m": 1.5,
            "tower_radius_m": 2.5,
            "tower_height_m": 13,
        },
        "description": "Enclos de plus de 6 ha, mur crénelé de 8-10 m à tourelles ; aux Hospitaliers depuis la suppression de l'ordre du Temple (1312). Emprise ALPAGE.",
    }
)
monuments.append(
    {
        "id": "temple_donjon",
        "name": "Grosse tour du Temple",
        "model": "belfry",
        "at": rnd(centroid(1117)),
        "angle_deg": axis(1117, False)[1],
        "clear_m": 3,
        "certainty": "probable",
        "params": {"size": 20, "height": 50, "top": "pyramid"},
        "description": "Donjon carré à quatre tourelles (XIIIe s., datation débattue : avant 1310) ; hauteur restituée.",
    }
)
tower(
    "tour_de_cesar_temple", "Tour de César (Temple)", 1118, 4.5, 24.0, cert="probable"
)
church(
    "temple_chapelle",
    "Église du Temple",
    1115,
    apse="round",
    height=15,
    desc="Rotonde et chœur de l'église des Templiers.",
)
# Abbeys.
church(
    "saint_germain_des_pres",
    "Abbatiale de Saint-Germain-des-Prés",
    667,
    length=90,
    width=24,
    height=19,
    apse="round",
    tower={"at": "west", "size": 10, "height": 50, "spire_m": 22},
    desc="Nef romane (vers 1000), chœur gothique consacré en 1163 ; clocher-porche. Emprise ALPAGE.",
)
church(
    "sgdp_chapelle_vierge",
    "Chapelle de la Vierge (Saint-Germain-des-Prés)",
    666,
    length=38,
    width=11,
    height=18,
    apse="round",
    tower=False,
    desc="Pierre de Montreuil, 1245-1255.",
)
boxy(
    "sgdp_couvent",
    "Bâtiments conventuels de Saint-Germain-des-Prés",
    670,
    "enclosure",
    params={"height_m": 12, "ranges": True},
)
sat, sang, _, _ = axis(669, False)
monuments.append(
    {
        "id": "sgdp_enclos",
        "name": "Enceinte de l'abbaye de Saint-Germain-des-Prés",
        "model": "castle",
        "at": sat,
        "angle_deg": sang,
        "clear_m": 3,
        "certainty": "probable",
        "params": {
            "ring": ring_local(669, sat, sang, 6.0),
            "height_m": 7,
            "thickness_m": 1.5,
            "tower_radius_m": 2.5,
            "tower_height_m": 10,
        },
        "description": "Mur d'enclos de l'abbaye (fortifié en 1368) ; tracé ALPAGE.",
    }
)
church(
    "sainte_genevieve",
    "Abbatiale Sainte-Geneviève",
    867,
    length=70,
    width=22,
    height=18,
    apse="round",
    tower=False,
)
monuments.append(
    {
        "id": "tour_clovis",
        "name": "Tour de Clovis (clocher de Sainte-Geneviève)",
        "model": "belfry",
        "at": rnd(centroid(868)),
        "angle_deg": axis(868, False)[1],
        "clear_m": 2,
        "certainty": "attested",
        "params": {"size": 9, "height": 42, "top": "pyramid"},
        "description": "Clocher roman et gothique de l'abbatiale (base XIe-XIIe s.) ; hauteur restituée.",
    }
)
boxy(
    "sainte_genevieve_cloitre",
    "Cloître de Sainte-Geneviève",
    870,
    "enclosure",
    params={"height_m": 10, "ranges": True},
)
church(
    "saint_victor",
    "Abbatiale Saint-Victor",
    933,
    length=70,
    width=20,
    height=17,
    apse="flat",
    cert="probable",
    desc="Abbaye de chanoines réguliers fondée au début du XIIe s. (1113 selon la tradition, à vérifier) ; église romane avant la reconstruction du XVIe s. Emprise ALPAGE.",
)
boxy(
    "saint_victor_couvent",
    "Bâtiments conventuels de Saint-Victor",
    942,
    "enclosure",
    params={"height_m": 11, "ranges": True},
)
church(
    "saint_martin_des_champs",
    "Prieuré Saint-Martin-des-Champs",
    1097,
    length=72,
    width=22,
    height=17,
    apse="round",
    desc="Chœur roman (vers 1135), nef gothique (XIIIe s.).",
)
boxy(
    "saint_martin_couvent",
    "Bâtiments du prieuré Saint-Martin-des-Champs",
    1095,
    "enclosure",
    params={"height_m": 11, "ranges": True},
)
mat, mang, _, _ = axis(1098, False)
monuments.append(
    {
        "id": "saint_martin_enclos",
        "name": "Enceinte du prieuré Saint-Martin-des-Champs",
        "model": "castle",
        "at": mat,
        "angle_deg": mang,
        "clear_m": 3,
        "certainty": "probable",
        "params": {
            "ring": ring_local(1098, mat, mang, 6.0),
            "height_m": 7,
            "thickness_m": 1.5,
            "tower_radius_m": 2.5,
            "tower_height_m": 11,
        },
        "description": "Mur à tourelles du prieuré (vers 1273) ; tracé ALPAGE.",
    }
)
# Parish and convent churches (ALPAGE footprints).
CH = [
    ("saint_germain_l_auxerrois", "Saint-Germain-l'Auxerrois", 459, {}),
    (
        "saint_jacques_la_boucherie",
        "Saint-Jacques-de-la-Boucherie",
        555,
        {"desc": "Église antérieure à la tour de 1509-1523."},
    ),
    (
        "saint_merri",
        "Saint-Merri",
        1653,
        {
            "cert": "probable",
            "desc": "Église antérieure à la reconstruction de 1500-1550.",
        },
    ),
    (
        "saint_gervais",
        "Saint-Gervais-Saint-Protais",
        220,
        {
            "cert": "probable",
            "desc": "Église antérieure à la reconstruction de 1494-1657.",
        },
    ),
    ("saint_jean_en_greve", "Saint-Jean-en-Grève", 192, {}),
    ("saint_paul", "Saint-Paul", 1002, {}),
    ("saints_innocents", "Église des Saints-Innocents", 485, {}),
    ("sainte_opportune", "Sainte-Opportune", 464, {}),
    (
        "saint_leu_saint_gilles",
        "Saint-Leu-Saint-Gilles",
        158,
        {"desc": "Construite en 1320."},
    ),
    ("saint_sauveur", "Saint-Sauveur", 405, {}),
    ("saint_nicolas_des_champs", "Saint-Nicolas-des-Champs", 1084, {}),
    ("saint_honore", "Collégiale Saint-Honoré", 420, {}),
    ("sainte_avoie", "Sainte-Avoie", 1054, {"cert": "probable"}),
    ("saint_severin", "Saint-Séverin", 779, {}),
    ("saint_andre_des_arts", "Saint-André-des-Arts", 699, {}),
    ("saint_julien_le_pauvre", "Saint-Julien-le-Pauvre", 794, {}),
    (
        "saint_etienne_du_mont",
        "Saint-Étienne-du-Mont (église de 1222)",
        860,
        {"tower": False},
    ),
    ("saint_benoit", "Saint-Benoît-le-Bétourné", 772, {}),
    ("saint_nicolas_du_chardonnet", "Saint-Nicolas-du-Chardonnet", 896, {}),
    ("saint_cosme", "Saint-Côme", 713, {}),
    ("saint_hilaire", "Saint-Hilaire-du-Mont", 833, {}),
    ("saint_etienne_des_gres", "Saint-Étienne-des-Grès", 842, {}),
    ("saint_sulpice", "Saint-Sulpice (église médiévale)", 671, {"cert": "probable"}),
    ("saint_medard", "Saint-Médard", 380, {"cert": "probable"}),
    ("saint_marcel", "Collégiale Saint-Marcel", 376, {}),
    (
        "saint_martin_du_cloitre",
        "Saint-Martin (bourg Saint-Marcel)",
        377,
        {"cert": "probable"},
    ),
    ("saint_hippolyte", "Saint-Hippolyte", 1216, {"cert": "probable"}),
    ("notre_dame_des_champs", "Notre-Dame-des-Champs", 1575, {"cert": "probable"}),
    ("saint_barthelemy", "Saint-Barthélemy", 578, {}),
    ("saint_denis_de_la_chartre", "Saint-Denis-de-la-Chartre", 272, {}),
    ("la_madeleine_cite", "La Madeleine-en-la-Cité", 586, {}),
    ("saint_germain_le_vieux", "Saint-Germain-le-Vieux", 588, {}),
    ("saint_landry", "Saint-Landry", 618, {}),
    ("saint_christophe", "Saint-Christophe", 261, {}),
    ("sainte_croix_cite", "Sainte-Croix-en-la-Cité", 251, {}),
    ("saint_pierre_des_arcis", "Saint-Pierre-des-Arcis", 249, {}),
    ("saint_eloi_cite", "Saint-Éloi", 582, {}),
    ("saint_pierre_aux_boeufs", "Saint-Pierre-aux-Bœufs", 606, {}),
    ("sainte_marine", "Sainte-Marine", 270, {}),
    ("saint_jean_le_rond", "Saint-Jean-le-Rond", 604, {}),
    ("saint_denis_du_pas", "Saint-Denis-du-Pas", 612, {}),
    ("cordeliers", "Église des Cordeliers", 707, {"height": 20}),
    ("jacobins", "Église des Jacobins (Saint-Jacques)", 751, {"cert": "probable"}),
    (
        "augustins",
        "Église des Grands-Augustins",
        636,
        {"desc": "Couvent installé sur la rive gauche en 1293."},
    ),
    (
        "carmes",
        "Église des Carmes (place Maubert)",
        824,
        {"desc": "Les Carmes s'installent place Maubert en 1318."},
    ),
    (
        "bernardins",
        "Église du collège des Bernardins",
        909,
        {
            "cert": "hypothetical",
            "desc": "Église commencée en 1338, jamais achevée : gabarit incertain en 1340.",
        },
    ),
    ("mathurins", "Les Mathurins", 777, {"cert": "probable"}),
    ("blancs_manteaux", "Église des Blancs-Manteaux", 1550, {}),
    ("billettes", "Chapelle des Billettes", 1555, {}),
    ("filles_dieu", "Chapelle des Filles-Dieu", 403, {}),
    ("sainte_catherine_du_val", "Sainte-Catherine-du-Val-des-Écoliers", 1047, {}),
    ("beguinage", "Chapelle du béguinage", 991, {}),
    ("chartreux", "Chapelle des Chartreux (Vauvert)", 1191, {}),
    ("sainte_croix_bretonnerie", "Sainte-Croix-de-la-Bretonnerie", 1552, {}),
    (
        "saint_jacques_de_l_hopital",
        "Saint-Jacques-de-l'Hôpital",
        487,
        {"desc": "Hôpital des pèlerins de Saint-Jacques, 1319-1324."},
    ),
    ("trinite", "Chapelle de l'hôpital de la Trinité", 573, {}),
    ("saint_magloire", "Abbatiale Saint-Magloire", 560, {}),
    (
        "saint_jacques_du_haut_pas",
        "Saint-Jacques-du-Haut-Pas",
        1196,
        {"cert": "probable"},
    ),
    (
        "saint_yves",
        "Chapelle Saint-Yves",
        333,
        {"from_year": 1357, "desc": "Fondée en 1348, construite vers 1352-1357."},
    ),
    (
        "celestins",
        "Église des Célestins",
        245,
        {
            "from_year": 1367,
            "desc": "Couvent fondé en 1352 ; église consacrée en 1370 (Charles V).",
        },
    ),
    (
        "saint_antoine_le_petit",
        "Chapelle du Petit-Saint-Antoine",
        1557,
        {"from_year": 1361},
    ),
    ("sainte_agnes", "Sainte-Agnès", 8, {"cert": "probable"}),
    (
        "saint_sepulcre",
        "Saint-Sépulcre",
        157,
        {"desc": "Fondé en 1325-1326 (confrérie du Saint-Sépulcre)."},
    ),
]
for mid, name, fid, kw in CH:
    kw = dict(kw)
    if kw.get("tower") is False:
        kw["tower"] = {}
    c = G[fid].representative_point()
    if Point(c.x, c.y).distance(Point(0, 0)) > EXTENT - 60:
        continue
    church(
        mid, name, fid, cert=kw.pop("cert", "attested"), desc=kw.pop("desc", None), **kw
    )
# Hospitals, convents, colleges (buildings as enclosures).
boxy(
    "quinze_vingts",
    "Hôpital des Quinze-Vingts",
    419,
    "enclosure",
    cert="attested",
    params={"height_m": 10, "ranges": True},
    desc="Fondé par Saint Louis vers 1260.",
)
boxy(
    "hopital_sainte_catherine",
    "Hôpital Sainte-Catherine",
    556,
    "enclosure",
    params={"height_m": 11, "ranges": True},
)
boxy(
    "hopital_saint_esprit",
    "Hôpital du Saint-Esprit",
    967,
    "enclosure",
    params={"height_m": 11, "ranges": True},
    from_year=1362,
)
boxy(
    "hopital_de_la_trinite",
    "Hôpital de la Trinité",
    572,
    "enclosure",
    params={"height_m": 11, "ranges": True},
)
boxy(
    "maison_aux_piliers",
    "Maison aux Piliers",
    966,
    "royal_palace",
    params={"height_m": 14, "width_m": 20},
    desc="Maison de la place de Grève, achetée par la municipalité en 1357 (Étienne Marcel).",
)
boxy(
    "couvent_des_augustins",
    "Couvent des Grands-Augustins",
    640,
    "enclosure",
    params={"height_m": 11, "ranges": True},
)
boxy(
    "couvent_des_cordeliers",
    "Couvent des Cordeliers",
    710,
    "enclosure",
    params={"height_m": 11, "ranges": True},
)
boxy(
    "couvent_des_jacobins",
    "Couvent des Jacobins",
    750,
    "enclosure",
    params={"height_m": 11, "ranges": True},
)
boxy(
    "couvent_des_bernardins",
    "Collège des Bernardins",
    912,
    "enclosure",
    params={"height_m": 12, "ranges": True},
    desc="Collège cistercien (1245-1253).",
)
boxy(
    "couvent_des_chartreux",
    "Chartreuse de Vauvert",
    1193,
    "enclosure",
    params={"height_m": 8, "ranges": True},
)
boxy(
    "couvent_des_beguines",
    "Grand béguinage",
    236,
    "enclosure",
    params={"height_m": 9, "ranges": True},
)
boxy(
    "couvent_sainte_croix",
    "Couvent de Sainte-Croix-de-la-Bretonnerie",
    1551,
    "enclosure",
    params={"height_m": 10, "ranges": True},
)
boxy(
    "sainte_catherine_couvent",
    "Couvent de Sainte-Catherine-du-Val",
    1051,
    "enclosure",
    params={"height_m": 10, "ranges": True},
)
boxy(
    "commanderie_saint_jean",
    "Commanderie Saint-Jean-de-Latran (Hospitaliers)",
    815,
    "enclosure",
    params={"height_m": 10, "ranges": True},
)
monuments = [m for m in monuments if math.hypot(*m["at"]) < EXTENT - 60]

# --- Bridges and quays -------------------------------------------------------------------------

bridges = [
    {
        "id": "grand_pont",
        "name": "Grand-Pont (pont aux Changeurs)",
        "from": [-229, 391],
        "to": [-130, 495],
        "width_m": 10.0,
        "deck_m": 7.0,
        "material": "wood",
        "arches": 9,
        "houses": True,
        "house_span": [0.05, 0.95],
        "gatehouses_at": [],
        "note": "Tracé ALPAGE (1380), de la rue de la Barillerie au Grand Châtelet ; changeurs et orfèvres dans les maisons des deux côtés. Pont de bois reconstruit après les crues de 1280 et 1296 : matériau et gabarit en 1340 incertains (les 106 × 27 m souvent cités sont ceux du pont Notre-Dame de 1413).",
    },
    {
        "id": "pont_aux_meuniers",
        "name": "Pont aux Meuniers",
        "from": [-230, 399],
        "to": [-158, 510],
        "width_m": 5.0,
        "deck_m": 6.0,
        "material": "wood",
        "arches": 8,
        "houses": False,
        "mills_at": [0.3, 0.5, 0.7],
        "note": "Pont de moulins en aval du Grand-Pont (tracé ALPAGE) ; nombre de moulins restitué.",
        "certainty": "probable",
    },
    {
        "id": "petit_pont",
        "name": "Petit-Pont",
        "from": [-205, 70],
        "to": [-222, 30],
        "width_m": 9.0,
        "deck_m": 6.0,
        "material": "stone",
        "arches": 3,
        "houses": True,
        "house_span": [0.1, 0.9],
        "note": "Pont de pierre de Maurice de Sully (1186), ≈ 40 m, bordé de maisons ; relie la Cité au Petit Châtelet. Tracé ALPAGE.",
    },
    {
        "id": "planches_de_mibray",
        "name": "Planches de Mibray",
        "from": [-82, 325],
        "to": [-32, 427],
        "width_m": 4.0,
        "deck_m": 5.0,
        "material": "wood",
        "arches": 10,
        "houses": False,
        "note": "Passerelle de bois de la Cité vers la Grève, à l'emplacement du futur pont Notre-Dame (1413) ; tracé ALPAGE, état en 1340 probable.",
    },
    {
        "id": "pont_saint_michel",
        "name": "Pont Saint-Michel (Petit-Pont-Neuf)",
        "from": [-361, 198],
        "to": [-388, 125],
        "width_m": 9.0,
        "deck_m": 6.0,
        "material": "stone",
        "arches": 4,
        "houses": True,
        "house_span": [0.1, 0.9],
        "from_year": 1378,
        "note": "Construit en 1378-1379 : absent en 1340.",
    },
    {
        "id": "pont_de_la_tournelle",
        "name": "Pont de la Tournelle (bois)",
        "from": [356, -337],
        "to": [414, -253],
        "width_m": 5.0,
        "deck_m": 5.0,
        "material": "wood",
        "arches": 6,
        "houses": False,
        "from_year": 1370,
        "note": "Pont de bois vers l'île Notre-Dame lié à l'enceinte de Charles V (vers 1370) ; absent en 1340.",
    },
]
voies = {
    r["fid"]: alpage.to_local(r["geom"], ORIGIN)
    for r in alpage.read_layer("1380_voies")
}


def line_of(*fids):
    pts = []
    for f in fids:
        g = voies[f]
        g = list(getattr(g, "geoms", [g]))[0]
        c = [rnd(p) for p in g.coords]
        if pts and math.dist(pts[-1], c[-1]) < math.dist(pts[-1], c[0]):
            c = c[::-1]
        pts.extend(c if not pts or c[0] != pts[-1] else c[1:])
    return pts


quays = [
    {
        "id": "quai_des_augustins",
        "name": "Quai des Augustins (1313)",
        "kind": "stone",
        "width_m": 12.0,
        "points": line_of(734, 733, 1049),
    },
    {
        "id": "quai_saulnerie",
        "name": "Quais de la Saulnerie, du Châtelet et de l'Écorcherie",
        "kind": "strand",
        "width_m": 10.0,
        "points": line_of(70, 71, 72, 73, 105, 223, 224, 106),
    },
    {
        "id": "quai_greve",
        "name": "Quais de la Tannerie, de la Grève et des Ormes",
        "kind": "strand",
        "width_m": 10.0,
        "points": line_of(428, 74, 429),
    },
    {
        "id": "port_de_greve",
        "name": "Port de Grève et quai des Ormes",
        "kind": "strand",
        "width_m": 12.0,
        "points": line_of(430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440),
    },
    {
        "id": "quai_saint_bernard",
        "name": "Quai Saint-Bernard (grève)",
        "kind": "strand",
        "width_m": 10.0,
        "points": line_of(872, 719, 1130, 1068),
    },
]

seine_parts = [
    G[fid]
    for fid, r in usage.items()
    if r.get("GROUPE") == "Seine" and r.get("NATURE") == "Fleuve"
]
seine = (
    unary_union([g.buffer(2.0) for g in seine_parts])
    .buffer(-2.0)
    .intersection(Point(0, 0).buffer(EXTENT - 80.0))
)
waters = []
for k, part in enumerate(
    sorted(getattr(seine, "geoms", [seine]), key=lambda g: -g.area)
):
    if part.area < 1500.0:
        continue
    part = part.simplify(2.0)
    waters.append(
        {
            "id": f"seine_1380_{k}",
            "name": "Seine (lit vers 1380)",
            "kind": "river",
            "origin": "alpage",
            "draw": False,
            "polygon": [rnd(q) for q in list(part.exterior.coords)[:-1]],
            "holes": [
                [rnd(q) for q in list(h.coords)[:-1]]
                for h in part.interiors
                if Polygon(h).area > 200.0
            ],
        }
    )
print(
    "seine parts",
    [(len(w["polygon"]), len(w["holes"])) for w in waters],
    file=sys.stderr,
)

city = {
    "format": 2,
    "id": "paris",
    "name": "Paris",
    "settlement": "set_paris",
    "landmark": "paris",
    "period": "vers 1340 (avant la peste de 1348, l'enceinte de Charles V de 1356, la tour de l'Horloge de 1350 et la Bastille de 1370)",
    "year": 1340,
    "seed": 1340075,
    "crs": "EPSG:3035",
    "origin_3035": ORIGIN,
    "extent_m": int(EXTENT),
    "sources": [
        {
            "title": "ALPAGE — Paris en 1380 (voies, îlots, usages du sol, hydrographie)",
            "author": "Paul Rouet, consortium ALPAGE (LAMOP, dir. Hélène Noizet)",
            "date": "2026-09-14 (mise en ligne)",
            "url": "https://alpage.huma-num.fr/paris-in-1380/",
            "license": "ODbL 1.0 (© ALPAGE : P. Rouet)",
            "extracted": True,
            "use": 'Réseau des rues vers 1380 (`origin: "alpage"`, regénéré par `cent-ans geo landmarks`) ; tracés des enceintes de Philippe Auguste et de Charles V, portes, poternes et tours ; emprises et orientations des églises, abbayes, palais, Louvre, Châtelets, Temple, halles ; îles, quais, cimetières, quartiers bâtis et faubourgs.',
        },
        {
            "title": "ALPAGE — données Vasserot version 1 (parcelles, 1810-1836)",
            "author": "Anne-Laure Bethe, consortium ALPAGE",
            "date": "2010",
            "url": "https://alpage.huma-num.fr/vasserot-data-version-1-2010-a-l-bethe/",
            "license": "ODbL 1.0 (© ALPAGE : A.-L. Bethe ; Arch. nat. F31 73-96 – Arch. Paris © ALPAGE)",
            "extracted": True,
            "use": "Gabarit des lanières (section `parcels`) : seules les parcelles qui donnent sur une rue de 1380, qu'aucune rue de 1380 ne traverse et qui ne recouvrent ni église, ni couvent, ni marché, ni cimetière, ni eau sont gardées (le cadastre est postérieur de cinq siècles).",
        },
        {
            "title": "Relief fin et fleuves de la carte (hydro_fine)",
            "author": "Projet Cent Ans (IGN RGE ALTI, Copernicus DEM, BD TOPAGE)",
            "date": "2026",
            "url": "https://geoservices.ign.fr/rgealti",
            "license": "Licence Ouverte 2.0 / licence Copernicus (voir docs/credits.md)",
            "extracted": True,
            "use": "Axe et largeur de la Seine recopiés de la carte fine (section `waters`) ; les hauteurs viennent du relief affiché à l'exécution.",
        },
        {
            "title": "Plan de Paris, dit plan de Bâle",
            "author": "Olivier Truschet et Germain Hoyau",
            "date": "vers 1550",
            "url": "https://commons.wikimedia.org/wiki/File:Map_of_Paris_by_Truschet_and_Hoyau_-_Basel_University_Library.jpg",
            "license": "Domaine public (Basel University Library)",
            "extracted": False,
            "use": "Contrôle humain : ponts habités, silhouettes des monuments, enceintes (état de 1550).",
        },
        {
            "title": "Paris en 1380 (Atlas des anciens plans de Paris)",
            "author": "Henri Legrand",
            "date": "1868",
            "url": "https://commons.wikimedia.org/wiki/File:Atlas_des_anciens_plans_de_Paris_-_Paris_en_1380_-_David_Rumsey.jpg",
            "license": "Domaine public (David Rumsey Map Collection)",
            "extracted": False,
            "use": "Contrôle humain : enceintes, portes, îles.",
        },
        {
            "title": "Dossier de sources VH (faits datés)",
            "author": "Projet Cent Ans",
            "date": "2026",
            "url": "https://fr.wikipedia.org/wiki/Enceinte_de_Philippe_Auguste",
            "license": "Faits (Wikipédia CC BY-SA, AFGC, France Pittoresque) recoupés dans docs/research/vh-sources.md",
            "extracted": False,
            "use": "Dates et gabarits : enceintes (1190-1215 ; Charles V 1356-1383), Notre-Dame (tours de 69 m, flèche vers 1250), Sainte-Chapelle (1248), tour de l'Horloge (1350-1353), Louvre (Grosse Tour de 15 m), Petit-Pont (1186), pont Saint-Michel (1378), Bastille (1370).",
        },
    ],
    "plan": {
        "frontage_m": [5.0, 8.0],
        "depth_m": [18.0, 36.0],
        "faubourg_frontage_m": [8.0, 16.0],
        "faubourg_depth_m": [25.0, 50.0],
        "street_width_m": {"main": 8.0, "secondary": 5.0, "lane": 3.0, "quay": 12.0},
        "house_kinds": {
            "intra": {"townhouse": 0.55, "timber": 0.3, "stonehouse": 0.15},
            "faubourg": {"timber": 0.35, "cottage": 0.35, "longere": 0.2, "barn": 0.1},
        },
        "detail_cell_m": 250.0,
    },
    "alpage": {
        "streets": {
            "layer": "1380_voies",
            "skip_regions": ["PONT"],
            "exclude": [
                "Quai de la Saulnerie",
                "Quai du Chatelet",
                "Quai de l'Ecorcherie",
                "Quai de la Tannerie",
                "Quai de Gruve",
                "Quai des Ormes",
                "Quai des Augustins",
                "Quai St-Bernard",
                "Quai de Seine   l'Ouest du Louvre",
                "Accès au Pont de la Tournelle",
                "Rue du Petit Pont Neuf",
            ],
            "main": [],
            "simplify_m": 1.0,
        },
        "parcels": {
            "layer": "Vasserot_1_Parcelles",
            "districts": ["cite", "ville", "universite"],
            "min_area_m2": 15.0,
            "max_area_m2": 2500.0,
            "max_front_m": 30.0,
            "max_depth_m": 60.0,
            "reach_m": 8.0,
            "max_landuse_overlap": 0.3,
            "exclude_landuse": [
                "Eglises et chapelles",
                "Edifices religieux",
                "Edifices religieux, batiment",
                "Edifices royaux et communaux",
                "Edifices royaux et communaux, batiment",
                "Marches, halles, foires",
                "Cimetieres",
                "Hopitaux",
                "Hydrographie",
                "Champs et cultures, terrains vagues",
                "Enceintes",
                "Jardin",
            ],
        },
    },
    "fine_rivers": [],
    "waters": waters,
    "walls": walls,
    "streets": [],
    "quays": quays,
    "bridges": bridges,
    "monuments": monuments,
    "districts": districts,
    "open_spaces": open_spaces,
    "parcels": [],
    "render": {"fade_valley": [0.35, 0.65]},
}
out = REPO / "data" / "landmarks_v2" / "paris.json"
landmarks_v2.write_city(out, city)
print(
    "walls",
    [len(w["points"]) for w in walls],
    "gates",
    [len(w["gates"]) for w in walls],
    "monuments",
    len(monuments),
    "spaces",
    len(open_spaces),
    file=sys.stderr,
)
