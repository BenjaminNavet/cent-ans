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

## Hauteur du soleil au-dessus de l'horizon et azimut d'où vient la lumière (0° = nord, 315° = nord-ouest).
@export_range(5.0, 89.0) var sun_elevation_deg: float = 38.0
@export_range(0.0, 360.0) var sun_azimuth_deg: float = 315.0

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
var _base_ssao_radius: float = -1.0


func _ready() -> void:
	var world_env := get_node_or_null(environment_path) as WorldEnvironment
	if world_env != null:
		_environment = world_env.environment
		_attributes = world_env.camera_attributes as CameraAttributesPractical
	_world_env = world_env
	_sun = get_node_or_null(sun_path) as DirectionalLight3D
	_rig = get_node_or_null(camera_rig_path) as CampaignCamera
	_orient_sun()
	if _world_env != null:
		RenderQuality.register(_world_env, _sun, "campaign")
	_update_season()


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


func apply_season(season: String) -> void:
	_season = season
	if _environment == null:
		return
	var look := AtmosphereLibrary.campaign_look(season)
	if look.is_empty():
		return
	var fog := AtmosphereLibrary._rgb((look["look"] as Dictionary).get("fog_color", []), _environment.fog_light_color)
	_environment.fog_light_color = fog
	AtmosphereLibrary.apply_to_environment(_environment, look, fog, fog.darkened(0.5))


func _process(delta: float) -> void:
	_season_check_s -= delta
	if _season_check_s <= 0.0:
		_season_check_s = 1.0
		_update_season()
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


func apply_distance(distance: float) -> void:
	var close := 1.0 - smoothstep(close_full_distance, close_begin_distance, distance)
	if _environment != null:
		_environment.fog_depth_begin = lerpf(distance * fog_begin_factor, maxf(close_fog_begin, distance * fog_begin_factor), close)
		_environment.fog_depth_end = lerpf(distance * fog_end_factor, maxf(close_fog_end, distance * fog_end_factor), close)
		if _base_ssao_radius < 0.0:
			_base_ssao_radius = _environment.ssao_radius
		_environment.ssao_radius = clampf(distance * ssao_radius_factor, ssao_radius_min, _base_ssao_radius)
		if _environment.ssil_enabled:  # rayon posé par RenderQuality (12 en campagne), idem
			_environment.ssil_radius = clampf(distance * 0.55, 0.1, 12.0)
	if _sun != null:
		_sun.shadow_enabled = distance < shadow_max_camera_distance
		_sun.directional_shadow_max_distance = maxf(distance * shadow_range_factor, lerpf(50.0, close_shadow_min, close))
	if _attributes != null:
		var closeness := (1.0 - clampf(distance / dof_max_distance, 0.0, 1.0)) * (1.0 - close)
		_attributes.dof_blur_far_enabled = closeness > 0.0
		_attributes.dof_blur_far_distance = distance * dof_begin_factor
		_attributes.dof_blur_far_transition = distance * dof_transition_factor
		_attributes.dof_blur_amount = dof_amount_near * closeness


func _orient_sun() -> void:
	if _sun == null:
		return
	_sun.basis = Basis.looking_at(sun_direction(), Vector3.UP)
