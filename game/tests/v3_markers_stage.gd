extends SceneTree

## Mise en scène des marqueurs d'armée du lot V3 (fenêtre réelle) : quatre armées factices
## dans l'ouest de la France — au repos, en marche, assiégeant, embarquée — puis capture.
## Usage : godot --path game --script res://tests/v3_markers_stage.gd -- <capture.png> [x,y,distance]


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


func _init() -> void:
	await process_frame
	var args := OS.get_cmdline_user_args()
	var out_path := args[0] if args.size() > 0 else "user://v3_markers.png"
	var focus := Vector3(2000, 3180, 230)
	if args.size() > 1:
		var parts := args[1].split(",")
		focus = Vector3(float(parts[0]), float(parts[1]), float(parts[2]))
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	var sim := FakeSim.new()
	sim.armies = {
		"army_fr": {"faction": "fac_france", "general": "chr_philippe_vi", "location": "prov_maine", "stance": "normal", "path": [],
			"units": _units([["unit_knights", 3, 60], ["unit_men_at_arms_foot", 4, 80], ["unit_crossbowmen", 3, 100]])},
		"army_en_march": {"faction": "fac_england", "location": "prov_normandie", "stance": "normal", "path": ["prov_ile_de_france"],
			"units": _units([["unit_longbowmen", 4, 100], ["unit_men_at_arms_foot", 2, 80]])},
		"army_en_siege": {"faction": "fac_england", "location": "prov_normandie_ouest", "stance": "siege", "path": [],
			"units": _units([["unit_men_at_arms_foot", 3, 80], ["unit_trebuchet", 1, 20]])},
		"army_bret_ship": {"faction": "fac_brittany", "location": "prov_bretagne", "stance": "normal", "embarked": true, "path": [],
			"units": _units([["unit_men_at_arms_foot", 2, 80]])},
	}
	var facade: Node = root.get_node("/root/SimFacade")
	var armies: ArmyMarkers = map.armies
	armies.refresh(sim, Callable(facade, "faction_color"), "fac_france")
	armies.set_selected("army_fr")
	var data: MapData = map.map_data
	var rig: CampaignCamera = map.camera_rig
	rig.look_at_point(Vector3(focus.x, data.surface_world_at(focus.x, focus.y), focus.y), focus.z)
	rig.snap()
	map.ui.visible = false
	for i in 60:
		await process_frame
	print("v3 stage: %d markers, plates: %s" % [armies.army_count(), [armies.plate_text("army_fr"), armies.plate_text("army_en_siege"), armies.plate_text("army_bret_ship")]])
	var image := root.get_texture().get_image()
	image.save_png(out_path)
	print("v3 stage: saved %s" % out_path)
	map.queue_free()
	await process_frame
	quit(0)
