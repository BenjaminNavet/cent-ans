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
##   --life-off=terrain,smoke,mills,ambient,scars  désactive une partie (mesures de coût).
## Chantier FK (carte vivante, `docs/design/2026-09-29-carte-vivante-folk.md`) : figurines de la
## vue rapprochée (`life_folk/`), mêmes crochets `refresh` / `update_view` :
##   --no-folk                                        désactive les figurines (mesures A/B) ;
##   --folk-off=routine,caravans,scenes,incidents     désactive une partie ;
##   --scene=<province>:<kind>[,...]                  force une scène de province (FK4, captures).

var enabled: bool = true
var seasons: SeasonVisuals = SeasonVisuals.new()
var terroir: TerroirMask = null
## Niveau visuel appliqué par colonie (id → niveau), lot CV1 § 3.
var levels: Dictionary = {}
var effects: LifeEffects = null
var ambient: LifeAmbient = null
## FK3 : réservoir de figurines et ses fournisseurs (FK4 : scènes, FK5 : incidents).
var folk: FolkPool = null
var folk_routine: FolkRoutine = null
var folk_caravans: FolkCaravans = null
var folk_scenes: FolkScenes = null
## FK5 : sceaux des incidents (nul : `--no-life`, `--no-folk`, `--folk-off=incidents`).
var incidents: IncidentMarkers = null
## TB4 : traces de la guerre et des fléaux (nul : `--no-life`, `--life-off=scars`).
var scars: WarScars = null
var folk_enabled: bool = true
var folk_off: Dictionary = {}
## FK4 : scènes forcées par `--scene=` ({province, kind, settlement, intensity}).
var forced_scenes: Array = []
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
## TB1 : mer de la saison (poids déjà appliqués à la mer).
var _sea: Sea = null
var _sea_weights := Vector4(-1, -1, -1, -1)
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
	_sea = map.get("sea") as Sea
	_parse_cmdline()
	if _terrain != null and _terrain.material != null:
		_terrain.material.set_shader_parameter("life_enabled", enabled and not _off.has("terrain"))
		var ground := SeasonLook.ground()  # A6-C4/C5 : teintes d'été et d'hiver du sol (données)
		for uniform_name: String in ground:
			_terrain.material.set_shader_parameter(uniform_name, ground[uniform_name])
		var south := SeasonLook.snow_south_fade()  # TB1 : limite sud de la neige de plaine (données)
		if south.x >= 0.0:
			var winter_south: Variant = _terrain.material.get_shader_parameter("winter_south")
			var retreat: float = (winter_south as Vector3).z if winter_south is Vector3 else 0.35
			_terrain.material.set_shader_parameter("winter_south", Vector3(south.x, south.y, retreat))
	if not enabled:
		seasons.set_season("summer", true)
		_sync_sea()
		return
	if forced_season != "":
		seasons.set_season(forced_season, true)
	_sync_sea()
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
	_setup_folk()
	_setup_incidents()
	_setup_scars()


## TB4 : fosses et portes marquées de la peste, champs de bataille, engins des camps de siège.
func _setup_scars() -> void:
	if _off.has("scars"):
		return
	scars = WarScars.new()
	add_child(scars)
	scars.setup(_map_data, _terrain, _map.get("armies") if _map != null else null)
	scars.plan_provider = _town_plan


## Plan de la ville 1:1 ordinaire affichée d'une colonie ; `{}` tant qu'elle n'est pas chargée.
## Villes emblématiques : `{none: true}`, pas de portes marquées (la caméra n'y descend pas sous
## `CloseCameraProfile.landmark_min_distance` : une croix de 2,2 m y ferait ≈ 1 px).
func _town_plan(id: String) -> Dictionary:
	if _settlements == null:
		return {}
	var cities := _settlements.landmark_cities
	if cities != null and cities.has_city(id):
		return {"none": true}
	var towns := _settlements.towns
	if towns != null and towns.data != null and towns.is_shown(id):
		return {"plan": towns.plan_of(id), "anchor": towns.data.anchor_of(id), "meters_per_unit": towns.data.meters_per_unit}
	return {}


## Tracé d'une route qui part de la colonie `id` (orienté vers l'extérieur) ; vide sans route.
func _road_from(id: String, center: Vector2) -> PackedVector2Array:
	if _settlements == null or _settlements.data == null:
		return PackedVector2Array()
	for key in _settlements.data.edge_paths:
		var edge := str(key)
		if not (edge.begins_with(id + "|") or edge.ends_with("|" + id)):
			continue
		var path: PackedVector2Array = _settlements.data.edge_paths[key]
		if path.size() < 2:
			continue
		if path[0].distance_to(center) > path[path.size() - 1].distance_to(center):
			path = path.duplicate()
			path.reverse()
		return path
	return PackedVector2Array()


## TB4 : colonies pestiférées du tour (scènes `plague` résolues par `FolkScenes`).
func _refresh_scars() -> void:
	if scars == null:
		return
	var sites: Array = []
	if folk_scenes != null:
		for scene in folk_scenes.staged:
			if str(scene["kind"]) == "plague":
				var site := folk_scenes.edge_frame(scene)
				site["road"] = _road_from(str(site["settlement"]), site["center"])
				sites.append(site)
	scars.set_plague_sites(sites)
	stats["scars"] = scars.stats


## FK3 : réservoir et fournisseurs, servis dans l'ordre d'enregistrement quand le plafond est
## atteint (FK4 : scènes avant les marchands ; FK5 : incidents).
func _setup_folk() -> void:
	if not folk_enabled:
		return
	folk = FolkPool.new()
	folk.name = "Folk"
	add_child(folk)
	folk.setup(_map_data, _terrain)
	folk.landmark_layer = _settlements
	var settlement_data: SettlementData = _settlements.data if _settlements != null else null
	if not folk_off.has("scenes"):
		folk_scenes = FolkScenes.new()
		folk_scenes.setup(_map_data, settlement_data, _settlements)
		folk_scenes.forced = forced_scenes
		folk.register(folk_scenes)
	if not folk_off.has("caravans"):
		folk_caravans = FolkCaravans.new()
		folk_caravans.setup(_map_data, settlement_data)
		folk.register(folk_caravans)
	if not folk_off.has("routine"):
		folk_routine = FolkRoutine.new()
		folk_routine.setup(_map_data, settlement_data)
		folk.register(folk_routine)


## FK5 : sceaux des incidents, à tous les zooms (hors réservoir : aucune figurine).
func _setup_incidents() -> void:
	if not folk_enabled or folk_off.has("incidents"):
		return
	incidents = IncidentMarkers.new()
	add_child(incidents)
	incidents.setup(_map)
	incidents.folk_scenes = folk_scenes


func _parse_cmdline() -> void:
	if CmdArgs.has("--no-life"):
		enabled = false
	for part in CmdArgs.list("--life-off"):
		_off[part] = true
	if CmdArgs.has("--no-folk"):
		folk_enabled = false
	for part in CmdArgs.list("--folk-off"):
		folk_off[part] = true
	if CmdArgs.has("--scene"):
		forced_scenes.append_array(FolkScenes.parse_forced(CmdArgs.value("--scene")))
	if CmdArgs.has("--season"):
		forced_season = CmdArgs.value("--season")
	for pair in CmdArgs.list("--devastate"):
		var parts := pair.split(":")
		if parts.size() == 2:
			forced_devastation[parts[0]] = float(parts[1])


## TB1 : mer de la saison (teinte, écume) et neige des toits des villes 1:1 quand les poids
## changent ; masque météo du terrain repris par la mer (écume de tempête).
func _sync_sea() -> void:
	if seasons.weights != _sea_weights:
		_sea_weights = seasons.weights
		var map_height := float(_map_data.size.y) if _map_data != null else 0.0
		RenderingServer.global_shader_parameter_set(SeasonLook.ROOF_SNOW_PARAM, SeasonLook.roof_snow(_sea_weights, map_height))
		if _sea != null:
			_sea.apply_season(_sea_weights)
	if _sea != null and _terrain != null:
		_sea.sync_weather(_terrain.material)


## Relit la saison, la dévastation et la population depuis la simulation.
func refresh(sim: Object) -> void:
	if not enabled or sim == null:
		return
	if incidents != null:  # FK5 : une décision prise dans le tour retire son sceau
		incidents.refresh(sim)
	if scars != null:  # TB4 : batailles de la main du joueur, sièges posés ou levés dans le tour
		scars.refresh(sim)
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
	_refresh_folk(sim)
	_refresh_scars()


## FK3 : état lu par la routine (population, dévastation, saison, cités, terroirs), puis
## relecture des fournisseurs (routes commerciales) ; une fois par tour.
func _refresh_folk(sim: Object) -> void:
	if folk == null:
		return
	if folk_routine != null:
		folk_routine.province_states = province_states
		folk_routine.season = seasons.season
		folk_routine.terroir = terroir
		var cities := PackedVector2Array()
		if _settlements != null and _settlements.data != null:
			# Cités : colonies de type « city » (Paris et les villes 1:1 ne passent pas par la
			# croissance) et colonies promues cité (`SettlementGrowth`, niveau 3).
			for entry in _settlements.data.settlements:
				var level := int(levels.get(str(entry["id"]), -2)) / 2
				if str(entry["kind"]) == "city" or level == 3:
					cities.append(entry["px"])
		folk_routine.cities = cities
	folk.refresh(sim)
	stats["folk"] = folk.stats
	if folk_scenes != null:
		# FK4 : disette → champs vides ; peste → cheminées éteintes ; bûchers et émeutes fument.
		if folk_routine != null:
			folk_routine.idle_provinces = folk_scenes.idle_provinces
		if effects != null:
			effects.quiet_settlements = folk_scenes.quiet_settlements
			effects.scene_fires = folk_scenes.fire_points
			effects.rebuild(province_states)
		stats["folk_scenes"] = folk_scenes.stats


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


## Niveau visuel des colonies (`levels`, lu par les foules pour les cités). VT : plus de maquette
## à remplacer (villes 1:1, la croissance visuelle CV1 est abandonnée).
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
	stats["growth_ms"] = Time.get_ticks_msec() - t0
	stats["levels"] = counts


func _refresh_terroir() -> void:
	if _terrain == null or _terrain.material == null or _settlements == null or _settlements.data == null:
		return
	# Clé : dévastation par paliers de 5 %, population par paliers de 10 %.
	var parts := PackedStringArray()
	var burnt := false
	for province_id in province_states:
		var state: Dictionary = province_states[province_id]
		burnt = burnt or float(state["devastation"]) >= TerroirMask.BURN_THRESHOLD
		parts.append("%s:%d:%d:%d" % [province_id, int(state["devastation"] / 5.0), int(log(maxf(state["population"], 1.0)) * 10.0), int(state["siege"])])
	# TB4 : le brûlis par parcelles n'est calculé que si une province au moins est dévastée.
	_terrain.material.set_shader_parameter("burn_active", burnt)
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
	# TB1 : plus de neige de saison sur les toits hors de la carte de campagne.
	RenderingServer.global_shader_parameter_set(SeasonLook.ROOF_SNOW_PARAM, Vector4.ZERO)
	if terroir != null:
		terroir.wait()


## Force une relecture complète au prochain `refresh` (chargement d'une partie).
func invalidate() -> void:
	_turn_key = ""
	if scars != null:  # TB4 : champs de bataille de l'ancienne partie oubliés
		scars.reset()


func update_view(camera_distance: float) -> void:
	_camera_distance = camera_distance
	if not enabled:
		return
	var tp := Time.get_ticks_usec()  # RS-K : sections `life/*` du banc `--bench-probe`
	if terroir != null and terroir.poll():
		_terrain.material.set_shader_parameter("terroir_mask", terroir.texture)
	tp = PerfProbe.lap("life/terroir", tp)
	seasons.update(get_process_delta_time())
	_sync_sea()
	tp = PerfProbe.lap("life/seasons", tp)
	if effects != null:
		effects.set_season(seasons.weights)
		effects.update_view(_camera_distance, _tiers)
		if _off.has("smoke"):
			effects.get_node("Chimneys").visible = false
			effects.get_node("Fires").visible = false
			effects.get_node("RegionalPlumes").visible = false  # RV-F
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
		# DV : bateaux et navires sur toute la vue normale (poids 1 − `strategic_weight`).
		var normal := 1.0 - _tiers.strategic_weight(_camera_distance)
		ambient.update_view(Vector2(focus.x, focus.z), _tiers.near_weight(_camera_distance) * keep, normal * keep)
	tp = PerfProbe.lap("life/ambient", tp)
	if folk != null and _tiers != null:
		var folk_rig := _map.get("camera_rig") as Node3D if _map != null else null
		var folk_focus: Vector3 = folk_rig.get("focus") if folk_rig != null else Vector3.ZERO
		folk.update_view(Vector2(folk_focus.x, folk_focus.z), _camera_distance, _tiers.near_weight(_camera_distance))
	tp = PerfProbe.lap("life/folk", tp)
	if scars != null:
		scars.update_view(_camera_distance, 1.0 - _tiers.strategic_weight(_camera_distance) if _tiers != null else 1.0)
	PerfProbe.lap("life/scars", tp)
