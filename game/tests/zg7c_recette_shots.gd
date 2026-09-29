extends SceneTree

## Captures de recette du lot ZG7c (fin du chantier ZG, ADR 0036) : douze lieux aux trois paliers
## (stratégique, vallée, site = distance minimale de la caméra au point), relief ZG8 actif.
## Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/zg7c_recette_shots.gd -- --out=<dossier>
## Options : `--only=<nom>[,<nom>]`, `--tiers=strat,vallee,site`, `--prefix=<préfixe>`,
## `--keep-fog` (garde le brouillard de guerre, coupé par défaut), `--extras` (vue parchemin et
## filtres MF1 au-dessus de Paris, interface visible ; `--only=none` pour ne faire qu'elles), et celles de la carte
## (`--map-weather=clear`, `--no-relief-exaggeration`…).
## JPEG ≤ 960 px de large, `<lieu>_<palier>.jpg`. Imprime une ligne par capture (distance,
## échelle verticale, étage le plus fin chargé sous le point).

## [nom, point carte (px 4096)]
const PLACES := [
	["paris", Vector2(2213.2, 3203.9)],
	["londres", Vector2(2018.0, 2767.5)],
	["orleans", Vector2(2152.0, 3346.5)],
	["rouen_seine", Vector2(2097.0, 3099.4)],
	["pyrenees", Vector2(1862.0, 4098.0)],
	["alpes", Vector2(2652.0, 3684.0)],
	["massif_central", Vector2(2233.0, 3685.0)],
	["galles", Vector2(1694.0, 2468.0)],
	["falaises_normandes", Vector2(2018.0, 3052.0)],
	["val_de_loire", Vector2(2052.0, 3407.3)],
	["amiens", Vector2(2224.0, 3043.6)],
	["crecy", Vector2(2191.5, 2986.0)],
]
## Distances des paliers (unités) ; « site » = distance minimale au point (+ 5 %).
const TIERS := {"strat": 60.0, "vallee": 6.0, "site": -1.0}


func _init() -> void:
	var out_dir := "user://zg7c"
	var only := ""
	var keep_fog := false
	var extras := false
	var prefix := ""
	var tiers: PackedStringArray = PackedStringArray(TIERS.keys())
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
		elif arg == "--extras":
			extras = true
		elif arg == "--keep-fog":
			keep_fog = true
		elif arg.begins_with("--prefix="):
			prefix = arg.substr(9)
		elif arg.begins_with("--tiers="):
			tiers = arg.substr(8).split(",")
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("zg7c shots: campaign map failed to load")
		quit(1)
		return
	var vegetation: Node = map.get_node_or_null("Vegetation")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	for place in PLACES:
		if only != "" and not (place[0] as String) in only.split(","):
			continue
		var point: Vector2 = place[1]
		for tier: String in tiers:
			var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
			var distance: float = TIERS[tier]
			if distance < 0.0:
				distance = rig.min_distance_at(focus) * 1.05
			rig.look_at_point(focus, distance)
			rig.snap()
			for i in 30:
				await process_frame
			var guard := 0
			while guard < 1500:
				guard += 1
				var busy := false
				if vegetation != null and vegetation.has_method("pending_jobs") and vegetation.pending_jobs() > 0:
					busy = true
				if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
					busy = true
				if ReliefLandcover.pending():
					busy = true
				if not busy:
					break
				await process_frame
			for i in 60:
				await process_frame
			for layer in root.find_children("*", "CanvasLayer", true, false):
				(layer as CanvasLayer).visible = false
			for i in 3:
				await process_frame
			if terrain != null and terrain.material != null and not keep_fog:
				# Brouillard de guerre coupé : le relief de tout le continent, pas la vue d'un camp.
				terrain.material.set_shader_parameter("fog_enabled", false)
				for i in 2:
					await process_frame
			var image := root.get_viewport().get_texture().get_image()
			if image.get_width() > 960:
				image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
			var path := out_dir.path_join("%s%s_%s.jpg" % [prefix, place[0], tier])
			image.save_jpg(path, 0.82)
			var level := terrain.quadtree.chunk_top(terrain.chunk_index_at(point.x, point.y)) if terrain != null and terrain.quadtree != null else -1
			print("ZG7c shot %s d=%.2f scale ×%.2f gain %.2f level E%d waited %d%s" % [
				path, distance, MapData.vertical_exaggeration(), MapData.relief_gain(), level, guard,
				" (NOT SETTLED)" if guard >= 1500 else ""])
	if extras:
		# Vue parchemin (distance maximale), puis filtres MF1 aux paliers stratégique et vallée.
		var paris := Vector2(2213.2, 1923.9)
		var focus := Vector3(paris.x, data.surface_world_at(paris.x, paris.y), paris.y)
		var shots := [["parchemin", "", rig.max_distance], ["filtre_richesse_strat", "wealth", 60.0],
			["filtre_richesse_vallee", "wealth", 6.0], ["filtre_ravitaillement_vallee", "supply", 6.0],
			["filtre_politique_site", "political", 2.73]]
		for shot: Array in shots:
			var modes: Node = map.get("map_modes")
			if modes != null and shot[1] != "":
				modes.call("set_mode", shot[1])
			rig.look_at_point(focus, shot[2])
			rig.snap()
			for i in 120:
				await process_frame
			if terrain != null:
				terrain.wait_fine_jobs()
			for i in 30:
				await process_frame
			var image := root.get_viewport().get_texture().get_image()
			if image.get_width() > 960:
				image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
			var path := out_dir.path_join("%s%s.jpg" % [prefix, shot[0]])
			image.save_jpg(path, 0.82)
			print("ZG7c shot %s d=%.2f" % [path, shot[2]])
	map.queue_free()
	await process_frame
	quit(0)
