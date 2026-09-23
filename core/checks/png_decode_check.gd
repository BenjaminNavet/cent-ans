extends SceneTree

## Vérification headless du décodage PNG côté Rust (GameDataStore.load_heightmap_u16,
## load_mask_u8, load_rgb8) et de get_province_owner_colors.
## Usage : godot --headless --path game --script <chemin absolu>/core/checks/png_decode_check.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures: int = 0


func _init() -> void:
	_run()
	quit(1 if _failures > 0 else 0)


func _run() -> void:
	if not ClassDB.class_exists("GameDataStore"):
		_fail("GameDataStore class not registered: GDExtension not loaded (run core/build.sh)")
		return

	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var map_dir := data_dir.path_join("map")
	var store: GameDataStore = GameDataStore.new()

	# Heightmap 16 bits.
	var t0 := Time.get_ticks_msec()
	var height := store.load_heightmap_u16(map_dir.path_join("heightmap.png"))
	var height_ms := Time.get_ticks_msec() - t0
	var size := store.get_last_image_size()
	print("heightmap: %d bytes, size %s, %d ms" % [height.size(), size, height_ms])
	_check(size.x > 0 and size.y > 0, "heightmap size should be non-zero")
	_check(height.size() == size.x * size.y * 2, "heightmap should be width*height*2 bytes")
	_check(height_ms < 500, "heightmap decode should take < 500 ms, took %d" % height_ms)
	if height.size() >= 2:
		# Little-endian : decode_u16 lit bien la valeur du premier pixel.
		var first := height.decode_u16(0)
		var middle := height.decode_u16((size.y / 2 * size.x + size.x / 2) * 2)
		print("heightmap sample: first=%d middle=%d" % [first, middle])
		_check(first != middle or first != 0, "heightmap samples should not be all zero")

	# Masque 8 bits.
	t0 = Time.get_ticks_msec()
	var mask := store.load_mask_u8(map_dir.path_join("land_mask.png"))
	print("land_mask: %d bytes, %d ms" % [mask.size(), Time.get_ticks_msec() - t0])
	var mask_size := store.get_last_image_size()
	_check(mask.size() == mask_size.x * mask_size.y, "mask should be width*height bytes")

	# Ids de province RGB8.
	t0 = Time.get_ticks_msec()
	var ids := store.load_rgb8(map_dir.path_join("province_ids.png"))
	print("province_ids: %d bytes, %d ms" % [ids.size(), Time.get_ticks_msec() - t0])
	var ids_size := store.get_last_image_size()
	_check(ids.size() == ids_size.x * ids_size.y * 3, "rgb should be width*height*3 bytes")

	# Fichier absent : tableau vide + erreur, taille remise à zéro.
	var missing := store.load_heightmap_u16(map_dir.path_join("does_not_exist.png"))
	_check(missing.is_empty(), "missing file should give an empty array")
	_check(store.get_last_image_size() == Vector2i.ZERO, "missing file should reset the image size")
	# Mauvais format : le masque 8 bits n'est pas une heightmap 16 bits.
	var wrong := store.load_heightmap_u16(map_dir.path_join("land_mask.png"))
	_check(wrong.is_empty(), "8-bit file should be refused by load_heightmap_u16")

	# Couleurs des propriétaires.
	_check(store.load(data_dir), "load(%s) should return true" % data_dir)
	var colors := store.get_province_owner_colors(PackedStringArray(["prov_normandie", "prov_atlantis"]))
	_check(colors.size() == 2, "should return one colour per requested province")
	if colors.size() == 2:
		_check(colors[0].is_equal_approx(Color("#1F3A93")), "Normandie owner colour should be France's #1F3A93, got %s" % str(colors[0]))
		_check(colors[1].is_equal_approx(Color.MAGENTA), "unknown province should give magenta")
	var all_colors := store.get_province_owner_colors(store.get_province_ids())
	_check(all_colors.size() == store.get_province_ids().size(), "one colour per province")

	if _failures == 0:
		print("png decode OK")


func _check(condition: bool, message: String) -> void:
	if not condition:
		_fail(message)


func _fail(message: String) -> void:
	_failures += 1
	push_error("png decode FAIL: " + message)
	printerr("png decode FAIL: " + message)
