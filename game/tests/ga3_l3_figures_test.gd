extends SceneTree

## Lots GA3-L3a/L3b : figurines générées à la place des figurines fines (longbowman `archer_0`,
## homme d'armes `infantry_0`, arbalétrier génois `archer_2`, sergent `infantry_1`, milicien
## `infantry_5`).
## Vérifie : manifeste `assets/models/battle_ga3/` fusionné (LOD, albédo, atlas FG3 retiré), les
## trois LOD chargent sous les plafonds des fantassins fins (11 900 / 1 350 / 260) et décroissent,
## os < nombre d'os du rig et poids normalisés, UV du corps dans [0, 1] et de l'équipement en
## u < 0, faces de livrée présentes (buste pour les armoiries), matériau en variante `GA3_TEX`
## (albédo et luminances posés, cadavres compris) ; avec `--no-ga3-fig`, la figurine fine
## d'origine ; les autres figurines (`archer_1`, `infantry_2`) ne changent pas. Lancer les deux
## modes.
## Usage : godot --headless --path game --script res://tests/ga3_l3_figures_test.gd [-- --no-ga3-fig]

const CAPS := [11900, 1350, 260]
const C_LIVERY := 0
## Figurines générées (famille, variante) et figurines témoins restées fines.
const GENERATED := [["archer", 0], ["infantry", 0], ["archer", 2], ["infantry", 1], ["infantry", 5]]
const UNTOUCHED := [["archer", 1], ["infantry", 2]]


func _check_generated(kind: String, variant: int) -> bool:
	var ok := true
	var name := "%s_%d" % [kind, variant]
	var fig := BattleSkinned.figure(kind, variant)
	if not fig.has("ga3_albedo") or fig.has("atlas_layer"):
		push_error("ga3_l3: %s non remplacée (%s)" % [name, fig.keys()])
		return false
	var bones := (BattleSkinned.rig(kind, variant).get("bones", []) as Array).size()
	var previous := 1 << 30
	for level in 3:
		var path := str((fig["lods"] as Array)[level])
		if not path.begins_with(BattleSkinned.GA3_DIR):
			push_error("ga3_l3: %s LOD%d hors de battle_ga3 : %s" % [name, level, path])
			ok = false
		var mesh := BattleSkinned.mesh(kind, variant, level)
		if mesh == null:
			push_error("ga3_l3: %s LOD%d illisible" % [name, level])
			ok = false
			continue
		var arrays := mesh.surface_get_arrays(0)
		var tris := (arrays[Mesh.ARRAY_INDEX] as PackedInt32Array).size() / 3
		if tris > CAPS[level] or tris >= previous:
			push_error("ga3_l3: %s LOD%d %d triangles (plafond %d, précédent %d)" % [name, level, tris, CAPS[level], previous])
			ok = false
		previous = tris
		var idx: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
		var w: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM1]
		var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
		var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
		var bad_bone := 0
		var bad_weight := 0
		var body := 0
		var gear := 0
		var livery := 0
		for i in uvs.size():
			var total := 0.0
			for k in 4:
				total += w[i * 4 + k]
				if int(idx[i * 4 + k] + 0.5) >= bones:
					bad_bone += 1
			if absf(total - 1.0) > 0.02:
				bad_weight += 1
			if uvs[i].x >= 0.0 and uvs[i].x <= 1.0 and uvs[i].y >= 0.0 and uvs[i].y <= 1.0:
				body += 1
			elif uvs[i].x < -1.0:
				gear += 1
			if int(colors[i].a * 16.0 + 0.5) == C_LIVERY:
				livery += 1
		if bad_bone > 0 or bad_weight > 0 or body == 0 or gear == 0 or livery == 0:
			push_error("ga3_l3: %s LOD%d os hors rig %d, poids %d, corps %d, équipement %d, livrée %d" % [name, level, bad_bone, bad_weight, body, gear, livery])
			ok = false
		print("ga3_l3: %s LOD%d %d triangles, %d sommets (corps %d, équipement %d, livrée %d)" % [name, level, tris, uvs.size(), body, gear, livery])
	if BattleSkinned.chest_box(kind, variant).is_empty():
		push_error("ga3_l3: %s sans buste en livrée (armoiries)" % name)
		ok = false
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, kind, variant)
	var tex := mat.get_shader_parameter("ga3_albedo") as Texture2D
	var lum: Vector2 = mat.get_shader_parameter("ga3_lum")
	if not mat.shader.code.contains("#define GA3_TEX") or tex == null or tex.get_width() > 1024 or lum.x <= 0.0:
		push_error("ga3_l3: %s matériau GA3 incomplet" % name)
		ok = false
	if not BattleSkinned.corpse_shader(kind, variant).code.contains("#define GA3_TEX"):
		push_error("ga3_l3: %s cadavre sans GA3_TEX" % name)
		ok = false
	return ok


func _check_fine(kind: String, variant: int) -> bool:
	var fig := BattleSkinned.figure(kind, variant)
	if fig.has("ga3_albedo") or not str((fig["lods"] as Array)[0]).begins_with(BattleSkinned.FINE_DIR):
		push_error("ga3_l3: %s_%d fine attendue (%s)" % [kind, variant, fig.get("lods")])
		return false
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, kind, variant)
	if mat.shader.code.contains("#define GA3_TEX"):
		push_error("ga3_l3: %s_%d en GA3_TEX" % [kind, variant])
		return false
	return true


func _init() -> void:
	var ok := true
	var cmd_off := OS.get_cmdline_user_args().has("--no-ga3-fig")
	if BattleSkinned.ga3_figures_enabled() == cmd_off:
		push_error("ga3_l3: option --no-ga3-fig mal lue")
		ok = false
	for f in GENERATED:
		if cmd_off:
			ok = _check_fine(f[0], f[1]) and ok
		else:
			ok = _check_generated(f[0], f[1]) and ok
	for f in UNTOUCHED:
		ok = _check_fine(f[0], f[1]) and ok
	print("ga3_l3: %s" % ("OK" if ok else "ECHEC"))
	quit(0 if ok else 1)
