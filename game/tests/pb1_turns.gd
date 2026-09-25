extends SceneTree

## Banc PB1 : durée des fins de tour complètes de la carte (`CampaignMap._on_end_turn`, cœur Rust
## et rafraîchissement GDScript), six tours d'affilée ; détail par étape si la carte expose un
## dictionnaire `_ET` (instrumentation temporaire, µs par étape).
## Usage : godot --path game --script res://tests/pb1_turns.gd

func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	var t0 := Time.get_ticks_msec()
	while not map.get("load_ok"):
		if Time.get_ticks_msec() - t0 > 60000:
			print("PB1_TURNS map load timeout")
			quit(1)
			return
		await process_frame
	for i in 30:
		await process_frame
	var totals: Array = []
	for turn in 6:
		var et: Variant = map.get("_ET")
		if et is Dictionary:
			(et as Dictionary).clear()
		var t := Time.get_ticks_usec()
		map.call("_on_end_turn")
		var total := (Time.get_ticks_usec() - t) / 1000.0
		totals.append(snappedf(total, 0.1))
		var parts: Dictionary = {}
		if et is Dictionary:
			for key in et:
				if int(et[key]) > 2000:
					parts[key] = int(et[key]) / 1000
		print("PB1_TURN %d total %.1f ms %s" % [turn, total, parts])
		for i in 5:
			await process_frame
		# Fenêtres ouvertes par la fin de tour (rapport, batailles) : fermées pour enchaîner.
		var ui: Object = map.get("ui")
		if ui != null and ui.has_method("close_all_dialogs"):
			ui.call("close_all_dialogs")
	print("PB1_TURNS ", totals)
	quit(0)
