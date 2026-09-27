extends SceneTree

## Lot AN1b : nouveaux clips cuits et leur branchement. Vérifie que le manifeste des figurines
## fines porte les clips (victoire, attentes, parade, coup par-dessus, impacts, cabrage,
## trébuchement), que les jeux de clips tiennent dans `MAX_SET` et le codage `ivec4` (deux
## indices par composante), que chaque clip d'un jeu existe dans le rig, que la table des clips
## tient dans `MAX_CLIPS`, et qu'avec `--coarse-figures` (kit Quaternius sans ces clips) les jeux
## se replient sur des clips présents.
## Usage : godot --headless --path game --script res://tests/an1b_clips_test.gd [-- --coarse-figures]

const HUMAN_NEW := ["victory", "victory_b", "victory_pike", "idle_look", "idle_lean", "idle_helm", "pike_look", "bow_look", "xbow_look", "parry", "overhead", "hit_b", "hit_c"]
const CAVALRY_NEW := ["c_rear", "c_stumble", "c_victory"]
const STATES := ["idle", "marching", "charging", "melee", "melee_pikes", "shooting", "routing", "victory", "brace"]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("AN1b FAIL: ", what)


func _init() -> void:
	var fine := BattleSkinned.fine_enabled()
	var rigs: Dictionary = BattleSkinned.manifest().get("rigs", {})
	for rig_name in rigs:
		var clips: Dictionary = rigs[rig_name].get("clips", {})
		_check(clips.size() <= BattleSkinned.MAX_CLIPS, "%s: %d clips > MAX_CLIPS" % [rig_name, clips.size()])
		# Rigs fins préfixés `fine_` (le kit grossier garde ses anciens clips).
		if fine and str(rig_name).begins_with(BattleSkinned.FINE_RIG_PREFIX):
			for c in (HUMAN_NEW if str(rig_name).ends_with("human") else CAVALRY_NEW):
				_check(clips.has(c), "%s: clip %s absent" % [rig_name, c])
	# Codage des jeux : 8 indices < 256, relus comme le shader (pick_clip).
	var ids := [3, 57, 12, 0, 63, 41, 9, 22]
	var v := BattleSkinned._ivec(ids)
	for i in ids.size():
		_check(((v[i & 3] >> ((i >> 2) * 8)) & 255) == ids[i], "ivec slot %d" % i)
	_check(BattleSkinned._ivec([5, 6]) == Vector4i(5, 6, 0, 0), "ivec: 4 clips au plus = ancien codage")
	# Jeux de chaque figurine et de chaque état : clips présents, taille bornée.
	for fig_name in BattleSkinned.manifest().get("figures", {}):
		var parts := str(fig_name).split("_")
		var kind := parts[0]
		if not ["infantry", "archer", "cavalry"].has(kind):
			continue
		var variant := int(parts[1])
		var rig_clips: Dictionary = BattleSkinned.rig(kind, variant).get("clips", {})
		for state in STATES:
			var config := BattleSkinned.state_config(kind, variant, state, false)
			var names: Array = config["names"]
			_check(names.size() >= 1 and names.size() <= BattleSkinned.MAX_SET, "%s %s: taille %d" % [fig_name, state, names.size()])
			for c in names:
				_check(rig_clips.has(str(c)), "%s %s: clip %s absent du rig" % [fig_name, state, c])
	if fine:
		var sword := BattleSkinned.state_config("infantry", 0, "melee", false)
		_check((sword["names"] as Array).has("overhead") and (sword["names"] as Array).has("parry"), "mêlée à l'épée sans parade ni coup par-dessus")
		_check((BattleSkinned.state_config("infantry", 0, "victory", false)["names"] as Array).has("victory"), "victoire à l'épée")
		_check((BattleSkinned.state_config("cavalry", 0, "melee_pikes", false)["names"] as Array).has("c_rear"), "cabrage devant les piques")
		_check((BattleSkinned.state_config("cavalry", 0, "charging", false)["names"] as Array).has("c_stumble"), "trébuchement en charge")
		# Le cycle de charge couvre un nombre entier de foulées de galop (pas de saut au cycle).
		var charge := BattleSkinned.state_config("cavalry", 0, "charging", false)
		var gallop := BattleSkinned.clip_seconds("cavalry", 0, "c_charge")
		var strides := float(charge["cycle"]) / gallop
		_check(absf(strides - roundf(strides)) < 0.01, "cycle de charge %.3f s / foulée %.3f s" % [float(charge["cycle"]), gallop])
		_check(is_equal_approx(BattleSkinned.clip_seconds("cavalry", 0, "c_stumble"), float(charge["cycle"])), "c_stumble = un cycle de charge")
		_check(BattleSkinned.clip_seconds("cavalry", 0, "c_rear") <= 1.4, "c_rear tient dans le cycle de mêlée")
	print("AN1b clips (fine=%s): %s" % [fine, "OK" if ok else "FAIL"])
	quit(0 if ok else 1)
