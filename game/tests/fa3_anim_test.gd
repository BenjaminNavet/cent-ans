extends SceneTree

## Lot FA3 : clips CC0 (Mesh2Motion, KayKit) reciblés sur le rig fin `human`, posés par-dessus le
## défaut NT14. Trois modes : sans option, seuls les clips marqués `default` dans le manifeste
## (champ de la table `data/fx/fa3_anim_sources.json`) sont substitués ; `-- --fa-anim` : tous ;
## kit grossier : aucun (manifeste du jeu d'avant FA3). Mêmes noms (donc mêmes indices de
## clip), images placées après celles du rig et de la couche NT14, texture d'os concaténée ; un
## clip non couvert (`thrust`) garde sa couche. Chaque clip substitué est joué : ses lignes de
## texture existent, sont finies et diffèrent du clip remplacé.
## Usage : godot --headless --path game --script res://tests/fa3_anim_test.gd [-- --fa-anim]

const RIG := "fine_human"
const MELEE := ["guard", "slash", "overhead", "parry", "hit", "death"]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("FA3 FAIL: ", what)


func _json(path: String) -> Dictionary:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _frames(dir: String, manifest: Dictionary) -> int:
	return BattleSkinned._texture_frames(dir + str(manifest.get("texture", "")))


## Ligne `row` de la texture d'os (toutes les matrices de l'image), en flottants.
func _row(image: Image, row: int) -> PackedFloat32Array:
	var out := PackedFloat32Array()
	for x in image.get_width():
		var c := image.get_pixel(x, row)
		out.append_array([c.r, c.g, c.b, c.a])
	return out


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var cmd_mode := BattleSkinned.FA_ALL if args.has("--fa-anim") else BattleSkinned.FA_DEFAULT
	if not BattleSkinned.fine_enabled():
		_check(BattleSkinned.fa_anim_mode() == BattleSkinned.FA_NONE, "couche inactive sur le kit grossier")
		print("FA3 anim (coarse): %s" % ("OK" if ok else "FAIL"))
		quit(0 if ok else 1)
		return
	_check(BattleSkinned.fa_anim_mode() == cmd_mode, "mode lu sur la ligne de commande")
	# Essais de la ligne de commande écartés : FA3 est comparé au jeu par défaut (NT14).
	BattleSkinned.video_trial_forced = 0
	BattleSkinned.mocap_trial_forced = 0
	BattleSkinned.melee_forced = 1
	var base: Dictionary = _json(BattleSkinned.FINE_DIR + "manifest.json").get("rigs", {}).get("human", {})
	var melee := _json(BattleSkinned.MELEE_DIR + "manifest.json")
	var fa := _json(BattleSkinned.FA_ANIM_DIR + "manifest.json")
	_check(not fa.is_empty(), "manifeste FA3 lisible")
	_check(fa.get("bones", []) == base.get("bones", []), "mêmes os que le rig fin")
	_check(int(fa.get("fps", 0)) == int(base.get("fps", 24)), "même cadence que le rig fin")
	var fa_clips: Dictionary = fa.get("clips", {})
	var sources: Dictionary = fa.get("clip_sources", {})
	for c in MELEE:
		_check(fa_clips.has(c), "clip de mêlée %s cuit" % c)
	var base_frames := _frames(BattleSkinned.FINE_DIR, base)
	var melee_frames := _frames(BattleSkinned.MELEE_DIR, melee)
	var fa_frames := _frames(BattleSkinned.FA_ANIM_DIR, fa)
	_check(base_frames > 0 and melee_frames > 0 and fa_frames > 0, "en-têtes CAB1 lisibles")
	# Nombres d'images cohérents : les clips couvrent la texture FA3 sans trou ni recouvrement.
	var total := 0
	var starts := {}
	for c in fa_clips:
		var clip: Dictionary = fa_clips[c]
		total += int(clip["frames"])
		_check(int(clip["frames"]) >= 2, "%s : au moins deux images" % c)
		_check(not starts.has(int(clip["start"])), "%s : début distinct" % c)
		starts[int(clip["start"])] = true
		_check(int(clip["start"]) + int(clip["frames"]) <= fa_frames, "%s dans la texture" % c)
		_check((base.get("clips", {}) as Dictionary).has(c), "%s remplace un clip du rig" % c)
		_check(bool(clip["loop"]) == bool(base["clips"][c]["loop"]), "%s : même bouclage" % c)
		var record: Dictionary = sources.get(c, {})
		_check(str(record.get("license", "")) == "CC0-1.0", "%s : licence CC0" % c)
		_check((record.get("quality", {}) as Dictionary).has("foot_slide_cm"), "%s : mesures" % c)
	_check(total == fa_frames, "images des clips = images de la texture (%d / %d)" % [total, fa_frames])
	# Le clip d'archer décoche à l'instant attendu par le mode VOLLEY (1,55 s).
	if fa_clips.has("bow_shoot"):
		_check(int(fa_clips["bow_shoot"]["frames"]) > int(1.55 * 24.0), "bow_shoot dépasse la décoche")
	# Les clips par défaut viennent du manifeste (donc de la table), pas du code.
	var defaults: Array = []
	for c in fa_clips:
		_check((fa_clips[c] as Dictionary).has("default"), "%s : champ default" % c)
		if bool(fa_clips[c].get("default", false)):
			defaults.append(c)
	_check(not defaults.is_empty() and defaults.size() < fa_clips.size(), "une partie des clips par défaut (%d)" % defaults.size())
	# Mode de la ligne de commande d'abord (non forcé), puis les trois modes forcés.
	BattleSkinned.reload_caches()
	_check_rig(base, melee, fa, base_frames, melee_frames, fa_frames, _covered(cmd_mode, fa_clips, defaults))
	for forced in [BattleSkinned.FA_NONE, BattleSkinned.FA_DEFAULT, BattleSkinned.FA_ALL]:
		BattleSkinned.fa_anim_forced = forced
		BattleSkinned.reload_caches()
		_check(BattleSkinned.fa_anim_mode() == forced, "forçage %d" % forced)
		_check_rig(base, melee, fa, base_frames, melee_frames, fa_frames, _covered(forced, fa_clips, defaults))
	# Un essai demandé n'est pas recouvert par les clips par défaut, mais l'est par `--fa-anim`.
	BattleSkinned.video_trial_forced = 1
	BattleSkinned.fa_anim_forced = BattleSkinned.FA_DEFAULT
	BattleSkinned.reload_caches()
	var trial: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(str(trial.get("mocap_trial_dir", "")) == BattleSkinned.VIDEO_TRIAL_DIR, "essai vidéo non recouvert par défaut")
	BattleSkinned.fa_anim_forced = BattleSkinned.FA_ALL
	BattleSkinned.reload_caches()
	trial = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(str(trial.get("mocap_trial_dir", "")) == BattleSkinned.FA_ANIM_DIR, "--fa-anim par-dessus un essai")
	BattleSkinned.video_trial_forced = 0
	# Sans la couche NT14 (`--keyframed-melee`), FA3 suit directement le rig.
	BattleSkinned.melee_forced = 0
	BattleSkinned.fa_anim_forced = BattleSkinned.FA_ALL
	BattleSkinned.reload_caches()
	var alone: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(int(alone["clips"]["guard"]["start"]) == base_frames + int(fa_clips["guard"]["start"]), "FA3 seul : après le rig")
	var tex_alone := BattleSkinned.bone_texture(RIG)
	_check(tex_alone != null and tex_alone.get_height() == base_frames + fa_frames, "FA3 seul : texture rig + FA3")
	BattleSkinned.melee_forced = -1
	BattleSkinned.fa_anim_forced = -1
	BattleSkinned.video_trial_forced = -1
	BattleSkinned.mocap_trial_forced = -1
	BattleSkinned.reload_caches()
	print("FA3 anim (mode=%s): %s" % [["none", "default", "all"][cmd_mode], "OK" if ok else "FAIL"])
	quit(0 if ok else 1)


## Clips FA3 substitués dans un mode.
func _covered(mode: int, fa_clips: Dictionary, defaults: Array) -> Array:
	if mode == BattleSkinned.FA_ALL:
		return fa_clips.keys()
	return defaults if mode == BattleSkinned.FA_DEFAULT else []


func _check_rig(base: Dictionary, melee: Dictionary, fa: Dictionary, base_frames: int, melee_frames: int, fa_frames: int, covered: Array) -> void:
	var entry: Dictionary = BattleSkinned.manifest().get("rigs", {}).get(RIG, {})
	_check(not entry.is_empty(), "rig %s présent" % RIG)
	var clips: Dictionary = entry.get("clips", {})
	var base_clips: Dictionary = base.get("clips", {})
	var melee_clips: Dictionary = melee.get("clips", {})
	var fa_clips: Dictionary = fa.get("clips", {})
	_check(clips.keys().size() == base_clips.keys().size(), "même nombre de clips")
	_check(clips.keys().size() <= BattleSkinned.MAX_CLIPS, "table de clips du shader assez grande")
	for c in fa_clips:
		_check(BattleSkinned.clip_index(entry, c) == BattleSkinned.clip_index(base, c), "indice du clip %s inchangé" % c)
	var layers: Array = entry.get("mocap_textures", [])
	var tex := BattleSkinned.bone_texture(RIG)
	_check(tex != null, "texture d'os chargée")
	if tex == null:
		return
	var image := tex.get_image()
	# Clips FA3 non substitués dans ce mode : ceux du jeu d'avant (NT14 ou rig).
	for c in fa_clips:
		if covered.has(c):
			continue
		var kept: Dictionary = clips.get(c, {})
		if melee_clips.has(c):
			_check(int(kept["start"]) == base_frames + int(melee_clips[c]["start"]), "%s : clip NT14 hors couche" % c)
		else:
			_check(kept == base_clips.get(c, {}), "%s : clip du rig hors couche" % c)
	if covered.is_empty():
		# Aucun clip FA3 : une seule couche (NT14), aucune image FA3.
		_check(layers.size() == 1 and str(layers[0]).begins_with(BattleSkinned.MELEE_DIR), "sans FA3 : couche NT14 seule")
		_check(tex.get_height() == base_frames + melee_frames, "sans FA3 : texture rig + NT14 (%d)" % tex.get_height())
		return
	_check(layers.size() == 2 and str(layers[1]).begins_with(BattleSkinned.FA_ANIM_DIR), "couche FA3 après la couche NT14")
	_check((entry.get("mocap_clips", []) as Array).size() == covered.size(), "%d clips substitués" % covered.size())
	_check(tex.get_height() == base_frames + melee_frames + fa_frames, "texture concaténée (%d)" % tex.get_height())
	_check(tex.get_height() <= 16384, "hauteur de texture dans la limite")
	for c in covered:
		var got: Dictionary = clips.get(c, {})
		_check(not got.has("default"), "%s : champ default hors de la table de clips" % c)
		var want: Dictionary = fa_clips[c]
		var start := base_frames + melee_frames + int(want["start"])
		_check(int(got.get("start", -1)) == start, "%s repointé après le rig et NT14" % c)
		_check(int(got.get("frames", -1)) == int(want["frames"]), "%s : longueur FA3" % c)
		_check(bool(got.get("loop", false)) == bool(want["loop"]), "%s : bouclage FA3" % c)
		# Joué : première, médiane et dernière images lisibles, finies, et le clip bouge ou
		# diffère du clip du rig qu'il remplace.
		var last := start + int(want["frames"]) - 1
		_check(last < tex.get_height(), "%s : dernière image dans la texture" % c)
		var first_row := _row(image, start)
		var finite := true
		for row in [start, (start + last) / 2, last]:
			for v in _row(image, row):
				if is_nan(v) or is_inf(v):
					finite = false
		_check(finite, "%s : matrices finies" % c)
		var old_row := _row(image, int(base_clips[c]["start"]))
		_check(first_row != old_row, "%s : pose différente du clip du rig" % c)
		# Durée lue par le jeu (durée de clip) = images / cadence.
		var seconds := float(want["frames"]) / float(entry.get("fps", 24))
		_check(seconds > 0.3 and seconds < 5.0, "%s : durée plausible (%.2f s)" % [c, seconds])
	# Clips non couverts : inchangés (`thrust` garde la couche NT14, `walk` le rig).
	if melee_clips.has("thrust") and not fa_clips.has("thrust"):
		_check(int(clips["thrust"]["start"]) == base_frames + int(melee_clips["thrust"]["start"]), "thrust garde NT14")
	for c in ["idle", "walk", "run", "xbow_shoot"]:
		_check(clips.get(c, {}) == base_clips.get(c, {}), "%s inchangé" % c)
	# Les jeux de clips des styles concernés se résolvent toujours.
	for pair in [["infantry", 0, "melee"], ["infantry", 1, "melee"], ["archer", 0, "shooting"], ["infantry", 0, "victory"]]:
		var config: Dictionary = BattleSkinned.state_config(str(pair[0]), int(pair[1]), str(pair[2]), false)
		_check(not (config.get("set", []) as Array).is_empty(), "jeu de clips %s/%s" % [pair[0], pair[2]])
