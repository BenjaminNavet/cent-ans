class_name AtmosphereLibrary
extends RefCounted

## Ciels HDRI et étalonnage (lot V3, A1-05), lus dans `data/fx/atmosphere.json` (schéma
## `data/schemas/fx_atmosphere.schema.json`) : matériau de ciel (`hdri_sky.gdshader`, panorama
## Poly Haven CC0 aligné sur le soleil de la scène) et LUT 3D d'étalonnage générée à partir de
## paramètres lisibles (température, lift/gamma/gain, contraste, saturation, virage), appliquée par
## `Environment.adjustment_color_correction`. Purement visuel.

const DATA_PATH := "fx/atmosphere.json"
const SKY_SHADER := preload("res://shaders/hdri_sky.gdshader")
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const SEASONS: Array[String] = ["spring", "summer", "autumn", "winter"]
## Repli de `battle_decor_saturation` si la saison n'en donne pas (valeur DA6).
const DEFAULT_DECOR_SATURATION := 0.6

static var _data: Dictionary = {}
static var _lut_cache: Dictionary = {}


static func data() -> Dictionary:
	if _data.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("AtmosphereLibrary: %s missing or invalid" % path)
	return _data


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


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


## DA7b : étalonnage de saison des batailles (`battle_seasons.<saison>.grade`, sinon celui de
## `seasons`, commun à la campagne).
static func battle_season_grade(season: String) -> Dictionary:
	var key := normalize_season(season)
	var battle_season: Dictionary = (data().get("battle_seasons", {}) as Dictionary).get(key, {})
	if battle_season.has("grade"):
		return battle_season["grade"]
	return (data().get("seasons", {}) as Dictionary).get(key, {})


## DA6/DA7b (bible § 3.3) : part de saturation gardée par l'albédo du sol et de l'herbe de bataille
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
	var prepared: Array = []
	for grade in grades:
		if grade is Dictionary and not (grade as Dictionary).is_empty():
			prepared.append(_prepare(grade))
	var layers: Array[Image] = []
	var scale := 1.0 / float(size - 1)
	for b in size:
		var image := Image.create_empty(size, size, false, Image.FORMAT_RGB8)
		for g in size:
			for r in size:
				var source := Vector3(r * scale, g * scale, b * scale)
				var color := source
				for grade in prepared:
					color = _grade(color, grade)
				color = source.lerp(color, strength)
				image.set_pixel(r, g, Color(clampf(color.x, 0.0, 1.0), clampf(color.y, 0.0, 1.0), clampf(color.z, 0.0, 1.0)))
		layers.append(image)
	var texture := ImageTexture3D.new()
	texture.create(Image.FORMAT_RGB8, size, size, size, false, layers)
	_lut_cache[key] = texture
	return texture


## Paramètres d'un étalonnage convertis en vecteurs (évite les accès au dictionnaire par texel).
static func _prepare(grade: Dictionary) -> Dictionary:
	var temperature := float(grade.get("temperature", 0.0))
	var tint := float(grade.get("tint", 0.0))
	var balance := Vector3(1.0 + 0.1 * temperature, 1.0 - 0.06 * tint, 1.0 - 0.12 * temperature)
	return {
		"balance": balance,
		"lift": _vec(grade.get("lift", [0, 0, 0]), Vector3.ZERO),
		"gamma": _vec(grade.get("gamma", [1, 1, 1]), Vector3.ONE),
		"gain": _vec(grade.get("gain", [1, 1, 1]), Vector3.ONE),
		"contrast": float(grade.get("contrast", 1.0)),
		"saturation": float(grade.get("saturation", 1.0)),
		"shadow_saturation": float(grade.get("shadow_saturation", 1.0)),
		"shadows": _vec(grade.get("shadows", [1, 1, 1]), Vector3.ONE),
		"highlights": _vec(grade.get("highlights", [1, 1, 1]), Vector3.ONE),
	}


static func _vec(values: Variant, fallback: Vector3) -> Vector3:
	if values is Array and (values as Array).size() >= 3:
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	return fallback


## Un texel : balance des blancs, lift/gain, gamma, virage ombres/lumières, contraste (pivot
## 0,45, adouci en S), saturation (luminance conservée).
static func _grade(c: Vector3, g: Dictionary) -> Vector3:
	c = c * (g["balance"] as Vector3)
	var lift: Vector3 = g["lift"]
	c = (c + lift * (Vector3.ONE - c)) * (g["gain"] as Vector3)
	var gamma: Vector3 = g["gamma"]
	c = Vector3(pow(maxf(c.x, 0.0), 1.0 / gamma.x), pow(maxf(c.y, 0.0), 1.0 / gamma.y), pow(maxf(c.z, 0.0), 1.0 / gamma.z))
	var luma := c.dot(Vector3(0.2126, 0.7152, 0.0722))
	var toning := (g["shadows"] as Vector3).lerp(g["highlights"] as Vector3, smoothstep(0.0, 1.0, luma))
	c = c * toning
	var contrast := float(g["contrast"])
	if not is_equal_approx(contrast, 1.0):
		var pivot := 0.45
		var linear := Vector3.ONE * pivot + (c - Vector3.ONE * pivot) * contrast
		# Les extrêmes sont adoucis (épaule et pied) pour ne pas écrêter.
		c = Vector3(_soft_clip(linear.x), _soft_clip(linear.y), _soft_clip(linear.z))
	var saturation := float(g["saturation"])
	if not is_equal_approx(saturation, 1.0):
		luma = c.dot(Vector3(0.2126, 0.7152, 0.0722))
		c = Vector3.ONE * luma + (c - Vector3.ONE * luma) * saturation
	# DA7b : ombres désaturées (la saturation HSV des tons sombres gonfle : max - min rapporté à
	# un max faible) ; pleine sur les tons sombres, nulle au-delà de la luminance 0,5.
	var shadow_saturation := float(g["shadow_saturation"])
	if not is_equal_approx(shadow_saturation, 1.0):
		luma = c.dot(Vector3(0.2126, 0.7152, 0.0722))
		var keep := lerpf(shadow_saturation, 1.0, smoothstep(0.0, 0.5, luma))
		c = Vector3.ONE * luma + (c - Vector3.ONE * luma) * keep
	return c


static func _soft_clip(x: float) -> float:
	if x < 0.05:
		return 0.05 * exp((x - 0.05) / 0.05) if x > -1.0 else 0.0
	if x > 0.95:
		return 1.0 - 0.05 * exp(-(x - 0.95) / 0.05)
	return x


# --- Saison de campagne ---------------------------------------------------------------------


## Saison ("spring"…) d'un libellé de date de la simulation (« Printemps 1337 »), "" si inconnue.
static func season_of_date(label: String) -> String:
	var words := {"printemps": "spring", "été": "summer", "ete": "summer", "automne": "autumn", "hiver": "winter"}
	var parts := label.strip_edges().split(" ", false)
	if parts.is_empty():
		return ""
	return str(words.get(parts[0].to_lower(), ""))
