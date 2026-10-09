extends TestCase

## Lot SC bt6 : le manifeste skinné fusionné est cuit hors ligne
## (`tools/cent_ans_tools/bake_skinned_manifest.py` -> `battle_skinned/manifest_merged.json`) et
## chargé tel quel. Vérifie les couches de clips (mêlée NT14, clips FA3 par défaut) contre leurs
## sources, la texture d'os concaténée et les figurines générées GA3.

const RIG := "fine_human"


func _json(path: String) -> Dictionary:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _frames(manifest: Dictionary, dir: String) -> int:
	return BattleSkinned._texture_frames(dir + str(manifest.get("texture", "")))


func _init() -> void:
	var baked := _json(BattleSkinned.DIR + BattleSkinned.MERGED_FILE)
	check(not baked.is_empty(), "manifeste cuit lisible")
	check(BattleSkinned.manifest().hash() == baked.hash(), "manifeste chargé = manifeste cuit")
	var base: Dictionary = _json(BattleSkinned.FINE_DIR + "manifest.json").get("rigs", {}).get("human", {})
	var melee := _json(BattleSkinned.FINE_DIR + "melee/manifest.json")
	var fa := _json(BattleSkinned.FINE_DIR + "fa3_anim/manifest.json")
	var base_frames := _frames(base, BattleSkinned.FINE_DIR)
	var melee_frames := _frames(melee, BattleSkinned.FINE_DIR + "melee/")
	var fa_frames := _frames(fa, BattleSkinned.FINE_DIR + "fa3_anim/")
	check(base_frames > 0 and melee_frames > 0 and fa_frames > 0, "en-têtes CAB1 lisibles")
	var entry: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	var clips: Dictionary = entry.get("clips", {})
	var base_clips: Dictionary = base.get("clips", {})
	check(clips.keys().size() == base_clips.keys().size(), "même nombre de clips que le rig")
	check(clips.keys().size() <= BattleSkinned.MAX_CLIPS, "table de clips du shader assez grande")
	var layers: Array = entry.get("mocap_textures", [])
	check(layers.size() == 2, "couches mêlée + FA3")
	var fa_default: Array = []
	for c in fa.get("clips", {}):
		if bool(fa["clips"][c].get("default", false)) and base_clips.has(c):
			fa_default.append(c)
	for c in clips:
		check(BattleSkinned.clip_index(entry, c) == BattleSkinned.clip_index(base, c), "indice du clip %s inchangé" % c)
		check(not (clips[c] as Dictionary).has("default"), "%s : pas de champ default" % c)
		var got: Dictionary = clips[c]
		if fa_default.has(c):
			check(int(got["start"]) == base_frames + melee_frames + int(fa["clips"][c]["start"]), "%s : couche FA3" % c)
		elif melee.get("clips", {}).has(c):
			check(int(got["start"]) == base_frames + int(melee["clips"][c]["start"]), "%s : couche mêlée" % c)
		else:
			check(got == base_clips[c], "%s : clip du rig" % c)
	check(not fa_default.is_empty(), "des clips FA3 par défaut")
	var tex := BattleSkinned.bone_texture(RIG)
	check(tex != null and tex.get_height() == base_frames + melee_frames + fa_frames, "texture d’os concaténée")
	var generated: Dictionary = _json(BattleSkinned.GA3_DIR + "manifest.json").get("figures", {})
	var figures: Dictionary = BattleSkinned.manifest().get("figures", {})
	for f in generated:
		check(figures.has(f) and figures[f].has("ga3_albedo"), "figurine générée %s" % f)
	finish()
