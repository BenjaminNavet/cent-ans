extends SceneTree

## Lot AS8a (ADR 0189) : clips humains tirés d'une vidéo libre (`vf/`), cuits mais non branchés.
## Vérifie que le manifeste et la texture d'os existent, que les clips `vf_` sont présents avec
## leurs mesures et leur source, qu'ils partagent les os du rig fin et qu'aucun n'est actif par
## défaut. Usage : godot --headless --path game --script res://tests/as8a_test.gd

const DIR := "res://assets/models/battle_fine/vf/"
const EXPECTED := ["vf_thrust", "vf_guard", "vf_strike"]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("AS8a FAIL: ", what)


func _json(path: String) -> Dictionary:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _init() -> void:
	var vf := _json(DIR + "manifest.json")
	_check(not vf.is_empty(), "manifeste vf lisible")
	var base: Dictionary = _json(BattleSkinned.FINE_DIR + "manifest.json").get("rigs", {}).get("human", {})
	_check(vf.get("bones", []) == base.get("bones", []), "mêmes os que le rig fin")
	var clips: Dictionary = vf.get("clips", {})
	var sources: Dictionary = vf.get("clip_sources", {})
	_check(clips.size() == EXPECTED.size(), "%d clips cuits" % EXPECTED.size())
	for c in EXPECTED:
		_check(clips.has(c), "clip %s cuit" % c)
		_check(int((clips.get(c, {}) as Dictionary).get("frames", 0)) > 5, "%s : longueur" % c)
		var src: Dictionary = sources.get(c, {})
		_check(str(src.get("source", "")).contains("CC BY-SA"), "%s : source et licence" % c)
		_check(src.has("quality"), "%s : mesures" % c)
	_check(FileAccess.file_exists(DIR + "SOURCE.md"), "SOURCE.md présent")
	var frames: int = BattleSkinned._texture_frames(DIR + str(vf.get("texture", "")))
	_check(frames > 0, "en-tête CAB1 lisible (%d images)" % frames)
	if BattleSkinned.fine_enabled():
		BattleSkinned.reload_caches()
		var entry: Dictionary = BattleSkinned.manifest().get("rigs", {}).get("fine_human", {})
		for c in EXPECTED:
			_check(not (entry.get("clips", {}) as Dictionary).has(c), "%s non branché par défaut" % c)
	print("AS8a vf clips: %s" % ("OK" if ok else "FAIL"))
	quit(0 if ok else 1)
