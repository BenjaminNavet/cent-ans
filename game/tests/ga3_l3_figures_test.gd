extends TestCase

## Lots GA3-L3a/L3b : figurines générées à la place des figurines fines (longbowman `archer_0`,
## homme d'armes `infantry_0`, arbalétrier génois `archer_2`, sergent `infantry_1`, milicien
## `infantry_5`) ; L3c : cavalier du chevalier `cavalry_0` sur le cheval fin FG4 exporté.
## Vérifie : manifeste `assets/models/battle_ga3/` fusionné (LOD, albédo, atlas FG3 retiré), les
## trois LOD chargent sous les plafonds des fantassins fins (11 900 / 1 350 / 260) et décroissent,
## os < nombre d'os du rig et poids normalisés, UV du corps dans [0, 1] et de l'équipement en
## u < 0, faces de livrée présentes (buste pour les armoiries), matériau en variante `GA3_TEX`
## (albédo et luminances posés, cadavres compris) ; les autres figurines (`archer_1`, `infantry_2`) ne changent pas. Chevalier : plafonds de la figurine montée fine (cavalier + cheval), atlas FG3 gardé
## (variantes `FG3_BAKED` + `GA3_TEX`), cavalier sur les os `R:` avec l'albédo, cheval sans
## UV d'albédo (u < 0, sauf armoiries du caparaçon).
## Usage : godot --headless --path game --script res://tests/ga3_l3_figures_test.gd

const CAPS := [11900, 1350, 260]
## Figurine montée : cavalier (`RIDER_CAP` 9 000 / 1 000 / 180, équipement compris) et cheval
## fin exporté (≈ 7 600 / 1 000 / 310).
const CAPS_MOUNTED := [16800, 2050, 620]
const C_LIVERY := 0
const C_ARMS := 6
## Figurines générées (famille, variante) et figurines témoins restées fines.
## L4 : les dix recettes restantes des mêmes types (infantry_2/3/4/6/7/8, archer_1/4, cavalry_3,
## standard_1), et au moins deux têtes (variantes) par figurine sauf `standard_1`.
const GENERATED := [
	["archer", 0], ["infantry", 0], ["archer", 2], ["infantry", 1], ["infantry", 5], ["cavalry", 0],
	["infantry", 2], ["infantry", 3], ["infantry", 4], ["infantry", 6], ["infantry", 7], ["infantry", 8],
	["archer", 1], ["archer", 4], ["cavalry", 3], ["standard", 1],
]
const SINGLE_HEAD := ["standard_1"]
const UNTOUCHED := [["archer", 3], ["cavalry", 1], ["standard", 0]]


func _check_generated(kind: String, variant: int) -> void:
	var name := "%s_%d" % [kind, variant]
	var fig := BattleSkinned.figure(kind, variant)
	var mounted := str(fig.get("rig", "")).ends_with("cavalry")
	if not fig.has("ga3_albedo") or fig.has("atlas_layer") != mounted:
		check(false, "ga3_l3: %s non remplacée (%s)" % [name, fig.keys()])
		return
	var bone_names: Array = BattleSkinned.rig(kind, variant).get("bones", [])
	var bones := bone_names.size()
	var caps: Array = CAPS_MOUNTED if mounted else CAPS
	var previous := 1 << 30
	for level in 3:
		var path := str((fig["lods"] as Array)[level])
		if not path.begins_with(BattleSkinned.GA3_DIR):
			check(false, "ga3_l3: %s LOD%d hors de battle_ga3 : %s" % [name, level, path])
		var mesh := BattleSkinned.mesh(kind, variant, level)
		if mesh == null:
			check(false, "ga3_l3: %s LOD%d illisible" % [name, level])
			continue
		var arrays := mesh.surface_get_arrays(0)
		var tris := (arrays[Mesh.ARRAY_INDEX] as PackedInt32Array).size() / 3
		if tris > caps[level] or tris >= previous:
			check(false, "ga3_l3: %s LOD%d %d triangles (plafond %d, précédent %d)" % [name, level, tris, caps[level], previous])
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
		var rider := 0
		var body_plain := 0  # corps texturé hors armoiries (caparaçon du cheval : C_ARMS)
		var horse_textured := 0
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
			var code := int(colors[i].a * 16.0 + 0.5)
			if code != C_ARMS and uvs[i].x >= 0.0 and uvs[i].x <= 1.0 and uvs[i].y >= 0.0 and uvs[i].y <= 1.0:
				body_plain += 1
			if code == C_LIVERY:
				livery += 1
			if mounted:
				var first := str(bone_names[mini(int(idx[i * 4] + 0.5), bones - 1)])
				if first.begins_with("R:"):
					rider += 1
				elif uvs[i].x >= 0.0 and code != C_ARMS:
					horse_textured += 1
		if bad_bone > 0 or bad_weight > 0 or body == 0 or gear == 0 or livery == 0:
			check(false, "ga3_l3: %s LOD%d os hors rig %d, poids %d, corps %d, équipement %d, livrée %d" % [name, level, bad_bone, bad_weight, body, gear, livery])
		if mounted and (rider < body_plain or horse_textured > 0):
			check(false, "ga3_l3: %s LOD%d cavalier %d sommets sur R: (corps %d), cheval texturé %d" % [name, level, rider, body, horse_textured])
		print("ga3_l3: %s LOD%d %d triangles, %d sommets (corps %d, équipement %d, livrée %d)" % [name, level, tris, uvs.size(), body, gear, livery])
	# L4 : têtes variantes (masques de variante, bit v) au LOD0, teint au-dessus du cou.
	var heads := int(fig.get("variants", 1))
	if heads < (1 if name in SINGLE_HEAD else 2) or float(fig.get("ga3_head_y", 99.0)) > 3.0:
		check(false, "ga3_l3: %s %d têtes, cou %s" % [name, heads, fig.get("ga3_head_y")])
	var lod0 := BattleSkinned.mesh(kind, variant, 0).surface_get_arrays(0)
	var masks: PackedVector2Array = lod0[Mesh.ARRAY_TEX_UV2]
	var seen := {}
	for m in masks:
		seen[int(m.x + 0.5) & 63] = true
	for v in heads if heads > 1 else 0:
		if not seen.has(1 << v):
			check(false, "ga3_l3: %s sans tête de variante %d" % [name, v])
	if BattleSkinned.chest_box(kind, variant).is_empty():
		check(false, "ga3_l3: %s sans buste en livrée (armoiries)" % name)
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, kind, variant)
	var tex := mat.get_shader_parameter("ga3_albedo") as Texture2D
	var lum: Vector2 = mat.get_shader_parameter("ga3_lum")
	if not mat.shader.code.contains("#define GA3_TEX") or tex == null or tex.get_width() > 1024 or tex.get_height() > 1536 or lum.x <= 0.0:
		check(false, "ga3_l3: %s matériau GA3 incomplet" % name)
	# UV en convention Blender (v vers le haut) : le shader lit la ligne 1 - v ; les sommets
	# texturés du LOD0 doivent y tomber sur l'albédo cuit, pas sur le fond noir de l'atlas.
	if tex != null and not _uvs_on_albedo(name, tex, lod0) or not mat.shader.code.contains("texture(ga3_albedo, vec2(UV.x, 1.0 - UV.y))"):
		check(false, "ga3_l3: %s UV hors de l'albédo (sens de v)" % name)
	var corpse := BattleSkinned.corpse_shader(kind, variant)
	if not corpse.code.contains("#define GA3_TEX"):
		check(false, "ga3_l3: %s cadavre sans GA3_TEX" % name)
	# L3c : le cheval fin garde ses cartes cuites (robe, caparaçon), vivant comme cadavre.
	if mounted and BattleSkinned.fine_maps_ready():
		if not mat.shader.code.contains("#define FG3_BAKED") or not corpse.code.contains("#define FG3_BAKED"):
			check(false, "ga3_l3: %s sans FG3_BAKED (cheval fin)" % name)
		if int(mat.get_shader_parameter("fine_layer")) != int(fig["atlas_layer"]):
			check(false, "ga3_l3: %s couche d'atlas non posée" % name)


## Part des sommets texturés (corps hors armoiries) lus sur un texel noir de l'albédo en
## (u, 1 - v) : ≈ 1-4 % à l'endroit, ≈ 45 % si v n'est pas retourné.
func _uvs_on_albedo(name: String, tex: Texture2D, arrays: Array) -> bool:
	var img := tex.get_image()
	if img.is_compressed():
		img.decompress()
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
	var textured := 0
	var black := 0
	for i in uvs.size():
		var uv := uvs[i]
		if int(colors[i].a * 16.0 + 0.5) == C_ARMS or uv.x < 0.0 or uv.x > 1.0 or uv.y < 0.0 or uv.y > 1.0:
			continue
		textured += 1
		var px := clampi(int(uv.x * img.get_width()), 0, img.get_width() - 1)
		var py := clampi(int((1.0 - uv.y) * img.get_height()), 0, img.get_height() - 1)
		var c := img.get_pixel(px, py)
		if c.r + c.g + c.b < 0.012:
			black += 1
	var share := float(black) / maxf(textured, 1.0)
	print("ga3_l3: %s UV sur texel noir %.1f %%" % [name, share * 100.0])
	return share < 0.1


func _check_fine(kind: String, variant: int) -> void:
	var fig := BattleSkinned.figure(kind, variant)
	if fig.has("ga3_albedo") or not str((fig["lods"] as Array)[0]).begins_with(BattleSkinned.FINE_DIR):
		check(false, "ga3_l3: %s_%d fine attendue (%s)" % [kind, variant, fig.get("lods")])
		return
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, kind, variant)
	if mat.shader.code.contains("#define GA3_TEX"):
		check(false, "ga3_l3: %s_%d en GA3_TEX" % [kind, variant])
		return


func _init() -> void:
	if not BattleSkinned.ga3_figures_enabled():
		check(false, "ga3_l3: figurines générées inactives")
	for f in GENERATED:
		_check_generated(f[0], f[1])
	for f in UNTOUCHED:
		_check_fine(f[0], f[1])
	finish()
