extends SceneTree

## Test headless du chantier NA (ADR 0217), bulle différée du décor naturel :
##  1. `CodexStore.entry_for_decor` (clés tree, battle_tree, fauna ; inconnue → vide) ;
##  2. minuterie de `DecorHover` : rien avant le délai, une bulle après, fermée quand la souris
##     s'éloigne, aucune répétition tant que la souris reste immobile, rien si désactivée, si un
##     objet prioritaire est dessous ou si le fournisseur ne trouve rien ;
##  3. carte de campagne : `pick_decor` retrouve l'essence d'un arbre construit (projeté à l'écran),
##     et le blocage d'une colonie sous le curseur ;
##  4. bataille : `BattleTerrain.decor_candidates` rend les arbres plantés autour d'un point.
## Usage : godot --headless --path game --script res://tests/decor_hover_test.gd

var _failures := 0
var _provider_calls := 0


func _init() -> void:
	await process_frame
	_test_mapping()
	await _test_timer()
	await _test_campaign()
	await _test_battle_index()
	if _failures > 0:
		push_error("decor_hover_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("decor_hover_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_mapping() -> void:
	var codex := CodexText.store()
	if not _check(codex != null, "CodexStore autoload"):
		return
	_check(str(codex.call("entry_for_decor", "tree", "willow")) == "cdx_saule", "tree:willow -> cdx_saule")
	_check(str(codex.call("entry_for_decor", "battle_tree", "willow")) == "cdx_saule", "battle_tree:willow -> cdx_saule")
	_check(str(codex.call("entry_for_decor", "tree", "juniper")) == "cdx_genievre", "tree:juniper -> cdx_genievre")
	_check(str(codex.call("entry_for_decor", "fauna", "animal_wolf_grey")) == "cdx_loups_et_louveterie", "fauna:animal_wolf_grey")
	_check(str(codex.call("entry_for_decor", "tree", "no_such_tree")) == "", "unknown species -> empty")


func _hover(delay: float, hit: Dictionary, blocked: bool = false) -> DecorHover:
	var hover := DecorHover.new()
	hover.delay_override = delay
	hover.ignore_gui = true
	_provider_calls = 0
	hover.provider = func(_at: Vector2) -> Dictionary:
		_provider_calls += 1
		return hit
	if blocked:
		hover.blocker = func(_at: Vector2) -> bool: return true
	root.add_child(hover)
	return hover


func _test_timer() -> void:
	var bubbles := root.get_node_or_null("CodexBubbles")
	if not _check(bubbles != null, "CodexBubbles autoload"):
		return
	bubbles.call("close_all")
	var willow := {"kind": "tree", "species": "willow"}
	var hover := _hover(1.0, willow)
	hover.note_mouse(Vector2(300, 300))
	hover.tick(0.9)
	_check(int(bubbles.call("bubble_count")) == 0, "no bubble before the delay")
	hover.tick(0.2)
	_check(int(bubbles.call("bubble_count")) == 1 and hover.shown_id == "cdx_saule", "one bubble after the delay (cdx_saule)")
	_check(str(bubbles.call("top_id")) == "cdx_saule", "bubble is the codex entry")
	# Quelques px : la bulle reste, aucun nouvel essai.
	hover.note_mouse(Vector2(304, 302))
	hover.tick(5.0)
	_check(int(bubbles.call("bubble_count")) == 1 and _provider_calls == 1, "stays open and no repeat while still")
	# La bulle n'est pas refermée par la grâce propre de CodexBubbles.
	await create_timer(0.7).timeout
	_check(int(bubbles.call("bubble_count")) == 1, "bubble survives the codex grace delay")
	# Souris loin : fermeture, puis nouveau décompte.
	hover.note_mouse(Vector2(700, 500))
	_check(int(bubbles.call("bubble_count")) == 0, "closed when the mouse leaves")
	hover.tick(1.1)
	_check(int(bubbles.call("bubble_count")) == 1 and _provider_calls == 2, "re-armed after a move")
	hover.note_mouse(Vector2(100, 100))
	bubbles.call("close_all")
	hover.queue_free()
	# Désactivée.
	var off := _hover(0.0, willow)
	off.note_mouse(Vector2(50, 50))
	off.tick(10.0)
	_check(int(bubbles.call("bubble_count")) == 0 and _provider_calls == 0, "disabled: no lookup, no bubble")
	off.queue_free()
	# Objet prioritaire dessous.
	var blocked := _hover(1.0, willow, true)
	blocked.note_mouse(Vector2(50, 50))
	blocked.tick(2.0)
	_check(int(bubbles.call("bubble_count")) == 0 and _provider_calls == 0, "blocker: army/settlement first")
	blocked.queue_free()
	# Rien sous le curseur, ou espèce sans fiche : une seule consultation, pas de bulle.
	for hit in [{}, {"kind": "tree", "species": "no_such_tree"}]:
		var empty := _hover(1.0, hit)
		empty.note_mouse(Vector2(50, 50))
		empty.tick(2.0)
		empty.tick(2.0)
		_check(int(bubbles.call("bubble_count")) == 0 and _provider_calls == 1, "no entry: one lookup, no bubble (%s)" % str(hit))
		empty.queue_free()
	# Réglage du joueur : -1 suit les données (1,5 s).
	var settings := _hover(-1.0, willow)
	_check(is_equal_approx(settings.delay(), float(DecorHover.style_value("delay_s"))) or root.get_node("Settings").call("get_value", DecorHover.SETTING_KEY) >= 0.0, "default delay from data")
	settings.queue_free()


func _test_campaign() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.get("load_ok") == true, "campaign map loads"):
		map.queue_free()
		return
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
	if not _check(vegetation != null, "Vegetation node"):
		map.queue_free()
		return
	var data: MapData = map.map_data
	var camera: Camera3D = map.camera
	var rig: CampaignCamera = map.camera_rig
	# Une vue boisée : plusieurs régions, jusqu'à ce que des arbres soient construits.
	var spots := [Vector2(2.65, 48.4), Vector2(2.0, 49.0), Vector2(7.0, 48.0), Vector2(-1.0, 47.5), Vector2(4.0, 45.5)]
	var chosen := {}
	for spot: Vector2 in spots:
		var px := FaunaLayer.lonlat_to_px(spot.x, spot.y, data)
		rig.look_at_point(Vector3(px.x, 0.0, px.y), 6.0)
		rig.snap()
		for i in 600:
			await process_frame
			if i > 5 and vegetation.pending_jobs() == 0 and vegetation.tile_count() > 0:
				break
		var hit: Dictionary = map.picker.pick_ray_screen(map.get_viewport().get_visible_rect().size * 0.5)
		if hit.is_empty():
			continue
		var candidates: Array = vegetation.decor_candidates(Vector2(float(hit["x"]), float(hit["z"])), 3.0)
		if candidates.size() >= 3:
			chosen = {"hit": hit, "candidates": candidates}
			break
	if not _check(not chosen.is_empty(), "built trees found in a campaign view"):
		map.queue_free()
		return
	# Arbre le plus isolé à l'écran : sans voisin à moins de 12 px.
	var candidates: Array = chosen["candidates"]
	var best_index := -1
	var best_gap := 0.0
	var screens: Array = []
	for candidate: Dictionary in candidates:
		var centre := (candidate["position"] as Vector3) + Vector3.UP * float(candidate["height"]) * 0.5
		screens.append(camera.unproject_position(centre))
	for i in candidates.size():
		var gap := INF
		for j in candidates.size():
			if i != j:
				gap = minf(gap, (screens[i] as Vector2).distance_to(screens[j]))
		if gap > best_gap and map.get_viewport().get_visible_rect().has_point(screens[i]):
			best_gap = gap
			best_index = i
	if _check(best_index >= 0, "an on-screen tree"):
		var target: Vector2 = screens[best_index]
		var expected := str(candidates[best_index]["species"])
		var got: Dictionary = map.pick_decor(target)
		print("decor_hover: %d candidates, isolated gap %.1f px, expected %s, got %s" % [candidates.size(), best_gap, expected, str(got)])
		_check(got.get("kind") == "tree" and got.get("species") == expected or best_gap < 12.0, "pick_decor finds the tree species")
		_check(map.pick_decor(Vector2(-500.0, -500.0)).is_empty(), "nothing off screen")
	# Priorité : une colonie sous le curseur bloque la bulle.
	var settlement_id := str(map.settlement_layer.data.settlements[0]["id"])
	var world: Vector3 = map.settlement_layer.world_position_of(settlement_id)
	rig.look_at_point(world, 6.0)
	rig.snap()
	for i in 20:
		await process_frame
	var at := camera.unproject_position(world)
	var target_info: Dictionary = map.pick_target(at)
	if _check(not target_info.is_empty(), "settlement is picked at its position"):
		_check(bool(map.decor_hover.blocker.call(at)), "blocker true over a settlement")
	_check(not bool(map.decor_hover.blocker.call(Vector2(-500.0, -500.0))), "blocker false on empty ground")
	await _test_fauna_and_rocks(map, data, rig, camera)
	map.queue_free()
	await process_frame


func _test_fauna_and_rocks(map: Node3D, data: MapData, rig: CampaignCamera, camera: Camera3D) -> void:
	var fauna: FaunaLayer = map.life.fauna
	var camargue := FaunaLayer.lonlat_to_px(4.55, 43.52, data)
	rig.look_at_point(Vector3(camargue.x, data.surface_world_at(camargue.x, camargue.y), camargue.y), 8.0)
	rig.snap()
	for i in 60:
		await process_frame
	var herds: Array = []
	for cy in range(floori((camargue.y - 40.0) / 96.0), floori((camargue.y + 40.0) / 96.0) + 1):
		for cx in range(floori((camargue.x - 40.0) / 96.0), floori((camargue.x + 40.0) / 96.0) + 1):
			herds.append_array(fauna.cell_herds(Vector2i(cx, cy)))
	if not herds.is_empty():
		var herd: Dictionary = herds[0]
		var found := fauna.decor_candidates(herd["center"], 0.5)
		var species_found := false
		for candidate: Dictionary in found:
			species_found = species_found or candidate["species"] == herd["species"]
		print("decor_hover: fauna visible cells %d, candidates near a herd: %d" % [int(fauna.stats["visible_cells"]), found.size()])
		if int(fauna.stats["visible_cells"]) > 0:
			_check(species_found, "fauna candidates include the herd species")
	var outcrops := map.get_node_or_null("RockOutcrops")
	if outcrops != null and outcrops.tile_count() > 0:
		var any_rock := false
		for tile_key: Vector2i in outcrops._tiles:
			for part: Dictionary in outcrops._tiles[tile_key]["parts"]:
				var points: PackedVector2Array = part["points"]
				if points.size() > 0 and (part["mmi"] as MultiMeshInstance3D).is_visible_in_tree():
					var found: Array = outcrops.decor_candidates(points[0])
					any_rock = any_rock or found.size() > 0
		print("decor_hover: rock tiles %d, candidate found: %s" % [outcrops.tile_count(), any_rock])


func _test_battle_index() -> void:
	var world := Node3D.new()
	root.add_child(world)
	var terrain := BattleTerrain.new()
	world.add_child(terrain)
	var nx := 121
	var nz := 81
	var heights := PackedFloat32Array()
	heights.resize(nx * nz)
	terrain.build({"nx": nx, "nz": nz, "resolution": 10.0, "heights": heights, "terrain": "forest", "season": "summer", "ground": "dry", "woodland": 0.8}, "clear")
	var all := terrain.decor_candidates(Vector2(600.0, 400.0), 2000.0)
	print("decor_hover: battle trees indexed: %d" % all.size())
	if not _check(all.size() > 50, "battle trees indexed (%d)" % all.size()):
		world.queue_free()
		return
	var sample: Dictionary = all[all.size() / 2]
	var at: Vector3 = sample["position"]
	var near := terrain.decor_candidates(Vector2(at.x, at.z), 5.0)
	var found := false
	for tree: Dictionary in near:
		found = found or (tree["position"] as Vector3).is_equal_approx(at)
		_check(BattleTrees.SPECIES.has(tree["species"]) and absf((tree["position"] as Vector3).x - at.x) <= 5.0, "indexed tree is a known species within reach")
	_check(found, "the sampled tree is found near its own position")
	var codex := CodexText.store()
	_check(codex != null and codex.call("entry_for_decor", "battle_tree", "willow") == "cdx_saule", "battle_tree key resolves")
	world.queue_free()
	await process_frame
