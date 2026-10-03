extends SceneTree

## Lot NT13 : clips tirés des vidéos du joueur (`video_trial/`). Sans option, le rig fin `human`
## est celui de la cuisson (aucune texture ajoutée, clips à leurs images d'origine). Avec
## `--video-trial` (ici forcé par `video_trial_forced`, puis relu par `reload_caches()`), les clips du
## manifeste `video_trial/manifest.json` sont substitués : mêmes noms (mêmes indices de clip),
## images placées après celles du rig, texture d'os concaténée. L'essai vidéo passe avant
## l'essai CMU (NT12) si les deux sont demandés. Kit grossier : aucun essai.
## Usage : godot --headless --path game --script res://tests/nt13_video_test.gd [-- --video-trial]

const RIG := "fine_human"
const EXPECTED := ["guard", "overhead", "parry", "slash", "thrust"]  # NT14 : + parry
const UNCHANGED := ["idle", "walk", "hit", "death", "victory", "pike_thrust"]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("NT13 FAIL: ", what)


func _json(path: String) -> Dictionary:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _init() -> void:
	# NT14 : les clips de mêlée par défaut (`melee/`) sont écartés, l'essai est comparé à la cuisson.
	BattleSkinned.melee_forced = 0
	BattleSkinned.fa_anim_forced = 0  # FA3 : la couche `--fa-anim` se poserait par-dessus
	BattleSkinned.reload_caches()
	if not BattleSkinned.fine_enabled():
		_check(not BattleSkinned.video_trial_enabled(), "essai inactif sur le kit grossier")
		print("NT13 video (coarse): %s" % ("OK" if ok else "FAIL"))
		quit(0 if ok else 1)
		return
	var base: Dictionary = _json(BattleSkinned.FINE_DIR + "manifest.json").get("rigs", {}).get("human", {})
	var trial := _json(BattleSkinned.VIDEO_TRIAL_DIR + "manifest.json")
	_check(not base.is_empty(), "manifeste fin lisible")
	_check(not trial.is_empty(), "manifeste de l'essai vidéo lisible")
	_check(trial.get("bones", []) == base.get("bones", []), "mêmes os que le rig fin")
	var trial_clips: Dictionary = trial.get("clips", {})
	for c in EXPECTED:
		_check(trial_clips.has(c), "clip %s cuit par l'essai vidéo" % c)
	var base_frames: int = BattleSkinned._texture_frames(BattleSkinned.FINE_DIR + str(base.get("texture", "")))
	var trial_frames: int = BattleSkinned._texture_frames(BattleSkinned.VIDEO_TRIAL_DIR + str(trial.get("texture", "")))
	_check(base_frames > 0 and trial_frames > 0, "en-têtes CAB1 lisibles")
	var cmd_on := OS.get_cmdline_user_args().has("--video-trial")
	_check(BattleSkinned.video_trial_enabled() == cmd_on, "option lue sur la ligne de commande")
	for forced in [0, 1]:
		BattleSkinned.video_trial_forced = forced
		BattleSkinned.reload_caches()
		_check(BattleSkinned.video_trial_enabled() == (forced == 1), "forçage %d" % forced)
		_check_rig(base, trial, base_frames, trial_frames, forced == 1)
	# Les deux essais demandés : la vidéo l'emporte.
	BattleSkinned.video_trial_forced = 1
	BattleSkinned.mocap_trial_forced = 1
	BattleSkinned.reload_caches()
	var both: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(str(both.get("mocap_trial_dir", "")) == BattleSkinned.VIDEO_TRIAL_DIR, "vidéo prioritaire sur CMU")
	BattleSkinned.video_trial_forced = -1
	BattleSkinned.mocap_trial_forced = -1
	BattleSkinned.melee_forced = -1
	BattleSkinned.reload_caches()
	print("NT13 video (cmd=%s): %s" % [cmd_on, "OK" if ok else "FAIL"])
	quit(0 if ok else 1)


func _check_rig(base: Dictionary, trial: Dictionary, base_frames: int, trial_frames: int, on: bool) -> void:
	var entry: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(not entry.is_empty(), "rig %s présent" % RIG)
	var clips: Dictionary = entry.get("clips", {})
	var base_clips: Dictionary = base.get("clips", {})
	_check(clips.keys().size() == base_clips.keys().size(), "même nombre de clips")
	for c in EXPECTED:
		_check(BattleSkinned.clip_index(entry, c) == BattleSkinned.clip_index(base, c), "indice du clip %s inchangé" % c)
	for c in UNCHANGED:
		_check(clips.get(c, {}) == base_clips.get(c, {}), "%s inchangé" % c)
	var tex := BattleSkinned.bone_texture(RIG)
	_check(tex != null, "texture d'os chargée")
	if on:
		_check(str(entry.get("mocap_trial_dir", "")) == BattleSkinned.VIDEO_TRIAL_DIR, "texture de l'essai vidéo")
		_check((entry.get("mocap_clips", []) as Array).size() == EXPECTED.size(), "%d clips substitués" % EXPECTED.size())
		for c in EXPECTED:
			var got: Dictionary = clips.get(c, {})
			var want: Dictionary = trial["clips"][c]
			_check(int(got.get("start", -1)) == base_frames + int(want["start"]), "%s repointé après le rig" % c)
			_check(int(got.get("frames", -1)) == int(want["frames"]), "%s : longueur de l'essai" % c)
			_check(bool(got.get("loop", false)) == bool(want["loop"]), "%s : boucle de l'essai" % c)
		if tex != null:
			_check(tex.get_height() == base_frames + trial_frames, "texture concaténée (%d)" % tex.get_height())
			_check(tex.get_height() <= 16384, "hauteur de texture dans la limite")
	else:
		_check(not entry.has("mocap_texture"), "pas de texture ajoutée sans l'option")
		for c in EXPECTED:
			_check(clips.get(c, {}) == base_clips.get(c, {}), "%s : clip d'origine sans l'option" % c)
		if tex != null:
			_check(tex.get_height() == base_frames, "texture du rig seule (%d)" % tex.get_height())
