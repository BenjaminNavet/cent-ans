extends SceneTree

## Captures de contrôle du lot GC2 (ADR 0158, maquettes stylisées des lieux à taille monde
## constante) : Paris (ville emblématique grossie) à d = 300, 150, 60 et 22, Amiens (ville
## ordinaire) à d = 150, la Flandre (Bruges, lieux serrés : réductions de collision) à d = 200,
## puis une ville par famille d'architecture : Constantinople (byz), Novgorod (rus), Tunis (isl),
## Florence (med) à d = 150 et 40, Saraï (steppe) à d = 150.
## Fenêtre réelle (pas headless) :
##   godot --path game --resolution 640x400 --script res://tests/gc_shots.gd -- --out=<dossier> --hide-armies
##   --map-weather=clear [--only=a,b] [--full=<dossier>] [--town-style=real]
## `--full` : copie à pleine résolution en plus (recadrages).
## JPEG `gc_<vue>.jpg` ≤ 640 px ; une ligne `GC shot` par capture (statistiques des maquettes).

## Positions de `data/map/settlements_px.json`.
const PARIS := Vector2(2213.2, 3203.9)
const AMIENS := Vector2(2224.0, 3043.6)
const BRUGES := Vector2(2335.0, 2850.0)
const CONSTANTINOPLE := Vector2(5196.4, 4176.6)
const NOVGOROD := Vector2(4689.7, 1492.7)
const TUNIS := Vector2(3013.2, 5099.8)
const FLORENCE := Vector2(3133.4, 4028.8)
const SARAI := Vector2(6677.8, 2307.2)
## [nom, point, distance du rig (unités), cap (degrés)]
const SHOTS := [
	["paris_d300", PARIS, 300.0, 0.0],
	["paris_d150", PARIS, 150.0, 0.0],
	["paris_d60", PARIS, 60.0, 0.0],
	["paris_d22", PARIS, 22.0, 0.0],
	["amiens_d150", AMIENS, 150.0, 0.0],
	["flandre_d200", BRUGES, 200.0, 0.0],
	["constantinople_d150", CONSTANTINOPLE, 150.0, 0.0],
	["constantinople_d40", CONSTANTINOPLE, 40.0, 0.0],
	["novgorod_d150", NOVGOROD, 150.0, 0.0],
	["novgorod_d40", NOVGOROD, 40.0, 0.0],
	["tunis_d150", TUNIS, 150.0, 0.0],
	["tunis_d40", TUNIS, 40.0, 0.0],
	["florence_d150", FLORENCE, 150.0, 0.0],
	["florence_d40", FLORENCE, 40.0, 0.0],
	["sarai_d150", SARAI, 150.0, 0.0],
]


func _init() -> void:
	var out_dir := "user://gc_shots"
	var only := ""
	var full_dir := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
		elif arg.begins_with("--full="):
			full_dir = arg.substr(7)
	DirAccess.make_dir_recursive_absolute(out_dir)
	if full_dir != "":
		DirAccess.make_dir_recursive_absolute(full_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("gc shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var maquettes: TownMaquetteLayer = settlements.maquettes if settlements != null else null
	print("GC style %s %s" % [TownMaquetteData.style(), JSON.stringify(maquettes.stats) if maquettes != null else "-"])
	for shot: Array in SHOTS:
		if only != "" and not (shot[0] as String) in only.split(","):
			continue
		var point: Vector2 = shot[1]
		var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
		rig.look_at_point(focus, maxf(float(shot[2]), rig.min_distance_at(focus)))
		rig.target_yaw = deg_to_rad(float(shot[3]))
		rig.snap()
		await _settle(terrain, settlements)
		for i in 4:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		if full_dir != "":
			image.save_png(full_dir.path_join("gc_%s_full.png" % shot[0]))
		if image.get_width() > 640:
			image.resize(640, roundi(image.get_height() * 640.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("gc_%s.jpg" % shot[0])
		image.save_jpg(path, 0.85)
		var cam := root.get_viewport().get_camera_3d()
		print("GC shot %s d=%.1f cam_to_focus=%.2f %s" % [path, rig.target_distance, cam.global_position.distance_to(focus), _near(settlements, point)])
	map.queue_free()
	await process_frame
	quit(0)


## Lieu le plus proche du point visé : id, type, famille, demi-largeur et réduction de sa maquette.
func _near(settlements: SettlementLayer, point: Vector2) -> String:
	if settlements == null or settlements.data == null:
		return "-"
	var best := -1
	var best_d := INF
	for i in settlements.data.settlements.size():
		var d := point.distance_to(settlements.data.settlements[i]["px"])
		if d < best_d:
			best_d = d
			best = i
	if best < 0:
		return "-"
	var entry: Dictionary = settlements.data.settlements[best]
	var maquettes := settlements.maquettes
	if maquettes == null:
		return "%s %s radius=%.2f (1:1)" % [entry["id"], entry["kind"], settlements.model_radius(best)]
	return "%s %s family=%s radius=%.2f factor=%.2f landmark=%s" % [
		entry["id"], entry["kind"], maquettes.family_of(best), maquettes.radius_of(best), maquettes.factor_of(best), maquettes.is_landmark(best)]


func _settle(terrain: TerrainBuilder, settlements: SettlementLayer) -> void:
	for i in 30:
		await process_frame
	var guard := 0
	while guard < 1500:
		guard += 1
		var busy := false
		if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
			busy = true
		if ReliefLandcover.pending():
			busy = true
		if not busy:
			break
		await process_frame
	if settlements != null:
		settlements.flush()
	for i in 40:
		await process_frame
