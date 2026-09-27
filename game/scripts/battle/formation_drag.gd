class_name FormationDrag
extends RefCounted

## CB1 : géométrie du glisser-droit et des groupes verrouillés (fonctions pures, testées par
## `game/tests/cb1_drag_formation_test.gd`). Aucune règle de jeu : la largeur retenue, les rangs
## bornés et la répartition au prorata d'un ordre de groupe sont calculés par le cœur
## (`sim-battle/src/formation_width.rs`) ; ce fichier traduit la souris en point, orientation
## et largeur, et place rigidement les régiments d'un groupe verrouillé.
##
## Conventions du cœur : `facing` = atan2(dx, dz) ; avant = (sin f, cos f) ; droite =
## (cos f, -sin f).

## Glisser plus court : un simple clic (pas de largeur).
const MIN_DRAG_M := 2.0


## Ligne d'un glisser-droit de `p0` à `p1` (points au sol), front tourné à l'opposé de la caméra
## `cam` : {ok, mid: Vector3, facing, width} ; `ok` faux pour un glisser trop court.
static func drag_line(p0: Vector3, p1: Vector3, cam: Vector3) -> Dictionary:
	var dir := Vector2(p1.x - p0.x, p1.z - p0.z)
	if dir.length() < MIN_DRAG_M:
		return {"ok": false, "mid": p1, "facing": NAN, "width": 0.0}
	var normal := Vector2(-dir.y, dir.x).normalized()
	var mid := (p0 + p1) * 0.5
	if normal.dot(Vector2(mid.x - cam.x, mid.z - cam.z)) < 0.0:
		normal = -normal
	return {"ok": true, "mid": mid, "facing": atan2(normal.x, normal.y), "width": dir.length()}


## Parts d'une largeur `total` entre des régiments de `counts` soldats (dans leur ordre
## gauche-droite) : `gap` mètres entre voisins, le reste au prorata des effectifs. Même calcul
## que `split_widths` du cœur (qui fait foi pour les ordres ; sert ici au déploiement).
static func split_widths(counts: Array, total: float, gap: float) -> Array[float]:
	var out: Array[float] = []
	if counts.is_empty():
		return out
	var share := maxf(total - gap * (counts.size() - 1), 0.0)
	var sum := 0.0
	for c in counts:
		sum += float(maxi(int(c), 1))
	for c in counts:
		out.append(share * float(maxi(int(c), 1)) / sum)
	return out


## Indices de `points` (Vector2 x, z) rangés de gauche à droite pour un front `facing`
## (projection sur l'axe droit, c'est-à-dire l'axe du glisser) ; à égalité, l'ordre d'origine.
static func lateral_order(points: Array, facing: float) -> Array[int]:
	var right := Vector2(cos(facing), -sin(facing))
	var keyed: Array = []
	for i in points.size():
		keyed.append([(points[i] as Vector2).dot(right), i])
	keyed.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0] or (a[0] == b[0] and a[1] < b[1]))
	var out: Array[int] = []
	for entry in keyed:
		out.append(int(entry[1]))
	return out


## Orientation moyenne (moyenne circulaire) de `facings`.
static func mean_facing(facings: Array) -> float:
	var s := 0.0
	var c := 0.0
	for f in facings:
		s += sin(float(f))
		c += cos(float(f))
	return atan2(s, c) if absf(s) + absf(c) > 1e-9 else 0.0


## Forme d'un groupe verrouillé à partir des unités `units` (dictionnaires de `get_units`) :
## {facing: orientation de référence, members: {id: Vector3(latéral, avant, écart d'orientation)}}
## — décalages au barycentre dans le repère du groupe.
static func lock_shape(units: Array) -> Dictionary:
	var members := {}
	if units.is_empty():
		return {"facing": 0.0, "members": members}
	var facings: Array = []
	var center := Vector2.ZERO
	for unit in units:
		facings.append(float(unit["facing"]))
		center += Vector2(float(unit["x"]), float(unit["z"]))
	center /= units.size()
	var ref := mean_facing(facings)
	var right := Vector2(cos(ref), -sin(ref))
	var forward := Vector2(sin(ref), cos(ref))
	for unit in units:
		var d := Vector2(float(unit["x"]), float(unit["z"])) - center
		members[int(unit["id"])] = Vector3(d.dot(right), d.dot(forward), angle_difference(ref, float(unit["facing"])))
	return {"facing": ref, "members": members}


## Places d'un groupe verrouillé déplacé en `point` (x, z) et tourné vers `facing` (NaN : son
## orientation de référence) : translation et rotation rigides des décalages, pas de
## redistribution. → [{id, x, z, facing}] dans l'ordre des ids `ids` (membres absents ignorés).
static func rigid_places(shape: Dictionary, ids: Array, point: Vector2, facing: float) -> Array:
	var f := facing if is_finite(facing) else float(shape.get("facing", 0.0))
	var right := Vector2(cos(f), -sin(f))
	var forward := Vector2(sin(f), cos(f))
	var members: Dictionary = shape.get("members", {})
	var out: Array = []
	for id in ids:
		if not members.has(int(id)):
			continue
		var m: Vector3 = members[int(id)]
		var at := point + right * m.x + forward * m.y
		out.append({"id": int(id), "x": at.x, "z": at.y, "facing": wrapf(f + m.z, -PI, PI)})
	return out
