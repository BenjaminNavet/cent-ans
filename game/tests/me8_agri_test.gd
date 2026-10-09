extends TestCase

## Lot ME8 (DN) : paysages agricoles régionaux. Repli des matières absentes, lignes 8+ de la
## table du sol, part prairie -> cultures, pose du masque (`hb_agri`) et variantes saisonnières
## d'arbres connues du catalogue.
## Usage : godot --headless --path game --script res://tests/me8_agri_test.gd


func _init() -> void:
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var doc := HbGround._read_json(data_dir.path_join(HbGround.AGRI_FILE))
	var arrays := GroundMaterials.load_arrays()
	if doc.is_empty() or arrays.is_empty():
		print("ME8: données absentes")
		failures += 1
		finish()
		return
	var layers: Dictionary = arrays["layers"]
	var rows := HbGround.landscape_rows(doc)
	var resolved := HbGround.resolve_landscapes(doc, layers)
	if resolved.size() != rows.size():
		check(false, "ME8: paysages résolus %d / %d" % [resolved.size(), rows.size()])
	# Repli : rice_paddy n'existe pas encore comme matière, il devient barley_green.
	var huerta: Dictionary = resolved[str(rows["irrigated_huerta"])]
	if layers.has("rice_paddy") or not (huerta["crops"] as Dictionary).has("barley_green") or (huerta["crops"] as Dictionary).has("rice_paddy"):
		check(false, "ME8: repli de matière incorrect %s" % str(huerta["crops"]))
	var mix: Variant = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join(HbGround.MIX_FILE)))
	for key in resolved:
		(mix["biomes"] as Dictionary)[key] = resolved[key]
	var table := HbGround.build_table(mix as Dictionary, layers)
	if table == null or table.get_width() != HbGround.TABLE_WIDTH or table.get_height() != HbGround.TABLE_HEIGHT:
		print("ME8: table de forme inattendue")
		failures += 1
		finish()
		return
	var south := int(rows["olive_orchard_south"])
	var north := int(rows["openfield_north"])
	if table.get_pixel(17, south).r < 0.5 or table.get_pixel(17, north).r > 0.01:
		check(false, "ME8: open_to_farm sud %.2f nord %.2f" % [table.get_pixel(17, south).r, table.get_pixel(17, north).r])
	# Cultures du sud : oliveraie en tête, cumul normalisé.
	var olive := int(layers["olive_grove"])
	if int(round(table.get_pixel(0, south).r * HbGround.LAYER_NORM)) != olive:
		check(false, "ME8: première culture du sud n'est pas l'oliveraie")
	# Pose complète sur un matériau.
	var material := ShaderMaterial.new()
	material.shader = load("res://shaders/terrain.gdshader")
	if not HbGround.apply(material, data_dir):
		check(false, "ME8: HbGround.apply a échoué")
	elif material.get_shader_parameter("hb_agri") == null or float(material.get_shader_parameter("hb_agri_texel")) != 4.0:
		check(false, "ME8: masque des paysages non posé")
	# Variantes saisonnières : modèles du catalogue.
	var catalogue: Array = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join("art/dn_catalog_map_extra.json")))
	var known := {}
	for entry in catalogue:
		known[str(entry["id"])] = true
	for variant in doc.get("seasonal_variants", []):
		if not known.has(str(variant["model_id"])):
			check(false, "ME8: modèle saisonnier inconnu %s" % variant["model_id"])
	var variants := AgriSeasons.load_variants(data_dir)
	if AgriSeasons.model_for(variants, {}, "oak", "autumn") != "":
		check(false, "ME8: modèle non ingéré rendu")
	if AgriSeasons.model_for(variants, {"env_autumn_oak_gold": {}}, "oak", "autumn") != "env_autumn_oak_gold":
		check(false, "ME8: modèle ingéré non rendu")
	if AgriSeasons.model_for(variants, {"env_autumn_oak_gold": {}}, "oak", "winter") != "":
		check(false, "ME8: mauvaise saison")
	finish()
