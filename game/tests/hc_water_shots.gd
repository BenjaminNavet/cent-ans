extends SceneTree

## Planche de contrôle du lot HC2 (ADR 0161 §3, eaux lisibles à hauteur de jeu) : UNE planche JPEG
## 2×2 (4 vues de 640×400, 1280×800 au plus) — par défaut Léman à rig 300, Dombes à 150, Sologne à
## 150, Fens à 300 (centres : `lakes.json` pour le Léman, `wetlands.json` pour les autres).
## Fenêtre réelle (pas headless), armées masquées, météo claire :
##   godot --path game --resolution 640x400 --script res://tests/hc_water_shots.gd -- --out=<dossier>
##   --hide-armies --map-weather=clear [--season=winter] [--name=hc_water] [--views=x,z,d;x,z,d;…]
##   [--param=<uniforme>=<float | r:g:b>]… (matériaux du terrain) [--no-board] (chiffres seuls)
## `--views` : au plus 4 vues (px carte et distance de rig). Une ligne `HC water` par vue : pixels
## écran par px carte, part d'eau intérieure, couleurs moyennes eau / reste (sRGB), nappes d'au
## moins 12 px (mesure par passage magenta, `hc_water_view.gd`, sur l'image réduite à 640×400).

const VIEW := preload("res://tests/hc_water_view.gd")
const CELL := Vector2i(640, 400)


func _init() -> void:
	var out_dir := "user://hc_water"
	var board_name := "hc_water"
	var views: Array = []
	var params := {}
	var board_wanted := true
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--name="):
			board_name = arg.trim_prefix("--name=")
		elif arg.begins_with("--views="):
			for spec in arg.trim_prefix("--views=").split(";", false):
				var xzd := spec.split(",")
				if xzd.size() == 3:
					views.append(["%d_%d_d%d" % [int(xzd[0]), int(xzd[1]), int(xzd[2])], Vector2(float(xzd[0]), float(xzd[1])), float(xzd[2])])
		elif arg.begins_with("--param="):
			var kv := arg.trim_prefix("--param=").split("=")
			var parts := kv[1].split(":")
			params[kv[0]] = Vector3(float(parts[0]), float(parts[1]), float(parts[2])) if parts.size() == 3 else float(kv[1])
		elif arg == "--no-board":
			board_wanted = false
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = await VIEW.open_map(self)
	if map == null:
		push_error("hc_water_shots: campaign map failed to load")
		quit(1)
		return
	if views.is_empty():
		var lakes: LakesRenderer = map.get("lakes")
		var geneva: Dictionary = lakes.lake_named("Léman") if lakes != null else {}
		var geneva_at: Vector2 = geneva.get("center", Vector2(2621.2, 3611.1))
		views = [
			["leman_d300", geneva_at, 300.0],
			["dombes_d150", VIEW.DOMBES, 150.0],
			["sologne_d150", VIEW.SOLOGNE, 150.0],
			["fens_d300", VIEW.FENS, 300.0],
		]
	views = views.slice(0, 4)
	var board := Image.create(CELL.x * 2, CELL.y * 2, false, Image.FORMAT_RGB8)
	for index in views.size():
		var view: Array = views[index]
		await VIEW.frame(self, map, view[1], view[2])
		for material in VIEW.terrain_materials(map):
			for key: String in params:
				material.set_shader_parameter(key, params[key])
		for i in 6:
			await process_frame
		var image: Image = await VIEW.grab(self)
		var mask: Image = await VIEW.water_mask(self, map)
		var window_width := float(image.get_width())
		image.resize(CELL.x, CELL.y, Image.INTERPOLATE_LANCZOS)
		mask.resize(CELL.x, CELL.y, Image.INTERPOLATE_NEAREST)
		image.convert(Image.FORMAT_RGB8)
		var stats: Dictionary = VIEW.water_stats(image, mask)
		var scale: Vector2 = VIEW.screen_px_per_map_px(self, map, view[1]) * float(CELL.x) / window_width
		print("HC water %s px/map_px %.2f x %.2f share %.4f water %s land %s blobs %d biggest %d" % [
			view[0], scale.x, scale.y, stats["share"], stats["water_rgb"], stats["land_rgb"], stats["blobs"], stats["biggest_px"]])
		board.blit_rect(image, Rect2i(Vector2i.ZERO, CELL), Vector2i((index % 2) * CELL.x, (index / 2) * CELL.y))
	if board_wanted:
		var path := out_dir.path_join("%s.jpg" % board_name)
		board.save_jpg(path, 0.88)
		print("HC water board %s" % path)
	map.queue_free()
	await process_frame
	quit(0)
