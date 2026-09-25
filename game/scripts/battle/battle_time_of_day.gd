class_name BattleTimeOfDay
extends RefCounted

## EP8 : lumière des batailles selon l'heure (aube, matin, midi, après-midi, crépuscule, nuit).
## Rendu seulement : l'heure et sa règle (visibilité des tireurs) viennent du cœur
## (`BattleSim.get_time_of_day()`). Les images clés de `data/fx/battle_staging.json`
## (`time_of_day.keyframes`) sont interpolées selon l'heure et appliquées **en facteurs** sur le
## préréglage météo posé par `BattleAtmosphere.apply` (capturé une fois par `capture`) : élévation
## et lacet du soleil (longueur et direction des ombres), énergie et couleur du soleil, couleurs
## du ciel, du brouillard et de l'ambiance, exposition. À midi toutes les valeurs sont neutres :
## le rendu est exactement celui d'avant EP8. Hors plein jour (clé `hdri: false`), le ciel HDRI
## laisse la place au ciel procédural (`battle_sky.gdshader`) teinté, dont le disque et le halo du
## soleil suivent la lumière rasante ; le panorama EP2 est reteinté par `BattleHorizon`.

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")

var keyframes: Array = []
var weather: String = "clear"
## Facteur d'énergie courant (1 en plein jour, ~0,15 la nuit) : ombres des nuages, feux de camp.
var light_level: float = 1.0
var current: Dictionary = {}

var _env: Environment = null
var _sun: DirectionalLight3D = null
var _base: Dictionary = {}
var _hdri_sky: Sky = null
var _proc_sky: Sky = null
var _proc_mat: ShaderMaterial = null
var _using_hdri: bool = true


func _init(p_keyframes: Array = []) -> void:
	keyframes = p_keyframes


## Mémorise l'éclairage de la météo (préréglage `BattleAtmosphere`) : base des facteurs.
func capture(env: Environment, sun: DirectionalLight3D, p_weather: String) -> void:
	_env = env
	_sun = sun
	weather = p_weather
	var preset: Dictionary = BattleAtmosphere.PRESETS.get(weather, BattleAtmosphere.PRESETS["clear"])
	_base = {
		"elevation": -rad_to_deg(sun.rotation.x),
		"energy": sun.light_energy,
		"sun_color": sun.light_color,
		"shadow_opacity": sun.shadow_opacity,
		"fog_color": env.fog_light_color,
		"ambient_energy": env.ambient_light_energy,
		"ambient_color": env.ambient_light_color,
		"exposure": env.tonemap_exposure,
		"volumetric_albedo": env.volumetric_fog_albedo,
		"zenith": preset["zenith"],
		"horizon": preset["horizon"],
		"coverage": preset["coverage"],
		"cloud_light": preset["cloud_light"],
		"cloud_shadow": preset["cloud_shadow"],
		"sky_energy": preset["sky_energy"],
	}
	_hdri_sky = env.sky
	var hdri_mat := _hdri_sky.sky_material as ShaderMaterial if _hdri_sky != null else null
	_using_hdri = hdri_mat != null and hdri_mat.shader != SKY_SHADER
	if hdri_mat != null:
		_base["sky_tint"] = hdri_mat.get_shader_parameter("tint") if _using_hdri else Color.WHITE
		_base["sky_exposure"] = hdri_mat.get_shader_parameter("exposure")
	_proc_mat = ShaderMaterial.new()
	_proc_mat.shader = SKY_SHADER
	_proc_mat.set_shader_parameter("cloud_coverage", _base["coverage"])
	_proc_sky = Sky.new()
	_proc_sky.sky_material = _proc_mat
	_proc_sky.radiance_size = Sky.RADIANCE_SIZE_128


## Images clés interpolées à `hour` (0-24) ; la dernière rejoint la première après minuit.
func sample(hour: float) -> Dictionary:
	if keyframes.is_empty():
		return {}
	hour = fposmod(hour, 24.0)
	var count := keyframes.size()
	var prev: Dictionary = keyframes[count - 1]
	var next: Dictionary = keyframes[0]
	for i in count:
		var k: Dictionary = keyframes[i]
		if float(k["hour"]) > hour:
			next = k
			prev = keyframes[(i - 1 + count) % count]
			break
		prev = k
		next = keyframes[(i + 1) % count]
	var h0 := float(prev["hour"])
	var h1 := float(next["hour"])
	var span := fposmod(h1 - h0, 24.0)
	var t := 0.0 if span <= 0.0 else clampf(fposmod(hour - h0, 24.0) / span, 0.0, 1.0)
	t = t * t * (3.0 - 2.0 * t)
	var out := {}
	for key in prev:
		var a: Variant = prev[key]
		var b: Variant = next.get(key, a)
		if key == "hdri":
			out[key] = bool(a) if t < 0.5 else bool(b)
		elif a is Array:
			out[key] = _rgb(a).lerp(_rgb(b), t)
		else:
			out[key] = lerpf(float(a), float(b), t)
	out["hour"] = hour
	return out


## Applique la lumière de `hour` à l'environnement et au soleil capturés.
func apply(hour: float) -> void:
	if _env == null or _sun == null or _base.is_empty():
		return
	var k := sample(hour)
	if k.is_empty():
		return
	current = k
	var elevation := clampf(float(_base["elevation"]) * float(k["elevation_mul"]), 2.0, 80.0)
	_sun.rotation = Vector3(deg_to_rad(-elevation), deg_to_rad(float(k["sun_yaw_deg"])), 0.0)
	light_level = float(k["energy_mul"])
	_sun.light_energy = float(_base["energy"]) * light_level
	_sun.light_color = (_base["sun_color"] as Color) * (k["sun_tint"] as Color)
	_sun.shadow_opacity = minf(float(_base["shadow_opacity"]), float(k["shadow_opacity"]))
	var fog_tint: Color = k["fog_tint"]
	_env.fog_light_color = (_base["fog_color"] as Color) * fog_tint
	_env.volumetric_fog_albedo = (_base["volumetric_albedo"] as Color) * fog_tint
	_env.ambient_light_energy = float(_base["ambient_energy"]) * float(k["ambient_mul"])
	_env.ambient_light_color = (_base["ambient_color"] as Color) * fog_tint
	_env.tonemap_exposure = float(_base["exposure"]) * float(k["exposure"])
	var want_hdri := _using_hdri and bool(k["hdri"])
	if want_hdri:
		if _env.sky != _hdri_sky:
			_env.sky = _hdri_sky
		var mat := _hdri_sky.sky_material as ShaderMaterial
		mat.set_shader_parameter("tint", (_base["sky_tint"] as Color) * (k["horizon_tint"] as Color))
	else:
		_proc_mat.set_shader_parameter("zenith_color", (_base["zenith"] as Color) * (k["zenith_tint"] as Color))
		_proc_mat.set_shader_parameter("horizon_color", (_base["horizon"] as Color) * (k["horizon_tint"] as Color))
		_proc_mat.set_shader_parameter("ground_color", ((_base["horizon"] as Color) * (k["horizon_tint"] as Color)).darkened(0.45))
		_proc_mat.set_shader_parameter("cloud_light", (_base["cloud_light"] as Color) * (k["sun_tint"] as Color).lerp(Color.WHITE, 0.35) * clampf(light_level * 1.3, 0.15, 1.0))
		_proc_mat.set_shader_parameter("cloud_shadow", (_base["cloud_shadow"] as Color) * (k["zenith_tint"] as Color))
		_proc_mat.set_shader_parameter("sun_disc", float(k["sun_disc"]) if weather == "clear" else 0.0)
		_proc_mat.set_shader_parameter("exposure", float(_base["sky_energy"]))
		if _env.sky != _proc_sky:
			_env.sky = _proc_sky


## Libellé HUD : « Crépuscule, 18 h 40 ».
static func clock_label(tod: Dictionary) -> String:
	if tod.is_empty():
		return ""
	var hour := fposmod(float(tod.get("hour", 12.0)), 24.0)
	var h := int(floor(hour))
	var m := int(floor((hour - h) * 60.0 / 10.0)) * 10
	return "%s, %d h %02d" % [str(tod.get("label", "")), h, m]


static func _rgb(values: Variant) -> Color:
	if values is Color:
		return values
	if values is Array and (values as Array).size() >= 3:
		return Color(float(values[0]), float(values[1]), float(values[2]))
	return Color.WHITE
