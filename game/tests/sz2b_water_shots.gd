extends SceneTree

## Captures du lot SZ2b (nappe d'eau des fleuves fins) : Rouen (vues VH4), Orléans, Tours,
## Londres, Bordeaux aux paliers vallée et site. Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/sz2b_water_shots.gd -- --out=<dossier> --map-weather=clear
##   [--only=a,b] [--prefix=avant_]
## JPEG ≤ 960 px, `<prefixe><vue>.jpg`. Imprime la surface affichée et le niveau d'eau fin sous
## le point de visée.

## [nom, point carte, distance (unités, < 0 : minimale × |d|), cap (degrés)]
const SHOTS := [
	["val_de_loire_site", Vector2(2052.0, 2127.3), 1.0, 0.0],
	["rouen_site", Vector2(2096.64, 1819.78), 1.6, 0.0],
	["rouen_pont", Vector2(2096.18, 1820.33), 0.55, -30.0],
	["rouen_seine_sud", Vector2(2096.54, 1820.18), 1.2, 180.0],
	["rouen_vallee", Vector2(2096.54, 1819.88), 4.0, 0.0],
	["orleans_site", Vector2(2151.9, 2067.0), 1.0, 20.0],
	["orleans_vallee", Vector2(2151.9, 2067.0), 5.0, 20.0],
	["tours_site", Vector2(2017.0, 2128.2), 1.0, 20.0],
	["tours_vallee", Vector2(2017.0, 2128.2), 5.0, 20.0],
	["londres_site", Vector2(2021.9, 1487.2), 1.0, 0.0],
	["londres_vallee", Vector2(2021.9, 1487.2), 5.0, 0.0],
	["bordeaux_site", Vector2(1832.8, 2502.3), 1.0, 90.0],
	["bordeaux_vallee", Vector2(1832.8, 2502.3), 5.0, 90.0],
]


func _init() -> void:
	var out_dir := "user://sz2b"
	var only := ""
	var prefix := ""
	debug_heights = OS.get_cmdline_user_args().has("--probe-heights")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
		elif arg.begins_with("--prefix="):
			prefix = arg.substr(9)
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("sz2b shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	for shot: Array in SHOTS:
		if only != "" and not (shot[0] as String) in only.split(","):
			continue
		var point: Vector2 = shot[1]
		var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
		var distance: float = shot[2]
		rig.look_at_point(focus, maxf(distance, rig.min_distance_at(focus)))
		rig.target_yaw = deg_to_rad(float(shot[3]))
		rig.snap()
		await _settle(terrain, settlements)
		for layer in root.find_children("*", "CanvasLayer", true, false):
			(layer as CanvasLayer).visible = false
		if terrain != null and terrain.material != null:
			terrain.material.set_shader_parameter("fog_enabled", false)
		for i in 4:
			await process_frame
		var image := root.get_viewport().get_texture().get_image()
		if image.get_width() > 960:
			image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("%s%s.jpg" % [prefix, shot[0]])
		image.save_jpg(path, 0.85)
		var fine: Node = map.find_child("FineGeo", true, false)
		if debug_heights and fine != null:
			for c: Vector4 in (fine.get("rivers") as RiversRenderer).covers:
				if Vector2(c.x, c.y).distance_to(point) < 3.0:
					print("  cover %s" % c)
		var built: Dictionary = fine.get("_built") if fine != null else {}
		var shown := 0
		var points := 0
		var river_vertices := 0
		for entry: Dictionary in built.values():
			if (entry["node"] as Node3D).visible:
				shown += 1
				points += int(entry.get("points", 0))
				var rm: Mesh = (entry["river"] as MeshInstance3D).mesh
				river_vertices += rm.surface_get_array_len(0) if rm != null else 0
				if rm != null and debug_heights:
					_probe_heights(rm, terrain, point)
		print("SZ2b shot %s d=%.2f fine visible=%s zone=%s tiles %d/%d points %d river vertices %d" % [path, rig.target_distance,
			fine.visible if fine != null else false, fine.get("_zone") if fine != null else "-", shown, built.size(), points, river_vertices])
	map.queue_free()
	await process_frame
	quit(0)


var debug_heights := false


## Diagnostic (`--probe-heights`) : sous les sommets d'axe proches du point, surface du relief
## − niveau d'eau (m) à l'axe et juste hors de l'eau (1,15 demi-largeur) ; médianes et extrêmes.
## Axe < 0 : lit creusé sous l'eau ; berge ≥ 0 : l'eau touche une berge plus haute (pas de mur).
func _probe_heights(mesh: Mesh, terrain: TerrainBuilder, point: Vector2) -> void:
	var arrays := mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	var scale := MapData.vertical_scale()
	var axis: Array[float] = []
	var bank: Array[float] = []
	for k in range(0, verts.size(), 2):
		var v := verts[k]
		if Vector2(v.x, v.z).distance_to(point) > 1.2 or uvs[k].x * 719.0 < 60.0:
			continue
		var water := MapData.display_height(v.y, v.x, v.z)
		var ground := terrain.surface_height_at(v.x, v.z)
		if is_nan(ground):
			continue
		axis.append((ground - water) / scale)
		if axis.size() == 3:
			var prof := "  profile at %.2f,%.2f z=%.1f w=%.0f gain %.2f:" % [v.x, v.z, v.y, uvs[k].x * 719.0, MapData.relief_gain()]
			for off in [-2.0, -1.5, -1.15, -0.5, 0.0, 0.5, 1.15, 1.5, 2.0]:
				var q: Vector3 = v + normals[k] * off * uvs[k].x * 0.5
				prof += " %+.2f:%.1f" % [off, (terrain.surface_height_at(q.x, q.z) - MapData.display_height(v.y, q.x, q.z)) / scale]
			print(prof)
		for side in [1.0, -1.0]:
			var q: Vector3 = v + normals[k] * side * uvs[k].x * 0.5 * 1.15
			var g := terrain.surface_height_at(q.x, q.z)
			if not is_nan(g):
				bank.append((g - MapData.display_height(v.y, q.x, q.z)) / scale)
	if axis.is_empty():
		return
	axis.sort()
	bank.sort()
	print("  heights: %d axis vertices, relief - water at axis: median %.1f max %.1f m ; at banks: p10 %.1f median %.1f min %.1f m" % [
		axis.size(), axis[axis.size() / 2], axis[-1], bank[bank.size() / 10], bank[bank.size() / 2], bank[0]])


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
	for i in 90:
		await process_frame
