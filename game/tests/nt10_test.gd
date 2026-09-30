extends SceneTree

## Lot NT10 : fondu des clips de rôle, imposteurs (copies, ombre, sang), herbe hors champ.
## Vérifie sans rendu :
## - la durée du fondu des rôles vient des données (0,15-0,35 s), arrive aux matériaux
##   (`role_blend`), vaut 0 avec `--no-nt10` ;
## - l'empaquetage de INSTANCE_CUSTOM.y (`pack_fade`) : emplacement, clip précédent et instant
##   du changement se relisent comme dans le shader, entier exact sous 2^24, jamais en avance ;
## - le suivi du clip précédent (`fade_prev`) : posé au changement, effacé après le fondu ;
## - les shaders portent le fondu (skinné : `custom_fade` ; drapeau : `pick_clip` et fondu) ;
## - les servants : au changement de geste, y porte le geste précédent et z son début ;
## - les figurines de rôle ont leurs indices globaux de clips ;
## - imposteurs : identifiants de cuisson (livrée / habit non teint selon `livery_share`),
##   shader d'ombre multiplicatif ;
## - herbe couchée : le rectangle déborde du champ, un corps tombé hors du champ couche l'herbe.
## Usage : godot --headless --path game --script res://tests/nt10_test.gd [-- --no-nt10]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("NT10 FAIL: ", what)


func _init() -> void:
	var off := "--no-nt10" in OS.get_cmdline_user_args()
	_check_blend(off)
	_check_pack()
	_check_tracker(off)
	_check_shaders()
	_check_roles()
	_check_crew(off)
	_check_impostors(off)
	_check_grass()
	print("NT10 (no_nt10=%s): %s" % [off, "OK" if ok else "FAIL"])
	quit(0 if ok else 1)


func _check_blend(off: bool) -> void:
	var data := float(BattleSkinned.animation_settings().get("role_blend_s", -1.0))
	_check(data >= 0.15 and data <= 0.35, "role_blend_s hors de 0,15-0,35 s : %.3f" % data)
	var expected := 0.0 if off else data
	_check(is_equal_approx(BattleSkinned.role_blend_s(), expected), "role_blend_s() = %.3f" % BattleSkinned.role_blend_s())
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, "infantry", 0)
	_check(is_equal_approx(float(mat.get_shader_parameter("role_blend")), expected), "role_blend du matériau")


## Relecture de y comme le shader (`anim_pose`, mode CUSTOM).
func _decode(y: float) -> Dictionary:
	var packed := floorf(y / 32.0)
	return {"slot": int(fposmod(y, 32.0)), "prev": int(fposmod(packed, 128.0) + 0.5) - 1, "at": floorf(y / 4096.0) / 32.0}


func _check_pack() -> void:
	_check(is_equal_approx(BattleSkinned.pack_fade(3, -1, 12.0), 3.0), "sans clip précédent : y = emplacement")
	for case in [[0, 0, 0.0], [6, 69, 17.37], [31, 95, 63.99], [2, 12, 1234.5]]:
		var y := BattleSkinned.pack_fade(int(case[0]), int(case[1]), float(case[2]))
		_check(y < 16777216.0 and y == floorf(y), "y entier sous 2^24 : %f" % y)
		var d := _decode(float(PackedFloat32Array([y])[0]))
		_check(int(d["slot"]) == int(case[0]), "emplacement relu %d / %d" % [d["slot"], case[0]])
		_check(int(d["prev"]) == int(case[1]), "clip précédent relu %d / %d" % [d["prev"], case[1]])
		var at := fposmod(float(case[2]), 64.0)
		_check(float(d["at"]) <= at + 1e-4 and at - float(d["at"]) < 1.0 / 32.0 + 1e-4, "instant relu %.4f / %.4f" % [d["at"], at])
		# Écart relu comme le shader : jamais négatif (le fondu ne part pas en avance).
		var since := fposmod(float(case[2]) - float(d["at"]), 64.0)
		_check(since < 0.05, "écart au changement : %.4f" % since)


func _check_tracker(off: bool) -> void:
	var state := {}
	_check(BattleSkinned.fade_prev(state, 10, 5.0) == -1, "premier clip : pas de fondu")
	_check(BattleSkinned.fade_prev(state, 10, 6.0) == -1, "même clip : pas de fondu")
	var prev := BattleSkinned.fade_prev(state, 14, 7.0)
	if off:
		_check(prev == -1, "--no-nt10 : changement sec")
		return
	_check(prev == 10, "changement : clip précédent 10 (%d)" % prev)
	_check(is_equal_approx(float(state["at"]), 7.0), "instant du changement")
	_check(BattleSkinned.fade_prev(state, 14, 7.1) == 10, "pendant le fondu")
	_check(BattleSkinned.fade_prev(state, 14, 7.0 + BattleSkinned.role_blend_s() + 0.2) == -1, "après le fondu")
	_check(BattleSkinned.fade_prev(state, 20, 9.0) == 14 and BattleSkinned.fade_prev(state, 20, 3.0) == -1, "horloge revenue en arrière")


func _check_shaders() -> void:
	var skinned := BattleSkinned.SHADER.code
	_check(skinned.contains("uniform int custom_fade") and skinned.contains("role_blend"), "shader skinné sans fondu de rôle")
	_check(skinned.contains("bake_id"), "shader skinné sans identifiant de cuisson")
	var flag: String = (load("res://shaders/battle_standard_flag.gdshader") as Shader).code
	_check(flag.contains("pick_clip(clip_set, slot)") and flag.contains("role_blend"), "drapeau : pick_clip et fondu")
	var shadow: String = (load("res://shaders/battle_impostor_shadow.gdshader") as Shader).code
	_check(shadow.contains("blend_mul") and shadow.contains("depth_draw_never"), "ombre des imposteurs multiplicative")
	var imp: String = (load("res://shaders/battle_impostor.gdshader") as Shader).code
	_check(imp.contains("uniform int copies") and imp.contains("uniform float blood"), "imposteur : copies et sang")


func _check_roles() -> void:
	for role in BattleStandards.FIGURES:
		var figure: Array = BattleStandards.FIGURES[role]
		var ids := BattleStandards.role_ids(role)
		if not BattleSkinned.has_figure(str(figure[0]), int(figure[1])):
			_check(ids.is_empty(), "%s sans figurine : pas d'indices" % role)
			continue
		_check(ids.size() == BattleStandards.role_set(role).size(), "%s : un indice par clip" % role)
		var clips: Dictionary = BattleSkinned.rig(str(figure[0]), int(figure[1])).get("clips", {})
		for i in ids:
			_check(int(i) >= 0 and int(i) < clips.size(), "%s : indice %d hors table" % [role, i])


func _check_crew(off: bool) -> void:
	if not SiegeCrewFx.enabled():
		print("NT10: servants absents (kit), vérification sautée")
		return
	var crew := SiegeCrewFx.new()
	crew.cfg = SiegeEnginesFx.settings().get("crew", {})
	var xform := Transform3D(Basis(), Vector3(10, 0, 10))
	crew.begin(1.0)
	crew.add("s0", xform, "crank", 0, "england")
	crew.finish(null)
	crew.begin(2.0)
	crew.add("s0", xform, "load", 0, "england")
	crew.finish(null)
	var layer: MultiMeshInstance3D = null
	for child in crew.get_children():
		if child.name.contains("load"):
			layer = child
	_check(layer != null, "couche du geste « load »")
	if layer != null:
		var buf := layer.multimesh.buffer
		var y := buf[13]
		if off:
			_check(y < 32.0, "--no-nt10 : servant sans fondu")
		else:
			var d := _decode(y)
			var rig := BattleSkinned.rig(str(crew.cfg.get("figure_kind", "crew")), 0)
			_check(int(d["prev"]) == BattleSkinned.clip_index(rig, "crank"), "servant : geste précédent crank (%d)" % d["prev"])
			_check(is_equal_approx(buf[14], 1.0), "servant : début du geste précédent (%.2f)" % buf[14])
			_check(int((layer.material_override as ShaderMaterial).get_shader_parameter("custom_fade")) == 2, "servants : custom_fade = 2")
	crew.free()


func _check_impostors(off: bool) -> void:
	var ids := BattleImpostors.bake_ids(3, 0.7)
	_check(ids.size() == 3, "trois identifiants")
	var wearing := 0
	for id in ids:
		var hx := BattleSkinned._hash1(float(id) * 1.37 + 0.11)
		var hz := BattleSkinned._hash1(float(id) * 0.73 + 1.9)
		if fposmod(hx * 7.31 + hz * 3.17, 1.0) < 0.7:
			wearing += 1
	_check(wearing == 2, "livrée : 2 copies sur 3 (%d)" % wearing)
	_check(ids[0] != ids[1] and ids[1] != ids[2] and ids[0] != ids[2], "identifiants distincts")
	var nobles := BattleImpostors.bake_ids(2, 0.95)
	for id in nobles:
		var hx := BattleSkinned._hash1(float(id) * 1.37 + 0.11)
		var hz := BattleSkinned._hash1(float(id) * 0.73 + 1.9)
		_check(fposmod(hx * 7.31 + hz * 3.17, 1.0) < 0.95, "nobles : livrée sur toutes les copies")
	_check(BattleImpostors.nt10_enabled() == not off, "interrupteur --no-nt10")


func _check_grass() -> void:
	var grass := BattleGrassFlatten.new()
	grass.setup(Vector2(1200.0, 800.0))
	_check(grass.RECT.position.x <= -BattleGrassFlatten.MARGIN + 0.01 and grass.RECT.end.x >= 1200.0 + BattleGrassFlatten.MARGIN - 0.01, "rectangle débordant sur les flancs")
	grass.on_corpse(Vector3(-50.0, 0.0, 400.0), "infantry", 0.8)
	grass.on_corpse(Vector3(1250.0, 0.0, 850.0), "cavalry", 0.0)
	grass.flush()
	_check(grass.flatten_at(-50.0, 400.0) > 0.5, "corps hors du champ (ouest) : herbe couchée (%.2f)" % grass.flatten_at(-50.0, 400.0))
	_check(grass.blood_at(-50.0, 400.0) > 0.2, "corps hors du champ : sang")
	_check(grass.flatten_at(1250.0, 850.0) > 0.5, "corps hors du champ (est) : herbe couchée")
	_check(grass.flatten_at(600.0, 400.0) < 0.01, "champ intact ailleurs")
	var mat := load("res://shaders/battle_grass.gdshader") as Shader
	_check(mat.code.contains("inside.x * inside.y"), "herbe : masque hors rectangle")
