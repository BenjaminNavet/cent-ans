extends TestCase

## Test headless RX `mapa` : les matériaux des modèles de la carte de campagne (champs `dn_fields.json`,
## lieux `dn_campaign_models.json`) ont tous une texture d'albédo non nulle, et le gain d'albédo est
## linéaire (un gain lu en sRGB faisait des champs et des maquettes blancs, surexposés).
## Les modèles dn/ sont hors dépôt : sans eux, la partie « texture » est ignorée avec un message.
## Usage : godot --headless --path game --script res://tests/rx_mapa_textures_test.gd

const MAX_EXPOSED_ALBEDO := 0.55  # albédo linéaire moyen après gain ; au-delà : blanc surexposé


func _init() -> void:
	await process_frame
	_run()


func _run() -> void:
	# Le gain est linéaire : sRGB -> linéaire redonne le gain demandé.
	for gain in [1.0, 2.0, 2.4]:
		var back := DnCampaignModels.gain_color(gain).srgb_to_linear().r
		check(absf(back - gain) < 0.01, "gain %s is not linear (got %s)" % [gain, back])
	var models := _collect_models()
	check(models.size() > 0, "no model referenced by the map configs")
	var checked := 0
	for entry: Dictionary in models:
		var mesh := OutbuildingLayer._load_glb_mesh(entry["file"]) as Mesh
		if mesh == null:
			continue  # paquet dn/ non installé
		checked += 1
		DnCampaignModels.brighten(mesh, {"albedo_gain": entry["gain"]})
		for surface in mesh.get_surface_count():
			var material := mesh.surface_get_material(surface) as BaseMaterial3D
			if not check(material != null, "%s: surface %d has no material" % [entry["file"], surface]):
				continue
			var texture := material.albedo_texture
			if not check(texture != null and texture.get_width() > 0, "%s: missing/null albedo texture" % entry["file"]):
				continue
			var linear_mean := _mean_linear(texture) * material.albedo_color.srgb_to_linear().r
			check(linear_mean <= MAX_EXPOSED_ALBEDO, "%s: albedo %.2f overexposed (white)" % [entry["file"], linear_mean])
	print("rx_mapa_textures: %d/%d models checked" % [checked, models.size()])
	finish()


func _mean_linear(texture: Texture2D) -> float:
	var image := texture.get_image()
	if image == null:
		return 0.0
	if image.is_compressed():
		image.decompress()
	image.resize(16, 16)
	var total := 0.0
	for x in 16:
		for y in 16:
			var c := image.get_pixel(x, y).srgb_to_linear()
			total += (c.r + c.g + c.b) / 3.0
	return total / 256.0


func _collect_models() -> Array:
	var out: Array = []
	var data_dir := OutbuildingLayer.data_dir()
	var fields := FieldLayer._read_json(data_dir.path_join(FieldLayer.CONFIG_FILE))
	var field_gain := float((fields.get("render", {}) as Dictionary).get("albedo_gain", 2.2))
	for material: String in fields.get("crops", {}):
		for variant: Dictionary in fields["crops"][material].get("variants", []):
			for level in 3:
				out.append({"file": "%s%s_lod%d.glb" % [FieldLayer.MODEL_ROOT, variant["id"], level], "gain": field_gain})
	var doc := DnCampaignModels.document()
	var default_gain := DnCampaignModels.default_value("albedo_gain", 1.0)
	var paths: Array = []
	_find_paths(doc.get("table", {}), paths)
	for entry: Dictionary in paths:
		var name := DnCampaignModels.model_name(entry)
		out.append({"file": "res://assets/models/%s.glb" % name, "gain": float(entry.get("albedo_gain", default_gain))})
	return out


func _find_paths(node: Variant, out: Array) -> void:
	if node is Dictionary:
		if (node as Dictionary).has("path"):
			out.append(node)
		for key in node:
			_find_paths(node[key], out)
	elif node is Array:
		for item in node:
			_find_paths(item, out)
