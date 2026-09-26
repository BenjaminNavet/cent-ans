extends SceneTree

## Test headless du lot DC6c (suites DC4, ADR 0082) : réduction et masquage des maquettes voisines
## à l'échelle effective.
##  1. `SettlementFit` (fonctions pures) : à la taille de carte, réduction DC4 inchangée ; de près,
##     une maquette qui a la place retrouve sa taille pleine ; le rayon affiché ne dépasse jamais
##     la place laissée par les voisines ; l'échelle du porteur ne dépasse jamais 1 ; masquage.
##  2. Carte de campagne (headless) autour de Lille (zone la plus dense) : au loin, échelle 1 pour
##     toutes les maquettes (vue stratégique inchangée) ; en se rapprochant, maquettes masquées et
##     réduites de moins en moins nombreuses, aucune paire affichée ne se recouvre à plus de 20 % ;
##     coût d'une réécriture d'échelle (réduction + masquage).
## Usage : godot --headless --path game --script res://tests/dc6c_fit_scale_test.gd

const MIN_FIT := 0.4
const LILLE := Vector2(2310.3, 1657.9)

var _failures := 0


func _init() -> void:
	await process_frame
	_test_pure()
	await _test_map()
	if _failures > 0:
		push_error("dc6c_fit_scale_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("dc6c_fit_scale_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_pure() -> void:
	# Deux villages de rayon 3 à 4 unités : place 2 chacun, réduits à 2/3 sur la carte.
	var base := 3.0
	var room := 2.0
	_check(is_equal_approx(SettlementFit.fit_factor(room, base, MIN_FIT), 2.0 / 3.0), "map fit")
	_check(is_equal_approx(SettlementFit.zoom_scale(base, room, 1.0, MIN_FIT), 1.0), "scale 1 at map size")
	# À mi-taille (rayon 1,5 < place 2) : taille pleine, porteur = 0,5 / (2/3) = 0,75.
	var half := SettlementFit.zoom_scale(base, room, 0.5, MIN_FIT)
	_check(is_equal_approx(half, 0.75), "full size once it fits (%.3f)" % half)
	# Aucune place (INF) : échelle = sigma.
	_check(is_equal_approx(SettlementFit.zoom_scale(base, INF, 0.3, MIN_FIT), 0.3), "no neighbour")
	# Balayage : rayon affiché ≤ max(place, plancher), échelle ≤ 1, rayon affiché croissant avec sigma.
	for r: float in [0.3, 1.0, 1.9, 2.5, 6.0]:
		var previous := 0.0
		var sigma := 0.02
		while sigma <= 1.0:
			var s := SettlementFit.zoom_scale(base, r, sigma, MIN_FIT)
			var shown := base * SettlementFit.fit_factor(r, base, MIN_FIT) * s
			_check(s <= 1.0 + 1e-6, "scale ≤ 1 (room %.1f sigma %.2f)" % [r, sigma])
			_check(shown <= maxf(r, MIN_FIT * base * sigma) + 1e-5, "shown ≤ room (room %.1f sigma %.2f)" % [r, sigma])
			_check(shown >= previous - 1e-6, "monotonic (room %.1f sigma %.2f)" % [r, sigma])
			previous = shown
			sigma += 0.02
	# Masquage : 0 prioritaire (rayon 4), 1 à 4,5 (rayon 2 : 4,5 < 4 + 0,5 × 2 → masquée),
	# 2 sous la 1 masquée (ne compte pas), 3 ville emblématique protégée.
	var pairs := PackedInt64Array([SettlementFit.pair_key(0, 1), SettlementFit.pair_key(1, 2), SettlementFit.pair_key(0, 3)])
	pairs.sort()
	var positions := PackedVector2Array([Vector2(0, 0), Vector2(4.5, 0), Vector2(6.0, 0), Vector2(0, 1)])
	var protected := PackedByteArray([0, 0, 0, 1])
	var hidden := SettlementFit.absorbed(pairs, positions, PackedFloat32Array([4.0, 2.0, 1.0, 1.0]), protected, 0.5)
	_check(hidden == PackedByteArray([0, 1, 0, 0]), "absorbed at map size %s" % hidden)
	# De près (rayons / 4) : plus rien de masqué.
	hidden = SettlementFit.absorbed(pairs, positions, PackedFloat32Array([1.0, 0.5, 0.25, 1.0]), protected, 0.5)
	_check(hidden == PackedByteArray([0, 0, 0, 0]), "nothing absorbed close up %s" % hidden)
	_check(SettlementFit.pair_key(7, 3) == SettlementFit.pair_key(3, 7) and SettlementFit.pair_key(3, 7) >> 16 == 7, "pair key")


func _test_map() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.get("load_ok") == true, "campaign map loads"):
		map.queue_free()
		return
	var layer: SettlementLayer = map.get("settlement_layer")
	var count := layer.data.settlements.size()
	var pairs: PackedInt64Array = layer.get("_model_pairs")
	print("dc6c: %d settlements, %d candidate pairs" % [count, pairs.size()])
	_check(pairs.size() > 0, "candidate pairs")
	var previous_hidden := 1 << 30
	var previous_shrunk := 1 << 30
	var summary := {}
	for d: float in [1000.0, 250.0, 28.0, 20.0, 14.0, 10.0, 7.0]:
		var t0 := Time.get_ticks_usec()
		_rescale(layer, d)
		var cost_ms := (Time.get_ticks_usec() - t0) / 1000.0
		var hidden := 0
		var shrunk := 0
		var ones := true
		for i in count:
			if layer.model_absorbed(i):
				hidden += 1
			elif layer.shown_fit(i) < 0.999:
				shrunk += 1
			if layer.model_holder(i) != null and not is_equal_approx(layer.model_scale(i), 1.0):
				ones = false
		var overlaps := _overlaps(layer, pairs)
		summary[d] = {"hidden": hidden, "shrunk": shrunk, "overlaps": overlaps, "rescale_ms": snappedf(cost_ms, 0.01)}
		_check(overlaps == 0, "no overlap > 20 %% at d=%.0f (%d)" % [d, overlaps])
		_check(hidden <= previous_hidden and shrunk <= previous_shrunk, "fewer hidden/shrunk closer (d=%.0f)" % d)
		if d >= 28.0:
			_check(ones, "map-scale models at d=%.0f" % d)
		previous_hidden = hidden
		previous_shrunk = shrunk
	print("dc6c: per distance %s" % summary)
	_check(int(summary[7.0]["hidden"]) < int(summary[1000.0]["hidden"]), "fewer absorbed at the valley tier")
	# Lille : voisines de la zone dense.
	var around := 0
	for i in count:
		if layer.model_px(i).distance_to(LILLE) < 40.0:
			around += 1
	print("dc6c: %d settlements within 40 u of Lille" % around)
	map.queue_free()
	await process_frame


## Échelle des maquettes à la distance `d` (réécriture forcée), sans attendre la caméra.
func _rescale(layer: SettlementLayer, d: float) -> void:
	layer.set("_camera_distance", d)
	layer.set("_settlement_scale_ref", -1.0)
	layer.call("_update_settlement_scale", d)


## Paires candidates affichées (non masquées) qui se recouvrent de plus de 20 % du plus petit rayon.
func _overlaps(layer: SettlementLayer, pairs: PackedInt64Array) -> int:
	var result := 0
	for key in pairs:
		var b := key >> 16
		var a := key & 0xFFFF
		if layer.model_absorbed(a) or layer.model_absorbed(b):
			continue
		var ra := layer.shown_radius(a)
		var rb := layer.shown_radius(b)
		if ra <= 0.0 or rb <= 0.0:
			continue
		var overlap := ra + rb - layer.model_px(a).distance_to(layer.model_px(b))
		if overlap > 0.2 * minf(ra, rb):
			result += 1
	return result
