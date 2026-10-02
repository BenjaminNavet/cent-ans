class_name TownGrowth
extends RefCounted

## Lot TB3 (ADR 0153), point 4 : croissance visible des villes 1:1 de `towns_1340.json`. Le plan
## de 1340 reste figé (ADR 0138) ; cette couche lui ajoute, par-dessus :
## - des **quartiers de faubourg** le long des routes des portes, au-delà des faubourgs de 1340,
##   quand la population simulée de la province dépasse celle de 1337 (`growth.suburbs`) ;
## - une **enceinte** (palissade, puis murs de pierre, tours et portes) quand un bâtiment de
##   fortification est construit en cours de partie et que la ville n'avait ni cette enceinte dans
##   son plan ni ce bâtiment en 1337 (`growth.enclosure`).
## Fonctions pures : elles rendent des instances (maillage, position, lacet, échelle) que
## `OutbuildingLayer` pose dans ses `MultiMesh`. Rendu seulement, aucune règle de jeu.

const KINDS: Array[String] = ["none", "palisade", "stone"]


## Nombre de quartiers de faubourg ajoutés pour un rapport population / population de 1337.
static func quarter_count(cfg: Dictionary, ratio: float) -> int:
	var suburbs: Dictionary = cfg.get("suburbs", {})
	var step := maxf(float(suburbs.get("population_step", 0.08)), 1e-3)
	return clampi(int(floor((ratio - 1.0) / step + 1e-6)), 0, int(suburbs.get("max_quarters", 6)))


## Rang de l'enceinte (0 aucune, 1 palissade, 2 pierre) que donnent des bâtiments construits.
static func enclosure_rank(cfg: Dictionary, buildings: Array) -> int:
	var best := 0
	for entry: Dictionary in (cfg.get("enclosure", {}) as Dictionary).get("kinds", []):
		for id in entry.get("buildings", []):
			if buildings.has(id):
				best = maxi(best, KINDS.find(str(entry.get("kind", "none"))))
	return best


## Enceinte à ajouter ("" : aucune) : celle des bâtiments construits, si elle dépasse à la fois
## l'enceinte du plan de 1340 et celle des bâtiments de départ (la fortification a monté).
static func enclosure_to_add(cfg: Dictionary, buildings: Array, initial_buildings: Array, plan_walls: String) -> String:
	var wanted := enclosure_rank(cfg, buildings)
	var had := maxi(enclosure_rank(cfg, initial_buildings), maxi(KINDS.find(plan_walls), 0))
	return KINDS[wanted] if wanted > had else ""


## Instances des quartiers de faubourg : maisons du kit des villes en deux rangs le long de la
## route de chaque porte. `height` : Callable(Vector2 unités) → altitude de pose (m).
static func suburb_instances(cfg: Dictionary, town: Dictionary, index: int, center: Vector2, mpu: float, quarters: int, height: Callable) -> Array:
	var out: Array = []
	if quarters <= 0:
		return out
	var suburbs: Dictionary = cfg.get("suburbs", {})
	var models: Array = suburbs.get("house_models", [])
	if models.is_empty():
		return out
	var radii := PackedFloat32Array(town.get("radii", []))
	if radii.is_empty():
		return out
	var houses := int(suburbs.get("houses_per_quarter", 14))
	var frontage := float(suburbs.get("frontage_m", 16.0))
	var setback := float(suburbs.get("setback_m", 10.0))
	var gap := float(suburbs.get("gap_m", 40.0))
	var bearings: Array = []
	for gate: Dictionary in town.get("gates", []):
		bearings.append(float(gate["bearing"]))
	var seed_value := absi(str(town.get("id", index)).hash())
	if bearings.is_empty():
		for k in 3:
			bearings.append(fposmod(float(seed_value % 360) + 120.0 * k, 360.0))
	var manifest := TownBuilder.manifest()
	for q in quarters:
		var bearing: float = bearings[q % bearings.size()]
		var ring := q / bearings.size()
		var angle := deg_to_rad(bearing)
		var dir := Vector2(cos(angle), sin(angle))
		var side := Vector2(-dir.y, dir.x)
		var start := TownPlan.radius_at(radii, angle)
		for fb: Dictionary in town.get("faubourgs", []):
			if absf(fposmod(float(fb["bearing"]) - bearing + 180.0, 360.0) - 180.0) < 20.0:
				start = maxf(start, float(fb["start_m"]) + float(fb["length_m"]))
		var length := ceilf(houses / 2.0) * frontage
		start += gap + float(ring) * (length + gap)
		for k in houses:
			var h := hash(Vector3i(seed_value, q, k))
			var model := str(models[h % models.size()])
			var row := 1.0 if k % 2 == 0 else -1.0
			var depth := float((manifest.get(model, {}) as Dictionary).get("depth", 7.0))
			var along := start + (float(k / 2) + 0.5) * frontage + float((h >> 8) % 100) / 100.0 * 3.0
			var local := dir * along + side * row * (setback + depth * 0.5 + float((h >> 16) % 100) / 100.0 * 4.0)
			var px := center + local / mpu
			# Façade (+Z du modèle) vers la route.
			var facing := -side * row
			out.append({
				"key": "kit:" + model,
				"settlement": index,
				"family": "suburb",
				"level": q + 1,
				"model": model,
				"px": px,
				"yaw": atan2(facing.x, facing.y) + (float((h >> 4) % 100) / 100.0 - 0.5) * 0.25,
				"base_m": float(height.call(px)),
				"scale": Vector3.ONE,
				"tint": 0.35 + float((h >> 12) % 100) / 100.0 * 0.3,
				"top": 15.0,
				"reach": 12.0,
			})
	return out


## Instances de l'enceinte `kind` (`palisade` ou `stone`) : un pan par relèvement des `radii`, à
## `margin_m` hors du bâti, tours aux angles (pierre) et portes sur les routes. `walls` :
## dimensions de `towns_1340.json` (`params.walls`).
static func enclosure_instances(cfg: Dictionary, walls: Dictionary, town: Dictionary, index: int, center: Vector2, mpu: float, kind: String, height: Callable) -> Array:
	var out: Array = []
	var radii := PackedFloat32Array(town.get("radii", []))
	var dims: Dictionary = walls.get(kind, {})
	if radii.is_empty() or dims.is_empty():
		return out
	var enclosure: Dictionary = cfg.get("enclosure", {})
	var margin := float(enclosure.get("margin_m", 35.0))
	var sink := float(enclosure.get("sink_m", 3.0))
	var stone := kind == "stone"
	var layer := "Masonry" if stone else "Planks"
	var wall_h := float(dims.get("height_m", 9.0))
	var thick := float(dims.get("thickness_m", 2.0))
	var rank := KINDS.find(kind)
	var n := radii.size()
	var points: Array[Vector2] = []
	for k in n:
		var a := TAU * float(k) / float(n)
		points.append(Vector2(cos(a), sin(a)) * (radii[k] + margin))
	var perimeter := 0.0
	for k in n:
		perimeter += points[k].distance_to(points[(k + 1) % n])
	for k in n:
		var a := points[k]
		var b := points[(k + 1) % n]
		var d := b - a
		var px := center + (a + b) * 0.5 / mpu
		out.append({
			"key": "box:" + layer,
			"settlement": index,
			"family": "enclosure",
			"part": "wall",
			"level": rank,
			"px": px,
			"yaw": atan2(-d.y, d.x),
			"base_m": float(height.call(px)),
			"lift": -sink,
			"scale": Vector3(d.length() + thick, wall_h + sink, thick),
			"top": wall_h + 6.0,
			"reach": d.length(),
		})
	var spacing := float(dims.get("tower_spacing_m", 0.0))
	if stone and spacing > 0.0:
		var every := maxi(1, int(round(spacing / maxf(perimeter / float(n), 1.0))))
		var tower_r := float(dims.get("tower_radius_m", 4.5))
		var tower_h := float(dims.get("tower_height_m", 14.0))
		for k in range(0, n, every):
			var px := center + points[k] / mpu
			out.append({
				"key": "tower",
				"settlement": index,
				"family": "enclosure",
				"part": "tower",
				"level": rank,
				"px": px,
				"yaw": 0.0,
				"base_m": float(height.call(px)),
				"lift": -sink,
				"scale": Vector3(tower_r, tower_h + sink, tower_r),
				"top": (tower_h + sink) * 1.5,
				"reach": tower_r,
			})
	for gate: Dictionary in town.get("gates", []):
		var angle := deg_to_rad(float(gate["bearing"]))
		var dir := Vector2(cos(angle), sin(angle))
		var px := center + dir * (TownPlan.radius_at(radii, angle) + margin) / mpu
		var gate_h := float(enclosure.get("gate_height_m", 16.0)) if stone else 7.0
		out.append({
			"key": "box:" + layer,
			"settlement": index,
			"family": "enclosure",
			"part": "gate",
			"level": rank,
			"px": px,
			"yaw": atan2(-dir.y, dir.x),
			"base_m": float(height.call(px)),
			"lift": -sink,
			"scale": Vector3(float(walls.get("gate_depth_m", 12.0)), gate_h + sink, 11.0),
			"top": gate_h + 6.0,
			"reach": 12.0,
		})
	return out
