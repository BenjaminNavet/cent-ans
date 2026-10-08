extends SceneTree

## DN-ME4 : eaux douces (`FreshwaterLayer`) : données chargées, semis des roselières et mares sur
## la Camargue (déterminisme, lagune : roselière sur les rives et eau au centre), absence hors zone,
## salins/torrents/cascades construits, branchement d'un glb par le manifeste.
## Usage : godot --headless --path game --script res://tests/dn_freshwater_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	var map_dir := DataFile.data_dir().path_join("map")
	var data := MapData.load_from_dir(map_dir)
	var layer := FreshwaterLayer.new()
	root.add_child(layer)
	layer.setup(data, null)
	_check(layer.active(), "layer active (config + sites loaded)")
	_check(layer.sites.get("torrents", []).size() > 100, "torrent runs present")
	_check(layer.sites.get("salt_pans", []).size() >= 10, "salt pans present")
	var camargue := _site(layer, "camargue")
	_check(not camargue.is_empty(), "camargue wetland site")
	var c := Vector2(camargue["center"][0], camargue["center"][1])
	_check(FreshwaterLayer.ellipse_radius(camargue, c) < 0.01, "ellipse radius 0 at centre")
	_check(FreshwaterLayer.ellipse_radius(camargue, c + Vector2(500, 500)) > 1.0, "outside ellipse")
	var size := layer.block_units(FreshwaterLayer.LEVEL_PATCH)
	var key := Vector2i(floori(c.x / size), floori(c.y / size))
	var reeds := 0
	var pools := 0
	for dx in range(-2, 3):
		for dy in range(-2, 3):
			var seeded := layer.seed_block(FreshwaterLayer.LEVEL_PATCH, key + Vector2i(dx, dy))
			reeds += seeded["reeds"].size() / 7
			pools += seeded["pools"].size() / 7
	print("dn_freshwater: camargue patch blocks 5x5 -> %d reeds, %d pools" % [reeds, pools])
	_check(reeds > 20, "reeds seeded in Camargue (%d)" % reeds)
	_check(pools > 5, "pools seeded in Camargue (%d)" % pools)
	var again := layer.seed_block(FreshwaterLayer.LEVEL_PATCH, key)
	_check(again["reeds"] == layer.seed_block(FreshwaterLayer.LEVEL_PATCH, key)["reeds"], "deterministic seeding")
	var empty := layer.seed_block(FreshwaterLayer.LEVEL_PATCH, Vector2i(0, 0))
	_check(empty["reeds"].is_empty() and empty["pools"].is_empty(), "no reeds outside wetlands")
	# Une mare de lagune est grande (6 x), une roselière de lagune reste sur les rives.
	for i in 6:
		var seeded := layer.seed_block(FreshwaterLayer.LEVEL_PATCH, key + Vector2i(i - 3, 0))
		var flat: PackedFloat32Array = seeded["reeds"]
		for n in flat.size() / 7:
			var r := FreshwaterLayer.ellipse_radius(camargue, Vector2(flat[n * 7], flat[n * 7 + 1]))
			if r < 1.0 and r < 0.5:
				_check(false, "lagoon reed inside open water (r=%.2f)" % r)
	# Mise à jour de vue : construit des blocs sans erreur, grossissement monotone.
	layer.update_view(c, 30.0)
	_check(layer.stats["blocks"] > 0, "update_view builds blocks (%s)" % str(layer.stats))
	_check(layer.scale_for(100.0) >= layer.scale_for(10.0) and is_equal_approx(layer.scale_for(1.0), 1.0), "scale grows with distance")
	layer.update_view(c, 1500.0)
	_check(not layer._static_root.visible, "hidden under the parchment view")
	# Branchement d'un modèle par le manifeste : id catalogue -> chemin.
	layer._manifest = {"env_waterfall_rock_step": {"files": ["rocks/hb/alpine_spire_lod0.glb"]}}
	_check(layer.model_path("waterfall").ends_with("alpine_spire_lod0.glb"), "manifest id resolves to a glb")
	layer._manifest = {}
	_check(layer.model_path("waterfall") == "", "no glb yet -> procedural stand-in")
	print("dn_freshwater_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _site(layer: FreshwaterLayer, id: String) -> Dictionary:
	for site: Dictionary in layer.sites.get("wetlands", []):
		if site["id"] == id:
			return site
	return {}


func _check(ok: bool, label: String) -> void:
	if not ok:
		_failures += 1
		push_error("FAIL: " + label)
		print("FAIL: " + label)
