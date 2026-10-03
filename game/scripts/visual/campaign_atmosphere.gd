class_name CampaignAtmosphere
extends Node

## Atmosphère de la carte de campagne (lot V1, ADR 0004) : oriente le soleil (lumière
## rasante venue du nord-ouest, convention cartographique qui fait « sortir » le relief)
## et adapte à la distance de la caméra le brouillard de profondeur, la portée des ombres
## et le flou de profondeur (effet maquette de près, brume de l'horizon de loin).
## Lot V3 (A1-05, A1-14) : ciel HDRI et étalonnage de la saison courante (`AtmosphereLibrary`,
## `data/fx/atmosphere.json`), suivis au fil des tours ; effets réglés par `RenderQuality`.
## Purement visuel : aucune règle de jeu ici.

@export var environment_path: NodePath = ^"../WorldEnvironment"
@export var sun_path: NodePath = ^"../Sun"
@export var camera_rig_path: NodePath = ^"../CameraRig"

## Lot PO3 (bible DA § 12.6, ADR 0097) : soleil rasant de fin d'après-midi par saison (hauteur,
## azimut d'où vient la lumière — 0° = nord, sens horaire —, couleur, énergie), perspective
## aérienne teintée et étalonnage de carte, lus dans `atmosphere.json` (`campaign.seasons.<saison>`,
## `resolve_preset`). Aucune valeur de soleil dans le code : sans préréglage, le soleil de la scène
## reste tel quel.
var sun_elevation_deg: float = -1.0
var sun_azimuth_deg: float = -1.0

## Brouillard de profondeur, en multiples de la distance caméra → point visé.
@export var fog_begin_factor: float = 1.6
@export var fog_end_factor: float = 7.0
## Portée des ombres en multiples de la distance caméra ; au-delà de `shadow_max_camera_distance`,
## les ombres portées sont coupées (au dézoom, les marqueurs agrandis projetteraient des ombres
## démesurées et l'ombrage du relief suffit).
@export var shadow_range_factor: float = 3.5
@export var shadow_max_camera_distance: float = 650.0
## Flou de profondeur (lointain) : début et transition en multiples de la distance caméra,
## intensité maximale de près, nulle au-delà de `dof_max_distance`.
@export var dof_begin_factor: float = 1.7
@export var dof_transition_factor: float = 2.5
@export var dof_amount_near: float = 0.08
@export var dof_max_distance: float = 700.0
## Lot ZG4 : vue rapprochée (sous `close_begin_distance`, pleinement sous `close_full_distance`) :
## le brouillard ne se règle plus sur la distance caméra (il noierait l'horizon à 2 km en vue
## rasante) mais laisse voir crêtes et vallées lointaines (`close_fog_begin` / `close_fog_end`
## unités) ; flou de profondeur coupé ; ombres plus serrées ; rayon de l'occlusion ambiante
## (unités monde) proportionnel à la distance.
@export var close_begin_distance: float = 30.0
@export var close_full_distance: float = 6.0
@export var close_fog_begin: float = 18.0
@export var close_fog_end: float = 240.0
@export var close_shadow_min: float = 12.0
@export var ssao_radius_factor: float = 0.14
@export var ssao_radius_min: float = 0.03

var _environment: Environment
var _attributes: CameraAttributesPractical
var _sun: DirectionalLight3D
var _rig: CampaignCamera
var _last_distance: float = -1.0
var _world_env: WorldEnvironment
var _season: String = ""
var _season_check_s: float = 0.0
## PF1 : facteur de portée et cascades des ombres selon le préréglage (`RenderQuality`).
var _quality_range: float = 1.0
var _quality_splits: int = 4
var _base_ssao_radius: float = -1.0
## PO3 : soleil de la saison à (ré)appliquer : au chargement (le relief ZG8 impose sa hauteur au
## soleil pendant la construction du terrain) et au changement de saison, hors soir de fin de tour.
var _sun_dirty: bool = false
var _preset: Dictionary = {}
## RV-B (`CampaignLighting`, `data/fx/campaign_lighting.json`) : soleil, ambiance et brume visés
## (saison + phase du tour) et courants ; la lumière glisse de l'un à l'autre, sans saut.
var _turn: int = -1
var _sun_target: Dictionary = {}
var _sun_current: Dictionary = {}
var _ambient_target: Dictionary = {}
var _fog_target: Color = Color(0, 0, 0, 0)
var _blend_s: float = 6.0
var _lighting_settled: bool = true


func _ready() -> void:
	var world_env := get_node_or_null(environment_path) as WorldEnvironment
	if world_env != null:
		_environment = world_env.environment
		_attributes = world_env.camera_attributes as CameraAttributesPractical
	_world_env = world_env
	_sun = get_node_or_null(sun_path) as DirectionalLight3D
	_rig = get_node_or_null(camera_rig_path) as CampaignCamera
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	if _world_env != null:
		RenderQuality.register(_world_env, _sun, "campaign")
	if _environment != null:
		var exposure: Dictionary = CampaignLighting.data().get("exposure", {})
		if not exposure.is_empty():
			_environment.tonemap_exposure = float(exposure.get("tonemap_exposure", _environment.tonemap_exposure))
			_environment.tonemap_white = float(exposure.get("tonemap_white", _environment.tonemap_white))
	_update_season()


## RV-B : numéro du tour (`--light-turn=N` force la phase pour les captures), -1 sans simulation.
func current_turn() -> int:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--light-turn="):
			return int(arg.trim_prefix("--light-turn="))
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim != null and sim.has_method("get_turn"):
		return int(sim.call("get_turn"))
	return -1


## Saison de la simulation (libellé de date) ; ciel et étalonnage changent avec elle.
func current_season() -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--season="):  # captures : saison forcée (rendu seulement)
			return AtmosphereLibrary.normalize_season(arg.trim_prefix("--season="))
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim != null and sim.has_method("get_date_label"):
		var season := AtmosphereLibrary.season_of_date(str(sim.call("get_date_label")))
		if season != "":
			return season
	return "spring"


func _update_season() -> void:
	var season := current_season()
	if season != _season:
		apply_season(season)
	var turn := current_turn()
	if turn != _turn:
		_turn = turn
		_retarget_light("turn")


## PO3 : préréglage complet de la carte pour `season` : `campaign_look` (ciel, étalonnage de
## saison) augmenté de l'étalonnage de saison de la carte (TB1, `SeasonLook.grade`) puis de
## l'étalonnage de carte (`grade`, appliqué après), du soleil (`sun_elevation`,
## `sun_azimuth`, `sun_color`, `sun_energy`) et de la brume (`fog_color`, `fog_sun_scatter`).
## {} si la saison n'a pas de soleil dans les données.
static func resolve_preset(season: String) -> Dictionary:
	var resolved := AtmosphereLibrary.campaign_look(season)
	if resolved.is_empty():
		return {}
	var look: Dictionary = resolved["look"]
	for key in ["sun_elevation", "sun_azimuth", "sun_color", "fog_color"]:
		if not look.has(key):
			return {}
	var grades: Array = (resolved["grades"] as Array).duplicate()
	# TB1 (ADR 0150) : étalonnage de saison propre à la carte (`data/ui/campaign_seasons.json`),
	# entre l'étalonnage de saison commun aux batailles et l'étalonnage de carte.
	var season_grade := SeasonLook.grade(AtmosphereLibrary.normalize_season(season))
	if not season_grade.is_empty():
		grades.append(season_grade)
	if look.has("grade"):
		grades.append(look["grade"])
	resolved["grades"] = grades
	resolved["sun_elevation"] = float(look["sun_elevation"])
	resolved["sun_azimuth"] = float(look["sun_azimuth"])
	resolved["sun_color"] = AtmosphereLibrary._rgb(look["sun_color"])
	resolved["sun_energy"] = float(look.get("sun_energy", 1.0))
	resolved["fog_color"] = AtmosphereLibrary._rgb(look["fog_color"])
	resolved["fog_sun_scatter"] = float(look.get("fog_sun_scatter", 0.0))
	return resolved


func apply_season(season: String) -> void:
	_season = season
	_preset = resolve_preset(season)
	if _preset.is_empty():
		push_warning("CampaignAtmosphere: no complete preset for season %s" % season)
		return
	_retarget_light("season")
	if _environment == null:
		return
	var fog: Color = _preset["fog_color"]
	_environment.fog_sun_scatter = float(_preset["fog_sun_scatter"])
	AtmosphereLibrary.apply_to_environment(_environment, _preset, fog, fog.darkened(0.5))


## RV-B : nouvelle cible de lumière (saison ou tour) ; la première est posée d'emblée, les
## suivantes sont rejointes en `transition_s.<reason>` secondes.
func _retarget_light(reason: String) -> void:
	if _preset.is_empty():
		return
	var cfg := CampaignLighting.data()
	_sun_target = CampaignLighting.sun_target(_preset, _turn, cfg)
	_ambient_target = CampaignLighting.ambient(_season, cfg)
	_fog_target = CampaignLighting.fog_color(_preset["fog_color"], cfg)
	var durations: Dictionary = cfg.get("transition_s", {})
	_blend_s = maxf(0.05, float(durations.get(reason, 6.0)))
	_lighting_settled = false
	if _sun_current.is_empty() or not _map_loaded:  # avant la carte : posé d'emblée
		_sun_current = _sun_target.duplicate()
		_apply_environment_light(1.0)
	_sun_dirty = true
	_apply_sun_if_free()


## RV-B : soleil visé (saison + tour) : {elevation, azimuth, color, energy}, {} sans préréglage.
func light_target() -> Dictionary:
	return _sun_target


## RV-B : pose la lumière visée d'emblée (tests, captures), hors soir de fin de tour.
func settle_light() -> void:
	if _sun_target.is_empty():
		return
	_sun_current = _sun_target.duplicate()
	_apply_environment_light(1.0)
	_lighting_settled = true
	_sun_dirty = true
	_apply_sun_if_free()


## Ambiance et teinte de brume rapprochées de leur cible (fraction `k`).
func _apply_environment_light(k: float) -> void:
	if _environment == null:
		return
	if not _ambient_target.is_empty():
		_environment.ambient_light_color = _environment.ambient_light_color.lerp(_ambient_target["color"], k)
		_environment.ambient_light_energy = lerpf(_environment.ambient_light_energy, float(_ambient_target["energy"]), k)
		_environment.ambient_light_sky_contribution = lerpf(_environment.ambient_light_sky_contribution, float(_ambient_target["sky_contribution"]), k)
	if _fog_target.a > 0.0:
		_environment.fog_light_color = _environment.fog_light_color.lerp(_fog_target, k)


## RV-B : un pas d'interpolation de la lumière (soleil, ambiance, brume) vers la cible.
func _step_light(delta: float) -> void:
	if _lighting_settled or _sun_target.is_empty() or not _sun_free():
		return
	var k := 1.0 - exp(-delta * 3.0 / _blend_s)  # ≈ 95 % de l'écart comblé en `_blend_s`
	_sun_current = CampaignLighting.blend_sun(_sun_current, _sun_target, k)
	_apply_environment_light(k)
	if CampaignLighting.sun_gap(_sun_current, _sun_target) < 0.05:
		_sun_current = _sun_target.duplicate()
		_apply_environment_light(1.0)
		_lighting_settled = true
	_sun_dirty = true
	_apply_sun_if_free()


## PO3 : pose le soleil de la saison sauf pendant le soir doré de fin de tour (`TurnLight`, qui
## rend ensuite la lumière mémorisée) ; sinon réessayé à la vérification suivante.
func _apply_sun_if_free() -> void:
	if not _sun_dirty or _sun == null or _preset.is_empty() or _sun_current.is_empty():
		return
	if not _sun_free():
		return
	sun_elevation_deg = float(_sun_current["elevation"])
	sun_azimuth_deg = float(_sun_current["azimuth"])
	_orient_sun()
	_sun.light_color = _sun_current["color"]
	_sun.light_energy = float(_sun_current["energy"])
	_sun_dirty = false


## Faux pendant le soir doré de fin de tour (`TurnLight` mémorise puis rend la lumière).
func _sun_free() -> bool:
	var turn_light := get_node_or_null(^"../TurnLight")
	return turn_light == null or not (float(turn_light.get("dusk")) > 0.0 or bool(turn_light.get("_active")))


func _process(delta: float) -> void:
	if not _map_loaded:
		_check_map_loaded()
	_season_check_s -= delta
	if _season_check_s <= 0.0:
		_season_check_s = 1.0
		_update_season()
	_step_light(delta)
	if _sun_dirty:
		_apply_sun_if_free()
	if _rig == null:
		return
	var distance := _rig.distance
	# Pas de mise à jour tant que le zoom ne bouge pas de plus de 1 %.
	if _last_distance > 0.0 and absf(distance - _last_distance) < _last_distance * 0.01:
		return
	_last_distance = distance
	apply_distance(distance)


## Direction de propagation de la lumière : vers le sud-est et vers le bas pour un soleil au nord-ouest.
func sun_direction() -> Vector3:
	var elevation := deg_to_rad(sun_elevation_deg)
	var azimuth := deg_to_rad(sun_azimuth_deg)
	# Carte : +x = est, +z = sud. Un azimut 0° (nord) pointe vers -z.
	var toward_sun := Vector3(sin(azimuth) * cos(elevation), sin(elevation), -cos(azimuth) * cos(elevation))
	return -toward_sun.normalized()


func apply_render_quality(p: Dictionary) -> void:
	_quality_range = float(p.get("map_shadow_range", 1.0))
	_quality_splits = int(p.get("map_shadow_splits", 4))
	if _sun != null:
		_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS if _quality_splits >= 4 else DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
	if _last_distance > 0.0:
		apply_distance(_last_distance)


func apply_distance(distance: float) -> void:
	var close := 1.0 - smoothstep(close_full_distance, close_begin_distance, distance)
	if _environment != null:
		# RV-B : perspective aérienne selon l'inclinaison (nette en vue basse, légère de dessus).
		var begin_factor := fog_begin_factor
		var end_factor := fog_end_factor
		var aerial := CampaignLighting.aerial(_rig.pitch_deg() if _rig != null else 50.0)
		if not aerial.is_empty():
			begin_factor = float(aerial.get("begin_factor", begin_factor))
			end_factor = float(aerial.get("end_factor", end_factor))
			_environment.fog_density = float(aerial.get("density", _environment.fog_density))
			_environment.fog_aerial_perspective = float(aerial.get("aerial_perspective", _environment.fog_aerial_perspective))
			_environment.fog_sky_affect = float(aerial["sky_affect"])
		_environment.fog_depth_begin = lerpf(distance * begin_factor, maxf(close_fog_begin, distance * begin_factor), close)
		_environment.fog_depth_end = lerpf(distance * end_factor, maxf(close_fog_end, distance * end_factor), close)
		if _base_ssao_radius < 0.0:
			_base_ssao_radius = _environment.ssao_radius
		_environment.ssao_radius = clampf(distance * ssao_radius_factor, ssao_radius_min, _base_ssao_radius)
		if _environment.ssil_enabled:  # rayon posé par RenderQuality (12 en campagne), idem
			_environment.ssil_radius = clampf(distance * 0.55, 0.1, 12.0)
	if _sun != null:
		_sun.shadow_enabled = distance < shadow_max_camera_distance
		_sun.directional_shadow_max_distance = maxf(distance * shadow_range_factor * _quality_range, lerpf(50.0, close_shadow_min, close))
	if _attributes != null:
		var closeness := (1.0 - clampf(distance / dof_max_distance, 0.0, 1.0)) * (1.0 - close)
		_attributes.dof_blur_far_enabled = closeness > 0.0
		_attributes.dof_blur_far_distance = distance * dof_begin_factor
		_attributes.dof_blur_far_transition = distance * dof_transition_factor
		_attributes.dof_blur_amount = dof_amount_near * closeness


## PO3 : une fois la carte chargée (terrain construit, hauteur du soleil ZG8 posée), le soleil de
## la saison reprend la main.
var _map_loaded: bool = false


func _check_map_loaded() -> void:
	if _map_loaded:
		return
	var map := get_parent()
	if map != null and bool(map.get("load_ok")):
		_map_loaded = true
		_sun_dirty = true


func _orient_sun() -> void:
	if _sun == null or sun_elevation_deg < 0.0:
		return
	_sun.basis = Basis.looking_at(sun_direction(), Vector3.UP)
