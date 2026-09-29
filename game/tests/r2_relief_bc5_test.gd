extends SceneTree

## Lot OMR-R2 : copie GPU du relief fin (`map.json.relief_shade.bc5`, BC5 + mipmaps) lue par
## `ReliefLandcover.load_bc5` : format RGTC RG, taille et chaîne de mipmaps complètes ; une bande de
## 256 lignes du niveau 0 décompressée est fidèle aux bandes PNG (R = L, G = A ; erreur BC4
## ≤ étendue du bloc / 14, moyenne < 1 niveau). Zones humides en BC1 (`map.json.wetlands_gpu`) :
## format DXT1, taille, bande de 256 lignes (la plus riche en marais) fidèle au PNG.
## Usage : godot --headless --path game --script res://tests/r2_relief_bc5_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
## Première ligne de pixels comparée (multiple de 4, loin du bord nord : terre et mer).
const ROW0 := 2048
const ROWS := 256
## Bande des zones humides comparée (la plus riche en texels non nuls de `wetlands.png`).
const WET_ROW0 := 2560


func _init() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var meta: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(map_dir.path_join("map.json")))
	if not (meta.get("relief_shade", {}) as Dictionary).has("bc5"):
		print("R2 BC5: pas de copie GPU dans %s (ignoré)" % map_dir)
		quit(0)
		return
	var ok := true
	var t0 := Time.get_ticks_msec()
	var image := ReliefLandcover.load_bc5(map_dir)
	var load_ms := Time.get_ticks_msec() - t0
	if image == null:
		print("R2 BC5: lecture impossible")
		quit(1)
		return
	var size: Array = meta["relief_shade"]["bc5"]["size_px"]
	if image.get_format() != Image.FORMAT_RGTC_RG or image.get_size() != Vector2i(int(size[0]), int(size[1])):
		print("R2 BC5: format %d ou taille %s inattendus" % [image.get_format(), image.get_size()])
		ok = false
	if not image.has_mipmaps() or image.get_mipmap_count() != 13:
		print("R2 BC5: chaîne de mipmaps incomplète (%d)" % image.get_mipmap_count())
		ok = false
	# Bande du niveau 0 : blocs 4 × 4 rangés par lignes de blocs (16 octets par bloc).
	var width := image.get_width()
	var start := (ROW0 / 4) * (width / 4) * 16
	var slice := image.get_data().slice(start, start + (ROWS / 4) * (width / 4) * 16)
	var strip := Image.create_from_data(width, ROWS, false, Image.FORMAT_RGTC_RG, slice)
	strip.decompress()
	strip.convert(Image.FORMAT_RG8)
	var band_rows := int(meta["relief_shade"]["bands"]["rows"])
	var band_name := str(meta["relief_shade"]["bands"]["pattern"]).replace("{band}", str(ROW0 / band_rows))
	var band := Image.load_from_file(map_dir.path_join(band_name))
	band.convert(Image.FORMAT_LA8)
	var source := band.get_data()
	var decoded := strip.get_data()
	var offset := (ROW0 % band_rows) * width * 2
	var worst := 0
	var total := 0.0
	for k in decoded.size():
		var error := absi(int(decoded[k]) - int(source[offset + k]))
		worst = maxi(worst, error)
		total += error
	var mean := total / decoded.size()
	print("R2 BC5: lecture %d ms, erreur moyenne %.3f, max %d" % [load_ms, mean, worst])
	if mean >= 1.0 or worst > 20:
		print("R2 BC5: bande trop éloignée des PNG")
		ok = false
	ok = _check_wetlands(map_dir, meta) and ok
	print("R2 BC5: %s" % ("OK" if ok else "ÉCHEC"))
	quit(0 if ok else 1)


func _check_wetlands(map_dir: String, meta: Dictionary) -> bool:
	if not meta.has("wetlands_gpu"):
		print("R2 BC1: pas de copie GPU des zones humides (ignoré)")
		return true
	var image := ReliefLandcover.load_wetlands_gpu(map_dir)
	if image == null or image.get_format() != Image.FORMAT_DXT1 or image.has_mipmaps():
		print("R2 BC1: lecture impossible ou format inattendu")
		return false
	var png := Image.load_from_file(map_dir.path_join("wetlands.png"))
	png.convert(Image.FORMAT_RGB8)
	if image.get_size() != png.get_size():
		print("R2 BC1: taille %s au lieu de %s" % [image.get_size(), png.get_size()])
		return false
	var width := image.get_width()
	var start := (WET_ROW0 / 4) * (width / 4) * 8
	var strip := Image.create_from_data(width, ROWS, false, Image.FORMAT_DXT1, image.get_data().slice(start, start + (ROWS / 4) * (width / 4) * 8))
	strip.decompress()
	strip.convert(Image.FORMAT_RGB8)
	var decoded := strip.get_data()
	var source := png.get_data()
	var offset := WET_ROW0 * width * 3
	var worst := 0
	var total := 0.0
	var marked := 0
	for k in decoded.size():
		var error := absi(int(decoded[k]) - int(source[offset + k]))
		worst = maxi(worst, error)
		total += error
		if source[offset + k] > 0:
			marked += 1
	var mean := total / decoded.size()
	print("R2 BC1: %d texels marqués, erreur moyenne %.4f, max %d" % [marked, mean, worst])
	if marked == 0 or mean >= 0.2 or worst > 48:
		print("R2 BC1: bande trop éloignée du PNG")
		return false
	return true
