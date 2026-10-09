extends TestCase

## RL1 : préréglage de qualité par défaut selon le GPU détecté, sans écraser le choix du joueur.
## Usage : godot --headless --path game --script res://tests/rl1_quality_test.gd
## Code de sortie 0 si tout passe, 1 sinon.


func _init() -> void:
	await process_frame
	var cases := {
		"Apple M4 Pro": "high", "Apple M2": "high", "Apple M3 Max": "high", "Apple M1 Pro": "high",
		"Apple M1": "medium", "Intel(R) Iris(TM) Plus Graphics": "medium", "": "medium",
		"AMD Radeon Pro 5500M": "medium", "Apple M10": "high",
	}
	for adapter: String in cases:
		check(RenderQuality.level_for_adapter(adapter) == cases[adapter], "%s -> %s (got %s)" % [adapter, cases[adapter], RenderQuality.level_for_adapter(adapter)])
	var settings: Node = root.get_node_or_null("Settings")
	check(settings != null, "Settings autoload")
	if settings != null:
		var path := "user://rl1_quality_%d.cfg" % OS.get_process_id()
		settings.call("use_test_file", path)
		check(str(settings.call("get_value", "video/quality")) == RenderQuality.AUTO, "default is auto")
		check(RenderQuality.current() == RenderQuality.detected_level(), "auto resolves to the detected level")
		# Choix du joueur enregistré puis relu : il l'emporte sur la détection.
		settings.call("set_value", "video/quality", "low")
		settings.call("load_settings", path)
		check(RenderQuality.current() == "low", "saved player choice kept (got %s)" % RenderQuality.current())
		# Ancien fichier sans la clé : détection.
		var config := ConfigFile.new()
		config.set_value("video", "vsync", true)
		config.save(path)
		settings.call("load_settings", path)
		check(RenderQuality.current() == RenderQuality.detected_level(), "missing key -> detected level")
		DirAccess.remove_absolute(path)
	finish()


		print("FAIL: " + message)
