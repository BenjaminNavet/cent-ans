class_name CampaignLife
extends Node3D

## Lot CV1 : campagne vivante, rendu seulement (aucune règle de jeu ; tout vient du pont).
## - saisons visibles (`SeasonVisuals`, paramètre global `campaign_season`) ;
## - terroirs autour des colonies (`TerroirMask` → `terrain.gdshader`) : cultures, vignes,
##   pâtures à la densité de population, brûlis selon la dévastation ;
## - colonies qui grandissent (`SettlementGrowth`) : village → bourg → ville → cité ;
## - fumées, moulins, oiseaux, bateaux (`LifeEffects`).
## Branché par `campaign_map.gd` : `setup(map)`, `refresh(sim)` après chaque changement d'état,
## `update_view(distance)` à chaque image.
## Options (ligne de commande, après `--`) :
##   --season=spring|summer|autumn|winter   force la saison affichée (captures) ;
##   --devastate=<province>:<0-100>[,...]   force une dévastation affichée (captures) ;
##   --no-life                               désactive la couche (mesures A/B).

var enabled: bool = true
var seasons: SeasonVisuals = SeasonVisuals.new()
var terroir: TerroirMask = null
var growth: SettlementGrowth = null
var effects: LifeEffects = null
var forced_season: String = ""
## province_id → dévastation forcée (captures).
var forced_devastation: Dictionary = {}
## province_id → {devastation, population} lus au dernier `refresh`.
var province_states: Dictionary = {}
var stats: Dictionary = {}

var _map: Node = null
var _terrain: TerrainBuilder = null
var _map_data: MapData = null
var _settlements: SettlementLayer = null
var _tiers: ZoomTiers = null
var _camera_distance := 0.0
var _refreshed_once := false
## Empreinte des états utilisés par le masque (reconstruit seulement s'il change).
var _terroir_key: String = ""


func setup(map: Node) -> void:
	_map = map
	_terrain = map.get("terrain") as TerrainBuilder
	_map_data = map.get("map_data") as MapData
	_settlements = map.get("settlement_layer") as SettlementLayer
	_parse_cmdline()
	if _terrain != null and _terrain.material != null:
		_terrain.material.set_shader_parameter("life_enabled", enabled)
	if not enabled:
		seasons.set_season("summer", true)
		return
	if forced_season != "":
		seasons.set_season(forced_season, true)
	_tiers = map.get("zoom_tiers") as ZoomTiers
	effects = LifeEffects.new()
	effects.name = "Effects"
	add_child(effects)
	effects.setup(_settlements, _terrain)


func _parse_cmdline() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-life":
			enabled = false
		elif arg.begins_with("--season="):
			forced_season = arg.trim_prefix("--season=")
		elif arg.begins_with("--devastate="):
			for pair in arg.trim_prefix("--devastate=").split(",", false):
				var parts := pair.split(":")
				if parts.size() == 2:
					forced_devastation[parts[0]] = float(parts[1])


## Relit la saison, la dévastation et la population depuis la simulation.
func refresh(sim: Object) -> void:
	if not enabled or sim == null:
		return
	if forced_season == "" and sim.has_method("get_date_label"):
		var season := SeasonVisuals.season_from_label(str(sim.call("get_date_label")))
		# Premier affichage (nouvelle partie, chargement) : sans transition.
		seasons.set_season(season, not _refreshed_once)
	_refreshed_once = true
	_read_provinces(sim)
	if not forced_devastation.is_empty() and _settlements != null:
		_settlements.override_devastation(forced_devastation)
	_refresh_terroir()


## Dévastation et population des provinces qui ont des colonies ou des hameaux.
func _read_provinces(sim: Object) -> void:
	province_states.clear()
	if _settlements == null or _settlements.data == null:
		return
	var ids := {}
	for entry in _settlements.data.settlements:
		ids[str(entry["province"])] = true
	for hamlet in _settlements.data.hamlets:
		ids[str(hamlet["province"])] = true
	var can_read := sim.has_method("get_province_state")
	for province_id in ids:
		var state: Dictionary = sim.call("get_province_state", province_id) if can_read else {}
		var devastation := float(state.get("devastation", 0.0))
		if forced_devastation.has(province_id):
			devastation = float(forced_devastation[province_id])
		province_states[province_id] = {
			"devastation": devastation,
			"population": float(state.get("population_total", TerroirMask.REFERENCE_POPULATION)),
			"siege": state.has("siege"),
		}


func _refresh_terroir() -> void:
	if _terrain == null or _terrain.material == null or _settlements == null or _settlements.data == null:
		return
	# Clé : dévastation par paliers de 5 %, population par paliers de 10 %.
	var parts := PackedStringArray()
	for province_id in province_states:
		var state: Dictionary = province_states[province_id]
		parts.append("%s:%d:%d:%d" % [province_id, int(state["devastation"] / 5.0), int(log(maxf(state["population"], 1.0)) * 10.0), int(state["siege"])])
	var key := ",".join(parts)
	if key == _terroir_key:
		return
	_terroir_key = key
	if terroir == null:
		terroir = TerroirMask.new()
	var landuse := VegetationFields.landuse(_map_data) if _map_data != null else null
	terroir.build(_settlements.data.settlements, _settlements.data.hamlets, province_states, landuse, Vector2(_map_data.size))
	_terrain.material.set_shader_parameter("terroir_mask", terroir.texture)
	_terrain.material.set_shader_parameter("has_terroir", true)
	stats["terroir_ms"] = terroir.build_ms
	if effects != null:
		effects.rebuild(province_states)
		stats.merge(effects.stats, true)
	print("CampaignLife: %s" % JSON.stringify(stats))


func update_view(camera_distance: float) -> void:
	_camera_distance = camera_distance
	if not enabled:
		return
	seasons.update(get_process_delta_time())
	if effects != null:
		effects.set_season(seasons.weights)
		effects.update_view(_camera_distance, _tiers)
