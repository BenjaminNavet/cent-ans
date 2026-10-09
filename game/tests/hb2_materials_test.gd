extends TestCase

## Lot HB2 (ADR 0143) : tableaux de matières de sol chargés (albédo, normale + rugosité,
## compressés en VRAM), nombre de couches = manifeste, correspondance id → couche complète.
## Usage : godot --headless --path game --script res://tests/hb2_materials_test.gd


func _init() -> void:
	var data := GroundMaterials.manifest()
	if data.is_empty():
		print("HB2: manifeste absent")
		failures += 1
		finish()
		return
	var arrays := GroundMaterials.load_arrays()
	if arrays.is_empty():
		print("HB2: tableaux indisponibles")
		failures += 1
		finish()
		return
	var count: int = (data["layers"] as Array).size()
	for key in ["albedo", "normal"]:
		var size: int = int(data["layer_size"] if key == "albedo" else data.get("normal_size", data["layer_size"]))
		var arr := arrays[key] as TextureLayered
		print("HB2 %s : %d couches %dx%d format %d" % [key, arr.get_layers(), arr.get_width(), arr.get_height(), arr.get_format()])
		if arr.get_layers() != count or arr.get_width() != size or arr.get_height() != size:
			check(false, "HB2: %s de forme inattendue" % key)
		if not arr is CompressedTexture2DArray:
			check(false, "HB2: %s non compressé" % key)
	var layers := arrays["layers"] as Dictionary
	if layers.size() != count:
		check(false, "HB2: correspondance id → couche incomplète (%d / %d)" % [layers.size(), count])
	var seen := {}
	for id in layers:
		var layer := int(layers[id])
		if layer < 0 or layer >= count or seen.has(layer):
			check(false, "HB2: couche invalide pour %s : %d" % [id, layer])
		seen[layer] = true
	if (arrays["means"] as PackedVector3Array).size() != count:
		failures += 1
	finish()
