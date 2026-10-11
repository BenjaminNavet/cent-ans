extends TestCase

## Capture de la salle du conseil (données de démonstration).
## Usage : tools/godot_bg.sh --path game --script res://tests/council_shot.gd
## Écrit user://council_shot.png (le rendu du SubViewport exige un pilote vidéo : en
## `--headless` strict l'image est vide).


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	root.size = Vector2i(1280, 720)
	var chamber: CouncilChamber = (load("res://scenes/ui/council_chamber.tscn") as PackedScene).instantiate()
	root.add_child(chamber)
	chamber.open_for("fac_france", "France", Color(0.2, 0.3, 0.7))
	chamber.room.set_sway(false)
	chamber.select("constable")
	chamber.room.set_hover("constable")
	chamber.show_card("constable")
	for _i in 30:
		await process_frame
	await RenderingServer.frame_post_draw
	var path := ProjectSettings.globalize_path("user://council_shot.png")
	root.get_texture().get_image().save_png(path)
	print("council shot: ", path)
	finish()
