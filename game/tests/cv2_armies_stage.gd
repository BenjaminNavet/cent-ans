extends SceneTree

## Mise en scène du lot CV2 (fenêtre réelle) : armées figurées sur la carte de campagne.
## Usage :
##   godot --path game --script res://tests/cv2_armies_stage.gd -- <préfixe> [options]
## Options :
##   --zooms=60,160,320,800   distances caméra des captures (<préfixe>_<distance>.png) ;
##   --focus=x,y              point visé (pixels de carte) ;
##   --walk                   l'armée « army_en_march » marche le long d'un chemin pendant les captures ;
##   --fps                    mesure (armées réelles de la partie, pas de mise en scène) : i/s moyens
##                            sur 300 images à chaque distance de `--zooms` ;
##   --legacy-army-markers    figurines M10/V3 d'avant CV2 (comparaison A/B).


class FakeSim:
	extends RefCounted
	var armies: Dictionary = {}

	func get_army_ids() -> PackedStringArray:
		return PackedStringArray(armies.keys())

	func get_army(id: String) -> Dictionary:
		return armies.get(id, {})


static func _units(spec: Array) -> Array:
	var units: Array = []
	for item in spec:
		for i in int(item[1]):
			units.append({"unit_type": item[0], "strength": int(item[2]), "max_strength": int(item[2]), "morale": 70})
	return units


static func _arg(args: PackedStringArray, key: String, fallback: String) -> String:
	for arg in args:
		if arg.begins_with(key + "="):
			return arg.substr(key.length() + 1)
	return fallback


func _init() -> void:
	await process_frame
	var args := OS.get_cmdline_user_args()
	var prefix := args[0] if args.size() > 0 and not args[0].begins_with("--") else "user://cv2"
	var zooms := PackedFloat32Array()
	for part in _arg(args, "--zooms", "60,160,320,800").split(","):
		zooms.append(float(part))
	var focus_parts := _arg(args, "--focus", "1985,1905").split(",")
	var focus := Vector2(float(focus_parts[0]), float(focus_parts[1]))
	var fps_mode := args.has("--fps")
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	var armies: ArmyMarkers = map.armies
	var data: MapData = map.map_data
	var rig: CampaignCamera = map.camera_rig
	map.ui.visible = false
	if not fps_mode:
		var sim := FakeSim.new()
		sim.armies = {
			"army_fr": {"faction": "fac_france", "general": "chr_philippe_vi", "location": "prov_maine", "stance": "normal", "path": [],
				"position": focus + Vector2(-22, 6),
				"units": _units([["unit_knights", 3, 60], ["unit_men_at_arms_foot", 4, 80], ["unit_genoese_crossbowmen", 3, 100], ["unit_urban_militia", 2, 120]])},
			"army_en_march": {"faction": "fac_england", "location": "prov_maine", "stance": "normal", "path": [],
				"position": focus + Vector2(18, -14),
				"planned_path": PackedVector2Array([focus + Vector2(60, -30)]),
				"units": _units([["unit_longbowmen", 4, 100], ["unit_men_at_arms_foot", 2, 80], ["unit_mounted_archers", 1, 60]])},
			"army_en_siege": {"faction": "fac_england", "location": "prov_normandie_ouest", "stance": "siege", "path": [],
				"units": _units([["unit_men_at_arms_foot", 3, 80], ["unit_flemish_pikemen", 2, 100], ["unit_trebuchet", 1, 20]])},
			"army_small": {"faction": "fac_brittany", "location": "prov_maine", "stance": "normal", "path": [],
				"position": focus + Vector2(-4, 34),
				"units": _units([["unit_urban_militia", 2, 120]])},
			"army_bret_ship": {"faction": "fac_brittany", "location": "prov_bretagne", "stance": "normal", "embarked": true, "path": [],
				"position": focus + Vector2(-70, -150),
				"units": _units([["unit_men_at_arms_foot", 6, 80], ["unit_crossbowmen", 4, 100]])},
		}
		var facade: Node = root.get_node("/root/SimFacade")
		armies.refresh(sim, Callable(facade, "faction_color"), "fac_france")
		armies.set_selected("army_fr")
	var start: Vector2 = focus + Vector2(18, -14)
	var goal: Vector2 = focus + Vector2(60, -30)
	for distance in zooms:
		rig.look_at_point(Vector3(focus.x, data.surface_world_at(focus.x, focus.y), focus.y), distance)
		rig.snap()
		for i in 90:
			if args.has("--walk") and not fps_mode:
				var t := float(i % 90) / 90.0
				armies.place_marker("army_en_march", start.lerp(goal, t * 0.5), goal - start)
			await process_frame
		if fps_mode:
			var t0 := Time.get_ticks_usec()
			for i in 300:
				await process_frame
			var seconds := (Time.get_ticks_usec() - t0) / 1000000.0
			print("cv2 fps: distance %d, %d markers, %.1f fps%s" % [int(distance), armies.army_count(), 300.0 / seconds, " (legacy)" if args.has("--legacy-army-markers") else ""])
			continue
		var image := root.get_texture().get_image()
		var out_path := "%s_%d.png" % [prefix, int(distance)]
		image.save_png(out_path)
		print("cv2 stage: saved %s (%d markers)" % [out_path, armies.army_count()])
	map.queue_free()
	await process_frame
	quit(0)
