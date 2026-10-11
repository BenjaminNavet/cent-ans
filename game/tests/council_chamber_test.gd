extends TestCase

## RG13c : la salle du conseil se construit avec les données de démonstration, six sièges
## (souverain + cinq charges), charge vacante = siège vide, conseils filtrés par domaine.
## Usage : godot --headless --path game --script res://tests/council_chamber_test.gd


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	var chamber: CouncilChamber = (load("res://scenes/ui/council_chamber.tscn") as PackedScene).instantiate()
	root.add_child(chamber)
	chamber.sim_source = Node.new()  # Pas de pont : démonstration
	chamber.open_for("fac_france", "France", Color(0.2, 0.3, 0.7))
	await wait_frames(3)
	check(chamber.using_demo, "demo data without bridge")
	check_eq(chamber.rows.size(), 5, "five offices")
	check_eq(chamber.room.figures.size(), 6, "six seats (ruler + 5)")
	check_eq(chamber.seat_count(), 5, "five councillor seats")
	check_eq(chamber.vacant_count(), 1, "one vacant seat")
	check(chamber.room.figures["chaplain"] == null, "vacant chaplain has no figure")
	check(chamber.room.figures["ruler"] != null and chamber.room.figures["chancellor"] != null, "figures present")
	var advice := chamber.advice_for_office("constable")
	check_eq(advice.size(), 1, "constable sees only military advice")
	check_eq(str(advice[0]["kind"]), "enemy_army", "enemy army under constable")
	check_eq(chamber.advice_for_office("treasurer").size(), 2, "treasurer: debt + free slot")
	check_eq(str(chamber.advice_for_office("treasurer")[0]["kind"]), "debt", "danger first")
	check(chamber.advice_for_office("chaplain").is_empty(), "no chaplain advice")
	var top := chamber.top_advice(3)
	check_eq(top.size(), 3, "overview shows top three")
	check_eq(str(top[0]["severity"]), "danger", "overview starts with danger")
	# Fiche, candidats, ordres de démonstration.
	chamber.select("chaplain")
	var accepted: Array = []
	chamber.order_done.connect(func(order: Dictionary) -> void: accepted.append(order))
	chamber.show_candidates("chaplain")
	var candidates := chamber.fetch_candidates("chaplain")
	check_eq(int(candidates[0]["skill_value"]), 8, "candidates sorted by skill")
	chamber.submit({"type": "appoint_councillor", "office": "chaplain", "character": "demo_x"})
	check_eq(accepted.size(), 1, "order accepted in demo mode")
	# Pointage : un rayon vers le connétable le trouve.
	var target := chamber.room.seat_position("constable") + Vector3(0, 1.0, 0)
	var origin := CouncilRoom.CAMERA_POSITION
	check_eq(chamber.room.pick_ray(origin, target - origin), "constable", "ray picks constable")
	check_eq(chamber.room.pick_ray(origin, Vector3(0, 0.2, 1)), "", "ray away picks nothing")
	chamber.room.set_hover("constable")
	check_eq(chamber.room.hovered_key, "constable", "hover set")
	chamber.show_card("constable")
	check(chamber.card.visible, "hover card visible")
	chamber.queue_free()
	await _check_styles()
	finish()


## RG13-V23 : les trois styles se construisent, repli multiview -> mesh, mêmes zones de clic.
func _check_styles() -> void:
	var occupied := {"chancellor": true, "constable": true, "treasurer": true, "chaplain": false, "chamberlain": true}
	var has_multiview := ResourceLoader.exists(CouncilRoom.MULTIVIEW_DIR + "council_ruler.glb")
	for wanted in CouncilRoom.STYLES:
		CouncilRoom.style_override = wanted
		var room := CouncilRoom.new()
		root.add_child(room)
		room.build(Color(0.2, 0.3, 0.7), "fac_france")
		room.populate(occupied)
		await wait_frames(2)
		var expected: String = wanted
		if wanted == "mesh_multiview" and not has_multiview:
			expected = "mesh"
		check_eq(room.requested_style, wanted, "requested style " + wanted)
		check_eq(room.style, expected, "effective style " + wanted)
		check_eq(room.figures.size(), 6, "six seats in " + wanted)
		check(room.figures["ruler"] is Node3D and room.figures["chaplain"] == null, "figures and vacancy in " + wanted)
		var painted := _count_painted(room)
		check_eq(painted, 5 if expected == "painted" else 0, "painted quads in " + wanted)
		var origin := CouncilRoom.CAMERA_POSITION
		for key in ["ruler", "chancellor", "constable", "chamberlain"]:
			var target: Vector3 = room.seat_position(key) + Vector3(0, 1.0, 0)
			check_eq(room.pick_ray(origin, target - origin), key, "ray picks %s in %s" % [key, wanted])
		room.set_hover("constable")
		check_eq(room.hovered_key, "constable", "hover in " + wanted)
		room.set_hover("")
		room.queue_free()
		await wait_frames(1)
	CouncilRoom.style_override = ""


func _count_painted(node: Node) -> int:
	var count := 1 if node.has_meta("painted") else 0
	for child in node.get_children():
		count += _count_painted(child)
	return count
