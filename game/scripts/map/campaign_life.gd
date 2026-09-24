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
var _refreshed_once := false


func setup(map: Node) -> void:
	_map = map
	_terrain = map.get("terrain") as TerrainBuilder
	_parse_cmdline()
	if _terrain != null and _terrain.material != null:
		_terrain.material.set_shader_parameter("life_enabled", enabled)
	if not enabled:
		seasons.set_season("summer", true)
		return
	if forced_season != "":
		seasons.set_season(forced_season, true)


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


func update_view(_camera_distance: float) -> void:
	if not enabled:
		return
	seasons.update(get_process_delta_time())
