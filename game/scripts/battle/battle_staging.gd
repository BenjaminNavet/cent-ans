class_name BattleStaging
extends Node3D

## EP8 (ADR 0055) : mise en scène des batailles, rendu seulement. Orchestre :
## - la lumière selon l'heure (`BattleTimeOfDay`) : l'heure vient du cœur
##   (`BattleSim.get_time_of_day()`, tirée par la campagne ou choisie en bataille rapide) et
##   avance avec la bataille ; le panorama EP2 est reteinté ;
## - les ombres de nuages (`BattleCloudShadows`) ;
## - la poussière enrichie des charges et des colonnes (`BattleEffects.configure_staging`) ;
## - les fumées (`BattleSmoke`) : feux de camp derrière les lignes, fumée des bombardes qui
##   s'attarde, colonnes des maisons en feu (incendies S2 des sièges) ;
## - les oiseaux (`BattleBirds`) ;
## - le plan cinématique au premier choc (`BattleCinematic`).
## Chaque effet suit les niveaux de qualité PF1. Paramètres : `data/fx/battle_staging.json`.
##
## API pour les autres lots (EP6 : camps) : `add_smoke_source(position, intensity, kind)`,
## `remove_smoke_source(id)` ; `auto_campfires = false` avant `setup` pour ne pas poser les feux
## de camp par défaut.

const FX_PATH := "fx/battle_staging.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

var cfg: Dictionary = {}
var auto_campfires: bool = true
var time_of_day: BattleTimeOfDay = null
var clouds: BattleCloudShadows = null
var smoke: BattleSmoke = null
var birds: BattleBirds = null
var cinematic: BattleCinematic = null
## Dernier état de l'heure lu au cœur (`get_time_of_day`).
var tod: Dictionary = {}

var _scene: Node = null
var _battle: Object = null
var _env: Environment = null
var _sun: DirectionalLight3D = null
var _horizon: Object = null
var _weather: String = "clear"
var _tod_timer: float = 0.0
var _last_hour: float = -100.0
var _fire_timer: float = 0.0
var _fire_sources: Dictionary = {}  # clé de maison -> id de source
var _height_at: Callable




static func load_config() -> Dictionary:
	if DataFile.exists(FX_PATH):
		var parsed: Variant = DataFile.read_json(FX_PATH)
		if parsed is Dictionary:
			return parsed
	return {}




## Branche la mise en scène sur la bataille `scene` (`BattleScene`, après `terrain.build` et
## `BattleAtmosphere.apply`). `terrain_data` : `get_terrain()`.
func setup(scene: Node, battle: Object, env: Environment, sun: DirectionalLight3D, weather: String, terrain_data: Dictionary) -> void:
	name = "Staging"
	_scene = scene
	_battle = battle
	_env = env
	_sun = sun
	_weather = weather
	cfg = load_config()
	var terrain: Node = scene.get("terrain")
	_horizon = terrain.get("horizon") if terrain != null else null
	var width := float(terrain_data.get("width", 1200.0))
	var depth := float(terrain_data.get("depth", 800.0))
	var height_at := func(x: float, z: float) -> float: return float(terrain.call("height_at", x, z))
	_height_at = height_at
	var center := Vector3(width * 0.5, 0.0, depth * 0.5)
	center.y = height_at.call(center.x, center.z)
	var wind := BattleStandards.wind_for(weather, int(scene.get("battle_seed")))
	if cfg.has("time_of_day"):
		time_of_day = BattleTimeOfDay.new((cfg["time_of_day"] as Dictionary).get("keyframes", []))
		time_of_day.capture(env, sun, weather)
		_refresh_time(true)
	if cfg.has("cloud_shadows"):
		clouds = BattleCloudShadows.new()
		add_child(clouds)
		# Le champ et ses abords (bois de l'anneau proche) : 600 m de marge de chaque côté.
		clouds.setup(cfg["cloud_shadows"], center, Vector2(width + 1200.0, depth + 1200.0), weather, wind, int(scene.get("battle_seed")) + 3)
		clouds.set_light_level(time_of_day.light_level if time_of_day != null else 1.0)
	if cfg.has("smoke"):
		smoke = BattleSmoke.new()
		add_child(smoke)
		smoke.setup(cfg["smoke"], wind)
		smoke.set_light_level(time_of_day.light_level if time_of_day != null else 1.0)
		if auto_campfires and terrain_data.get("siege") == null:
			_place_campfires(terrain_data, height_at)
	if cfg.has("birds"):
		birds = BattleBirds.new()
		add_child(birds)
		birds.setup(cfg["birds"], terrain_data, height_at, int(scene.get("battle_seed")) + 17)
	if cfg.has("cinematic"):
		cinematic = BattleCinematic.new()
		add_child(cinematic)
		var rig: Node3D = scene.get("camera_rig")
		cinematic.setup(cfg["cinematic"], scene, rig, rig.get("camera") if rig != null else null)
		var settings := scene.get_node_or_null("/root/Settings")
		if settings != null:
			cinematic.enabled = bool(settings.call("get_value", "battle/cinematic"))
			cinematic.slowmo = bool(settings.call("get_value", "battle/cinematic_slowmo"))


## Poussière enrichie (B4) : réglages de `dust` passés aux effets de la bataille.
func configure_effects(effects: BattleEffects, terrain_key: String, season: String) -> void:
	if effects == null or not cfg.has("dust"):
		return
	effects.configure_staging(cfg["dust"], terrain_key, season)


## Pose une source de fumée durable (EP6 : camps) ; -1 si coupé ou hors budget.
func add_smoke_source(position: Vector3, intensity: float = 1.0, kind: String = "campfire") -> int:
	if smoke == null:
		return -1
	return smoke.add_smoke_source(position, intensity, kind)


func remove_smoke_source(id: int) -> void:
	if smoke != null:
		smoke.remove_smoke_source(id)


## Fumée de bombarde qui s'attarde (branchée sur `BattleEffects.cannon_fired`).
func on_cannon_fired(muzzle: Vector3) -> void:
	if smoke != null:
		smoke.bombard_smoke(muzzle + Vector3(0, 1.0, 0))


## Une image : `units` (`get_units`), `dt` temps de bataille écoulé, `real_dt` temps réel.
func update(units: Array, dt: float, real_dt: float, finished: bool) -> void:
	_tod_timer -= real_dt
	if _tod_timer <= 0.0:
		_tod_timer = float((cfg.get("time_of_day", {}) as Dictionary).get("update_period_s", 1.5))
		_refresh_time(false)
	if clouds != null:
		clouds.advance(dt)
	if birds != null:
		birds.update(units, dt, finished)
	if smoke != null and _battle != null:
		_fire_timer -= real_dt
		if _fire_timer <= 0.0:
			_fire_timer = 1.0
			_sync_fires()
	if cinematic != null and not finished:
		cinematic.check(units, Callable(_scene, "_closeup_shot"))


## Facteur de temps de la bataille (ralenti du plan cinématique).
func time_scale() -> float:
	return cinematic.time_scale if cinematic != null and cinematic.active else 1.0


func _refresh_time(force: bool) -> void:
	if _battle == null:
		return
	tod = _battle.call("get_time_of_day")
	if tod.is_empty() or time_of_day == null:
		return
	var hour := float(tod.get("hour", 12.0))
	# Le jour avance de 0,2 min par seconde de bataille : pas besoin de recalculer le ciel
	# (carte de radiance) plus souvent que toutes les 3 minutes de jour.
	if not force and absf(hour - _last_hour) < 0.05:
		return
	_last_hour = hour
	time_of_day.apply(hour)
	if _horizon != null:
		_horizon.call("apply_atmosphere", _env, _sun, _weather)
	if clouds != null:
		clouds.set_light_level(time_of_day.light_level)
	if smoke != null:
		smoke.set_light_level(time_of_day.light_level)


## Feux de camp derrière la ligne de chaque camp (EP6 posera les siens : `auto_campfires`).
func _place_campfires(terrain_data: Dictionary, height_at: Callable) -> void:
	var no_fire: Array = (cfg["smoke"] as Dictionary).get("no_campfire_weather", [])
	if no_fire.has(_weather):
		return
	var per_side: Dictionary = (cfg["smoke"] as Dictionary).get("camp_fires_per_side", {})
	var count := int(per_side.get(RenderQuality.current(), per_side.get("high", 2)))
	var width := float(terrain_data.get("width", 1200.0))
	var depth := float(terrain_data.get("depth", 800.0))
	var back := float((cfg["smoke"] as Dictionary).get("camp_depth_m", 170.0))
	var lines := {
		"attacker": float(terrain_data.get("attacker_line_z", depth * 0.5 - 150.0)) - back,
		"defender": float(terrain_data.get("defender_line_z", depth * 0.5 + 150.0)) + back,
	}
	var rng := RandomNumberGenerator.new()
	rng.seed = int(_scene.get("battle_seed")) + 5
	for side in lines:
		var z := clampf(float(lines[side]), 20.0, depth - 20.0)
		for i in count:
			var x := width * (0.3 + 0.4 * (i + 0.5) / count) + rng.randf_range(-40.0, 40.0)
			var p := Vector3(x, 0.0, z + rng.randf_range(-25.0, 25.0))
			p.y = float(height_at.call(p.x, p.z))
			add_smoke_source(p, rng.randf_range(0.6, 1.0), "campfire")


## Incendies S2 (sièges) : une colonne sombre par maison en feu, visible de loin.
func _sync_fires() -> void:
	# PB3c : lecture de l'image partagée avec la scène quand elle existe.
	var frame_siege: Variant = _scene.get("_frame_siege") if _scene != null else null
	var siege: Dictionary = frame_siege if frame_siege is Dictionary and not (frame_siege as Dictionary).is_empty() else _battle.call("get_siege")
	if siege.is_empty():
		return
	var seen := {}
	var houses: Array = siege.get("houses", [])
	for i in houses.size():
		var house: Dictionary = houses[i]
		var fire: Variant = house.get("fire", null)
		if not (fire is Dictionary) or str((fire as Dictionary).get("state", "")) != "burning":
			continue
		var intensity := float((fire as Dictionary).get("intensity", 0.5))
		seen[i] = true
		if _fire_sources.has(i):
			smoke.set_smoke_intensity(int(_fire_sources[i]), intensity)
		else:
			var p := Vector3(float(house.get("x", 0.0)), 0.0, float(house.get("z", 0.0)))
			p.y = float(_height_at.call(p.x, p.z)) + 8.0
			var id := smoke.add_smoke_source(p, intensity, "column")
			if id >= 0:
				_fire_sources[i] = id
	for key in _fire_sources.keys():
		if not seen.has(key):
			smoke.remove_smoke_source(int(_fire_sources[key]))
			_fire_sources.erase(key)
