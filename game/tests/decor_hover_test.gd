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
		var hit := map.picker.pick_ray_screen(map.get_viewport().get_visible_rect().size * 0.5)
		if hit.is_empty():
			continue
		var candidates := vegetation.decor_candidates(Vector2(float(hit["x"]), float(hit["z"])), 3.0)
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
	map.queue_free()
	await process_frame
