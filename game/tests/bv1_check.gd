extends SceneTree

## Lot BV1 : contrôles sans rendu (`godot --headless --path game --script res://tests/bv1_check.gd`).
## Volées : nombre de traits (taille d'unité), traits fichés dans la zone visée, pavois ;
## sang : réglage « désactivé » sans rien créer, flaques en mêlée ; poussière selon le sol.

var _failures := 0


func _init() -> void:
	var world := Node3D.new()
	root.add_child(world)
	var flat := func(_x: float, _z: float) -> float: return 0.0
	var dry := func(_x: float, _z: float) -> int: return 0
	var volleys := BattleVolleys.new()
	world.add_child(volleys)
	volleys.setup(flat)
	var archers := {"id": 1, "side": "attacker", "x": 0.0, "y": 0.0, "z": 0.0, "width": 40.0, "depth": 3.0, "facing": 0.0}
	var target := {"id": 2, "side": "defender", "x": 0.0, "y": 0.0, "z": 170.0, "width": 50.0, "depth": 4.0, "facing": PI}
	var by_id := {1: archers, 2: target}
	var shot := {"shooter": 1, "target": 2, "aim": Vector2(0, 170), "missiles": 100, "kills": 3.0, "kind": "arrow", "incendiary": false, "cover": "none"}
	var hits: Array = volleys.on_shot(shot, by_id, Vector3(0, 10, 0))
	_check(volleys.launched == 300, "3 traits par homme : %d" % volleys.launched)
	_check(hits.size() == 3, "une gerbe par tué : %d" % hits.size())
	_check(volleys.stuck_count >= 50 and volleys.stuck_count <= 96, "traits confiés à la couche plantée : %d" % volleys.stuck_count)
	var chunk := {"src": Vector3(0, 0, 0), "tgt": Vector3(0, 0, 170), "launch": 0.0, "shw": 20.0, "shd": 1.5, "thw": 25.0, "thd": 2.0, "slope": Vector2.ZERO, "code": 0, "seed": 7, "count": 256}
	var sh := BattleVolleys._pcg(7)
	var inside := 0
	for i in 256:
		var a: Dictionary = volleys.arrow_landing(chunk, sh, i)
		var p: Vector3 = a["pos"]
		if absf(p.x) <= 27.0 and absf(p.z - 170.0) <= 6.2 + 0.01:
			inside += 1
		_check(float(a["time"]) > 3.0 and float(a["time"]) < 6.5, "durée de vol plausible : %.2f" % float(a["time"]))
		_check(Vector3(a["dir"]).y < -0.3, "le trait retombe")
	_check(inside == 256, "traits dans la zone visée : %d/256" % inside)
	# R4 : une volée en cloche (`indirect`) vole plus longtemps et retombe plus raide, au même point.
	var flat_arrow: Dictionary = volleys.arrow_landing(chunk, sh, 0)
	chunk["code"] = 32
	var lobbed_arrow: Dictionary = volleys.arrow_landing(chunk, sh, 0)
	chunk["code"] = 0
	_check(float(lobbed_arrow["time"]) > float(flat_arrow["time"]) + 0.5, "cloche plus longue : %.2f / %.2f" % [float(lobbed_arrow["time"]), float(flat_arrow["time"])])
	_check(Vector3(lobbed_arrow["dir"]).y < Vector3(flat_arrow["dir"]).y, "cloche plus raide")
	_check(Vector3(lobbed_arrow["pos"]).is_equal_approx(Vector3(flat_arrow["pos"])), "même point de chute")
	# Hachage : identique à `volley_pcg` (GLSL, uint 32 bits) sur quelques valeurs de référence.
	_check(BattleVolleys._pcg(0) == 129708002, "pcg(0) = %d" % BattleVolleys._pcg(0))
	_check(BattleVolleys._pcg(1) == 2831084092, "pcg(1) = %d" % BattleVolleys._pcg(1))
	# Pavois : une partie des traits se fiche en l'air, devant la cible.
	chunk["code"] = 4
	var raised := 0
	for i in 256:
		var a: Dictionary = volleys.arrow_landing(chunk, sh, i)
		if bool(a["cover"]):
			raised += 1
			_check(Vector3(a["pos"]).y > 0.3 and absf(Vector3(a["pos"]).z - (170.0 - 2.9)) < 0.01, "trait dans le pavois")
	_check(raised > 40 and raised < 180, "traits dans les pavois : %d/256" % raised)
	# Pieux plantés une fois.
	target["stakes"] = true
	target["present"] = true
	volleys.update_fieldworks([target])
	volleys.update_fieldworks([target])
	# Sang désactivé : rien.
	var off := BattleBlood.new()
	world.add_child(off)
	off.setup(flat, BattleBlood.OFF, dry)
	off.add_hit(Vector3(0, 0, 170), 0.0, Vector3.ZERO)
	off.tick_time(1.0)
	_check(off.decal_count == 0 and off.get_child_count() == 0, "sang désactivé : aucun nœud")
	# Sang complet : pertes en mêlée → flaques.
	var full := BattleBlood.new()
	world.add_child(full)
	full.setup(flat, BattleBlood.FULL, dry)
	var fighting := {"id": 5, "present": true, "state": "melee", "soldiers": 100, "x": 0.0, "y": 0.0, "z": 0.0, "facing": 0.0, "width": 30.0, "depth": 4.0}
	full.update([fighting], Vector3.ZERO)
	fighting["soldiers"] = 96
	full.tick_time(0.5)
	full.update([fighting], Vector3.ZERO)
	_check(full.decal_count >= 4, "flaques en mêlée : %d" % full.decal_count)
	full.add_hit(Vector3(0, 0, 10), 2.0, Vector3.ZERO)
	var before := full.decal_count
	full.tick_time(1.9)
	_check(full.decal_count == before, "la touche attend l'arrivée du trait")
	full.tick_time(2.1)
	_check(full.decal_count > before, "touche : flaque à l'arrivée")
	# Poussière : pas sur la boue ni sous la pluie.
	var fx := BattleEffects.new()
	world.add_child(fx)
	fx.setup("clear", flat, dry)
	fx.configure_ground("muddy", "clear")
	_check(not fx.enabled_dust, "pas de poussière dans la boue")
	fx.configure_ground("dry", "rain")
	_check(not fx.enabled_dust, "pas de poussière sous la pluie")
	fx.configure_ground("dry", "clear")
	_check(fx.enabled_dust, "poussière sur sol sec")
	print("bv1_check: %s" % ("OK" if _failures == 0 else "%d FAILURES" % _failures))
	quit(0 if _failures == 0 else 1)


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		push_error("bv1_check FAILED: " + what)
