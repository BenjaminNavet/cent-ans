extends SceneTree

## Test headless du lot S1 (`WallCollapseFx`, rendu seulement) sur des pans factices :
##  - réglages lus depuis `data/fx/siege_fx.json` ;
##  - un pan qui tombe produit N blocs (N dans [blocks_min, blocks_max]) + un corps par merlon,
##    même N et même position du premier bloc pour le même index (graine fixe) ;
##  - éboulis cachés pendant la chute puis montrés après `rubble_reveal_seconds` ;
##  - état initial (avant `prime`) enregistré sans effet ;
##  - palier de dégâts : 3-6 pierres ; porte : planches ; plafond de corps actifs respecté ;
##  - quelques images de physique (Jolt) sans erreur, puis corps libérés ou gelés.
## Usage : godot --headless --path game --script res://tests/wall_collapse_fx_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	var fx := WallCollapseFx.new()
	root.add_child(fx)
	fx.setup(func(_x: float, _z: float) -> float: return 0.0)
	if not _check(fx.enabled, "settings not loaded from data/fx/siege_fx.json"):
		quit(1)
		return
	var cfg: Dictionary = fx.settings["wall"]
	var lo := int(cfg["blocks_min"])
	var hi := int(cfg["blocks_max"])

	# État initial silencieux : un pan déjà effondré avant `prime` ne s'anime pas.
	var pre := _make_piece(fx, 99, false, Vector3(0, 0, -200))
	fx.sync_piece(99, pre, 0.0, false)
	_check(fx.body_count() == 0, "initial collapsed piece should not animate")
	fx.prime()

	# Effondrement : N blocs + merlons.
	var view := _make_piece(fx, 3, false, Vector3.ZERO)
	var merlons := ((view["wall"] as Node3D).get_child(1) as MultiMeshInstance3D).multimesh.instance_count
	fx.sync_piece(3, view, 0.0, false)
	var blocks := fx.body_count() - merlons
	_check(blocks >= lo and blocks <= hi, "collapse: %d blocks, expected %d..%d" % [blocks, lo, hi])
	_check(not (view["wall"] as Node3D).visible, "collapse: intact wall should be hidden")
	_check(not (view["rubble"] as Node3D).visible, "collapse: rubble hidden while blocks fall")
	var first := _first_body_position(fx)

	# Même index, même fracture.
	var fx2 := WallCollapseFx.new()
	root.add_child(fx2)
	fx2.setup(func(_x: float, _z: float) -> float: return 0.0)
	fx2.prime()
	fx2.sync_piece(3, _make_piece(fx2, 3, false, Vector3.ZERO), 0.0, false)
	_check(fx2.body_count() == fx.body_count(), "same index should give the same block count")
	_check(_first_body_position(fx2).is_equal_approx(first), "same index should give the same first block")
	fx2.queue_free()

	# Quelques images de physique réelle.
	for _i in 30:
		await physics_frame
	fx.advance(float(cfg["rubble_reveal_seconds"]) + 0.01)
	_check((view["rubble"] as Node3D).visible, "rubble should appear after the reveal delay")

	# Palier de dégâts sur un pan intact.
	var hit := _make_piece(fx, 5, false, Vector3(60, 0, 0))
	var before := fx.body_count()
	fx.sync_piece(5, hit, 0.8, true)
	_check(fx.body_count() == before, "no stones above the first threshold")
	fx.sync_piece(5, hit, 0.7, true)
	var stones := fx.body_count() - before
	var p: Dictionary = fx.settings["parapet"]
	_check(stones >= int(p["stones_min"]) and stones <= int(p["stones_max"]), "parapet: %d stones" % stones)

	# Porte.
	var gate := _make_piece(fx, 7, true, Vector3(-60, 0, 0))
	before = fx.body_count()
	fx.sync_piece(7, gate, 0.0, false)
	var planks := fx.body_count() - before
	_check(planks == 2 * int(fx.settings["gate"]["planks_per_leaf"]), "gate: %d planks" % planks)
	for child in (gate["wall"] as Node3D).get_children():
		if String(child.name).begins_with("Door"):
			_check(not (child as Node3D).visible, "gate: leaves should be hidden")

	# Plafond : 8 pans de plus.
	for k in 8:
		fx.sync_piece(20 + k, _make_piece(fx, 20 + k, false, Vector3(0, 0, 60.0 * (k + 1))), 0.0, false)
	var cap := int(fx.settings["max_active_bodies"])
	_check(fx.active_body_count() <= cap, "cap: %d active bodies > %d" % [fx.active_body_count(), cap])
	_check(fx.body_count() <= cap + int(fx.settings["max_kept_bodies"]), "cap: too many kept bodies")
	for _i in 10:
		await physics_frame

	# Fin des chutes : les corps sont libérés ou gelés.
	fx.advance(30.0)
	_check(fx.active_body_count() == 0, "all bodies should be settled after 30 s")
	print("wall collapse fx: %d blocks + %d merlons, %d stones, %d planks, %d bodies kept" % [blocks, merlons, stones, planks, fx.body_count()])
	fx.queue_free()
	await process_frame
	if _failures == 0:
		print("wall collapse fx OK")
	quit(0 if _failures == 0 else 1)


## Pan factice au format de `BattleSiege._pieces` (mêmes dimensions que `_build_piece`).
func _make_piece(fx: Node3D, index: int, gate: bool, at: Vector3) -> Dictionary:
	var length := 24.0
	var height := 8.5
	var thickness := 3.0
	var node := Node3D.new()
	node.name = "Piece%d" % index
	node.position = at + Vector3(0, -0.5, 0)
	fx.get_parent().add_child(node)
	var wall := Node3D.new()
	node.add_child(wall)
	var mat := StandardMaterial3D.new()
	if gate:
		for side in [-1.0, 1.0]:
			var door := MeshInstance3D.new()
			var door_box := BoxMesh.new()
			door_box.size = Vector3(length * 0.5 - 0.1, 5.0, 0.5)
			door.mesh = door_box
			door.name = "Door"
			door.position = Vector3(side * length * 0.25, 2.5, thickness * 0.5 - 0.2)
			wall.add_child(door, true)
	else:
		var body := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(length + 0.4, height, thickness)
		body.mesh = box
		body.material_override = mat
		body.position = Vector3(0, height * 0.5, 0)
		wall.add_child(body)
		var merlon := BoxMesh.new()
		merlon.size = Vector3(1.0, 1.2, 0.6)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = merlon
		mm.instance_count = int(length / 2.2)
		for i in mm.instance_count:
			mm.set_instance_transform(i, Transform3D(Basis(), Vector3(-length * 0.5 + (i + 0.5) * length / mm.instance_count, height + 0.6, 1.2)))
		var merlons := MultiMeshInstance3D.new()
		merlons.multimesh = mm
		wall.add_child(merlons)
	var rubble := Node3D.new()
	rubble.visible = false
	node.add_child(rubble)
	return {"node": node, "wall": wall, "rubble": rubble, "material": mat, "gate": gate, "ratio": 1.0}


func _first_body_position(fx: Node) -> Vector3:
	for child in fx.get_children():
		if child is RigidBody3D:
			return (child as RigidBody3D).global_position
	return Vector3.INF


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + message)
	return condition
