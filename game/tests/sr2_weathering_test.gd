extends SceneTree

## Lot SR2 : usure des figurines fines cuites (GA3-L4 : et des figurines générées, `GA3_TEX`). Vérifie que la variante `FG3_BAKED` se compile
## (liste d'uniformes lue par le compilateur de shaders) et expose `weathering` (défaut 0,85),
## que BattleSkinned le pose à 0,6 (0 avec `--no-sr2`), la hauteur de boue des cavaliers, et
## qu'avec `--coarse-figures` le shader par défaut reste inchangé (sans uniforme SR2).
## Usage : godot --headless --path game --script res://tests/sr2_weathering_test.gd
##         [-- --no-sr2 | --coarse-figures]


## Défaut d'un uniforme float : présent dans la liste compilée (le rendu factice compile tout le
## shader, une erreur la vide), valeur lue dans le code (le rendu factice ne la garde pas).
func _uniform_default(shader: Shader, uniform_name: String) -> Variant:
	for u in shader.get_shader_uniform_list():
		if str(u["name"]) == uniform_name:
			var re := RegEx.create_from_string("uniform float %s = ([0-9.]+);" % uniform_name)
			var found := re.search(shader.code)
			return float(found.get_string(1)) if found else null
	return null


## Matériau de `kind`/`variant` : variante compilée (FG3_BAKED, ou GA3_TEX pour une figurine
## générée) qui expose `weathering` (défaut 0,85) posé à `expected`, et la hauteur de boue.
func _check(kind: String, variant: int, expected: float, generated: bool) -> bool:
	var ok := true
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, kind, variant)
	var define := "#define GA3_TEX" if generated else "#define FG3_BAKED"
	if mat.shader == BattleSkinned.SHADER or not mat.shader.code.contains(define):
		print("SR2 %s_%d : pas de variante %s" % [kind, variant, define])
		return false
	var default_value = _uniform_default(mat.shader, "weathering")
	if default_value == null or not is_equal_approx(float(default_value), 0.85):
		print("SR2 %s_%d : défaut du shader %s (0,85 attendu ; variante non compilée ?)" % [kind, variant, default_value])
		ok = false
	var value = mat.get_shader_parameter("weathering")
	if value == null or not is_equal_approx(float(value), expected):
		print("SR2 %s_%d : weathering %s, attendu %s" % [kind, variant, value, expected])
		ok = false
	var mud = mat.get_shader_parameter("sr2_mud_height")
	var mud_expected := (
		BattleSkinned.SR2_MUD_HEIGHT_HORSE if kind == "cavalry" else BattleSkinned.SR2_MUD_HEIGHT
	)
	if mud == null or not is_equal_approx(float(mud), mud_expected):
		print("SR2 %s_%d : sr2_mud_height %s, attendu %s" % [kind, variant, mud, mud_expected])
		ok = false
	return ok


func _init() -> void:
	var ok := true
	var args := OS.get_cmdline_user_args()
	var expected := 0.0 if args.has("--no-sr2") else 0.85
	# Shader par défaut : aucune ligne SR2 compilée (rendu des figurines grossières inchangé).
	if BattleSkinned.SHADER.get_shader_uniform_list().is_empty():
		print("SR2 : le shader par défaut ne compile pas")
		ok = false
	if _uniform_default(BattleSkinned.SHADER, "weathering") != null:
		print("SR2 : le shader par défaut expose weathering")
		ok = false
	if not BattleSkinned.fine_enabled():
		var mat := ShaderMaterial.new()
		mat.shader = BattleSkinned.SHADER
		BattleSkinned.setup_material(mat, "infantry", 0)
		if mat.shader != BattleSkinned.SHADER or mat.get_shader_parameter("weathering") != null:
			print("SR2 : --coarse-figures a changé le matériau par défaut")
			ok = false
		print("SR2_WEATHERING %s" % ("OK" if ok else "FAIL"))
		quit(0 if ok else 1)
		return
	var checked := 0
	var generated := 0
	for kind in ["infantry", "cavalry", "archer"]:
		# GA3-L4 : les figurines générées (`archer_0`, `infantry_0`…) ont aussi l'usure, dans
		# la variante `GA3_TEX` ; on vérifie chacune, puis la première figurine fine.
		var variant := 0
		while BattleSkinned.figure(kind, variant).has("ga3_albedo"):
			if not _check(kind, variant, expected, true):
				ok = false
			generated += 1
			variant += 1
		# Toutes les recettes d'une famille peuvent être générées (L4 : fantassins).
		if BattleSkinned.figure(kind, variant).is_empty():
			continue
		if not _check(kind, variant, expected, false):
			ok = false
		checked += 1
	print("SR2 figurines générées vérifiées : %d" % generated)
	print("SR2 matériaux vérifiés : %d (weathering %.2f)" % [checked, expected])
	print("SR2_WEATHERING %s" % ("OK" if ok else "FAIL"))
	quit(0 if ok else 1)
