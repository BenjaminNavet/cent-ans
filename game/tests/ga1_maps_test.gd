extends SceneTree

## Lot GA1 (ADR 0104) : matières générées des figurines fines. Vérifie que les tableaux
## `fine_detail_ga1.png` (RG/B/A) et `fine_detail_albedo.png` se chargent compressés, une
## couche par matière de `data/art/materials.yaml`, que l'ajout mémoire par rapport aux tuiles
## FG3 d'origine reste <= 4 Mo et que les figurines cuites reçoivent `ga1_detail`. Avec
## `--no-ga1` : tuiles FG3 d'origine, sans albédo de détail.
## Usage : godot --headless --path game --script res://tests/ga1_maps_test.gd [-- --no-ga1]

const MATERIAL_LAYERS := 12
const FG3_LAYERS := 8
const FG3_TILE := 512
const MAX_ADDED_BYTES := 4 * 1048576


## Octets VRAM d'un tableau BC7/ASTC 4x4 (1 octet par texel, mipmaps = x4/3).
func _bytes(arr: TextureLayered) -> int:
	return int(arr.get_width() * arr.get_height() * arr.get_layers() * 4.0 / 3.0)


func _compressed(arr: TextureLayered) -> bool:
	var fmt := arr.get_format()
	return fmt == Image.FORMAT_BPTC_RGBA or fmt == Image.FORMAT_ASTC_4x4


func _init() -> void:
	var ok := true
	var no_ga1 := OS.get_cmdline_user_args().has("--no-ga1")
	var maps := BattleSkinned.fine_maps()
	var detail = maps.get("detail")
	if not detail is TextureLayered:
		print("GA1 detail absent")
		quit(1)
		return
	var arr := detail as TextureLayered
	var albedo = maps.get("detail_albedo")
	print("GA1 detail : %dx%d x%d, %.2f Mo" % [arr.get_width(), arr.get_height(), arr.get_layers(), _bytes(arr) / 1048576.0])
	if not _compressed(arr):
		print("GA1 detail non compressé (format %d)" % arr.get_format())
		ok = false
	if no_ga1:
		if BattleSkinned.ga1_enabled() or albedo != null or arr.get_layers() != FG3_LAYERS:
			print("GA1 --no-ga1 : tuiles FG3 attendues")
			ok = false
	else:
		if not BattleSkinned.ga1_enabled() or not albedo is TextureLayered:
			print("GA1 albédo de détail absent")
			quit(1)
			return
		var alb := albedo as TextureLayered
		print("GA1 albedo : %dx%d x%d, %.2f Mo" % [alb.get_width(), alb.get_height(), alb.get_layers(), _bytes(alb) / 1048576.0])
		if arr.get_layers() != MATERIAL_LAYERS or alb.get_layers() != MATERIAL_LAYERS:
			print("GA1 : %d couches attendues" % MATERIAL_LAYERS)
			ok = false
		if not _compressed(alb):
			print("GA1 albedo non compressé (format %d)" % alb.get_format())
			ok = false
		var fg3 := int(FG3_TILE * FG3_TILE * FG3_LAYERS * 4.0 / 3.0)
		var added := _bytes(arr) + _bytes(alb) - fg3
		print("GA1 ajout mémoire : %.2f Mo (plafond 4)" % (added / 1048576.0))
		if added > MAX_ADDED_BYTES:
			ok = false
	# Une figurine cuite reçoit le drapeau et le tableau d'albédo.
	var figures: Dictionary = BattleSkinned.manifest().get("figures", {})
	var checked := 0
	for fig_name in figures:
		if not (figures[fig_name] as Dictionary).has("atlas_layer"):
			continue
		var parts := str(fig_name).rsplit("_", true, 1)
		var mat := ShaderMaterial.new()
		mat.shader = BattleSkinned.SHADER
		BattleSkinned.setup_material(mat, parts[0], int(parts[1]))
		var flag = mat.get_shader_parameter("ga1_detail")
		if bool(flag) == no_ga1:
			print("GA1 %s : ga1_detail = %s" % [fig_name, flag])
			ok = false
		checked += 1
		break
	if checked == 0 and BattleSkinned.fine_enabled():
		print("GA1 : aucune figurine cuite")
		ok = false
	print("GA1_MAPS %s" % ("OK" if ok else "FAIL"))
	quit(0 if ok else 1)
