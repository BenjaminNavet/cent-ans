"""Fine river network sources for lot ZG5a (ADR 0036).

Each source is read into a :class:`LinkTable` (directed links, EPSG:3035 metres,
flowing from ``start`` to ``end`` node), then Strahler orders are computed from the
topology and links are chained into *strokes* (a river from its source to the
confluence where it loses its name/order, see :func:`build_strokes`).

Sources (all checked 2026-09-25, anonymous HTTPS, no account):

- France: IGN/OFB **BD TOPAGE®** 2025, ``TronconHydrographique_FXX`` (SANDRE),
  Licence Ouverte 2.0 (Etalab).
- Great Britain: **OS Open Rivers** (Ordnance Survey), Open Government Licence v3.
- Belgium, Netherlands, Luxembourg, Germany and the rest of the core: Copernicus
  **EU-Hydro** River Network Database v1.3 through the anonymous EEA ArcGIS REST
  service (``image.discomap.eea.europa.eu``), Copernicus open data policy
  (Regulation (EU) 1159/2013), attribution required.
- Elsewhere: Natural Earth 10m (public domain), the lines of ``rivers.geojson``.

No OpenStreetMap data is used.
"""

from __future__ import annotations

import json
import re
import time
import zipfile
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

import httpx
import numpy as np
import shapely

from cent_ans_tools.geo import download

HYDRO_RAW = download.RAW_DIR / "hydro"
CACHE_DIR = HYDRO_RAW / "cache"

TOPAGE_URL = (
    "https://services.sandre.eaufrance.fr/telechargement/geo/ETH/BDTopage/2025/"
    "TronconHydrographique/TronconHydrographique_FXX-gpkg.zip"
)
TOPAGE_ZIP = "TronconHydrographique_FXX-gpkg.zip"
TOPAGE_GPKG = "TronconHydrographique_FXX.gpkg"
OSOR_URL = (
    "https://api.os.uk/downloads/v1/products/OpenRivers/downloads"
    "?area=GB&format=GeoPackage&redirect"
)
OSOR_ZIP = "oprvrs_gpkg_gb.zip"
EUHYDRO_URL = (
    "https://image.discomap.eea.europa.eu/arcgis/rest/services/EUHydro/"
    "EUHydro_RiverNetworkDatabase/MapServer"
)
#: Strahler layers of the EU-Hydro service (layer id per order).
EUHYDRO_LAYERS = {1: 5, 2: 6, 3: 7, 4: 8, 5: 9, 6: 10, 7: 11, 8: 12, 9: 13}
EUHYDRO_PAGE = 1000

#: Upper bound (metres) of the BD TOPAGE width classes (``ClasseLargeurTH``).
TOPAGE_WIDTH = {
    "0_5": 5.0,
    "5_15": 15.0,
    "15_50": 50.0,
    "50": 250.0,
    "50_250": 250.0,
    "250_1250": 1250.0,
    "1250": 3000.0,
}
TOPAGE_WIDTH_MIN = {
    "0_5": 0.5,
    "5_15": 5.0,
    "15_50": 15.0,
    "50": 50.0,
    "50_250": 50.0,
    "250_1250": 250.0,
    "1250": 1250.0,
}
TOPAGE_DROP_NATURE = {"Conduit buse", "Aqueduc", "Conduit forcé"}
EUHYDRO_CANAL_DFDD = {"BH020", "BH030"}  # canal, ditch


@dataclass
class LinkTable:
    """Directed river links of one source.

    Attributes:
        source: ``topage``, ``osor``, ``euhydro`` or ``naturalearth``.
        lines: Vertices of each link, EPSG:3035 metres, in the flow direction.
        start, end: Node ids (flow goes from ``start`` to ``end``).
        name: River name (``""`` if unknown).
        code: River identifier (same value along one river), else the name.
        width_min, width_max: Width class bounds in metres (NaN if unknown).
        strahler: Strahler order (0 before :func:`compute_strahler`).
        canal: Artificial canal according to the source.
        intermittent: Seasonal stream.
        tidal: Saline / estuary link.
    """

    source: str
    lines: list[np.ndarray] = field(default_factory=list)
    start: np.ndarray = field(default_factory=lambda: np.zeros(0, np.int64))
    end: np.ndarray = field(default_factory=lambda: np.zeros(0, np.int64))
    name: np.ndarray = field(default_factory=lambda: np.zeros(0, object))
    code: np.ndarray = field(default_factory=lambda: np.zeros(0, object))
    width_min: np.ndarray = field(default_factory=lambda: np.zeros(0))
    width_max: np.ndarray = field(default_factory=lambda: np.zeros(0))
    strahler: np.ndarray = field(default_factory=lambda: np.zeros(0, np.int16))
    canal: np.ndarray = field(default_factory=lambda: np.zeros(0, bool))
    intermittent: np.ndarray = field(default_factory=lambda: np.zeros(0, bool))
    tidal: np.ndarray = field(default_factory=lambda: np.zeros(0, bool))
    secondary: np.ndarray | None = None

    def __post_init__(self) -> None:
        """Default ``secondary`` (side arm of a braid) to false."""
        if self.secondary is None:
            self.secondary = np.zeros(len(self.lines), dtype=bool)

    def __len__(self) -> int:
        """Number of links."""
        return len(self.lines)

    def lengths(self) -> np.ndarray:
        """Length of each link in metres."""
        return np.array(
            [
                float(np.hypot(*np.diff(line, axis=0).T).sum())
                if len(line) > 1
                else 0.0
                for line in self.lines
            ]
        )

    def subset(self, mask: np.ndarray) -> LinkTable:
        """Links where ``mask`` is true."""
        index = np.flatnonzero(mask)
        return LinkTable(
            source=self.source,
            lines=[self.lines[i] for i in index],
            start=self.start[index],
            end=self.end[index],
            name=self.name[index],
            code=self.code[index],
            width_min=self.width_min[index],
            width_max=self.width_max[index],
            strahler=self.strahler[index],
            canal=self.canal[index],
            intermittent=self.intermittent[index],
            tidal=self.tidal[index],
            secondary=self.secondary[index],
        )

    # ------------------------------------------------------------ persistence

    def save(self, path: Path) -> None:
        """Compact ``.npz`` cache (vertices concatenated)."""
        path.parent.mkdir(parents=True, exist_ok=True)
        counts = np.array([len(line) for line in self.lines], dtype=np.int64)
        coords = (
            np.concatenate(self.lines).astype(np.float64)
            if self.lines
            else np.zeros((0, 2))
        )
        tmp = path.with_name(path.stem + ".tmp.npz")
        np.savez(
            tmp,
            source=np.array(self.source),
            counts=counts,
            coords=coords,
            start=self.start,
            end=self.end,
            name=self.name.astype(str),
            code=self.code.astype(str),
            width_min=self.width_min,
            width_max=self.width_max,
            strahler=self.strahler,
            canal=self.canal,
            intermittent=self.intermittent,
            tidal=self.tidal,
            secondary=self.secondary,
        )
        tmp.replace(path)

    @classmethod
    def load(cls, path: Path) -> LinkTable:
        """Inverse of :meth:`save`."""
        data = np.load(path, allow_pickle=False)
        counts = data["counts"]
        splits = np.cumsum(counts)[:-1]
        lines = np.split(data["coords"], splits) if len(counts) else []
        return cls(
            source=str(data["source"]),
            lines=list(lines),
            start=data["start"],
            end=data["end"],
            name=data["name"].astype(object),
            code=data["code"].astype(object),
            width_min=data["width_min"],
            width_max=data["width_max"],
            strahler=data["strahler"].astype(np.int16),
            canal=data["canal"],
            intermittent=data["intermittent"],
            tidal=data["tidal"],
            secondary=data.get("secondary", None),
        )


# ------------------------------------------------------------------- topology


def node_ids(values: np.ndarray) -> np.ndarray:
    """Dense integer ids of arbitrary node keys."""
    _, inverse = np.unique(np.asarray(values).astype(str), return_inverse=True)
    return inverse.astype(np.int64)


def endpoint_nodes(
    lines: list[np.ndarray], snap_m: float = 1.0
) -> tuple[np.ndarray, np.ndarray]:
    """Node ids from rounded end coordinates (sources without topology)."""
    keys = []
    for line in lines:
        for point in (line[0], line[-1]):
            keys.append(f"{round(point[0] / snap_m)}_{round(point[1] / snap_m)}")
    ids = node_ids(np.array(keys))
    return ids[0::2], ids[1::2]


def compute_strahler(table: LinkTable) -> np.ndarray:
    """Strahler order of every link from the directed topology.

    Braided and anastomosing reaches (splits that rejoin: side arms, mill races,
    ditches of flood plains) would inflate a plain Strahler count at every
    rejoining. At each split only the *main* outgoing link (see
    :func:`_split_key`) continues the tree; the other ones are distributaries:
    they inherit the order of their parent and never raise the order where they
    rejoin. A confluence of two links of the same river (same ``code``) does not
    raise the order either. Links left in cycles take the order of their processed
    parents.
    """
    n = len(table)
    incoming: dict[int, list[int]] = defaultdict(list)
    outgoing: dict[int, list[int]] = defaultdict(list)
    for i in range(n):
        incoming[int(table.end[i])].append(i)
        outgoing[int(table.start[i])].append(i)
    parents = [incoming.get(int(table.start[i]), []) for i in range(n)]
    lengths = table.lengths()
    main_child = np.full(n, -1, dtype=np.int64)
    for j in range(n):
        children = outgoing.get(int(table.end[j]), [])
        if len(children) == 1:
            main_child[j] = children[0]
        elif children:
            main_child[j] = max(
                children, key=lambda c: _split_key(table, j, c, lengths, False)
            )
    branch = np.zeros(n, dtype=bool)
    pending = np.array([len(p) for p in parents], dtype=np.int64)
    order = np.zeros(n, dtype=np.int16)
    queue = [i for i in range(n) if pending[i] == 0]
    done = np.zeros(n, dtype=bool)

    def resolve(i: int) -> int:
        known = [j for j in parents[i] if order[j] > 0]
        if not known:
            return 1
        ups = [j for j in known if main_child[j] == i and not branch[j]]
        if not ups:  # distributary: inherits, never counts downstream
            branch[i] = True
            return int(max(order[j] for j in known))
        best = max(order[j] for j in ups)
        top = [j for j in ups if order[j] == best]
        codes = {table.code[j] for j in top}
        if len(top) >= 2 and (len(codes) >= 2 or not table.code[top[0]]):
            return int(best) + 1
        return int(best)

    while True:
        while queue:
            i = queue.pop()
            if done[i]:
                continue
            order[i] = resolve(i)
            done[i] = True
            for k in outgoing.get(int(table.end[i]), []):
                pending[k] -= 1
                if pending[k] <= 0 and not done[k]:
                    queue.append(k)
        rest = np.flatnonzero(~done)
        if len(rest) == 0:
            break
        # Cycles (two-way links, loops): release every pending link that has a
        # processed parent at once (O(n) per round), or all of them if none has.
        ready = [int(i) for i in rest if any(done[j] for j in parents[i])]
        queue.extend(ready if ready else [int(i) for i in rest])
    return order


def upstream_length(table: LinkTable, lengths: np.ndarray) -> np.ndarray:
    """Longest upstream path length (metres) at the downstream end of each link."""
    n = len(table)
    incoming: dict[int, list[int]] = defaultdict(list)
    for i in range(n):
        incoming[int(table.end[i])].append(i)
    parents = [incoming.get(int(table.start[i]), []) for i in range(n)]
    memo = np.full(n, -1.0)
    for root in range(n):
        if memo[root] >= 0:
            continue
        stack = [root]
        on_stack = set()
        while stack:
            i = stack[-1]
            if memo[i] >= 0:
                stack.pop()
                continue
            todo = [j for j in parents[i] if memo[j] < 0 and j not in on_stack]
            if todo and i not in on_stack:
                on_stack.add(i)
                stack.extend(todo)
                continue
            known = [memo[j] for j in parents[i] if memo[j] >= 0]
            memo[i] = lengths[i] + (max(known) if known else 0.0)
            on_stack.discard(i)
            stack.pop()
    return memo


@dataclass
class Stroke:
    """A river from its source (or entry) to its confluence or mouth.

    Attributes:
        links: Indices into the link table, upstream first.
        points: Concatenated vertices (flow direction).
        link_of_point: Link index of each vertex.
    """

    links: list[int]
    points: np.ndarray
    link_of_point: np.ndarray


def _split_key(
    table: LinkTable,
    parent: int,
    candidate: int,
    lengths: np.ndarray,
    use_order: bool = True,
) -> tuple:
    """Preference of ``candidate`` as the continuation of ``parent`` at a split."""
    width = table.width_max[candidate]
    return (
        int(table.strahler[candidate]) if use_order else 0,
        bool(table.code[parent]) and table.code[candidate] == table.code[parent],
        not bool(table.secondary[candidate]),
        float(width) if np.isfinite(width) else 0.0,
        not bool(table.intermittent[candidate]),
        -float(lengths[candidate]),
    )


def build_strokes(table: LinkTable, lengths: np.ndarray | None = None) -> list[Stroke]:
    """Chain links into strokes following the main stem at each confluence.

    The main parent of a link is the incoming link of highest Strahler order, then
    of the same river code, then with the longest upstream length. A stroke
    continues through main parents only; each link belongs to one stroke.
    """
    n = len(table)
    if lengths is None:
        lengths = table.lengths()
    upstream = upstream_length(table, lengths)
    incoming: dict[int, list[int]] = defaultdict(list)
    for i in range(n):
        incoming[int(table.end[i])].append(i)
    child = np.full(n, -1, dtype=np.int64)
    for b in range(n):
        ups = incoming.get(int(table.start[b]), [])
        if not ups:
            continue
        main = max(
            ups,
            key=lambda j: (
                int(table.strahler[j]),
                bool(table.code[j]) and table.code[j] == table.code[b],
                upstream[j],
            ),
        )
        # A link that splits (braids) continues into its best child only: the
        # same river, main arm, widest, permanent, then the most direct arm.
        current = child[main]
        if current < 0 or _split_key(table, main, b, lengths) > _split_key(
            table, main, int(current), lengths
        ):
            child[main] = b
    has_parent = np.zeros(n, dtype=bool)
    has_parent[child[child >= 0]] = True
    strokes = []
    seen = np.zeros(n, dtype=bool)

    def walk(head: int) -> None:
        chain = []
        i = head
        while i >= 0 and not seen[i]:
            seen[i] = True
            chain.append(i)
            i = int(child[i])
        pts = []
        owner = []
        for k, link in enumerate(chain):
            line = table.lines[link]
            if k > 0 and len(pts) and np.allclose(pts[-1][-1], line[0], atol=1.0):
                line = line[1:]
            pts.append(line)
            owner.append(np.full(len(line), link, dtype=np.int64))
        points = np.concatenate(pts) if pts else np.zeros((0, 2))
        if len(points) >= 2:
            strokes.append(Stroke(chain, points, np.concatenate(owner)))

    for head in np.flatnonzero(~has_parent):
        walk(int(head))
    for rest in np.flatnonzero(~seen):  # pure cycles
        walk(int(rest))
    return strokes


# ---------------------------------------------------------------- canal notes


@dataclass(frozen=True)
class CanalFilter:
    """Name patterns of ``historical_hydro_notes.json``."""

    modern: list[re.Pattern]
    kept: list[re.Pattern]

    @classmethod
    def from_notes(cls, notes: dict) -> CanalFilter:
        """Compile the patterns of the notes file."""

        def compile_all(entries: list[dict]) -> list[re.Pattern]:
            return [
                re.compile(p, re.IGNORECASE) for e in entries for p in e["patterns"]
            ]

        return cls(
            compile_all(notes["modern_canals"]), compile_all(notes["kept_artificial"])
        )

    def is_modern(self, name: str) -> bool:
        """The name is a post-1340 canal."""
        return bool(name) and any(p.search(name) for p in self.modern)

    def is_kept(self, name: str) -> bool:
        """The name is a pre-1340 artificial watercourse."""
        return bool(name) and any(p.search(name) for p in self.kept)

    def excluded(self, table: LinkTable) -> np.ndarray:
        """Links to drop: source canals not kept, and named modern canals."""
        names = [str(v) for v in table.name]
        kept = np.array([self.is_kept(v) for v in names], dtype=bool)
        modern = np.array([self.is_modern(v) for v in names], dtype=bool)
        return (table.canal & ~kept) | modern


# ------------------------------------------------------------------- download


def ensure_topage(force: bool = False) -> Path:
    """BD TOPAGE GeoPackage (downloaded and unzipped once, ~0.9 + 3 GB)."""
    gpkg = HYDRO_RAW / "topage" / TOPAGE_GPKG
    if gpkg.exists() and not force:
        return gpkg
    archive = download.download_file(TOPAGE_URL, HYDRO_RAW / TOPAGE_ZIP, force)
    with zipfile.ZipFile(archive) as zf:
        zf.extractall(HYDRO_RAW / "topage")
    return gpkg


def ensure_osor(force: bool = False) -> Path:
    """OS Open Rivers GeoPackage (~50 MB zip)."""
    gpkg = HYDRO_RAW / "osor" / "Data" / "oprvrs_gb.gpkg"
    if gpkg.exists() and not force:
        return gpkg
    archive = download.download_file(OSOR_URL, HYDRO_RAW / OSOR_ZIP, force)
    with zipfile.ZipFile(archive) as zf:
        zf.extractall(HYDRO_RAW / "osor")
    return gpkg


# --------------------------------------------------------------------- readers


def _lines_3035(
    geometries: np.ndarray, crs: str
) -> tuple[list[np.ndarray], np.ndarray]:
    """Project line geometries to EPSG:3035; returns vertices and a keep mask."""
    import geopandas as gpd

    series = gpd.GeoSeries(geometries, crs=crs).to_crs("EPSG:3035")
    series = series.force_2d() if hasattr(series, "force_2d") else series
    lines: list[np.ndarray] = []
    keep = np.zeros(len(series), dtype=bool)
    for i, geom in enumerate(series.values):
        if geom is None or geom.is_empty:
            lines.append(np.zeros((0, 2)))
            continue
        if geom.geom_type == "MultiLineString":
            merged = shapely.line_merge(geom)
            geom = (
                merged
                if merged.geom_type == "LineString"
                else max(merged.geoms, key=lambda g: g.length)
            )
        coords = np.asarray(geom.coords)[:, :2]
        lines.append(coords)
        keep[i] = len(coords) >= 2
    return lines, keep


def read_topage(
    gpkg: Path, bbox_3035: tuple[float, float, float, float] | None = None
) -> LinkTable:
    """BD TOPAGE links (surface water only), flow-oriented, EPSG:3035."""
    import pyogrio

    columns = [
        "TopoOH",
        "NatureTH",
        "PositionParRapportSolTH",
        "PersistanceTH",
        "SaliniteTH",
        "SensEcoulementTH",
        "ClasseLargeurTH",
        "CdCoursEau_1",
        "CdNoeudDebut",
        "CdNoeudFin",
        "BrasTH",
    ]
    frame = pyogrio.read_dataframe(gpkg, columns=columns)
    frame = frame[~frame["NatureTH"].isin(TOPAGE_DROP_NATURE)]
    frame = frame[frame["PositionParRapportSolTH"].fillna("surface") != "souterrain"]
    lines, keep = _lines_3035(frame.geometry.values, str(frame.crs))
    frame = frame[keep]
    lines = [line for line, k in zip(lines, keep, strict=True) if k]
    start = frame["CdNoeudDebut"].fillna("").astype(str).to_numpy()
    end = frame["CdNoeudFin"].fillna("").astype(str).to_numpy()
    # Missing node codes: fall back on rounded coordinates.
    coord_start, coord_end = endpoint_nodes(lines)
    start = np.where(start == "", np.char.add("c", coord_start.astype(str)), start)
    end = np.where(end == "", np.char.add("c", coord_end.astype(str)), end)
    ids = node_ids(np.concatenate([start, end]))
    s, e = ids[: len(start)], ids[len(start) :]
    width_class = frame["ClasseLargeurTH"].fillna("").astype(str).to_numpy()
    names = frame["TopoOH"].fillna("").astype(str).to_numpy()
    codes = frame["CdCoursEau_1"].fillna("").astype(str).to_numpy()
    codes = np.where(codes == "", names, codes)
    table = LinkTable(
        source="topage",
        lines=lines,
        start=s,
        end=e,
        name=names.astype(object),
        code=codes.astype(object),
        width_min=np.array([TOPAGE_WIDTH_MIN.get(c, np.nan) for c in width_class]),
        width_max=np.array([TOPAGE_WIDTH.get(c, np.nan) for c in width_class]),
        strahler=np.zeros(len(lines), dtype=np.int16),
        canal=(frame["NatureTH"].to_numpy() == "Canal"),
        intermittent=(frame["PersistanceTH"].to_numpy() == "intermittent"),
        secondary=(frame["BrasTH"].to_numpy() == "secondaire"),
        tidal=(frame["SaliniteTH"].fillna(False).to_numpy().astype(bool))
        | (frame["NatureTH"].to_numpy() == "Plan d'eau - estuaire"),
    )
    if bbox_3035 is not None:
        table = table.subset(_in_bbox(table.lines, bbox_3035))
    return table


def read_osor(gpkg: Path) -> LinkTable:
    """OS Open Rivers watercourse links, flow-oriented, EPSG:3035."""
    import pyogrio

    frame = pyogrio.read_dataframe(gpkg, layer="watercourse_link")
    lines, keep = _lines_3035(frame.geometry.values, str(frame.crs))
    frame = frame[keep]
    lines = [line for line, k in zip(lines, keep, strict=True) if k]
    start = frame["start_node"].astype(str).to_numpy()
    end = frame["end_node"].astype(str).to_numpy()
    reverse = frame["flow_direction"].astype(str).str.contains("opposite").to_numpy()
    for i in np.flatnonzero(reverse):
        lines[i] = lines[i][::-1]
    s0 = np.where(reverse, end, start)
    e0 = np.where(reverse, start, end)
    ids = node_ids(np.concatenate([s0, e0]))
    primary = frame["watercourse_name"].fillna("").astype(str).to_numpy()
    alternative = (
        frame["watercourse_name_alternative"].fillna("").astype(str).to_numpy()
    )
    # Welsh first names (``Afon Hafren``) carry the English one as alternative.
    english = np.char.startswith(alternative.astype(str), "River ")
    names = np.where(english, alternative, primary)
    form = frame["form"].astype(str).to_numpy()
    return LinkTable(
        source="osor",
        lines=lines,
        start=ids[: len(s0)],
        end=ids[len(s0) :],
        name=names.astype(object),
        code=names.astype(object),
        width_min=np.full(len(lines), np.nan),
        width_max=np.full(len(lines), np.nan),
        strahler=np.zeros(len(lines), dtype=np.int16),
        canal=(form == "canal"),
        intermittent=np.zeros(len(lines), dtype=bool),
        tidal=(form == "tidalRiver"),
    )


def _in_bbox(
    lines: list[np.ndarray], bbox: tuple[float, float, float, float]
) -> np.ndarray:
    minx, miny, maxx, maxy = bbox
    return np.array(
        [
            len(line) > 0
            and line[:, 0].max() >= minx
            and line[:, 0].min() <= maxx
            and line[:, 1].max() >= miny
            and line[:, 1].min() <= maxy
            for line in lines
        ],
        dtype=bool,
    )


def fetch_euhydro(
    bbox_lonlat: tuple[float, float, float, float],
    orders: tuple[int, ...],
    target: Path,
    cell_deg: float = 1.0,
) -> Path:
    """Download EU-Hydro lines of ``orders`` over ``bbox_lonlat`` (resumable).

    Queries the anonymous EEA ArcGIS REST service cell by cell (1° cells, paged by
    1 000 features, EPSG:3035 GeoJSON); each finished cell is cached as a file.
    Returns the directory of cell files.
    """
    target.mkdir(parents=True, exist_ok=True)
    lon0, lat0, lon1, lat1 = bbox_lonlat
    fields = "OBJECT_ID,DFDD,nameText,nameTxtInt,STRAHLER,FNODE,TNODE,TR"
    with httpx.Client(timeout=120.0) as client:
        for order in orders:
            layer = EUHYDRO_LAYERS[order]
            for lon in np.arange(lon0, lon1, cell_deg):
                for lat in np.arange(lat0, lat1, cell_deg):
                    path = target / f"s{order}_{lon:+.1f}_{lat:+.1f}.geojson"
                    if path.exists():
                        continue
                    envelope = json.dumps(
                        {
                            "xmin": float(lon),
                            "ymin": float(lat),
                            "xmax": float(min(lon + cell_deg, lon1)),
                            "ymax": float(min(lat + cell_deg, lat1)),
                            "spatialReference": {"wkid": 4326},
                        }
                    )
                    features: list[dict] = []
                    offset = 0
                    while True:
                        params = {
                            "where": "1=1",
                            "geometry": envelope,
                            "geometryType": "esriGeometryEnvelope",
                            "spatialRel": "esriSpatialRelIntersects",
                            "outFields": fields,
                            "outSR": "3035",
                            "orderByFields": "OBJECTID",
                            "resultOffset": str(offset),
                            "resultRecordCount": str(EUHYDRO_PAGE),
                            "f": "geojson",
                        }
                        page = None
                        for attempt in range(4):
                            try:
                                response = client.get(
                                    f"{EUHYDRO_URL}/{layer}/query", params=params
                                )
                                response.raise_for_status()
                                page = response.json()
                                break
                            except (httpx.HTTPError, ValueError):
                                time.sleep(2.0 * (attempt + 1))
                        if page is None or "features" not in page:
                            raise RuntimeError(
                                f"EU-Hydro: échec de la requête {path.name}"
                            )
                        features.extend(page["features"])
                        if len(page["features"]) < EUHYDRO_PAGE and not page.get(
                            "exceededTransferLimit"
                        ):
                            break
                        offset += len(page["features"])
                    tmp = path.with_suffix(".tmp")
                    tmp.write_text(
                        json.dumps({"type": "FeatureCollection", "features": features}),
                        encoding="utf-8",
                    )
                    tmp.replace(path)
    return target


_EUHYDRO_CUTS = (
    " DU CONFLUENT",
    " DE SA SOURCE",
    " DE LA SOURCE",
    " DEPUIS",
    " DESDE",
    " HASTA",
    " DU ",
    " À ",
    " AU ",
    " ET ",
    " (",
    ",",
)
_EUHYDRO_PREFIXES = (
    "UNTERE ",
    "OBERE ",
    "MITTLERE ",
    "UNTERER ",
    "OBERER ",
    "MITTLERER ",
)
_ROMAN = re.compile(r"^(?:[IVX]+|\d+[A-Z]?)$")


def clean_euhydro_name(raw: str) -> str:
    """River name from an EU-Hydro water-body name (``MEUSE 6`` -> ``Meuse``).

    EU-Hydro carries the names of the Water Framework Directive water bodies:
    upper case, reach numbers, "from ... to ..." descriptions.
    """
    text = raw.replace("\xa0", " ").strip()
    if text.upper() in ("", "UNK", "N_A", "NONE"):
        return ""
    upper = text.upper()
    for cut in _EUHYDRO_CUTS:
        index = upper.find(cut, 1)
        if index > 0:
            text, upper = text[:index], upper[:index]
    for prefix in _EUHYDRO_PREFIXES:
        if upper.startswith(prefix):
            text, upper = text[len(prefix) :], upper[len(prefix) :]
    words = [w for w in text.split() if not _ROMAN.match(w.upper())]
    text = " ".join(words).strip(" .-")
    if len(text) <= 2:
        return ""
    if text.isupper():
        text = " ".join(part.capitalize() for part in text.lower().split())
        text = re.sub(r"\bL'(\w)", lambda m: "l'" + m.group(1).upper(), text)
        text = re.sub(r"^(Le|La|Les|R[ií]o) ", lambda m: m.group(1).lower() + " ", text)
    return text


def read_euhydro(directory: Path) -> LinkTable:
    """EU-Hydro links from the cell files of :func:`fetch_euhydro` (deduplicated)."""
    seen: set[str] = set()
    lines: list[np.ndarray] = []
    starts, ends, names, codes, orders, canal = [], [], [], [], [], []
    for path in sorted(directory.glob("*.geojson")):
        for feature in json.loads(path.read_text(encoding="utf-8"))["features"]:
            props = feature.get("properties") or {}
            key = str(props.get("OBJECT_ID"))
            geometry = feature.get("geometry") or {}
            if key in seen or geometry.get("type") not in (
                "LineString",
                "MultiLineString",
            ):
                continue
            seen.add(key)
            coords = geometry["coordinates"]
            if geometry["type"] == "MultiLineString":
                coords = [pt for part in coords for pt in part]
            line = np.asarray(coords, dtype=np.float64)[:, :2]
            if len(line) < 2:
                continue
            lines.append(line)
            starts.append(str(props.get("FNODE") or f"s{key}"))
            ends.append(str(props.get("TNODE") or f"e{key}"))
            name = props.get("nameText") or props.get("nameTxtInt") or ""
            names.append(clean_euhydro_name(str(name)))
            codes.append(names[-1])
            orders.append(int(props.get("STRAHLER") or 0))
            canal.append(str(props.get("DFDD") or "") in EUHYDRO_CANAL_DFDD)
    ids = node_ids(np.array(starts + ends))
    n = len(lines)
    return LinkTable(
        source="euhydro",
        lines=lines,
        start=ids[:n],
        end=ids[n:],
        name=np.array(names, dtype=object),
        code=np.array(codes, dtype=object),
        width_min=np.full(n, np.nan),
        width_max=np.full(n, np.nan),
        strahler=np.array(orders, dtype=np.int16),
        canal=np.array(canal, dtype=bool),
        intermittent=np.zeros(n, dtype=bool),
        tidal=np.zeros(n, dtype=bool),
    )


def read_natural_earth(rivers_geojson: Path, grid) -> LinkTable:  # noqa: ANN001
    """Natural Earth lines of ``rivers.geojson`` (map pixels) back in EPSG:3035.

    Without topology nor flow direction: the caller orients them by altitude.
    """
    features = json.loads(rivers_geojson.read_text(encoding="utf-8"))["features"]
    lines, names, ranks = [], [], []
    for feature in features:
        geometry = feature.get("geometry") or {}
        parts = (
            [geometry["coordinates"]]
            if geometry.get("type") == "LineString"
            else geometry.get("coordinates", [])
        )
        for part in parts:
            pts = np.asarray(part, dtype=np.float64)
            if len(pts) < 2:
                continue
            x, y = grid.pixel_to_projected(pts[:, 0], pts[:, 1])
            lines.append(np.column_stack([x, y]))
            props = feature.get("properties") or {}
            names.append(str(props.get("name") or ""))
            ranks.append(int(props.get("scalerank") or 10))
    start, end = endpoint_nodes(lines, snap_m=50.0)
    n = len(lines)
    # Strahler proxy from Natural Earth's scalerank (0 = biggest river).
    strahler = np.clip(9 - np.array(ranks, dtype=np.int16) // 2, 3, 8).astype(np.int16)
    return LinkTable(
        source="naturalearth",
        lines=lines,
        start=start,
        end=end,
        name=np.array(names, dtype=object),
        code=np.array(names, dtype=object),
        width_min=np.full(n, np.nan),
        width_max=np.full(n, np.nan),
        strahler=strahler,
        canal=np.zeros(n, dtype=bool),
        intermittent=np.zeros(n, dtype=bool),
        tidal=np.zeros(n, dtype=bool),
    )
