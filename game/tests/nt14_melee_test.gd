extends SceneTree

## Lot NT14 (ADR 0129, complément) : clips de mêlée par défaut du rig fin `human`, la meilleure
## source par geste, cuits dans `melee/`. Par défaut ils remplacent les clips keyframés du même
## nom (mêmes indices de clip, images après celles du rig, texture d'os concaténée) ;
## `--keyframed-melee` (ici forcé par `melee_forced` = 0) rétablit la cuisson d'origine ; les
## options d'essai passent avant. Kit grossier : aucun changement.
## Usage : godot --headless --path game --script res://tests/nt14_melee_test.gd [-- --keyframed-melee]

const RIG := "fine_human"
const EXPECTED := ["guard", "overhead", "parry", "thrust"]
const KEYFRAMED := ["slash", "hit", "death", "idle", "walk", "victory", "pike_thrust"]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("NT14 FAIL: ", what)


func _json(path: String) -> Dictionary:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _init() -> void:
	var cmd_keyframed := OS.get_cmdline_user_args().has("--keyframed-melee")
	if not BattleSkinned.fine_enabled():
		_check(not BattleSkinned.melee_default_enabled(), "défaut inactif sur le kit grossier")
		print("NT14 melee (coarse): %s" % ("OK" if ok else "FAIL"))
		quit(0 if ok else 1)
		return
	_check(BattleSkinned.melee_default_enabled() == not cmd_keyframed, "défaut lu sur la ligne de commande")
	var base: Dictionary = _json(BattleSkinned.FINE_DIR + "manifest.json").get("rigs", {}).get("human", {})
	var melee := _json(BattleSkinned.MELEE_DIR + "manifest.json")
	_check(not melee.is_empty(), "manifeste melee lisible")
	_check(melee.get("bones", []) == base.get("bones", []), "mêmes os que le rig fin")
	_check((melee.get("clips", {}) as Dictionary).keys().size() == EXPECTED.size(), "%d clips cuits" % EXPECTED.size())
	for c in EXPECTED:
		_check((melee.get("clips", {}) as Dictionary).has(c), "clip %s cuit" % c)
		_check((melee.get("clip_sources", {}) as Dictionary).has(c), "source et mesures de %s" % c)
	var base_frames: int = BattleSkinned._texture_frames(BattleSkinned.FINE_DIR + str(base.get("texture", "")))
	var melee_frames: int = BattleSkinned._texture_frames(BattleSkinned.MELEE_DIR + str(melee.get("texture", "")))
	_check(base_frames > 0 and melee_frames > 0, "en-têtes CAB1 lisibles")
	for forced in [0, 1]:
		BattleSkinned.melee_forced = forced
		BattleSkinned.reload_caches()
		_check(BattleSkinned.melee_default_enabled() == (forced == 1), "forçage %d" % forced)
		_check_rig(base, melee, base_frames, melee_frames, forced == 1)
	# Une option d'essai passe avant le défaut.
	BattleSkinned.melee_forced = 1
	BattleSkinned.video_trial_forced = 1
	BattleSkinned.reload_caches()
	var both: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(str(both.get("mocap_trial_dir", "")) == BattleSkinned.VIDEO_TRIAL_DIR, "essai vidéo prioritaire")
	BattleSkinned.melee_forced = -1
	BattleSkinned.video_trial_forced = -1
	BattleSkinned.reload_caches()
	print("NT14 melee (keyframed=%s): %s" % [cmd_keyframed, "OK" if ok else "FAIL"])
	quit(0 if ok else 1)


func _check_rig(base: Dictionary, melee: Dictionary, base_frames: int, melee_frames: int, on: bool) -> void:
	var entry: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(not entry.is_empty(), "rig %s présent" % RIG)
	var clips: Dictionary = entry.get("clips", {})
	var base_clips: Dictionary = base.get("clips", {})
	_check(clips.keys().size() == base_clips.keys().size(), "même nombre de clips")
	for c in EXPECTED:
		_check(BattleSkinned.clip_index(entry, c) == BattleSkinned.clip_index(base, c), "indice du clip %s inchangé" % c)
	for c in KEYFRAMED:
		_check(clips.get(c, {}) == base_clips.get(c, {}), "%s keyframé inchangé" % c)
	var tex := BattleSkinned.bone_texture(RIG)
	_check(tex != null, "texture d'os chargée")
	if on:
		_check(str(entry.get("mocap_trial_dir", "")) == BattleSkinned.MELEE_DIR, "texture melee")
		for c in EXPECTED:
			var got: Dictionary = clips.get(c, {})
			var want: Dictionary = melee["clips"][c]
			_check(int(got.get("start", -1)) == base_frames + int(want["start"]), "%s repointé après le rig" % c)
			_check(int(got.get("frames", -1)) == int(want["frames"]), "%s : longueur cuite" % c)
			_check(bool(got.get("loop", false)) == bool(want["loop"]), "%s : boucle" % c)
		if tex != null:
			_check(tex.get_height() == base_frames + melee_frames, "texture concaténée (%d)" % tex.get_height())
	else:
		_check(not entry.has("mocap_texture"), "pas de texture ajoutée avec --keyframed-melee")
		for c in EXPECTED:
			_check(clips.get(c, {}) == base_clips.get(c, {}), "%s : clip keyframé" % c)
		if tex != null:
			_check(tex.get_height() == base_frames, "texture du rig seule (%d)" % tex.get_height())
