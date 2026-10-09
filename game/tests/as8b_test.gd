extends TestCase

## Lot AS8b : allures du cheval tirées de planches libres. Vérifie que les clips cuits gardent
## leur durée (cadence inchangée), que les cadences de `battle_gore.json` sont celles mesurées
## (vitesse d'appui des sabots dans le clip), et que la table de courbes versionnée est complète.
## Usage : godot --headless --path game --script res://tests/as8b_test.gd [-- --coarse-figures]

const CLIPS_SECONDS := {"c_trot": 18.0 / 24.0, "c_gallop": 15.0 / 24.0, "c_charge": 15.0 / 24.0, "c_std_charge": 30.0 / 24.0}
const MEASURED_CADENCE := {"c_trot": 3.4, "c_bow_trot": 3.4, "c_javelin_trot": 3.4, "c_trot_turn_l": 3.4, "c_trot_turn_r": 3.4, "c_gallop": 4.5, "c_charge": 4.5}


func _init() -> void:
	for clip in CLIPS_SECONDS:
		var seconds := BattleSkinned.clip_seconds("cavalry", 0, clip)
		check(absf(seconds - float(CLIPS_SECONDS[clip])) < 0.05, "%s dure %.3f s (attendu %.3f)" % [clip, seconds, CLIPS_SECONDS[clip]])
	var cadence: Dictionary = BattleGore.settings().get("cadence", {})
	for clip in MEASURED_CADENCE:
		check(absf(float(cadence.get(clip, 0.0)) - float(MEASURED_CADENCE[clip])) < 0.01, "cadence de %s" % clip)
	# Table de courbes versionnée (outil Blender) : trot et galop, quatre membres, 48 pas.
	var path := ProjectSettings.globalize_path("res://").path_join("../tools/blender_scripts/data/horse_gaits_free.json")
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	check(parsed is Dictionary, "horse_gaits_free.json lisible")
	if parsed is Dictionary:
		for gait in ["trot", "gallop"]:
			var table: Dictionary = parsed.get("gaits", {}).get(gait, {})
			check(not table.is_empty(), "allure %s absente" % gait)
			for leg in ["F1", "F2", "H1", "H2"]:
				var curve: Dictionary = table.get("legs", {}).get(leg, {})
				for key in ["u", "d", "h", "upper", "lower"]:
					check(curve.get(key, []).size() == int(table.get("samples", 48)), "%s %s %s : %d pas" % [gait, leg, key, curve.get(key, []).size()])
	finish()
