extends SceneTree

## Lot GA3-L3 : figurine générée (longbowman, `archer_0`) à la place de la figurine fine.
## Vérifie : manifeste `assets/models/battle_ga3/` fusionné (LOD, albédo, atlas FG3 retiré), les
## trois LOD chargent sous les plafonds des fantassins fins (11 900 / 1 350 / 260) et décroissent,
## os < nombre d'os du rig et poids normalisés, UV du corps dans [0, 1] et de l'équipement en
## u < 0, faces de livrée présentes (buste pour les armoiries), matériau en variante `GA3_TEX`
## (albédo et luminances posés, cadavres compris) ; avec `--no-ga3-fig`, la figurine fine
## d'origine ; les autres figurines ne changent pas. Lancer les deux modes.
## Usage : godot --headless --path game --script res://tests/ga3_l3_figures_test.gd [-- --no-ga3-fig]

const CAPS := [11900, 1350, 260]
const C_LIVERY := 0


func _check_generated() -> bool:
	var ok := true
	var fig := BattleSkinned.figure("archer", 0)
	if not fig.has("ga3_albedo") or fig.has("atlas_layer"):
		push_error("ga3_l3: archer_0 non remplacée (%s)" % fig.keys())
		return false
	var bones := (BattleSkinned.rig("archer", 0).get("bones", []) as Array).size()
	var previous := 1 << 30
	for level in 3:
		var path := str((fig["lods"] as Array)[level])
		if not path.begins_with(BattleSkinned.GA3_DIR):
			push_error("ga3_l3: LOD%d hors de battle_ga3 : %s" % [level, path])
			ok = false
		var mesh := BattleSkinned.mesh("archer", 0, level)
		if mesh == null:
			push_error("ga3_l3: LOD%d illisible" % level)
			ok = false
			continue
		var arrays := mesh.surface_get_arrays(0)
		var tris := (arrays[Mesh.ARRAY_INDEX] as PackedInt32Array).size() / 3
		if tris > CAPS[level] or tris >= previous:
			push_error("ga3_l3: LOD%d %d triangles (plafond %d, précédent %d)" % [level, tris, CAPS[level], previous])
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
			push_error("ga3_l3: LOD%d os hors rig %d, poids %d, corps %d, équipement %d, livrée %d" % [level, bad_bone, bad_weight, body, gear, livery])
			ok = false
		print("ga3_l3: LOD%d %d triangles, %d sommets (corps %d, équipement %d, livrée %d)" % [level, tris, uvs.size(), body, gear, livery])
	if BattleSkinned.chest_box("archer", 0).is_empty():
		push_error("ga3_l3: pas de buste en livrée (armoiries)")
		ok = false
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, "archer", 0)
	var tex := mat.get_shader_parameter("ga3_albedo") as Texture2D
	var lum: Vector2 = mat.get_shader_parameter("ga3_lum")
	if not mat.shader.code.contains("#define GA3_TEX") or tex == null or tex.get_width() > 1024 or lum.x <= 0.0:
		push_error("ga3_l3: matériau GA3 incomplet")
		ok = false
	if not BattleSkinned.corpse_shader("archer", 0).code.contains("#define GA3_TEX"):
		push_error("ga3_l3: cadavre sans GA3_TEX")
		ok = false
	return ok


func _check_fine() -> bool:
	var fig := BattleSkinned.figure("archer", 0)
	if fig.has("ga3_albedo") or not str((fig["lods"] as Array)[0]).begins_with(BattleSkinned.FINE_DIR):
		push_error("ga3_l3: --no-ga3-fig n'a pas rendu la figurine fine (%s)" % [fig.get("lods")])
		return false
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, "archer", 0)
	if mat.shader.code.contains("#define GA3_TEX"):
		push_error("ga3_l3: GA3_TEX avec --no-ga3-fig")
		return false
	return true


func _init() -> void:
	var ok := true
	var cmd_off := OS.get_cmdline_user_args().has("--no-ga3-fig")
	if BattleSkinned.ga3_figures_enabled() == cmd_off:
		push_error("ga3_l3: option --no-ga3-fig mal lue")
		ok = false
	var other := str((BattleSkinned.figure("archer", 1)["lods"] as Array)[0])
	if cmd_off:
		ok = _check_fine() and ok
	else:
		ok = _check_generated() and ok
	if str((BattleSkinned.figure("archer", 1)["lods"] as Array)[0]) != other or not other.begins_with(BattleSkinned.FINE_DIR):
		push_error("ga3_l3: archer_1 modifiée")
		ok = false
	print("ga3_l3: %s" % ("OK" if ok else "ECHEC"))
	quit(0 if ok else 1)
