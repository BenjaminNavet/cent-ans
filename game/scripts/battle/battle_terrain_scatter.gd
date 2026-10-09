class_name BattleTerrainScatter
extends RefCounted

## Semis du terrain de bataille (SC BT7, découpé de `battle_terrain.gd`) : arbres, buissons, haies, vergers,
## feuillus DA6, rochers et emprises du décor. L'état reste sur `BattleTerrain` (`host`).

var host: BattleTerrain


func _init(p_terrain: BattleTerrain) -> void:
	host = p_terrain


## Emprises du décor dont les arbres et buissons semés s'écartent : bâtiments (+ 4 m), zones
## (hameaux, cimetières, manoirs, fermes, vignes, labours, prés, vergers : ceux-ci ont leurs
## propres arbres), accessoires, camps et convois.
func _prepare_decor() -> void:
	host._decor_clear.clear()
	host.decor_on = false
	var decor: Dictionary = host.terrain.get("decor", {})
	if decor.is_empty():
		return
	for b in decor.get("buildings", []):
		host._decor_clear.append([Vector2(float(b["x"]), float(b["z"])), Vector2(float(b["length"]), float(b["width"])) * 0.5 + Vector2(4, 4), float(b["yaw"])])
	for a in decor.get("areas", []):
		host._decor_clear.append([Vector2(float(a["x"]), float(a["z"])), Vector2(float(a["length"]), float(a["width"])) * 0.5 + Vector2(2, 2), float(a["yaw"])])
	for p in decor.get("props", []):
		host._decor_clear.append([Vector2(float(p["x"]), float(p["z"])), Vector2(float(p["length"]), float(p["depth"])) * 0.5 + Vector2(1.5, 1.5), float(p["yaw"])])
	for camp in decor.get("camps", []):
		var a: Dictionary = camp["area"]
		host._decor_clear.append([Vector2(float(a["x"]), float(a["z"])), Vector2(float(a["length"]), float(a["width"])) * 0.5 + Vector2(6, 6), float(a["yaw"])])
		for w in camp.get("convoy", []):
			host._decor_clear.append([Vector2(float(w["x"]), float(w["z"])), Vector2(float(w["length"]), float(w["depth"])) * 0.5 + Vector2(2, 2), float(w["yaw"])])


func _in_decor(p: Vector2) -> bool:
	for r in host._decor_clear:
		var local: Vector2 = (p - (r[0] as Vector2)).rotated(-float(r[2]))
		var half: Vector2 = r[1]
		if absf(local.x) <= half.x and absf(local.y) <= half.y:
			return true
	return false


## Vergers : pommiers et poiriers en quinconce (6,5 m), petits houppiers ; en fleurs au printemps.
func _plant_orchards(sets: Dictionary, tints: Dictionary) -> void:
	var decor: Dictionary = host.terrain.get("decor", {})
	var blossom := bool(decor.get("orchard_blossom", false))
	var rng := RandomNumberGenerator.new()
	rng.seed = 6606
	for area in decor.get("areas", []):
		if str(area["kind"]) != "orchard":
			continue
		var c := Vector2(float(area["x"]), float(area["z"]))
		var yaw := float(area["yaw"])
		var hl := float(area["length"]) * 0.5 - 3.0
		var hw := float(area["width"]) * 0.5 - 3.0
		var step := 6.5
		var v := -hw
		var row := 0
		while v <= hw:
			var u := -hl + (step * 0.5 if row % 2 == 1 else 0.0)
			while u <= hl:
				var p := c + Vector2(u + rng.randf_range(-0.6, 0.6), v + rng.randf_range(-0.6, 0.6)).rotated(yaw)
				u += step
				if rng.randf() < 0.06 or _near_road(p, 3.0):
					continue  # un arbre mort arraché, ou le chemin
				# Fruitiers à part (essence « fruit », taille réelle : échelle 0,85-1,1).
				var t := _tree_transform(rng, p.x, p.y, 0.85, 1.1)
				t.origin.y = host.height_at(p.x, p.y) - 0.2
				sets["fruit"].append(t)
				if blossom:
					# Fleurs blanc rosé mêlées aux jeunes feuilles : éclairci, pas blanc pur.
					var w := rng.randf_range(1.12, 1.35)
					tints["fruit"].append(Color(w * 1.08, w * rng.randf_range(0.92, 1.0), w * rng.randf_range(0.82, 0.92)))
				else:
					tints["fruit"].append(_tree_tint(rng) * Color(0.95, 1.05, 0.9))
			v += step
			row += 1


func _near_road(p: Vector2, margin: float) -> bool:
	for road in host.roads:
		for i in range(road.size() - 1):
			var a := road[i]
			var b := road[i + 1]
			if absf(a.y - p.y) > 200.0 and absf(b.y - p.y) > 200.0 and absf(a.x - p.x) > 200.0:
				continue
			if Geometry2D.get_closest_point_to_segment(p, a, b).distance_to(p) < margin:
				return true
	return false


## Place un arbre : transformée (échelle, lacet) + teinte par instance.
func _tree_transform(rng: RandomNumberGenerator, x: float, z: float, scale_min: float, scale_max: float) -> Transform3D:
	var s := rng.randf_range(scale_min, scale_max)
	var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * rng.randf_range(0.9, 1.15), s))
	return Transform3D(basis, Vector3(x, host.world_height(x, z) - 0.25, z))


func _tree_tint(rng: RandomNumberGenerator) -> Color:
	# Feuillage de la saison (roux d'automne, feuilles sèches et brunes des chênes l'hiver).
	var autumn_share := 0.12
	match host.season_key:
		"autumn":
			autumn_share = 0.55
		"summer":
			autumn_share = 0.04
		"winter":
			# Feuillus nus (ramilles grises) ; la teinte ne module que la luminance.
			var g := rng.randf_range(0.85, 1.12)
			return Color(g, g * 0.98, g * 0.95)
	var autumn := rng.randf() < autumn_share
	if autumn and host.season_key == "autumn":
		# Automne marqué : roux, ocre et or (la texture des feuilles est verte : forte modulation).
		return Color(rng.randf_range(1.5, 2.0), rng.randf_range(0.8, 1.05), rng.randf_range(0.3, 0.45))
	if autumn:
		return Color(rng.randf_range(1.05, 1.25), rng.randf_range(0.9, 1.0), rng.randf_range(0.55, 0.7))
	var v := rng.randf_range(0.78, 1.12)
	return Color(v * rng.randf_range(0.9, 1.05), v, v * rng.randf_range(0.85, 1.05))


func _build_trees() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 1337
	var sets := {"oak": [], "poplar": [], "bush": [], "far": [], "hedge": [], "fruit": []}
	var tints := {"oak": [], "poplar": [], "bush": [], "far": [], "hedge": [], "fruit": []}
	var siege_center := Vector2(-1e6, -1e6)
	if host.terrain.has("siege"):
		siege_center = host.terrain["siege"].get("center", Vector2(600, 560))
	var near_woods := float(host.biome["near_woods"])
	var far_woods := float(host.biome["far_woods"])
	# Densité des bois selon le terrain (forêt serrée, lande clairsemée).
	var area_per_tree := lerpf(85.0, 36.0, host.woodland)
	# Bois de la simulation : denses, lisière de buissons. R2 : un bois est une grappe de disques
	# qui se chevauchent (ancre, lobes, bosquets) : un arbre tiré dans un disque déjà couvert par
	# un disque précédent est écarté (densité uniforme), les buissons de lisière ne restent que
	# sur le pourtour de la réunion ; les arbres des lisières sont plus petits et plus clairsemés.
	var forests: Array = host.terrain.get("forests", [])
	for k in forests.size():
		var zone: Dictionary = forests[k]
		var r := float(zone["radius"])
		var count := clampi(int(PI * r * r / area_per_tree), 4, 1000)
		for _i in count:
			var angle := rng.randf() * TAU
			var dist := sqrt(rng.randf()) * r
			var x := float(zone["x"]) + cos(angle) * dist
			var z := float(zone["z"]) + sin(angle) * dist
			if host._in_zones_before(forests, k, x, z):
				continue
			# Routes et ruisseaux restent dégagés dans les bois.
			if _near_road(Vector2(x, z), 4.0) or host.in_water(x, z):
				continue
			var edge := host._zones_edge_distance(forests, x, z)
			if edge < 6.0 and rng.randf() < 0.35:
				continue
			var kind := "poplar" if rng.randf() < 0.15 else "oak"
			var grow := lerpf(0.75, 1.0, clampf(edge / 14.0, 0.0, 1.0))
			sets[kind].append(_tree_transform(rng, x, z, 0.75 * grow, 1.3 * grow))
			tints[kind].append(_tree_tint(rng))
		for _i in int(TAU * r / 6.0):
			var angle := rng.randf() * TAU
			var x := float(zone["x"]) + cos(angle) * (r + rng.randf_range(-2.0, 5.0))
			var z := float(zone["z"]) + sin(angle) * (r + rng.randf_range(-2.0, 5.0))
			if host._zones_edge_distance(forests, x, z) > 1.0:
				continue
			sets["bush"].append(_tree_transform(rng, x, z, 0.6, 1.4))
			tints["bush"].append(_tree_tint(rng))
	var field := Rect2(0, 0, host.FIELD_W, host.FIELD_D)
	# Bois décoratifs de l'anneau proche, haies et arbres isolés.
	var step := 13.0
	var z := host.NEAR_RECT.position.y
	while z < host.NEAR_RECT.end.y:
		var x := host.NEAR_RECT.position.x
		while x < host.NEAR_RECT.end.x:
			var px := x + rng.randf_range(-5.0, 5.0)
			var pz := z + rng.randf_range(-5.0, 5.0)
			x += step
			var p := Vector2(px, pz)
			if field.grow(25.0).has_point(p) or p.distance_to(siege_center) < 280.0:
				continue
			var n := host._woods.get_noise_2d(px, pz)
			var isolated := rng.randf() < 0.004
			if n < near_woods and not isolated:
				continue
			if host._in_sea(px, pz) or host.river_distance(px, pz) < host.river_span_at(px) + 6.0 or _near_road(p, 9.0):
				continue
			if n >= near_woods and n < near_woods + 0.06 and rng.randf() < 0.6:
				sets["bush"].append(_tree_transform(rng, px, pz, 0.7, 1.5))
				tints["bush"].append(_tree_tint(rng))
				continue
			var kind := "poplar" if rng.randf() < (0.35 if isolated else 0.1) else "oak"
			sets[kind].append(_tree_transform(rng, px, pz, 0.8, 1.35))
			tints[kind].append(_tree_tint(rng))
		z += step
	# Quelques buissons épars dans le champ, hors du centre.
	for _i in int(140.0 * host.FIELD_W * host.FIELD_D / 960000.0):
		var p := Vector2(rng.randf_range(0.0, host.FIELD_W), rng.randf_range(0.0, host.FIELD_D))
		if absf(p.x - host.FIELD_W * 0.5) < 420.0 * host.field_scale_x() and absf(p.y - host.FIELD_D * 0.5) < host.FIELD_D * 0.5 - 120.0:
			continue
		if host.river_distance(p.x, p.y) < host.river_span_at(p.x) or _near_road(p, 6.0) or p.distance_to(siege_center) < 200.0 or host.in_water(p.x, p.y):
			continue
		if _in_site_clearing(p):
			continue
		sets["bush"].append(_tree_transform(rng, p.x, p.y, 0.6, 1.3))
		tints["bush"].append(_tree_tint(rng))
	_plant_hedges(rng, sets, tints)
	# Bois lointains (anneau lointain) : arbres simplifiés, plus gros, sans ombre.
	step = 42.0
	z = host.FAR_RECT.position.y
	while z < host.FAR_RECT.end.y:
		var x := host.FAR_RECT.position.x
		while x < host.FAR_RECT.end.x:
			var px := x + rng.randf_range(-15.0, 15.0)
			var pz := z + rng.randf_range(-15.0, 15.0)
			x += step
			if host.NEAR_RECT.grow(-60.0).has_point(Vector2(px, pz)):
				continue
			if host._woods.get_noise_2d(px * 0.6, pz * 0.6) < _far_woods_at(px, pz, far_woods) or host._in_sea(px, pz):
				continue
			sets["far"].append(_tree_transform(rng, px, pz, 1.3, 2.0))
			tints["far"].append(_tree_tint(rng))
		z += step
	host.tree_count = 0
	_plant_orchards(sets, tints)
	for kind in sets:
		host.tree_count += (sets[kind] as Array).size()
	_plant_da6(sets, tints)


func _index_decor_tree(species: String, t: Transform3D) -> void:
	var params: Dictionary = BattleTrees.SPECIES.get(species, {})
	if params.is_empty():
		return
	var key := Vector2i(floori(t.origin.x / host.TREE_TILE), floori(t.origin.z / host.TREE_TILE))
	if not host._decor_trees.has(key):
		host._decor_trees[key] = []
	(host._decor_trees[key] as Array).append({"species": species, "position": t.origin, "height": float(params["height"]) * t.basis.y.length(), "radius": float(params["width"]) * 0.5 * t.basis.x.length()})


## Essences des feuillus (chêne, hêtre, frêne ; saule et peuplier près de l'eau), tuiles par
## niveau de détail (choix par instance dans les shaders) et imposteurs au-delà de 300 m.
func _plant_da6(sets: Dictionary, tints: Dictionary) -> void:
	host._decor_trees.clear()
	host.tree_view = BattleTrees.new()
	host.tree_view.name = "Trees"
	host.add_child(host.tree_view)
	var by_species := {}
	var impostors := {}  # tuile (640 m) -> [transforms, tints, rows]
	for kind in sets:
		var transforms: Array = sets[kind]
		for i in transforms.size():
			var t: Transform3D = transforms[i]
			var species := str(kind)
			match kind:
				"oak", "far":
					species = _broadleaf_species(t.origin, kind == "oak")
				"hedge":
					species = "bush"
			if kind == "far":
				# Anneau lointain : arbres ramenés à la taille réelle, imposteurs seuls.
				t.basis = t.basis * 0.62
			if not by_species.has(species):
				by_species[species] = [[], []]
			if kind != "far":
				by_species[species][0].append(t)
				by_species[species][1].append(tints[kind][i])
				_index_decor_tree(species, t)
			var row := BattleTrees.impostor_row(species)
			if row >= 0:
				var key := Vector2i(floori(t.origin.x / (host.TREE_TILE * 4.0)), floori(t.origin.z / (host.TREE_TILE * 4.0)))
				if not impostors.has(key):
					impostors[key] = [[], [], []]
				impostors[key][0].append(t)
				impostors[key][1].append(tints[kind][i])
				impostors[key][2].append(row)
	var winter := host.season_key == "winter"
	var lod_k := RenderQuality.battle_lod_scale
	var lod1 := BattleTrees.LOD1_DISTANCE * lod_k
	var far := BattleTrees.IMPOSTOR_DISTANCE
	var slack := host.TREE_TILE * 0.75 + BattleTrees.LOD_BAND
	for species in by_species:
		var transforms: Array = by_species[species][0]
		var tile_tints: Array = by_species[species][1]
		if transforms.is_empty():
			continue
		var tiles := {}
		for i in transforms.size():
			var o: Vector3 = (transforms[i] as Transform3D).origin
			var key := Vector2i(floori(o.x / host.TREE_TILE), floori(o.z / host.TREE_TILE))
			if not tiles.has(key):
				tiles[key] = []
			(tiles[key] as Array).append(i)
		for key in tiles:
			var members: Array = tiles[key]
			if species == "bush":
				# Buissons et haies : maillage unique ; portée des tuiles comme avant.
				var reach := (host.HEDGE_DISTANCE if sets["hedge"].size() > 0 else host.BUSH_DISTANCE) * lod_k
				var bush_lod1 := BattleTrees.BUSH_LOD1_DISTANCE * lod_k
				_da6_tile(species, 0, winter, 0.0, bush_lod1, members, transforms, tile_tints, key, 0.0, bush_lod1 + slack, false)
				_da6_tile(species, 1, winter, bush_lod1, 100000.0, members, transforms, tile_tints, key, maxf(bush_lod1 - slack, 0.0), reach, false)
				continue
			_da6_tile(species, 0, winter, 0.0, lod1, members, transforms, tile_tints, key, 0.0, lod1 + slack, true)
			_da6_tile(species, 1, winter, lod1, far, members, transforms, tile_tints, key, maxf(lod1 - slack, 0.0), far + slack, false)
	for key in impostors:
		var entry: Array = impostors[key]
		host.tree_view.add_impostor_tile("Impostors_%d_%d" % [key.x, key.y], entry[0], entry[1], entry[2])
	host.tree_view.bake_impostors.call_deferred(winter)


func _da6_tile(species: String, lod: int, winter: bool, lod_near: float, lod_far: float, members: Array, transforms: Array, tints: Array, key: Vector2i, range_begin: float, range_end: float, shadows: bool) -> void:
	var tile_xforms: Array = []
	var tile_tints: Array = []
	for i in members:
		tile_xforms.append(transforms[i])
		tile_tints.append(tints[i])
	MultiMeshKit.make(BattleTrees.mesh(species, lod, winter, lod_near, lod_far), tile_xforms, {"colors": true, "name": "Trees_%s_%d_%d_%d" % [species, lod, key.x, key.y], "shadow": shadows, "range_begin": range_begin, "range_end": range_end, "parent": host.tree_view}, tile_tints)


## Essence d'un feuillu selon le lieu (tirage haché, stable) : saules et peupliers au bord de
## l'eau ; chênes, hêtres (forêts, collines) et frênes (bocage, fonds frais) ailleurs.
func _broadleaf_species(o: Vector3, near: bool) -> String:
	var h := fposmod(sin(o.x * 12.9898 + o.z * 78.233) * 43758.5453, 1.0)
	var wet := near and (host.river_distance(o.x, o.z) < host.river_span_at(o.x) + 28.0 or host._in_zones(host._pools, o.x, o.z, 20.0))
	if wet or host.terrain_key == "marsh":
		if h < 0.55:
			return "willow"
		if h < 0.75:
			return "poplar"
		return "ash"
	var beech := 0.35 if host.terrain_key in ["forest", "hills", "mountains"] else 0.2
	var ash := 0.3 if host.terrain_key == "bocage" else 0.2
	if h < beech:
		return "beech"
	if h < beech + ash:
		return "ash"
	return "oak"


## Seuil des bois lointains ; au loin, les forêts réelles de la tuile d'horizon.
func _far_woods_at(x: float, z: float, biome_threshold: float) -> float:
	if host.horizon == null or not host.horizon.active:
		return biome_threshold
	var w := host.horizon.weight(x, z)
	if w <= 0.0:
		return biome_threshold
	return lerpf(biome_threshold, lerpf(0.55, -0.9, host.horizon.forest_at(x, z)), w)


## Pas de buissons épars dans les mares et sur la plage.
func _in_site_clearing(p: Vector2) -> bool:
	if host._in_zones(host._pools, p.x, p.y, 4.0):
		return true
	if not host._decor_clear.is_empty() and _in_decor(p):
		return true
	if not host._coast.is_empty():
		var west := str(host._coast["flank"]) == "west"
		var from_edge := p.x if west else host.FIELD_W - p.x
		if from_edge < float(host._coast["beach"]) + 5.0:
			return true
	return false


## Haies vives (buissons serrés tous les 1,8 m, portée longue), chênes têtards dans les
## haies du bocage .
func _plant_hedges(rng: RandomNumberGenerator, sets: Dictionary, tints: Dictionary) -> void:
	var bocage := host.terrain_key == "bocage"
	for o in host.terrain.get("obstacles", []):
		if str(o["kind"]) != "hedge":
			continue
		var a: Vector2 = o["a"]
		var b: Vector2 = o["b"]
		var length := a.distance_to(b)
		var steps := maxi(int(length / 1.8), 1)
		var normal := (b - a).normalized().orthogonal()
		var since_tree := rng.randf_range(0.0, 20.0)
		for k in steps + 1:
			var p := a.lerp(b, float(k) / float(steps)) + normal * rng.randf_range(-0.4, 0.4)
			var s := rng.randf_range(0.8, 1.1)
			var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * rng.randf_range(1.1, 1.45), s))
			sets["hedge"].append(Transform3D(basis, Vector3(p.x, host.height_at(p.x, p.y) - 0.3, p.y)))
			tints["hedge"].append(_tree_tint(rng) * Color(0.92, 0.95, 0.9))
			since_tree += 1.8
			if since_tree > (18.0 if bocage else 40.0) and rng.randf() < 0.35:
				since_tree = 0.0
				var t := _tree_transform(rng, p.x, p.y, 0.6, 0.95)
				t.origin.y = host.height_at(p.x, p.y) - 0.25
				sets["oak"].append(t)
				tints["oak"].append(_tree_tint(rng))


## Arbres par tuiles : une tuile de `TREE_TILE` m par MultiMesh pour que le moteur
## écarte ce qui est hors champ ou hors des ombres. Chênes et peupliers ont deux niveaux de
## détail par distance (`visibility_range`) : houppier complet avec ombre portée près, maillage
## allégé sans ombre au-delà de `TREE_LOD_DISTANCE`.
func _tree_layer(kind: String, transforms: Array, tints: Array) -> void:
	if transforms.is_empty():
		return
	var tile := host.TREE_TILE * (4.0 if kind == "far" else 1.0)
	var tiles := {}
	for i in transforms.size():
		var t: Transform3D = transforms[i]
		var key := Vector2i(floori(t.origin.x / tile), floori(t.origin.z / tile))
		if not tiles.has(key):
			tiles[key] = []
		(tiles[key] as Array).append(i)
	# Distances de LOD des arbres et buissons selon le préréglage de qualité.
	var lod_k := RenderQuality.battle_lod_scale
	for key in tiles:
		var members: Array = tiles[key]
		match kind:
			"oak", "poplar":
				_tree_tile(kind, members, transforms, tints, key, 0.0, host.TREE_LOD_DISTANCE * lod_k, true)
				_tree_tile(kind + "_lod", members, transforms, tints, key, host.TREE_LOD_DISTANCE * lod_k, 0.0, false)
			"bush":
				_tree_tile(kind, members, transforms, tints, key, 0.0, host.BUSH_DISTANCE * lod_k, false)
			"hedge":
				_tree_tile(kind, members, transforms, tints, key, 0.0, host.HEDGE_DISTANCE * lod_k, false)
			_:
				_tree_tile(kind, members, transforms, tints, key, 0.0, 0.0, false)


func _tree_tile(kind: String, members: Array, transforms: Array, tints: Array, key: Vector2i, range_begin: float, range_end: float, shadows: bool) -> void:
	var tile_xforms: Array = []
	var tile_tints: Array = []
	for i in members:
		tile_xforms.append(transforms[i])
		tile_tints.append(tints[i])
	MultiMeshKit.make(BattleMeshes.tree("bush" if kind == "hedge" else kind), tile_xforms, {"colors": true, "name": "Trees_%s_%d_%d" % [kind, key.x, key.y], "shadow": shadows, "range_begin": range_begin, "range_end": range_end, "parent": host}, tile_tints)


## Rochers : sur les pentes raides du champ et dans les collines de l'anneau proche.
func _build_rocks() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 777
	var transforms: Array[Transform3D] = []
	for _i in 2600:
		var x := rng.randf_range(host.NEAR_RECT.position.x, host.NEAR_RECT.end.x)
		var z := rng.randf_range(host.NEAR_RECT.position.y, host.NEAR_RECT.end.y)
		var h := host.world_height(x, z)
		var slope := Vector2(host.world_height(x + 4.0, z) - host.world_height(x - 4.0, z), host.world_height(x, z + 4.0) - host.world_height(x, z - 4.0)).length() / 8.0
		var in_field := x >= 0.0 and x <= host.FIELD_W and z >= 0.0 and z <= host.FIELD_D
		var chance := (smoothstep(0.12, 0.35, slope) + (0.0 if in_field else 0.04)) * float(host.biome["rocks"])
		if rng.randf() > chance or host.river_distance(x, z) < host.river_span_at(x) or host._in_sea(x, z) or host.in_water(x, z):
			continue
		if in_field and absf(x - host.FIELD_W * 0.5) < 380.0 * host.field_scale_x() and absf(z - host.FIELD_D * 0.5) < host.FIELD_D * 0.5 - 150.0:
			continue
		var s := rng.randf_range(0.5, 2.6)
		var basis := Basis(Vector3.UP, rng.randf() * TAU).rotated(Vector3.RIGHT, rng.randf_range(-0.3, 0.3)).scaled(Vector3(s * rng.randf_range(0.8, 1.5), s * rng.randf_range(0.5, 0.9), s))
		transforms.append(Transform3D(basis, Vector3(x, h - s * 0.25, z)))
	if transforms.is_empty():
		return
	MultiMeshKit.make(BattleMeshes.rock(), transforms, {"name": "Rocks", "parent": host})
