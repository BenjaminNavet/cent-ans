extends TestCase

## Lot NT14 (ADR 0129, complément) : clips de mêlée par défaut du rig fin `human`, la meilleure
## source par geste, cuits dans `melee/`. Par défaut ils remplacent les clips keyframés du même
## nom (mêmes indices de clip, images après celles du rig, texture d'os concaténée) ;
## `--keyframed-melee` (ici forcé par `melee_forced` = 0) rétablit la cuisson d'origine ; les
## options d'essai passent avant. Kit grossier : aucun changement.
## Usage : godot --headless --path game --script res://tests/nt14_melee_test.gd [-- --keyframed-melee]

const RIG := "fine_human"
const EXPECTED := ["guard", "overhead", "parry", "thrust"]
const KEYFRAMED := ["slash", "hit", "death", "idle", "walk", "victory", "pike_thrust"]


func _json(path: String) -> Dictionary:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _init() -> void:
	var cmd_keyframed := CmdArgs.has("--keyframed-melee")
	if not BattleSkinned.fine_enabled():
		check(not BattleSkinned.melee_default_enabled(), "défaut inactif sur le kit grossier")
		finish()
		return
	check(BattleSkinned.melee_default_enabled() == not cmd_keyframed, "défaut lu sur la ligne de commande")
	var base: Dictionary = _json(BattleSkinned.FINE_DIR + "manifest.json").get("rigs", {}).get("human", {})
	var melee := _json(BattleSkinned.MELEE_DIR + "manifest.json")
	check(not melee.is_empty(), "manifeste melee lisible")
	check(melee.get("bones", []) == base.get("bones", []), "mêmes os que le rig fin")
	check((melee.get("clips", {}) as Dictionary).keys().size() == EXPECTED.size(), "%d clips cuits" % EXPECTED.size())
	for c in EXPECTED:
		check((melee.get("clips", {}) as Dictionary).has(c), "clip %s cuit" % c)
		check((melee.get("clip_sources", {}) as Dictionary).has(c), "source et mesures de %s" % c)
	var base_frames: int = BattleSkinned._texture_frames(BattleSkinned.FINE_DIR + str(base.get("texture", "")))
	var melee_frames: int = BattleSkinned._texture_frames(BattleSkinned.MELEE_DIR + str(melee.get("texture", "")))
	check(base_frames > 0 and melee_frames > 0, "en-têtes CAB1 lisibles")
	# Options d'essai de la ligne de commande écartées : on compare le défaut à la cuisson.
	BattleSkinned.video_trial_forced = 0
	BattleSkinned.fa_anim_forced = 0  # FA3 : la couche `--fa-anim` se poserait par-dessus
	BattleSkinned.mocap_trial_forced = 0
	for forced in [0, 1]:
		BattleSkinned.melee_forced = forced
		BattleSkinned.reload_caches()
		check(BattleSkinned.melee_default_enabled() == (forced == 1), "forçage %d" % forced)
		_check_rig(base, melee, base_frames, melee_frames, forced == 1)
	# Une option d'essai passe avant le défaut.
	BattleSkinned.melee_forced = 1
	BattleSkinned.video_trial_forced = 1
	BattleSkinned.reload_caches()
	var both: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	check(str(both.get("mocap_trial_dir", "")) == BattleSkinned.VIDEO_TRIAL_DIR, "essai vidéo prioritaire")
	BattleSkinned.melee_forced = -1
	BattleSkinned.video_trial_forced = -1
	BattleSkinned.mocap_trial_forced = -1
	BattleSkinned.reload_caches()
	finish()


func _check_rig(base: Dictionary, melee: Dictionary, base_frames: int, melee_frames: int, on: bool) -> void:
	var entry: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	check(not entry.is_empty(), "rig %s présent" % RIG)
	var clips: Dictionary = entry.get("clips", {})
	var base_clips: Dictionary = base.get("clips", {})
	check(clips.keys().size() == base_clips.keys().size(), "même nombre de clips")
	for c in EXPECTED:
		check(BattleSkinned.clip_index(entry, c) == BattleSkinned.clip_index(base, c), "indice du clip %s inchangé" % c)
	for c in KEYFRAMED:
		check(clips.get(c, {}) == base_clips.get(c, {}), "%s keyframé inchangé" % c)
	var tex := BattleSkinned.bone_texture(RIG)
	check(tex != null, "texture d'os chargée")
	if on:
		check(str(entry.get("mocap_trial_dir", "")) == BattleSkinned.MELEE_DIR, "texture melee")
		for c in EXPECTED:
			var got: Dictionary = clips.get(c, {})
			var want: Dictionary = melee["clips"][c]
			check(int(got.get("start", -1)) == base_frames + int(want["start"]), "%s repointé après le rig" % c)
			check(int(got.get("frames", -1)) == int(want["frames"]), "%s : longueur cuite" % c)
			check(bool(got.get("loop", false)) == bool(want["loop"]), "%s : boucle" % c)
		if tex != null:
			check(tex.get_height() == base_frames + melee_frames, "texture concaténée (%d)" % tex.get_height())
	else:
		check(not entry.has("mocap_texture"), "pas de texture ajoutée avec --keyframed-melee")
		for c in EXPECTED:
			check(clips.get(c, {}) == base_clips.get(c, {}), "%s : clip keyframé" % c)
		if tex != null:
			check(tex.get_height() == base_frames, "texture du rig seule (%d)" % tex.get_height())
