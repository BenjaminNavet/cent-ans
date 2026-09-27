class_name BattleSecondaryMotion
extends RefCounted

## Lot AN1a (ADR 0096) : mouvement secondaire des figurines de bataille, calculé dans le shader
## de sommets (`battle_soldier_skinned.gdshader`, `battle_standard_flag.gdshader`) — aucun os
## ni aucune donnée cuite en plus. Rendu seulement.
## - Pièces souples reconnues sans recuisson : bas du surcot et de la jaque (livrée, gambison :
##   poids = part des os des jambes, 0 à la taille, ~0,85 à l'ourlet), caparaçon (armoiries sur
##   les os du cheval, poids selon la hauteur de repos entre la selle et l'ourlet), queue (os
##   `Tail1-7`, poids = rang dans la chaîne), crinière (robe du cheval à la teinte exacte des
##   crins, `HAIR_RGB` de `battle_fine_cavalry.py`).
## - Moteurs : vitesse du régiment (recul vers l'arrière), vent de la bataille
##   (`data/fx/battle_finish.json`, `wind`) et rafales ; amplitudes et échelle du vent dans
##   `data/fx/atmosphere.json` (`secondary_motion`).

## Teinte (8 bits) des crins du cheval fin : 0,33 → 84/255.
const MANE_TINT := 84.0 / 255.0
const LEG_BONES: Array[String] = ["UpperLeg.L", "LowerLeg.L", "Foot.L", "UpperLeg.R", "LowerLeg.R", "Foot.R"]
const PARTS: Array[String] = ["skirt", "caparison", "mane", "tail"]

## Vent de la bataille en cours (`BattleStandards.wind_for`), posé par la scène avant la création
## des matériaux ; repli : vent par défaut des données.
static var _wind: Dictionary = {}


static func settings() -> Dictionary:
	return AtmosphereLibrary.data().get("secondary_motion", {})


## Éteint par les données (`enabled`) ou par `--no-an1a` (banc A/B).
static func enabled() -> bool:
	return bool(settings().get("enabled", false)) and not OS.get_cmdline_user_args().has("--no-an1a")


static func set_wind(wind: Dictionary) -> void:
	_wind = wind


static func wind() -> Dictionary:
	if _wind.is_empty():
		var entry: Dictionary = (BattleStandards.settings().get("wind", {}) as Dictionary).get("default", {})
		return {"dir": Vector2(1, 0), "strength": float(entry.get("strength", 0.5)), "gust": float(entry.get("gust", 0.2))}
	return _wind


## Plage d'indices (min, max) des os nommés `names` (préfixe `prefix`) du rig ; (-1, -2) sinon.
static func bone_range(bones: Array, names: Array, prefix: String) -> Vector2i:
	var lo := 1 << 30
	var hi := -1
	for name in names:
		var k := bones.find(prefix + str(name))
		if k >= 0:
			lo = mini(lo, k)
			hi = maxi(hi, k)
	return Vector2i(lo, hi) if hi >= 0 else Vector2i(-1, -2)


## Uniformes du mouvement secondaire d'un matériau de figurine skinnée (après
## `BattleSkinned.setup_material`, qui pose `sever_bones` : os du cheval).
static func setup_material(mat: ShaderMaterial, kind: String, variant: int) -> void:
	var cfg := settings()
	if not enabled():
		mat.set_shader_parameter("sm_enabled", false)
		return
	var bones: Array = BattleSkinned.rig(kind, variant).get("bones", [])
	var mounted := not bones.is_empty() and str(bones[bones.size() - 1]).begins_with("R:")
	var tails: Array[String] = []
	for k in range(1, 8):
		tails.append("Tail%d" % k)
	mat.set_shader_parameter("sm_enabled", true)
	mat.set_shader_parameter("sm_leg_bones", bone_range(bones, LEG_BONES, "R:" if mounted else ""))
	mat.set_shader_parameter("sm_tail_bones", bone_range(bones, tails, "") if mounted else Vector2i(-1, -2))
	apply_settings(mat, cfg)
	apply_wind(mat)


## Amplitudes (`secondary_motion`) : par pièce (recul, vent, flottement, fréquence) et évasement.
static func apply_settings(mat: ShaderMaterial, cfg: Dictionary) -> void:
	var amps: Array[Vector4] = []
	var lift := Vector4.ZERO
	for i in PARTS.size():
		var p: Dictionary = cfg.get(PARTS[i], {})
		amps.append(Vector4(float(p.get("speed_amp_m", 0.0)), float(p.get("wind_amp_m", 0.0)), float(p.get("flutter_amp_m", 0.0)), float(p.get("flutter_hz", 1.0))))
		lift[i] = float(p.get("lift", 0.0))
	var cap: Dictionary = cfg.get("caparison", {})
	mat.set_shader_parameter("sm_amp", amps)
	mat.set_shader_parameter("sm_lift", lift)
	mat.set_shader_parameter("sm_cap_band", Vector2(float(cap.get("hem_y_m", 0.48)), float(cap.get("top_y_m", 1.45))))
	mat.set_shader_parameter("sm_distance", float(cfg.get("max_distance_m", 160.0)))
	mat.set_shader_parameter("sm_speed_ref", float(cfg.get("speed_ref_m_s", 5.0)))
	mat.set_shader_parameter("sm_wind_scale", float(cfg.get("wind_scale", 1.0)))
	mat.set_shader_parameter("sm_mane_tint", MANE_TINT)


static func apply_wind(mat: ShaderMaterial) -> void:
	var w := wind()
	mat.set_shader_parameter("wind_dir", (w.get("dir", Vector2(1, 0)) as Vector2).normalized())
	mat.set_shader_parameter("wind_strength", float(w.get("strength", 0.5)))
	mat.set_shader_parameter("wind_gust", float(w.get("gust", 0.2)))


## Étoffe des étendards portés (`battle_standard_flag.gdshader`) : onde et vent apparent de
## l'allure du porteur.
static func setup_flag(mat: ShaderMaterial) -> void:
	var flag: Dictionary = settings().get("flag", {})
	if flag.is_empty() or not enabled():
		return
	mat.set_shader_parameter("flag_amp", Vector2(float(flag["amp_min"]), float(flag["amp_max"])))
	mat.set_shader_parameter("flag_wave_speed", Vector2(float(flag["wave_speed_min"]), float(flag["wave_speed_max"])))
	mat.set_shader_parameter("flag_ripple", float(flag["ripple"]))
	var gait: Array = flag.get("gait_wind", [0.0, 0.0, 0.0, 0.0])
	mat.set_shader_parameter("flag_gait_wind", Vector4(float(gait[0]), float(gait[1]), float(gait[2]), float(gait[3])))
	mat.set_shader_parameter("flag_wind_scale", float(settings().get("wind_scale", 1.0)))
