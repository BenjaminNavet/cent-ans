extends SceneTree

## NT8 : rendu du château de siège. Monte un siège de château (`debug_stage_place_siege`) et
## vérifie : un seul donjon, carré (fût `KeepBody` à la hauteur donnée par le cœur, emprise
## carrée), nettement plus haut que les tours ; tours du château fines (rayon ≤ seuil) ;
## basse-cour meublée (puits et accessoires, pas d'étals).
## Usage : godot --headless --path game --script res://tests/nt8_castle_test.gd

const MAX_TOWER_RADIUS := 6.0
const KEEP_ABOVE_TOWERS := 8.0

var _failures := 0


func _init() -> void:
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("nt8_castle_test: no campaign")
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_place_siege", armies[0], "castle")
	_check(index >= 0, "castle siege staged")
	if index < 0:
		quit(1)
		return
	BattleScene.demo_args = PackedStringArray(["--no-speech"])
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 7)
	root.add_child(scene)
	for _i in 20:
		await process_frame
	var siege: Dictionary = scene.battle.call("get_siege")
	_check(str(siege.get("place", "")) == "castle", "place is a castle")
	var towers: Array = siege.get("towers", [])
	var widest := 0.0
	var tallest := 0.0
	for tower in towers:
		widest = maxf(widest, float(tower["radius"]))
		tallest = maxf(tallest, float(tower["height"]))
	_check(widest > 0.0 and widest <= MAX_TOWER_RADIUS, "castle towers slim (widest radius %.1f m)" % widest)
	var keeps: Array = (siege.get("houses", []) as Array).filter(func(h: Dictionary) -> bool: return bool(h.get("keep", false)))
	_check(keeps.size() == 1, "one keep (%d)" % keeps.size())
	var render: Node = scene.siege_view
	var keep_nodes: Array = render.find_children("Keep", "Node3D", false, false) if render != null else []
	_check(keep_nodes.size() == 1, "one keep drawn (%d)" % keep_nodes.size())
	if keeps.size() == 1 and keep_nodes.size() == 1:
		var keep: Dictionary = keeps[0]
		var h := float(keep.get("height", 0.0))
		_check(h >= tallest + KEEP_ABOVE_TOWERS, "keep %.1f m above the towers (%.1f m)" % [h, tallest])
		var body := (keep_nodes[0] as Node3D).get_node_or_null("KeepBody") as MeshInstance3D
		_check(body != null, "square keep body")
		if body != null:
			var size: Vector3 = (body.mesh as BoxMesh).size
			_check(absf(size.y - h) < 0.01, "keep drawn at the core's height (%.1f)" % size.y)
			_check(absf(size.x - float(keep["length"])) < 0.01 and absf(size.z - float(keep["depth"])) < 0.01, "keep on the core's footprint")
		_check_keep_variant(keep_nodes[0] as Node3D, bool(keep.get("terrace", false)))
		# NT11 : l'autre toit, dessiné sur le même site (les deux variantes sont rendues).
		var sites: Array = (render.get("house_sites") as Array).filter(func(s: Dictionary) -> bool: return bool(s["keep"]))
		if _check_ret(sites.size() == 1, "keep site"):
			var other: Dictionary = (sites[0] as Dictionary).duplicate()
			other["terrace"] = not bool(keep.get("terrace", false))
			render.call("_build_keep", other)
			_check_keep_variant(render.get_child(render.get_child_count() - 1) as Node3D, bool(other["terrace"]))
	var props: Array = siege.get("props", [])
	var wells := props.filter(func(p: Dictionary) -> bool: return str(p["kind"]) == "well").size()
	var stalls := props.filter(func(p: Dictionary) -> bool: return str(p["kind"]) == "stall").size()
	var bailey := props.filter(func(p: Dictionary) -> bool: return int(p["house"]) < 0).size()
	_check(wells == 1 and stalls == 0 and bailey >= 4, "bailey: well %d, stalls %d, props %d" % [wells, stalls, bailey])
	print("nt8_castle_test: towers<=%.1f m keep_nodes=%d props=%d" % [widest, keep_nodes.size(), props.size()])
	scene.queue_free()
	await process_frame
	print("nt8_castle_test: %s" % ("OK" if _failures == 0 else "%d FAILURE(S)" % _failures))
	quit(1 if _failures > 0 else 0)


## NT11 : toit (pavillon ou terrasse crénelée), porte haute et escalier extérieur d'un donjon.
func _check_keep_variant(keep: Node3D, terrace: bool) -> void:
	var roof := keep.get_node_or_null("KeepRoof")
	var deck := keep.get_node_or_null("KeepTerrace")
	if terrace:
		_check(roof == null and deck != null and keep.get_node_or_null("KeepWatch") != null, "terrace keep: crenellated deck and watch turret")
	else:
		_check(roof != null and deck == null, "pavilion roof keep")
	var stair := keep.get_node_or_null("KeepStair")
	_check(stair != null and stair.get_node_or_null("KeepStairLanding") != null, "outside stair with a landing")
	var door := keep.get_node_or_null("KeepDoor") as Node3D
	_check(door != null and door.position.y > 5.0, "raised keep door (%.1f m)" % (door.position.y if door != null else -1.0))


func _check_ret(ok: bool, label: String) -> bool:
	_check(ok, label)
	return ok


func _check(ok: bool, label: String) -> void:
	if not ok:
		_failures += 1
		push_error("nt8_castle_test FAILED: " + label)
	else:
		print("  ok: " + label)
