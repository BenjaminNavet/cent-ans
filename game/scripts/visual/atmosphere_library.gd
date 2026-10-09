class_name AtmosphereLibrary
extends RefCounted

## Ciels HDRI et étalonnage, lus dans `data/fx/atmosphere.json` (schéma
## `data/schemas/fx_atmosphere.schema.json`) : matériau de ciel (`hdri_sky.gdshader`, panorama
## Poly Haven CC0 aligné sur le soleil de la scène) et LUT 3D d'étalonnage générée à partir de
## paramètres lisibles (température, lift/gamma/gain, contraste, saturation, virage), appliquée par
## `Environment.adjustment_color_correction`. Purement visuel.

const DATA_PATH := "fx/atmosphere.json"
const SKY_SHADER := preload("res://shaders/hdri_sky.gdshader")
const SEASONS: Array[String] = ["spring", "summer", "autumn", "winter"]
## Repli de `battle_decor_saturation` si la saison n'en donne pas.
const DEFAULT_DECOR_SATURATION := 0.6

static var _lookup := JsonLookup.new(DATA_PATH)
static var _lut_cache: Dictionary = {}


static func data() -> Dictionary:
	return _lookup.data()


static func normalize_season(season: String) -> String:
	return season if season in SEASONS else "summer"


## Description d'un ciel (`skies.<id>`), {} si inconnu.
static func sky_info(sky_id: String) -> Dictionary:
	return (data().get("skies", {}) as Dictionary).get(sky_id, {})


## Réglages de bataille pour la météo `weather` et la saison : {sky_id, sky, look, grades}.
static func battle_look(weather: String, season: String) -> Dictionary:
	var battle: Dictionary = data().get("battle", {})
	var look: Dictionary = battle.get(weather, battle.get("clear", {}))
	if look.is_empty():
		return {}
	var sky_id := str((look.get("sky", {}) as Dictionary).get(normalize_season(season), ""))
	var season_grade: Dictionary = battle_season_grade(season)
	return {"sky_id": sky_id, "sky": sky_info(sky_id), "look": look, "grades": [season_grade, look.get("grade", {})], "strength": 1.0}


## Étalonnage de saison des batailles (`battle_seasons.<saison>.grade`, sinon celui de
## `seasons`, commun à la campagne).
static func battle_season_grade(season: String) -> Dictionary:
	var key := normalize_season(season)
	var battle_season: Dictionary = (data().get("battle_seasons", {}) as Dictionary).get(key, {})
	if battle_season.has("grade"):
		return battle_season["grade"]
	return (data().get("seasons", {}) as Dictionary).get(key, {})


## Part de saturation gardée par l'albédo du sol et de l'herbe de bataille
## (`battle_seasons.<saison>.decor_saturation`, 0,6 par défaut).
static func battle_decor_saturation(season: String) -> float:
	var battle_season: Dictionary = (data().get("battle_seasons", {}) as Dictionary).get(normalize_season(season), {})
	return float(battle_season.get("decor_saturation", DEFAULT_DECOR_SATURATION))


## Réglages de campagne pour la saison : {sky_id, sky, look, grades, strength}.
static func campaign_look(season: String) -> Dictionary:
	var campaign: Dictionary = data().get("campaign", {})
	var look: Dictionary = (campaign.get("seasons", {}) as Dictionary).get(normalize_season(season), {})
	if look.is_empty():
		return {}
	var sky_id := str(look.get("sky", ""))
	var season_grade: Dictionary = (data().get("seasons", {}) as Dictionary).get(normalize_season(season), {})
	return {"sky_id": sky_id, "sky": sky_info(sky_id), "look": look, "grades": [season_grade], "strength": float(campaign.get("grade_strength", 1.0))}


## Matériau de ciel pour un `look` (battle_look / campaign_look) ; null si le panorama manque.
static func sky_material(resolved: Dictionary, haze_color: Color, ground_color: Color) -> ShaderMaterial:
	var sky: Dictionary = resolved.get("sky", {})
	if sky.is_empty() or not ResourceLoader.exists(str(sky["path"])):
		return null
	var look: Dictionary = resolved.get("look", {})
	var material := ShaderMaterial.new()
	material.shader = SKY_SHADER
	material.set_shader_parameter("panorama", load(str(sky["path"])))
	material.set_shader_parameter("panorama_sun_azimuth", float(sky["sun_azimuth_deg"]))
	material.set_shader_parameter("exposure", float(look.get("sky_luminance", 0.6)) / maxf(float(sky["mean"]), 0.05))
	material.set_shader_parameter("saturation", float(look.get("sky_saturation", 1.0)))
	material.set_shader_parameter("tint", _rgb(look.get("sky_tint", [1, 1, 1])))
	material.set_shader_parameter("sun_disc", float(look.get("sun_disc", 0.0)) if bool(sky.get("sunny", false)) else 0.0)
	material.set_shader_parameter("haze_amount", float(look.get("haze", 0.35)))
	material.set_shader_parameter("haze_color", haze_color)
	material.set_shader_parameter("ground_color", ground_color)
	return material


## Élévation du soleil peint dans le panorama s'il est ensoleillé, sinon `fallback`.
static func sun_elevation(resolved: Dictionary, fallback: float) -> float:
	var sky: Dictionary = resolved.get("sky", {})
	if sky.is_empty() or not bool(sky.get("sunny", false)):
		return fallback
	return clampf(float(sky["sun_elevation_deg"]), 12.0, 60.0)


## Applique ciel et étalonnage à `env` (ciel conservé si le panorama manque).
static func apply_to_environment(env: Environment, resolved: Dictionary, haze_color: Color, ground_color: Color) -> void:
	if resolved.is_empty():
		return
	var material := sky_material(resolved, haze_color, ground_color)
	if material != null:
		var sky := Sky.new()
		sky.sky_material = material
		sky.radiance_size = Sky.RADIANCE_SIZE_256
		sky.process_mode = Sky.PROCESS_MODE_QUALITY
		env.sky = sky
		env.background_mode = Environment.BG_SKY
	env.adjustment_enabled = true
	env.adjustment_color_correction = grade_lut(resolved.get("grades", []), float(resolved.get("strength", 1.0)))


static func _rgb(values: Variant, fallback: Color = Color.WHITE) -> Color:
	if values is Array and (values as Array).size() >= 3:
		return Color(float(values[0]), float(values[1]), float(values[2]))
	return fallback


# --- LUT d'étalonnage --------------------------------------------------------------------


## LUT 3D (espace sRGB, taille `lut_size`) composant les étalonnages dans l'ordre, mêlée à
## l'identité selon `strength`. Mise en cache par contenu.
static func grade_lut(grades: Array, strength: float = 1.0) -> ImageTexture3D:
	var size := clampi(int(data().get("lut_size", 24)), 8, 64)
	var key := JSON.stringify([grades, strength, size])
	if _lut_cache.has(key):
		return _lut_cache[key]
	# Cuisson native (`GradeLut`, MS8) : octets RGB8, rouge puis vert puis couche bleue.
	var bytes: PackedByteArray = ClassDB.instantiate("GradeLut").bake(JSON.stringify(grades), strength, size)
	var layers: Array[Image] = []
	var layer_bytes := size * size * 3
	for b in size:
		layers.append(Image.create_from_data(size, size, false, Image.FORMAT_RGB8, bytes.slice(b * layer_bytes, (b + 1) * layer_bytes)))
	var texture := ImageTexture3D.new()
	texture.create(Image.FORMAT_RGB8, size, size, size, false, layers)
	_lut_cache[key] = texture
	return texture


# --- Saison de campagne ---------------------------------------------------------------------


## Saison ("spring"…) d'un libellé de date de la simulation (« Printemps 1337 »), "" si inconnue.
static func season_of_date(label: String) -> String:
	var words := {"printemps": "spring", "été": "summer", "ete": "summer", "automne": "autumn", "hiver": "winter"}
	var parts := label.strip_edges().split(" ", false)
	if parts.is_empty():
		return ""
	return str(words.get(parts[0].to_lower(), ""))
