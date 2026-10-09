extends TestCase

## Lot NT12 : essai de mocap gratuite. Sans option, le rig fin `human` est celui de la cuisson
## (aucune texture mocap, clips à leurs images d'origine). Avec `-- --mocap-trial`, les clips de
## `mocap_trial/manifest.json` sont substitués : même jeu de noms (donc mêmes indices de clip),
## images placées après celles du rig, texture d'os concaténée (hauteur = rig + essai).
## Usage : godot --headless --path game --script res://tests/nt12_mocap_test.gd [-- --mocap-trial]

const RIG := "fine_human"
const EXPECTED := ["guard", "slash", "overhead", "parry", "hit", "death"]


func _json(path: String) -> Dictionary:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _init() -> void:
	# NT14 : les clips de mêlée par défaut (`melee/`) sont écartés, l'essai est comparé à la cuisson.
	BattleSkinned.melee_forced = 0
	BattleSkinned.fa_anim_forced = 0  # FA3 : la couche `--fa-anim` se poserait par-dessus
	BattleSkinned.video_trial_forced = 0  # l'essai vidéo passerait avant l'essai CMU
	BattleSkinned.reload_caches()
	var trial_on := BattleSkinned.mocap_trial_enabled()
	if not BattleSkinned.fine_enabled():
		# Kit grossier (`--coarse-figures`) : l'essai ne s'applique pas.
		check(not trial_on, "essai inactif sur le kit grossier")
		finish()
		return
	var base: Dictionary = _json(BattleSkinned.FINE_DIR + "manifest.json").get("rigs", {}).get("human", {})
	var trial := _json(BattleSkinned.MOCAP_TRIAL_DIR + "manifest.json")
	check(not base.is_empty(), "manifeste fin lisible")
	check(not trial.is_empty(), "manifeste de l'essai lisible")
	var entry: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	check(not entry.is_empty(), "rig %s présent" % RIG)
	var clips: Dictionary = entry.get("clips", {})
	var base_clips: Dictionary = base.get("clips", {})
	# Jeu de noms inchangé : les indices de clip (tri alphabétique) et les jeux restent valides.
	check(clips.keys().size() == base_clips.keys().size(), "même nombre de clips")
	for c in EXPECTED:
		check(BattleSkinned.clip_index(entry, c) == BattleSkinned.clip_index(base, c), "indice du clip %s inchangé" % c)
		check((trial.get("clips", {}) as Dictionary).has(c), "clip %s cuit par l'essai" % c)
	var base_frames: int = BattleSkinned._texture_frames(BattleSkinned.FINE_DIR + str(base.get("texture", "")))
	var trial_frames: int = BattleSkinned._texture_frames(BattleSkinned.MOCAP_TRIAL_DIR + str(trial.get("texture", "")))
	check(base_frames > 0 and trial_frames > 0, "en-têtes CAB1 lisibles")
	var tex := BattleSkinned.bone_texture(RIG)
	check(tex != null, "texture d'os chargée")
	if trial_on:
		check(entry.has("mocap_texture"), "texture mocap déclarée")
		check((entry.get("mocap_clips", []) as Array).size() == EXPECTED.size(), "6 clips substitués")
		for c in EXPECTED:
			var got: Dictionary = clips.get(c, {})
			var want: Dictionary = trial["clips"][c]
			check(int(got.get("start", -1)) == base_frames + int(want["start"]), "%s repointé après le rig" % c)
			check(int(got.get("frames", -1)) == int(want["frames"]), "%s : longueur de l'essai" % c)
		# Clips non substitués inchangés.
		for c in ["idle", "walk", "thrust", "victory"]:
			check(clips.get(c, {}) == base_clips.get(c, {}), "%s inchangé" % c)
		if tex != null:
			check(tex.get_height() == base_frames + trial_frames, "texture concaténée (%d)" % tex.get_height())
			check(tex.get_height() <= 16384, "hauteur de texture dans la limite")
	else:
		check(not entry.has("mocap_texture"), "pas de texture mocap sans l'option")
		for c in EXPECTED:
			check(clips.get(c, {}) == base_clips.get(c, {}), "%s : clip d'origine sans l'option" % c)
		if tex != null:
			check(tex.get_height() == base_frames, "texture du rig seule (%d)" % tex.get_height())
	finish()
