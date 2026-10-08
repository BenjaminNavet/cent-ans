extends SceneTree

## Lot NT7 : fondu entre clips successifs d'un cycle de mêlée et clips propres des rôles.
## Vérifie :
## - la durée du fondu vient des données (0,15-0,25 s) et arrive au matériau (`cycle_blend`),
## - le miroir GDScript du tirage du shader (`cycle_blend_at`) : au changement de clip, le
##   fondu est actif (clip précédent distinct, poids strictement entre 0 et 1), le poids croît
##   jusqu'à 1 à la fin de la fenêtre ; aucun fondu quand le tirage redonne le même clip ;
## - les clips de rôle (porte-étendard, musiciens, servants, porte-étendard monté) sont dans les
##   rigs fins, dans les jeux des rôles, et choisis par état (charge, victoire, attente) ;
## - les servants alternent leurs gestes et le chargeur de trébuchet prend la pierre lourde ;
## - avec `--coarse-figures` (kit sans ces clips) : repli sur les jeux EP5 / SG3.
## Usage : godot --headless --path game --script res://tests/nt7_anim_test.gd [-- --coarse-figures]

const HUMAN_NEW := ["std_charge", "std_plant", "std_victory", "drum_run", "drum_victory", "horn_run", "horn_victory", "load_heavy", "push_shoulder"]
const CAVALRY_NEW := ["c_std_charge"]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("NT7 FAIL: ", what)


func _init() -> void:
	var fine := BattleSkinned.fine_enabled()
	_check_blend()
	_check_mirror()
	_check_roles(fine)
	_check_crew(fine)
	print("NT7 anim (fine=%s): %s" % [fine, "OK" if ok else "FAIL"])
	quit(0 if ok else 1)


func _check_blend() -> void:
	var data := float(BattleSkinned.animation_settings().get("cycle_blend_s", -1.0))
	_check(data >= 0.15 and data <= 0.25, "cycle_blend_s hors de 0,15-0,25 s : %.3f" % data)
	var expected := data
	_check(is_equal_approx(BattleSkinned.cycle_blend_s(), expected), "cycle_blend_s() = %.3f" % BattleSkinned.cycle_blend_s())
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, "infantry", 0)
	_check(is_equal_approx(float(mat.get_shader_parameter("cycle_blend")), expected), "cycle_blend du matériau")
	_check(BattleSkinned.SHADER.code.contains("cycle_prev("), "shader sans fondu de cycle")


## Miroir du shader : un soldat de mêlée, instants autour de ses changements de cycle.
func _check_mirror() -> void:
	var config := BattleSkinned.state_config("infantry", 0, "melee", false)
	_check(int(config["mode"]) == BattleSkinned.M_CYCLE and (config["set"] as Array).size() >= 2, "mêlée à l'épée : jeu CYCLE")
	var blend := 0.2
	var cycle := float(config["cycle"])
	var found_blend := false
	var found_same := false
	for s in 40:
		var h := Vector4(fmod(0.137 * s + 0.05, 1.0), fmod(0.61 * s + 0.3, 1.0), fmod(0.29 * s + 0.7, 1.0), 0.5)
		var rate := lerpf(0.9, 1.1, h.y)
		# Début d'un cycle : local = k * cycle, anim_time = (k * cycle - h.z * cycle) / rate.
		for k in range(2, 8):
			var start := (float(k) * cycle - h.z * cycle) / rate
			var first := BattleSkinned.cycle_blend_at(config, h, start + 0.002, blend)
			if int(first["prev"]) == int(first["clip"]):
				found_same = true
				_check(is_equal_approx(float(first["weight"]), 1.0), "même clip : pas de fondu")
				continue
			found_blend = true
			var w0 := float(first["weight"])
			_check(w0 >= 0.0 and w0 < 0.05, "poids au début du fondu : %.3f" % w0)
			_check(float(first["prev_t"]) > cycle, "le clip précédent continue au-delà du cycle")
			var last := w0
			for step in range(1, 10):
				var r := BattleSkinned.cycle_blend_at(config, h, start + 0.002 + blend * float(step) / 10.0 / rate, blend)
				var w := float(r["weight"])
				_check(int(r["clip"]) == int(first["clip"]), "clip courant stable pendant le fondu")
				_check(w > last - 1e-6, "poids croissant (%.3f après %.3f)" % [w, last])
				last = w
			_check(last > 0.8 and last < 1.0, "poids vers 1 en fin de fenêtre : %.3f" % last)
			var after := BattleSkinned.cycle_blend_at(config, h, start + (blend + 0.05) / rate, blend)
			_check(is_equal_approx(float(after["weight"]), 1.0), "fondu fini après la fenêtre")
	_check(found_blend, "aucun changement de clip trouvé")
	_check(found_same, "aucun tirage répété trouvé")
	var none := BattleSkinned.cycle_blend_at(config, Vector4(0.3, 0.4, 0.5, 0.5), 3.0, 0.0)
	_check(is_equal_approx(float(none["weight"]), 1.0), "blend 0 : changement sec")


func _check_roles(fine: bool) -> void:
	var human: Dictionary = BattleSkinned.rig("standard", 0).get("clips", {})
	var cavalry: Dictionary = BattleSkinned.rig("standard", 1).get("clips", {})
	_check(human.size() <= BattleSkinned.MAX_CLIPS and cavalry.size() <= BattleSkinned.MAX_CLIPS, "table des clips > MAX_CLIPS")
	if fine:
		for c in HUMAN_NEW:
			_check(human.has(c), "rig humain : clip %s absent" % c)
		for c in CAVALRY_NEW:
			_check(cavalry.has(c), "rig cavalier : clip %s absent" % c)
	for role in BattleStandards.FIGURES:
		var names := BattleStandards.role_set(role)
		_check(names.size() >= 4 and names.size() <= BattleSkinned.MAX_SET, "%s : %d clips" % [role, names.size()])
		var figure: Array = BattleStandards.FIGURES[role]
		if BattleSkinned.has_figure(str(figure[0]), int(figure[1])):
			var clips: Dictionary = BattleSkinned.rig(str(figure[0]), int(figure[1])).get("clips", {})
			for c in names:
				_check(clips.has(str(c)), "%s : clip %s absent du rig" % [role, c])
		for state in ["idle", "marching", "charging", "melee", "routing", "victory"]:
			for phase in [0.0, 0.3, 0.71]:
				var i := BattleStandards.clip_for(role, state, false, phase)
				_check(i >= 0 and i < names.size(), "%s %s : indice %d" % [role, state, i])
	var wanted: Dictionary = BattleSkinned.animation_settings().get("role_clips", {})
	for role in wanted:
		var names := BattleStandards.role_set(role)
		for state in ["charging", "victory"]:
			var clip := str((wanted[role] as Dictionary).get(state, ""))
			if clip == "":
				continue
			var got := str(names[BattleStandards.clip_for(role, state, false)])
			if fine:
				_check(got == clip, "%s %s : %s au lieu de %s" % [role, state, got, clip])
	if fine:
		var idle := {}
		for p in 20:
			idle[str(BattleStandards.role_set("standard")[BattleStandards.clip_for("standard", "idle", false, 0.05 * p)])] = true
		_check(idle.has("std_idle") and idle.has("std_plant"), "attentes du porte-étendard : %s" % [idle.keys()])
	else:
		_check(BattleStandards.role_set("standard").size() == 4 or BattleSkinned.fine_enabled(), "kit grossier : jeu EP5")


func _check_crew(fine: bool) -> void:
	var crew := SiegeCrewFx.new()
	crew.cfg = SiegeEnginesFx.settings().get("crew", {})
	_check(not crew.cfg.is_empty(), "réglages des servants")
	_check(crew.clip_of("loader", "bombard", 0) == "load", "chargeur de bombarde : load")
	_check(crew.clip_of("winch", "trebuchet", 1) == "crank", "treuil : crank")
	if fine:
		_check(crew.clip_of("loader", "trebuchet", 0) == "load_heavy", "chargeur de trébuchet : load_heavy")
		var pushes := {}
		for k in 4:
			pushes[crew.clip_of("pusher", "ram", k)] = true
		_check(pushes.has("push") and pushes.has("push_shoulder"), "pousseurs alternés : %s" % [pushes.keys()])
	else:
		var rig: Dictionary = BattleSkinned.rig("crew", 0).get("clips", {})
		_check(rig.has(crew.clip_of("loader", "trebuchet", 0)), "kit grossier : chargeur replié sur un clip présent")
	crew.free()
