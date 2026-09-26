extends SceneTree

## Banc PB1 : durée des fins de tour complètes de la carte (`CampaignMap._on_end_turn`, cœur Rust
## et rafraîchissement GDScript), six tours d'affilée ; détail par étape si la carte expose un
## dictionnaire `_ET` (instrumentation temporaire, µs par étape).
## Usage : godot --path game --script res://tests/pb1_turns.gd
## PB3d : fin de tour lancée comme par le joueur (`_on_end_turn(true)` : fil du cœur si le pont le
## permet) ; `total` = jusqu'à la carte rafraîchie, `worst frame` = pire image pendant ce temps.

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
	var worst_frames: Array = []
	for turn in 6:
		var et: Variant = map.get("_ET")
		if et is Dictionary:
			(et as Dictionary).clear()
		var timing := await measure_end_turn(map, self)
		var total: float = timing["total_ms"]
		totals.append(snappedf(total, 0.1))
		worst_frames.append(snappedf(timing["worst_frame_ms"], 0.1))
		var parts: Dictionary = {}
		if et is Dictionary:
			for key in et:
				if int(et[key]) > 2000:
					parts[key] = int(et[key]) / 1000
		print("PB1_TURN %d total %.1f ms worst frame %.1f ms %s" % [turn, total, timing["worst_frame_ms"], parts])
		for i in 5:
			await process_frame
		# Fenêtres ouvertes par la fin de tour (rapport, batailles) : fermées pour enchaîner.
		var ui: Object = map.get("ui")
		if ui != null and ui.has_method("close_all_dialogs"):
			ui.call("close_all_dialogs")
	print("PB1_TURNS ", totals)
	print("PB1_WORST_FRAMES ", worst_frames)
	quit(0)


## PB3d : lance une fin de tour « joueur » et attend la carte rafraîchie. Renvoie `total_ms`
## (appel → carte rafraîchie) et `worst_frame_ms` (plus long intervalle entre deux images sur
## cette durée, image de l'appel comprise). Carte sans `end_turns_refreshed` (avant PB3d) :
## l'appel est synchrone jusqu'au rafraîchissement.
static func measure_end_turn(map: Node, tree: SceneTree) -> Dictionary:
	# Rejeu des marches de l'IA du tour précédent : passé, sinon la fin de tour est ignorée.
	var replay: Object = map.get("ai_replay")
	while replay != null and bool(replay.get("playing")):
		replay.call("skip")
		await tree.process_frame
	await tree.process_frame
	var frame_start := Time.get_ticks_usec()
	var started := frame_start
	var has_counter := map.get("end_turns_refreshed") != null
	var before := int(map.get("end_turns_refreshed")) if has_counter else 0
	if has_counter:
		map.call("_on_end_turn", true)
	else:
		map.call("_on_end_turn")
	var done_at := Time.get_ticks_usec()
	var worst := 0.0
	var deadline := Time.get_ticks_msec() + 60000
	while has_counter and int(map.get("end_turns_refreshed")) == before and Time.get_ticks_msec() < deadline:
		await tree.process_frame
		var now := Time.get_ticks_usec()
		worst = maxf(worst, (now - frame_start) / 1000.0)
		frame_start = now
		done_at = now
	await tree.process_frame
	worst = maxf(worst, (Time.get_ticks_usec() - frame_start) / 1000.0)
	return {"total_ms": (done_at - started) / 1000.0, "worst_frame_ms": worst}
