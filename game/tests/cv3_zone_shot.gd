extends SceneTree

## Captures du lot CV3-5 : zone atteignable à deux tons (ce tour / tour suivant) de la première
## armée du joueur en vue régionale, puis gros plan sur le général « lord » qui porte l'étendard.
## Usage (avec affichage ; sans écran, le rendu est vide) :
##   godot --path game --script res://tests/cv3_zone_shot.gd
## Écrit `docs/img/cv3/zone-regionale.png` et `docs/img/cv3/zone-lord.png` (640 px de large).

const WIDTH := 640


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img/cv3").simplify_path()
	DirAccess.make_dir_recursive_absolute(out_dir)
	root.size = Vector2i(1280, 800)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 10:
		await process_frame
	if map.movement_ctl == null or not map.movement_ctl.available():
		push_error("cv3_zone_shot: free movement API missing (run core/build.sh)")
		quit(1)
		return
	map.camera_rig.edge_pan_enabled = false
	map.movement_ctl.stage_screenshot(false)
	var bubble: ArmyMovementBubble = map.movement_ctl.bubble
	print("cv3_zone_shot: %d cells this turn, %d next turn" % [bubble.cell_count(), bubble.next_cell_count()])
	await _settle()
	_save(out_dir.path_join("zone-regionale.png"))
	# Gros plan : le général agrandi, étendard en main (palier près).
	var army: Dictionary = map.sim.call("get_army", map.selected_army)
	var point: Vector2 = army.get("position", Vector2.ZERO)
	map.camera_rig.look_at_point(Vector3(point.x, map.map_data.surface_world_at(point.x, point.y), point.y), 55.0)
	map.camera_rig.snap()
	await _settle()
	_save(out_dir.path_join("zone-lord.png"))
	quit(0)


func _settle() -> void:
	for i in 40:
		await process_frame


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cv3_zone_shot: empty frame (no display?)")
		return
	image.resize(WIDTH, int(round(float(image.get_height()) * WIDTH / image.get_width())), Image.INTERPOLATE_LANCZOS)
	image.save_png(path)
	print("cv3_zone_shot: wrote %s" % path)
