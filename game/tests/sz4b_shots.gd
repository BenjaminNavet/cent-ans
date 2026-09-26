extends SceneTree

## Captures du lot SZ4b (maquettes de colonies continues, forêts denses au palier vallée), dérivé de
## `sz4_shots.gd`. Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/sz4b_shots.gd -- --out=<dossier> [--prefix=avant_]
## Options : `--only=<nom>[,<nom>]`, `--tiers=<nom>:<distance>[,…]` (défaut d6,d10,d14,d20,d60),
## `--keep-fog`, `--full` (pleine résolution),
## `--settle-towns` (villes ZG6 construites avant la capture), et celles de la carte (`--map-weather=clear`…). JPEG ≤ 960 px, `<lieu>_<palier>.jpg`.

## [nom, point carte (px 4096)]
const PLACES := [
	["crecy", Vector2(2191.5, 1706.0)],
	["val_de_loire", Vector2(2052.0, 2127.3)],
	["amiens", Vector2(2224.0, 1763.6)],
	["foret_orleans", Vector2(2180.0, 2053.0)],
	["foret_compiegne", Vector2(2285.0, 1844.0)],
]
const DEFAULT_TIERS := "d6:6,d10:10,d14:14,d20:20,d60:60"


func _init() -> void:
	var out_dir := "user://sz4b"
	var only := ""
	var keep_fog := false
	var prefix := ""
	var tier_spec := DEFAULT_TIERS
	var full := false
	var settle_towns := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
		elif arg == "--keep-fog":
			keep_fog = true
		elif arg.begins_with("--prefix="):
			prefix = arg.substr(9)
		elif arg == "--settle-towns":
			settle_towns = true
		elif arg == "--full":
			full = true
		elif arg.begins_with("--tiers="):
			tier_spec = arg.substr(8)
	var tiers: Array = []
	for part in tier_spec.split(",", false):
		var kv := part.split(":")
		tiers.append([kv[0], float(kv[1])])
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	# Intro du premier tour (interface, conseils) passée avant la première capture.
	for i in 180:
		await process_frame
	# Interface masquée à chaque image (les fenêtres du premier tour peuvent se rouvrir).
	process_frame.connect(func() -> void:
		for layer in root.find_children("*", "CanvasLayer", true, false):
			(layer as CanvasLayer).visible = false)
	if not map.get("load_ok"):
		push_error("sz4b shots: campaign map failed to load")
		quit(1)
		return
	var vegetation: Node = map.get_node_or_null("Vegetation")
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false  # souris au bord de la fenêtre : pas de défilement pendant l'attente
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	for place in PLACES:
		if only != "" and not (place[0] as String) in only.split(","):
			continue
		var point: Vector2 = place[1]
		for tier: Array in tiers:
			var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
			var distance: float = tier[1]
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
			var layer_s: SettlementLayer = map.get("settlement_layer")
			if settle_towns and layer_s != null and layer_s.towns != null:
				layer_s.towns.flush()
			# SZ4b : forêt dense semée avant la capture.
			var forest: Variant = vegetation.get("forest_detail") if vegetation != null else null
			if forest != null:
				(forest as Object).call("flush", point, distance)
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
			# L'interface peut se réafficher (premier tour, conseils) : masquée de nouveau.
			for i in 4:
				for layer in root.find_children("*", "CanvasLayer", true, false):
					(layer as CanvasLayer).visible = false
				await process_frame
			var image := root.get_viewport().get_texture().get_image()
			if image.get_width() > 960 and not full:
				image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
			var path := out_dir.path_join("%s%s_%s.jpg" % [prefix, place[0], tier[0]])
			image.save_jpg(path, 0.82)
			var level := terrain.quadtree.chunk_top(terrain.chunk_index_at(point.x, point.y)) if terrain != null and terrain.quadtree != null else -1
			var detail: Variant = vegetation.get("forest_detail") if vegetation != null else null
			print("  vegetation %d instances%s" % [vegetation.instance_count() if vegetation != null else 0,
				(" ; forest detail %s" % (detail as Object).get("stats")) if detail != null else ""])
			print("SZ4b shot %s d=%.2f scale ×%.2f gain %.2f level E%d waited %d%s" % [
				path, distance, MapData.vertical_exaggeration(), MapData.relief_gain(), level, guard,
				" (NOT SETTLED)" if guard >= 1500 else ""])
			if layer_s != null and layer_s.towns != null:
				print("  towns active=%s %s" % [layer_s.towns.active, layer_s.towns.stats])
	map.queue_free()
	await process_frame
	quit(0)
