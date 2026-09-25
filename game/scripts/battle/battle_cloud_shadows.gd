class_name BattleCloudShadows
extends Node3D

## EP8 : ombres de nuages qui glissent sur le champ de bataille. Une texture de bruit
## (`cloud_shadow_noise.gdshader`, rendue dans un `SubViewport` de `texture_px` pixels) est
## projetée d'en haut par un seul `Decal` qui couvre le champ et ses abords : relief, soldats,
## bâtiments et arbres s'assombrissent sous les nuages. Le bruit défile dans le sens du vent
## (`BattleStandards.wind_for`, même vent que les étendards) à `wind_speed_m_s` × force du vent.
## Intensité et couverture selon la météo (`data/fx/battle_staging.json`, `cloud_shadows`),
## atténuées quand la lumière baisse (aube, crépuscule, nuit : `set_light_level`). Coupé au
## niveau de qualité Basse (PF1) et par `--no-cloud-shadows`.

const NOISE_SHADER := preload("res://shaders/cloud_shadow_noise.gdshader")
const DECAL_HEIGHT := 900.0

var cfg: Dictionary = {}
var wind_dir: Vector2 = Vector2(1, 0)
var wind_strength: float = 0.5
var base_strength: float = 0.0
var coverage: float = 0.0
var light_level: float = 1.0
## Décalage courant du bruit (tuiles) : avance avec le temps de bataille.
var offset: Vector2 = Vector2.ZERO

var _viewport: SubViewport = null
var _mat: ShaderMaterial = null
var _decal: Decal = null
var _size: Vector2 = Vector2(1200, 800)
var _refresh: float = 0.0
var _quality_on: bool = true


## `center`/`size` : emprise au sol (x, z) ; `weather` : clé météo du rendu ; `wind` :
## `BattleStandards.wind_for`.
func setup(p_cfg: Dictionary, center: Vector3, size: Vector2, weather: String, wind: Dictionary) -> void:
	cfg = p_cfg
	name = "CloudShadows"
	_size = size
	var by_weather: Dictionary = cfg.get("by_weather", {})
	var w: Dictionary = by_weather.get(weather, by_weather.get("clear", {}))
	base_strength = float(w.get("strength", 0.0))
	coverage = float(w.get("coverage", 0.0))
	wind_dir = (wind.get("dir", Vector2(1, 0)) as Vector2).normalized()
	wind_strength = float(wind.get("strength", 0.5))
	var px := int(cfg.get("texture_px", 256))
	_viewport = SubViewport.new()
	_viewport.name = "NoiseViewport"
	_viewport.size = Vector2i(px, px)
	_viewport.transparent_bg = true
	_viewport.disable_3d = true
	_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(_viewport)
	var rect := ColorRect.new()
	rect.size = Vector2(px, px)
	_mat = ShaderMaterial.new()
	_mat.shader = NOISE_SHADER
	var tile := float(cfg.get("tile_m", 2400.0))
	_mat.set_shader_parameter("tiles", Vector2(size.x / tile, size.y / tile))
	_mat.set_shader_parameter("coverage", coverage)
	rect.material = _mat
	_viewport.add_child(rect)
	_decal = Decal.new()
	_decal.name = "CloudDecal"
	_decal.size = Vector3(size.x, DECAL_HEIGHT, size.y)
	_decal.position = center + Vector3(0, DECAL_HEIGHT * 0.25, 0)
	_decal.texture_albedo = _viewport.get_texture()
	_decal.albedo_mix = 1.0
	_decal.upper_fade = 0.0
	_decal.lower_fade = 0.0
	_decal.distance_fade_enabled = false
	_decal.cull_mask = 0xFFFFF
	add_child(_decal)
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	_update_material()


## Lumière de l'heure (1 plein jour) : les ombres pâlissent au crépuscule, disparaissent la nuit.
func set_light_level(level: float) -> void:
	light_level = level
	_update_material()


## Intensité effective (0 : rien à dessiner).
func effective_strength() -> float:
	var night_mul := float(cfg.get("night_strength_mul", 0.0))
	var light := clampf((light_level - 0.3) / 0.6, 0.0, 1.0)
	return base_strength * lerpf(night_mul, 1.0, light)


## Avance le bruit de `dt` secondes de bataille (vent).
func advance(dt: float) -> void:
	if _decal == null or not _decal.visible:
		return
	var speed := float(cfg.get("wind_speed_m_s", 7.0)) * maxf(wind_strength, 0.15)
	var tile := float(cfg.get("tile_m", 2400.0))
	offset += wind_dir * speed * dt / tile
	# Le bruit est périodique (4 tuiles) : le décalage reste borné sans saut.
	offset = Vector2(fposmod(offset.x, 4.0), fposmod(offset.y, 4.0))
	_refresh -= dt
	if _refresh <= 0.0:
		_refresh = float(cfg.get("update_period_s", 0.25))
		# Le vent vient de `wind_dir` : le motif avance vers +wind (UV x = monde x, UV y = monde z).
		_mat.set_shader_parameter("offset", -offset)
		_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE


func apply_render_quality(_preset: Dictionary) -> void:
	var quality: Dictionary = cfg.get("quality", {})
	_quality_on = bool(quality.get(RenderQuality.current(), true))
	_update_material()


func _update_material() -> void:
	if _mat == null or _decal == null:
		return
	var s := effective_strength()
	_mat.set_shader_parameter("strength", s)
	_decal.visible = _quality_on and s > 0.01 and coverage > 0.0
	if _viewport != null:
		_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
