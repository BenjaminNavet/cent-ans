extends SceneTree

## Lot GA5 : matières de bâtiments en 2k (données `data/art/building_materials.json`, lue par
## `BuildingMaterials`) — vérifie que les 12 matières texturées (dont `TimberFrame`, torchis/
## colombage, câblée au kit par le lot TF) chargent, mesure la mémoire VRAM ajoutée par le passage 1k → 2k
## (albédo) / 512 → 1k (normale, rugosité), format réel comme `ga2_ground_test.gd`.
## Usage : godot --headless --path game --script res://tests/ga5_building_test.gd


func _bytes_per_texel(fmt: int) -> float:
	if fmt == Image.FORMAT_BPTC_RGBA or fmt == Image.FORMAT_ASTC_4x4 or fmt == Image.FORMAT_DXT3 or fmt == Image.FORMAT_DXT5 or fmt == Image.FORMAT_RGTC_RG:
		return 1.0
	elif fmt == Image.FORMAT_DXT1 or fmt == Image.FORMAT_RGTC_R:
		return 0.5
	return 4.0


func _texture_bytes(tex: Texture2D) -> int:
	if tex == null:
		return 0
	var fmt := (tex as CompressedTexture2D).get_format() if tex is CompressedTexture2D else Image.FORMAT_RGBA8
	var per_texel := _bytes_per_texel(fmt)
	# Facteur mipmaps 4/3 (comme ga2_ground_test.gd), mipmaps activées (lot GA5).
	return int(tex.get_width() * tex.get_height() * per_texel * 4.0 / 3.0)


## Lot SR5b : les matières texturées sont des ShaderMaterial (`building_pbr.gdshader`), ou des
## StandardMaterial3D avec `--no-sr5`.
func _tex(mat: Material, param: String) -> Texture2D:
	if mat is ShaderMaterial:
		return (mat as ShaderMaterial).get_shader_parameter(param) as Texture2D
	return mat.get(param) as Texture2D


func _init() -> void:
	var ok := true
	var names := BuildingMaterials.SPECS.keys()
	# _ensure_data() est privé ; le premier appel à material() charge les données (cf.
	# `BuildingMaterials.material()`), donc on force ce chargement avant de lire SPECS.keys().
	BuildingMaterials.material("Plaster")
	names = BuildingMaterials.SPECS.keys()
	print("GA5 matières texturées : %d" % names.size())
	if names.size() != 12:
		print("GA5: 12 matières texturées attendues (11 historiques + TimberFrame), trouvé %d" % names.size())
		ok = false
	if not ("TimberFrame" in names):
		print("GA5: TimberFrame absente des données")
		ok = false
	# Lot TF : `TimberFrame` câblée, dernière couche de l'atlas (indice 14), texturée malgré sa
	# place après les couches unies (détail : `tf_timber_frame_test.gd`).
	if BuildingMaterials.ATLAS_LAYERS.find("TimberFrame") != 14:
		print("GA5: TimberFrame attendue en couche 14 de l'atlas (lot TF), trouvé %d" % BuildingMaterials.ATLAS_LAYERS.find("TimberFrame"))
		ok = false

	var total := 0
	for name in names:
		var mat := BuildingMaterials.material(name)
		if mat == null:
			print("GA5: matériau %s introuvable" % name)
			ok = false
			continue
		var albedo_tex := _tex(mat, "albedo_texture")
		var normal_tex := _tex(mat, "normal_texture")
		var rough_tex := _tex(mat, "roughness_texture")
		if albedo_tex == null:
			print("GA5: %s sans albédo_texture" % name)
			ok = false
			continue
		var diff_bytes := _texture_bytes(albedo_tex)
		var nor_bytes := _texture_bytes(normal_tex)
		var rough_bytes := _texture_bytes(rough_tex)
		total += diff_bytes + nor_bytes + rough_bytes
		print(
			"GA5 %s: albedo %dx%d (%.2f Mo), normal %s, rough %s" % [
				name,
				albedo_tex.get_width(),
				albedo_tex.get_height(),
				diff_bytes / 1048576.0,
				("%.2f Mo" % (nor_bytes / 1048576.0)) if normal_tex else "absente",
				("%.2f Mo" % (rough_bytes / 1048576.0)) if rough_tex else "absente",
			]
		)

	print("GA5 mémoire totale des matières de bâtiments : %.2f Mo" % (total / 1048576.0))
	if total > 60 * 1048576:
		print("GA5: mémoire des bâtiments > 60 Mo (repli 1k à envisager, cf. docs/wip/ga.md)")
		ok = false

	print("GA5 OK" if ok else "GA5 ECHEC")
	quit(0 if ok else 1)
