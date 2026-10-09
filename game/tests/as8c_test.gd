extends TestCase

## Lot AS8c (ADR 0189) : mouvement des bêtes et charrettes tiré de vidéos libres. Vérifie par valeurs
## (le shader de sommets ne tourne pas sans GPU) : les courbes de pas, le cahot et le roulis des
## données sont ceux de `data/fx/animal_motion_measured.json`, ils arrivent dans les uniformes des
## accessoires, le shader les lit, et les chevaux de camp reçoivent la durée de descente de la tête
## et le mâchonnement mesurés.
## Usage : godot --headless --path game --script res://tests/as8c_test.gd [-- --no-as1]

const SHADER_INC := "res://shaders/animal_motion.gdshaderinc"
const CAMP_SHADER_PATH := "res://shaders/camp_horse.gdshader"


func _measured() -> Dictionary:
	var path := DataFile.path_of("fx/animal_motion_measured.json")
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _camp_file() -> Dictionary:
	var path := DataFile.path_of("fx/camp_horse_motion.json")
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _init() -> void:
	var off := CmdArgs.has("--no-as1")
	var measured := _measured()
	check(not measured.is_empty(), "données mesurées absentes")
	var defaults: Dictionary = AnimalMotion.settings().get("campaign", {}).get("defaults", {})
	var swing: Array = defaults.get("swing_lut", [])
	var lift: Array = defaults.get("lift_lut", [])
	check(swing.size() == 16 and lift.size() == 16, "tables de pas : 16 échantillons attendus")
	if measured.has("cattle"):
		check(swing == measured["cattle"]["swing_lut"], "swing_lut différent de la mesure")
		check(lift == measured["cattle"]["lift_lut"], "lift_lut différent de la mesure")
	# La table de pas : zéro en montée à la phase 0, amplitude 1, profil non sinusoïdal (mesuré).
	if swing.size() == 16:
		check(absf(AnimalMotion.lut_at(swing, 0.0)) < 0.05, "swing_lut : phase 0 n'est pas le milieu du balancé")
		check(AnimalMotion.lut_at(swing, 1.0 / 16.0) > 0.4, "swing_lut : le balancé avant ne monte pas")
		var peak := 0.0
		var worst := 0.0
		for i in 64:
			var u := float(i) / 64.0
			peak = maxf(peak, absf(AnimalMotion.lut_at(swing, u)))
			worst = maxf(worst, absf(AnimalMotion.lut_at(swing, u) - sin(TAU * u)))
		check(peak > 0.9 and peak < 1.1, "swing_lut : amplitude %.2f" % peak)
		check(worst > 0.1, "swing_lut : profil indistinguable d'un sinus (%.2f)" % worst)
		check(is_equal_approx(AnimalMotion.lut_at(swing, 0.25 + 1.0), AnimalMotion.lut_at(swing, 0.25)), "lut_at : pas de bouclage")
	# Uniformes d'un attelage et d'une bête.
	for role in ["ox", "cow", "horse", "sheep", "stone_cart", "merchant_cart", "dead_cart"]:
		var mesh := FolkModels.prop_mesh(role)
		check(mesh != null, "%s : maillage" % role)
		if mesh == null:
			continue
		var mat := mesh.surface_get_material(0) as ShaderMaterial
		var p := AnimalMotion.model_params(str(FolkModels.PROPS[role]["model"]))
		if off or p.is_empty():
			continue
		var arr: PackedFloat32Array = mat.get_shader_parameter("am_swing")
		check(arr.size() == 16, "%s : am_swing posé (%d)" % [role, arr.size()])
		if arr.size() == 16:
			check(is_equal_approx(arr[3], float(p["swing_lut"][3])), "%s : am_swing[3]" % role)
		var amps: Vector3 = mat.get_shader_parameter("am_jolt_amp")
		check(amps.x > 0.0 and amps.x >= amps.y and amps.y >= amps.z, "%s : raies de cahot (%s)" % [role, amps])
		var ratios: Vector3 = mat.get_shader_parameter("am_jolt_ratio")
		check(ratios.x > ratios.y and ratios.y > ratios.z and ratios.z > 0.0, "%s : périodes des raies (%s)" % [role, ratios])
		var extra: Vector4 = mat.get_shader_parameter("am_extra")
		check(is_equal_approx(extra.x, float(p["graze_ramp_s"])) and is_equal_approx(extra.y, float(p["roll_rad"])), "%s : am_extra" % role)
	# Valeurs mesurées reprises par les modèles.
	var cow := AnimalMotion.model_params("cow")
	var sheep := AnimalMotion.model_params("sheep")
	var horse := AnimalMotion.model_params("horse")
	if measured.has("cattle"):
		var withers := float(cow["withers_m"])
		check(absf(float(cow["stride_m"]) - float(measured["cattle"]["stride_withers"]) * withers) < 0.03, "foulée de la vache = foulée mesurée x garrot")
		check(absf(float(cow["chew_hz"]) - float(measured["cattle"]["bite_peaks_hz"][0][0])) < 0.05, "cadence de broutage de la vache")
		check(absf(float(cow["graze_rad"]) - float(measured["cattle"]["graze_delta_rad"])) < 0.05, "descente de tête de la vache")
	if measured.has("sheep"):
		check(absf(float(sheep["graze_rad"]) - float(measured["sheep"]["graze_delta_rad"])) < 0.05, "descente de tête du mouton")
	if measured.has("wagon"):
		check(absf(float(horse["stride_m"]) - float(measured["wagon"]["horse_stride_m"])) < 0.05, "foulée du cheval de trait")
	# Chevaux de camp.
	var camp := AnimalMotion.camp_horse_material(null, false)
	var head3: Vector4 = camp.get_shader_parameter("ch_head3")
	var camp_settings: Dictionary = AnimalMotion.settings()["camp_horse"]
	check(is_equal_approx(head3.w, float(camp_settings["head_ramp_s"])), "camp : durée de descente de la tête")
	check(is_equal_approx(head3.y, float(camp_settings["chew_hz"])), "camp : mâchonnement")
	var rest: Dictionary = _camp_file().get("horse_rest", {})
	check(not measured.has("horse_rest"), "mesures Rama hors de animal_motion_measured.json")
	if not rest.is_empty():
		check(absf(float(camp_settings["head_lift_rad"]) - float(rest["head_lift_rad"])) < 0.01, "camp : tête relevée mesurée")
		check(absf(float(camp_settings["tail_hz"]) - float(rest["tail_hz"])) < 0.01, "camp : fréquence de queue mesurée")
	# Le shader lit bien les tables et les raies (et non plus le sinus unique de l'AS1).
	var inc := FileAccess.get_file_as_string(SHADER_INC)
	check(inc.contains("am_swing_at(leg_u)") and inc.contains("am_lift_at(leg_u)"), "shader : tables de pas non lues")
	check(not inc.contains("sin(leg_phase)"), "shader : sinus de pas de l'AS1 encore présent")
	check(inc.contains("am_jolt_amp[k]") and inc.contains("am_extra.y"), "shader : cahot / roulis mesurés non lus")
	check(FileAccess.get_file_as_string(CAMP_SHADER_PATH).contains("ch_head3.w"), "shader de camp : durée de descente non lue")
	finish()
