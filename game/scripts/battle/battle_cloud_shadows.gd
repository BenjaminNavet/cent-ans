class_name BattleCloudShadows
extends Node3D

## EP8 : ombres de nuages qui glissent sur le champ de bataille. Une texture de bruit
## périodique (`FastNoiseLite.get_seamless_image`, une tuile de `tile_m` mètres répétée) est
## projetée d'en haut par un seul `Decal` qui couvre le champ et ses abords : relief, soldats,
## bâtiments et arbres s'assombrissent sous les nuages. Le décal glisse dans le sens du vent
## (`BattleStandards.wind_for`, même vent que les étendards) à `wind_speed_m_s` × force du vent,
## et revient d'une tuile en arrière quand il en a parcouru une (texture périodique : aucun
## saut visible). La texture est calculée une fois (aucune lecture GPU, aucun rendu hors écran).
## Intensité et couverture selon la météo (`data/fx/battle_staging.json`, `cloud_shadows`),
## atténuées quand la lumière baisse (aube, crépuscule, nuit : `set_light_level`). Coupé au
## niveau de qualité Basse (PF1).

const DECAL_HEIGHT := 900.0

var cfg: Dictionary = {}
var wind_dir: Vector2 = Vector2(1, 0)
var wind_strength: float = 0.5
var base_strength: float = 0.0
var coverage: float = 0.0
var light_level: float = 1.0
## Glissement courant du décal (m), dans [0, tile_m) sur chaque axe.
var shift: Vector2 = Vector2.ZERO

var _decal: Decal = null
var _center: Vector3 = Vector3.ZERO
var _tile: float = 1200.0
var _size: Vector2 = Vector2.ZERO
var _quality_on: bool = true


## `center`/`area` : emprise au sol (x, z) à couvrir ; `weather` : clé météo du rendu ; `wind` :
## `BattleStandards.wind_for` ; `seed_value` : graine du bruit.
func setup(p_cfg: Dictionary, center: Vector3, area: Vector2, weather: String, wind: Dictionary, seed_value: int = 1) -> void:
	cfg = p_cfg
	name = "CloudShadows"
	_center = center
	var by_weather: Dictionary = cfg.get("by_weather", {})
	var w: Dictionary = by_weather.get(weather, by_weather.get("clear", {}))
	base_strength = float(w.get("strength", 0.0))
	coverage = float(w.get("coverage", 0.0))
	wind_dir = (wind.get("dir", Vector2(1, 0)) as Vector2).normalized()
	wind_strength = float(wind.get("strength", 0.5))
	_tile = maxf(float(cfg.get("tile_m", 1200.0)), 100.0)
	# Couvre l'emprise quel que soit le glissement (une tuile de plus sur chaque axe).
	var tiles := Vector2i(int(ceil(area.x / _tile)) + 1, int(ceil(area.y / _tile)) + 1)
	_size = Vector2(tiles.x * _tile, tiles.y * _tile)
	add_to_group(RenderQuality.CLIENT_GROUP)
	_quality_on = _quality_allows()
	if base_strength <= 0.0 or coverage <= 0.0:
		return
	_decal = Decal.new()
	_decal.name = "CloudDecal"
	_decal.size = Vector3(_size.x, DECAL_HEIGHT, _size.y)
	_decal.texture_albedo = _texture(tiles, int(cfg.get("texture_px", 256)) / 2, seed_value)
	_decal.albedo_mix = 1.0
	_decal.upper_fade = 0.0
	_decal.lower_fade = 0.0
	_decal.distance_fade_enabled = false
	add_child(_decal)
	_place()
	_update_strength()


## Lumière de l'heure (1 plein jour) : les ombres pâlissent au crépuscule, disparaissent la nuit.
func set_light_level(level: float) -> void:
	light_level = level
	_update_strength()


## Intensité effective (0 : rien à dessiner).
func effective_strength() -> float:
	var night_mul := float(cfg.get("night_strength_mul", 0.0))
	var light := clampf((light_level - 0.3) / 0.6, 0.0, 1.0)
	return base_strength * lerpf(night_mul, 1.0, light)


## Avance le décal de `dt` secondes de bataille (vent).
func advance(dt: float) -> void:
	if _decal == null or not _decal.visible or dt <= 0.0:
		return
	var speed := float(cfg.get("wind_speed_m_s", 7.0)) * maxf(wind_strength, 0.15)
	shift += wind_dir * speed * dt
	shift = Vector2(fposmod(shift.x, _tile), fposmod(shift.y, _tile))
	_place()


func apply_render_quality(_preset: Dictionary) -> void:
	_quality_on = _quality_allows()
	_update_strength()


func _quality_allows() -> bool:
	return bool((cfg.get("quality", {}) as Dictionary).get(RenderQuality.current(), true))


func _place() -> void:
	_decal.position = _center + Vector3(shift.x - _tile * 0.5, DECAL_HEIGHT * 0.25, shift.y - _tile * 0.5)


func _update_strength() -> void:
	if _decal == null:
		return
	var s := effective_strength()
	_decal.modulate = Color(1, 1, 1, s)
	_decal.visible = _quality_on and s > 0.01


## Texture des ombres : une tuile de bruit sans couture (`px` × `px`), noire, dont l'alpha est
## la densité du nuage (seuil selon `coverage`), répétée `tiles` fois.
func _texture(tiles: Vector2i, px: int, seed_value: int) -> ImageTexture:
	px = clampi(px, 32, 512)
	var noise := FastNoiseLite.new()
	noise.seed = seed_value
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = float(cfg.get("noise_cycles", 3.0)) / float(px)
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = int(cfg.get("octaves", 2))
	var tile_img := noise.get_seamless_image(px, px, false, false, 0.1, true)
	tile_img.convert(Image.FORMAT_L8)
	var values := tile_img.get_data()
	var data := PackedByteArray()
	data.resize(px * px * 2)
	var edge := 1.0 - coverage
	var softness := maxf(float(cfg.get("edge_softness", 0.2)), 0.05)
	for i in px * px:
		var n := values[i] / 255.0
		var t := clampf((n - (edge - 0.08)) / softness, 0.0, 1.0)
		data[i * 2] = 0
		data[i * 2 + 1] = int(t * t * (3.0 - 2.0 * t) * 255.0)
	var tile := Image.create_from_data(px, px, false, Image.FORMAT_LA8, data)
	var image := Image.create(px * tiles.x, px * tiles.y, false, Image.FORMAT_LA8)
	for ty in tiles.y:
		for tx in tiles.x:
			image.blit_rect(tile, Rect2i(0, 0, px, px), Vector2i(tx * px, ty * px))
	image.generate_mipmaps()
	return ImageTexture.create_from_image(image)
