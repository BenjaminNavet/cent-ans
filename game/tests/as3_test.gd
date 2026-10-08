extends SceneTree

## Lot AS3 : trot, virages et cheval sans cavalier. Vérifie que le rig cavalerie porte les clips
## cuits (trot en boucle d'une foulée, virages, chute allongée à 4,5 s), que chaque style de
## cavalerie sert ses états d'allure avec des clips présents (aussi avec `--coarse-figures`), que
## la cadence de chaque clip d'allure est dans `data/fx/battle_gore.json`, et le choix d'allure
## (bandes de vitesse, hystérésis, sens du virage, désactivation par `--no-as3`).
## Usage : godot --headless --path game --script res://tests/as3_test.gd [-- --coarse-figures]

const GAIT_STATES := ["trotting", "turn_l", "turn_r", "trot_turn_l", "trot_turn_r"]
const NEW_CLIPS := [
	"c_trot", "c_bow_trot", "c_javelin_trot", "c_std_trot",
	"c_turn_l", "c_turn_r", "c_trot_turn_l", "c_trot_turn_r",
	"c_bow_turn_l", "c_bow_turn_r", "c_bow_trot_turn_l", "c_bow_trot_turn_r",
	"c_javelin_turn_l", "c_javelin_turn_r", "c_javelin_trot_turn_l", "c_javelin_trot_turn_r",
]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("AS3 FAIL: ", what)


func _init() -> void:
	var rig_clips: Dictionary = BattleSkinned.rig("cavalry", 0).get("clips", {})
	_check(rig_clips.size() <= BattleSkinned.MAX_CLIPS, "%d clips > MAX_CLIPS" % rig_clips.size())
	for c in NEW_CLIPS:
		_check(rig_clips.has(c), "clip %s absent du rig" % c)
		if rig_clips.has(c):
			_check(bool(rig_clips[c].get("loop", false)), "%s doit boucler" % c)
	# Une foulée de trot = 0,75 s (18 images à 24 i/s), un tour de pas inchangé.
	_check(absf(BattleSkinned.clip_seconds("cavalry", 0, "c_trot") - 0.75) < 0.01, "c_trot = 0,75 s")
	# La chute dure autant que la fuite du cheval dans le shader (code 6 : disparaît à t = 4,5 s).
	_check(absf(BattleSkinned.clip_seconds("cavalry", 0, "c_fall") - 4.5) < 0.05, "c_fall = 4,5 s")
	_check(BattleSkinned.death_index("cavalry", 0, "c_fall") >= 0, "c_fall dans le jeu des morts")
	# États d'allure de chaque figurine de cavalerie : clips présents dans le rig, jeu non vide.
	var cadence: Dictionary = BattleGore.settings().get("cadence", {})
	for variant in 7:
		var clips: Dictionary = BattleSkinned.rig("cavalry", variant).get("clips", {})
		for state in GAIT_STATES:
			var config := BattleSkinned.state_config("cavalry", variant, state, false)
			var names: Array = config["names"]
			_check(not names.is_empty(), "cavalry_%d %s: jeu vide" % [variant, state])
			for c in names:
				_check(clips.has(str(c)), "cavalry_%d %s: clip %s absent" % [variant, state, c])
			_check(cadence.has(str(names[0])), "cadence de %s absente de battle_gore.json" % names[0])
			if state == "trotting":
				_check(str(names[0]).ends_with("trot"), "cavalry_%d trotting = %s" % [variant, names[0]])
			if state == "trot_turn_l":
				_check(str(names[0]).ends_with("trot_turn_l"), "cavalry_%d %s" % [variant, names[0]])
	# Choix d'allure.
	BattleCavalryGaits.set_override({"enabled": true, "trot_min_speed": 3.2, "gallop_min_speed": 5.6, "hysteresis_mps": 0.4, "turn_min_rate": 0.16, "turn_hysteresis": 0.05})
	var mem := {}
	_check(BattleCavalryGaits.pick(mem, 1.8, false, 0.0) == "marching", "pas à 1,8 m/s")
	_check(BattleCavalryGaits.pick(mem, 3.4, false, 0.0) == "marching", "hystérésis : 3,4 m/s depuis le pas reste au pas")
	_check(BattleCavalryGaits.pick(mem, 4.0, false, 0.0) == "trotting", "trot à 4 m/s")
	_check(BattleCavalryGaits.pick(mem, 3.0, false, 0.0) == "trotting", "hystérésis : 3,0 m/s depuis le trot reste au trot")
	_check(BattleCavalryGaits.pick(mem, 2.5, false, 0.0) == "marching", "retour au pas à 2,5 m/s")
	mem = {}
	_check(BattleCavalryGaits.pick(mem, 4.5, true, 0.0) == "trotting", "régiment qui court mais lent : trot")
	_check(BattleCavalryGaits.pick(mem, 7.0, true, 0.0) == "running", "galop à 7 m/s")
	_check(BattleCavalryGaits.pick(mem, 5.4, true, 0.0) == "running", "hystérésis : 5,4 m/s depuis le galop reste au galop")
	_check(BattleCavalryGaits.pick(mem, 7.0, false, 0.0) == "trotting", "sans l'ordre de courir, pas de galop")
	mem = {}
	_check(BattleCavalryGaits.pick(mem, 1.8, false, 0.3) == "turn_l", "virage à gauche au pas")
	_check(BattleCavalryGaits.pick(mem, 1.8, false, 0.12) == "turn_l", "hystérésis du virage")
	_check(BattleCavalryGaits.pick(mem, 1.8, false, 0.05) == "marching", "fin du virage")
	_check(BattleCavalryGaits.pick(mem, 4.0, false, -0.3) == "trot_turn_r", "virage à droite au trot")
	mem = {}
	_check(BattleCavalryGaits.pick(mem, 7.0, true, 0.5) == "running", "pas de virage au galop")
	# Sens : positif = vers la gauche (x = sin f), saut de ±π sans effet.
	_check(BattleCavalryGaits.turn_rate(0.0, 0.1, 1.0) > 0.0, "turn_rate > 0 vers la gauche")
	_check(absf(BattleCavalryGaits.turn_rate(3.1, -3.1, 1.0) - (2.0 * PI - 6.2)) < 1e-4, "turn_rate au passage de ±π")
	_check(BattleCavalryGaits.turn_rate(0.0, 0.1, 0.0) == 0.0, "turn_rate sans temps écoulé")
	# Les réglages livrés respectent la bande trot entre pas et galop.
	BattleCavalryGaits.set_override({})
	var cfg := BattleCavalryGaits.settings()
	_check(float(cfg["trot_min_speed"]) > float(cadence["c_walk"]) and float(cfg["gallop_min_speed"]) < 1.8 * float(cadence["c_gallop"]), "seuils entre c_walk et c_gallop (cadence jusqu'à ×1,8, AS8b)")
	_check(float(cfg["trot_min_speed"]) < float(cfg["gallop_min_speed"]), "trot_min < gallop_min")
	print("AS3 cavalry gaits (fine=%s): %s" % [BattleSkinned.fine_enabled(), "OK" if ok else "FAIL"])
	quit(0 if ok else 1)
