extends SceneTree

## Test headless du lot EN (ADR 0155, ennemis lisibles hors mode Diplomatie) sur la vraie
## simulation, France jouée :
##  1. frontières : un ennemi a le rouge d'alerte, nous l'or ; une faction en paix garde une
##     couleur héraldique assourdie (jamais plus saturée que le plafond) ;
##  2. armées : plaque ennemie bordée de rouge avec sa marque, plaque du joueur dorée ;
##  3. villes : nom d'une ville ennemie à l'encre rouge, nos villes à l'encre ordinaire ;
##  4. le mode Diplomatie garde sa palette, puis la rend en sortie.
## Usage : godot --headless --path game --script res://tests/en_stance_cues_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PLAYER := "fac_france"

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("en_stance_cues_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("en_stance_cues_test: " + message)
	return condition


func _palette_color(borders: FactionBorders, faction: String) -> Color:
	var texture: Texture2D = borders.get_param("fr1_palette")
	if texture == null:
		return Color(0, 0, 0, 0)
	return texture.get_image().get_pixel(borders.faction_index(faction), 0)


func _same(a: Color, b: Color) -> bool:
	return absf(a.r - b.r) < 0.02 and absf(a.g - b.g) < 0.02 and absf(a.b - b.b) < 0.02


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = PLAYER
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var borders: FactionBorders = map.faction_borders
	var stances := StanceCues.stances(sim, PLAYER)
	if not _check(not stances.is_empty(), "get_faction_stances_for missing (rebuild core)"):
		map.queue_free()
		return
	var tuning := StanceCues.tuning()
	var enemy_red := Color.html(str(tuning["border"]["enemy"]))

	# 1. Frontières.
	var enemies := {}
	var peaceful := 0
	var saturation_max := float(tuning["border"]["other"]["saturation_max"])
	for faction: String in stances:
		if borders.faction_index(faction) <= 0:
			continue
		var cue := StanceCues.category_of(faction, PLAYER, stances)
		var shown := _palette_color(borders, faction)
		if cue == StanceCues.ENEMY:
			enemies[faction] = true
			_check(_same(shown, enemy_red), "%s is at war: its border should be the enemy red" % faction)
		elif cue == StanceCues.OTHER:
			peaceful += 1
			_check(shown.s <= saturation_max + 0.03, "%s is at peace: border saturation %.2f above the cap" % [faction, shown.s])
			_check(not _same(shown, enemy_red), "%s is at peace: its border must not be the enemy red" % faction)
	_check(not enemies.is_empty(), "France should have at least one enemy on the map in 1337")
	_check(peaceful > 0, "France should have peaceful neighbours")
	_check(_same(_palette_color(borders, PLAYER), Color.html(str(tuning["border"]["self"]))), "player borders should use the 'self' colour")

	# 2. Armées.
	var armies: ArmyMarkers = map.armies
	var enemy_plates := 0
	var own_plates := 0
	for army_id in sim.call("get_army_ids"):
		var plate: PanelContainer = armies._plates.get(army_id)
		var marker: ArmyMarker = armies._markers.get(army_id)
		if plate == null or marker == null:
			continue
		var style := plate.get_theme_stylebox("panel") as StyleBoxFlat
		var glyph := plate.find_child("EnemyGlyph", true, false)
		if enemies.has(marker.faction_id):
			enemy_plates += 1
			_check(marker.cue == StanceCues.ENEMY, "enemy army %s should carry the enemy cue" % army_id)
			_check(_same(style.border_color, Color.html(str(tuning["army"]["plate"]["enemy"]["color"]))), "enemy plate %s should have a red border" % army_id)
			_check(glyph != null, "enemy plate %s should carry the enemy glyph" % army_id)
		else:
			_check(glyph == null, "plate %s (%s) is not an enemy: no glyph" % [army_id, marker.faction_id])
			if marker.faction_id == PLAYER:
				own_plates += 1
				_check(marker.cue == StanceCues.SELF, "player army %s should carry the self cue" % army_id)
	_check(own_plates > 0, "player armies should have plates")
	# Les armées ennemies peuvent être sous le brouillard : plaque ennemie construite à la main.
	for army_id in armies._markers:
		var marker: ArmyMarker = armies._markers[army_id]
		if marker.faction_id != PLAYER:
			continue
		marker.set_cue(StanceCues.ENEMY)
		var plate := ArmyMarkers.build_plate(marker)
		var style := plate.get_theme_stylebox("panel") as StyleBoxFlat
		_check(_same(style.border_color, Color.html(str(tuning["army"]["plate"]["enemy"]["color"]))), "an enemy plate should have a red border")
		_check(plate.find_child("EnemyGlyph", true, false) != null, "an enemy plate should carry the enemy glyph")
		_check(_same(marker.selection.modulate, Color.html(str(tuning["army"]["ring"]["enemy"]["color"]))), "an enemy army should stand on a red ring")
		plate.free()
		marker.set_cue(StanceCues.SELF)
		break
	print("en_stance_cues_test: enemies %s, %d enemy plate(s), %d peaceful faction(s)" % [enemies.keys(), enemy_plates, peaceful])

	# 3. Villes.
	var layer: SettlementLayer = map.settlement_layer
	var enemy_ink := Color.html(str(tuning["town"]["label"]["enemy"]))
	var enemy_towns := 0
	for i in layer.data.settlements.size():
		var controller := str(layer.data.settlements[i]["controller"])
		if enemies.has(controller):
			enemy_towns += 1
			_check(_same(layer.label_ink(i), enemy_ink), "town %s is held by an enemy: red ink expected" % layer.data.settlements[i]["id"])
		else:
			_check(_same(layer.label_ink(i), layer.label_color), "town %s is not held by an enemy: ordinary ink expected" % layer.data.settlements[i]["id"])
	_check(enemy_towns > 0, "at least one town should be held by an enemy")

	# 4. Le mode Diplomatie garde sa palette et rend celle des relations en sortie.
	var modes: Object = map.map_modes
	var any_enemy: String = enemies.keys()[0] if not enemies.is_empty() else ""
	modes.set_mode("diplomacy")
	_check(_same(_palette_color(borders, PLAYER), DiplomaticStances.COLORS["self"]), "diplomacy mode should keep its own palette")
	modes.set_mode("political")
	if any_enemy != "":
		_check(_same(_palette_color(borders, any_enemy), enemy_red), "leaving diplomacy mode should restore the enemy red")
	map.queue_free()
	await process_frame
