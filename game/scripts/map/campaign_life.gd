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
##   --no-life                               désactive la couche (mesures A/B) ;
##   --life-off=terrain,smoke,mills,ambient  désactive une partie (mesures de coût).

var enabled: bool = true
var seasons: SeasonVisuals = SeasonVisuals.new()
var terroir: TerroirMask = null
## Niveau visuel appliqué par colonie (id → niveau), lot CV1 § 3.
var levels: Dictionary = {}
var effects: LifeEffects = null
var ambient: LifeAmbient = null
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
var _turn_key: String = ""
var _off: Dictionary = {}


func setup(map: Node) -> void:
	_map = map
	_terrain = map.get("terrain") as TerrainBuilder
	_map_data = map.get("map_data") as MapData
	_settlements = map.get("settlement_layer") as SettlementLayer
	_parse_cmdline()
	if _terrain != null and _terrain.material != null:
		_terrain.material.set_shader_parameter("life_enabled", enabled and not _off.has("terrain"))
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
	ambient = LifeAmbient.new()
	ambient.name = "Ambient"
	add_child(ambient)
	ambient.setup(_map_data, _terrain, _settlements.data if _settlements != null else null)
	stats.merge(ambient.stats, true)


func _parse_cmdline() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-life":
			enabled = false
		elif arg.begins_with("--life-off="):
			for part in arg.trim_prefix("--life-off=").split(",", false):
				_off[part] = true
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
	# Population, dévastation, sièges et bâtiments ne changent qu'en fin de tour : relecture une
	# fois par tour (refresh_all est aussi appelé après chaque ordre du joueur).
	var turn_key := "%s|%s" % [sim.call("get_turn") if sim.has_method("get_turn") else 0, sim.call("get_date_label") if sim.has_method("get_date_label") else ""]
	if turn_key == _turn_key:
		return
	_turn_key = turn_key
	_read_provinces(sim)
	if not forced_devastation.is_empty() and _settlements != null:
		_settlements.override_devastation(forced_devastation)
	var first_growth := levels.is_empty()
	_refresh_growth(sim)
	if first_growth and _map != null:
		# Emprises agrandies (cités, faubourgs) : clairières de la végétation avant sa construction.
		var vegetation := _map.get_node_or_null("Vegetation")
		if vegetation != null and _settlements != null:
			vegetation.set("extra_exclusions", _settlements.vegetation_exclusions())
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
	# PB3d : instantané groupé (partagé avec les autres calques du même rafraîchissement).
	var snapshot := ProvinceSnapshot.of(sim, _map_data) if _map_data != null else ProvinceSnapshot.read(sim, PackedStringArray(ids.keys()))
	for province_id in ids:
		var i := snapshot.index_of(str(province_id))
		var known := snapshot.has(i)
		var devastation := float(snapshot.devastation[i]) if known else 0.0
		if forced_devastation.has(province_id):
			devastation = float(forced_devastation[province_id])
		province_states[province_id] = {
			"devastation": devastation,
			"population": float(snapshot.population_total[i]) if known else TerroirMask.REFERENCE_POPULATION,
			"siege": known and snapshot.besieged[i] != 0,
		}


## Croissance des colonies : remplace la maquette quand le niveau visuel change. La maquette
## par défaut de chaque type (village → village, ville → ville murée) n'est pas remplacée tant
## que le niveau lui correspond.
func _refresh_growth(sim: Object) -> void:
	if _settlements == null or _settlements.data == null:
		return
	var t0 := Time.get_ticks_msec()
	var can_detail := sim.has_method("settlement_detail")
	# PB3d : bâtiments, fortification et rang de cité de toutes les colonies en un appel groupé
	# (au lieu d'un `settlement_detail` complet — revenus, panneaux — par colonie).
	var live: Dictionary = sim.call("get_settlements_live") if sim.has_method("get_settlements_live") else {}
	var live_index: Dictionary = {}
	var live_ids: PackedStringArray = live.get("id", PackedStringArray())
	for k in live_ids.size():
		live_index[live_ids[k]] = k
	var replaced := 0
	var counts := [0, 0, 0, 0]
	for i in _settlements.data.settlements.size():
		var entry: Dictionary = _settlements.data.settlements[i]
		var kind := str(entry["kind"])
		if kind != "village" and kind != "town" and kind != "city":
			continue
		var id := str(entry["id"])
		var detail: Dictionary = {}
		if live_index.has(id):
			var k: int = live_index[id]
			detail = {"buildings": Array(live["buildings"][k]), "fortification_level": int(live["fortification_level"][k]), "is_city": live["is_city"][k] != 0}
		elif can_detail:
			detail = sim.call("settlement_detail", id)
		var population := float(province_states.get(str(entry["province"]), {}).get("population", 0.0))
		var level := SettlementGrowth.level_of(entry, detail, population)
		if level < 0:
			continue
		counts[level] += 1
		var castle := level == 3 and SettlementGrowth.has_castle(detail)
		var key := level * 2 + (1 if castle else 0)
		var default_key := {"village": 0, "town": 4, "city": -1}[kind] as int
		var current: int = levels.get(id, default_key)
		if key == current:
			continue
		levels[id] = key
		var holder := _settlements.model_holder(i)
		if holder == null:
			continue
		var model := SettlementGrowth.build_model(level, absi(id.hash()) / 7, castle)
		if model == null:
			continue
		# DC4 : maquette à pleine taille ; `replace_model` applique la réduction des voisines.
		_settlements.replace_model(i, model)
		replaced += 1
	stats["growth_ms"] = Time.get_ticks_msec() - t0
	stats["growth_replaced"] = replaced
	stats["levels"] = counts


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
	var landuse := VegetationFields.landuse(_map_data) if _map_data != null else null
	if terroir == null:
		# Premier affichage : masque tout de suite (pas de terroir « nu » au lancement).
		terroir = TerroirMask.new()
		terroir.build(_settlements.data.settlements, _settlements.data.hamlets, province_states, landuse, Vector2(_map_data.size))
		_terrain.material.set_shader_parameter("terroir_mask", terroir.texture)
		_terrain.material.set_shader_parameter("has_terroir", true)
	else:
		# PB1 : fins de tour suivantes, calcul dans un fil (installé par `update_view`).
		terroir.build_async(_settlements.data.settlements, _settlements.data.hamlets, province_states, landuse, Vector2(_map_data.size))
	stats["terroir_ms"] = terroir.build_ms
	if effects != null:
		effects.rebuild(province_states)
		stats.merge(effects.stats, true)
	print("CampaignLife: %s" % JSON.stringify(stats))


func _exit_tree() -> void:
	if terroir != null:
		terroir.wait()


## Force une relecture complète au prochain `refresh` (chargement d'une partie).
func invalidate() -> void:
	_turn_key = ""


func update_view(camera_distance: float) -> void:
	_camera_distance = camera_distance
	if not enabled:
		return
	var tp := Time.get_ticks_usec()  # RS-K : sections `life/*` du banc `--bench-probe`
	if terroir != null and terroir.poll():
		_terrain.material.set_shader_parameter("terroir_mask", terroir.texture)
	tp = PerfProbe.lap("life/terroir", tp)
	seasons.update(get_process_delta_time())
	tp = PerfProbe.lap("life/seasons", tp)
	if effects != null:
		effects.set_season(seasons.weights)
		effects.update_view(_camera_distance, _tiers)
		if _off.has("smoke"):
			effects.get_node("Chimneys").visible = false
			effects.get_node("Fires").visible = false
		if _off.has("mills"):
			effects.get_node("WindmillBodies").visible = false
			effects.get_node("WindmillSails").visible = false
	tp = PerfProbe.lap("life/effects", tp)
	if _off.has("ambient"):
		if ambient != null:
			ambient.visible = false
	elif ambient != null and _tiers != null:
		var rig := _map.get("camera_rig") as Node3D if _map != null else null
		var focus: Vector3 = rig.get("focus") if rig != null else Vector3.ZERO
		# ZG4 : navires, bateaux et oiseaux à l'échelle de la carte masqués au palier « site ».
		var keep := 1.0 - _tiers.site_weight(_camera_distance)
		ambient.update_view(Vector2(focus.x, focus.z), _tiers.near_weight(_camera_distance) * keep, _tiers.medium_weight(_camera_distance) * keep)
	PerfProbe.lap("life/ambient", tp)
