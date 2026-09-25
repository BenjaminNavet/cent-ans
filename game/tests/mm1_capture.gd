extends SceneTree

## Lot MM1 — banc et captures des écrans d'accueil, fenêtré (pas en headless) :
##   godot --path game --script res://tests/mm1_capture.gd -- --scene=menu --out=<png>
##   [--wait=<s>] [--fps-seconds=<s>] [--menu-shot=<n>] [--menu-shot-t=<0..1>]
## `--scene` : `backdrop` (décor 3D seul), `menu` (start_menu.tscn). Imprime le temps de
## chargement (jusqu'à la première image) et, avec `--fps-seconds`, les images/s moyennes et le
## pire intervalle.

var _out := ""
var _scene := "menu"
var _wait := 3.0
var _fps_seconds := 0.0


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--scene="):
			_scene = arg.trim_prefix("--scene=")
		elif arg.begins_with("--wait="):
			_wait = float(arg.trim_prefix("--wait="))
		elif arg.begins_with("--fps-seconds="):
			_fps_seconds = float(arg.trim_prefix("--fps-seconds="))
	_run.call_deferred()
	# Garde-fou : jamais plus de deux minutes (fenêtre masquée, erreur de script).
	create_timer(120.0).timeout.connect(func() -> void:
		print("mm1_capture: timeout")
		quit(1))


func _run() -> void:
	await process_frame
	var started := Time.get_ticks_msec()
	var node: Node
	if _scene == "backdrop":
		node = (load("res://scripts/ui/menu_backdrop_3d.gd") as GDScript).new()
	else:
		node = (load("res://scenes/start_menu.tscn") as PackedScene).instantiate()
	root.add_child(node)
	current_scene = node
	await process_frame
	await RenderingServer.frame_post_draw
	print("mm1_capture: first frame after %d ms" % (Time.get_ticks_msec() - started))
	if _fps_seconds > 0.0:
		var frames := 0
		var worst := 0.0
		var begin := Time.get_ticks_usec()
		var last := begin
		while (Time.get_ticks_usec() - begin) < int(_fps_seconds * 1.0e6):
			await process_frame
			var now := Time.get_ticks_usec()
			worst = maxf(worst, (now - last) / 1000.0)
			last = now
			frames += 1
		var seconds := (Time.get_ticks_usec() - begin) / 1.0e6
		print("mm1_capture: %.1f fps over %.1f s, worst frame %.1f ms" % [frames / seconds, seconds, worst])
	else:
		var until := Time.get_ticks_msec() + int(_wait * 1000.0)
		while Time.get_ticks_msec() < until:
			await process_frame
	if _out != "":
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		DirAccess.make_dir_recursive_absolute(_out.get_base_dir())
		print("mm1_capture: %s (%s)" % [_out, error_string(image.save_png(_out))])
	quit()
