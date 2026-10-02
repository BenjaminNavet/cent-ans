extends SceneTree

## Test headless du lot TB2 (désencombrement de la carte de campagne, plan
## `docs/design/2026-10-02-campagne-tob.md` § 3) : un seul signe par ville et par palier de zoom,
## étiquettes entières dans l'écran, noms de région en vue moyenne, frontières discrètes hors
## sélection, brouillard de guerre en voile de parchemin, nuages réservés à la météo réelle.
## Usage : godot --headless --path game --script res://tests/tb2_declutter_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PARIS := Vector2(2213.2, 3203.9)
## Paliers de zoom vérifiés : vue large, moyenne, proche (distance caméra, unités carte).
const SCREEN := Vector2i(1600, 900)
const TIERS: Array[float] = [1100.0, 400.0, 90.0]

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	if _failures == 0:
		print("tb2_declutter OK")
	quit(1 if _failures > 0 else 0)


func _run() -> void:
	# Fenêtre factice du mode headless (64 px) : taille d'écran réaliste pour les mesures.
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_DISABLED
	root.size = SCREEN
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	_test_fog(map)
	_test_borders(map)
	await _test_signs(map)
	await _test_labels(map)
	await _test_clouds(map)
	map.queue_free()
	await process_frame


## 1. Brouillard de guerre : voile de parchemin sombre et désaturé, plus de nappe blanche.
func _test_fog(map: Node3D) -> void:
	var material: ShaderMaterial = map.terrain.material
	var fog := MapReadability.section("fog_of_war")
	_check(not fog.is_empty(), "fog_of_war block missing in data/ui/campaign_map.json")
	var veil: Color = material.get_shader_parameter("fog_veil_color")
	_check(veil.r > veil.g and veil.g > veil.b, "fog veil should be a warm parchment tone, got %s" % veil)
	_check(veil.get_luminance() < 0.75, "fog veil should not be a white wash (luminance %.2f)" % veil.get_luminance())
	var dim := float(material.get_shader_parameter("fog_dim"))
	_check(dim >= 0.55 and dim < 0.9, "fogged ground should be dimmed but readable (dim %.2f)" % dim)
	_check(float(material.get_shader_parameter("fog_desaturation")) >= 0.4, "fogged ground should be desaturated")
	# La brume résiduelle ne doit plus couvrir le relief (elle montait à 0,66 avant TB2). Le
	# palier de zoom peut encore la réduire (`_apply_close_tiers`), jamais l'augmenter.
	_check(float(material.get_shader_parameter("fog_mist_max")) <= 0.15, "residual mist must stay thin")
	_check(float(material.get_shader_parameter("fog_mist_glow")) <= 0.05, "mist must not glow (it washed the ground white)")
	print("tb2 fog: veil %s dim %.2f desaturation %.2f mist_max %.2f" % [veil.to_html(false), dim, float(material.get_shader_parameter("fog_desaturation")), float(material.get_shader_parameter("fog_mist_max"))])


## 2. Frontières : style au repos hors sélection, survol et mode Diplomatie.
func _test_borders(map: Node3D) -> void:
	var borders: FactionBorders = map.faction_borders
	var modes: Object = map.map_modes
	var material: ShaderMaterial = map.terrain.material
	var data: MapData = map.map_data
	var paris: int = data.index_of_id("prov_ile_de_france")
	var guyenne: int = data.index_of_id("prov_guyenne")
	map.refresh_all()
	map.hovered_index = 0
	map.selected_index = 0
	borders.update_view(300.0)
	_check(float(material.get_shader_parameter("fr1_rest_width_scale")) <= 0.7, "rest borders should be thinner")
	_check(float(material.get_shader_parameter("fr1_rest_saturation")) <= 0.6, "rest borders should be less saturated")
	_check(float(material.get_shader_parameter("fr1_rest_glow_scale")) <= 0.4, "rest borders should barely glow")
	_check(is_zero_approx(float(material.get_shader_parameter("fr1_focus"))), "political map: no global full intensity")
	_check(borders.focus_factions() == Vector2i.ZERO, "nothing hovered or selected: no realm in focus")
	var rest := borders.intensity_of(paris)
	_check(rest <= 0.5, "border intensity out of selection should be at most half, got %.2f" % rest)
	# Sélection : le royaume de la province passe à pleine intensité, les autres restent au repos.
	map.selected_index = paris
	borders.update_view(301.0)
	var france := borders.faction_index("fac_france")
	_check(france > 0 and int(material.get_shader_parameter("fr1_focus_b")) == france, "selecting Paris should put France in focus")
	_check(is_equal_approx(borders.intensity_of(paris), 1.0), "selected realm at full intensity")
	_check(borders.intensity_of(guyenne) <= 0.5, "other realms stay at rest while France is selected")
	# Survol.
	map.selected_index = 0
	map.hovered_index = guyenne
	borders.update_view(302.0)
	_check(int(material.get_shader_parameter("fr1_focus_a")) == borders.faction_index("fac_england"), "hovering Guyenne should put England in focus")
	_check(is_equal_approx(borders.intensity_of(guyenne), 1.0) and borders.intensity_of(paris) <= 0.5, "hovered realm only at full intensity")
	map.hovered_index = 0
	# Mode Diplomatie : tout à pleine intensité.
	modes.set_mode("diplomacy")
	borders.update_view(303.0)
	_check(is_equal_approx(float(material.get_shader_parameter("fr1_focus")), 1.0), "diplomacy map shows every border in full")
	_check(is_equal_approx(borders.intensity_of(paris), 1.0), "diplomacy map: full intensity")
	modes.set_mode("political")
	borders.update_view(304.0)
	_check(borders.intensity_of(paris) <= 0.5, "back to political: borders at rest again")
	print("tb2 borders: rest intensity %.2f (width %.2f x alpha %.2f), saturation %.2f, glow %.2f ; full on selection, hover, diplomacy" % [rest, borders.rest_value("width_scale"), borders.rest_value("alpha_scale"), borders.rest_value("saturation"), borders.rest_value("glow_scale")])


## Place la caméra au-dessus de `px` (carte) à la distance `distance`, puis refait le
## dé-encombrement des noms.
func _look(map: Node3D, px: Vector2, distance: float) -> void:
	var data: MapData = map.map_data
	var ground := Vector3(px.x, data.surface_world_at(px.x, px.y), px.y)
	map.camera_rig.look_at_point(ground, distance)
	map.camera_rig.snap()
	for i in 4:
		await process_frame
	map.settlement_layer.declutter()
	await process_frame


## Positions écran des signes visibles autres que l'écu : marteaux, sceaux, sites de rencontre.
func _extra_signs(map: Node3D) -> PackedVector2Array:
	var points := PackedVector2Array()
	var camera: Camera3D = map.camera
	if map.construction_markers.visible:
		for label in map.construction_markers.get_children():
			if (label as Node3D).visible and not camera.is_position_behind((label as Node3D).global_position):
				points.append(camera.unproject_position((label as Node3D).global_position))
	var layers: Array = []
	if map.life != null and map.life.incidents != null:
		layers.append(map.life.incidents.get_node("IncidentSeals"))
	if map.encounters != null:
		layers.append_array(map.encounters.find_children("*", "CanvasLayer", false, false))
	for layer: Node in layers:
		for control in layer.get_children():
			if control is Control and (control as Control).visible:
				points.append((control as Control).position + (control as Control).size * 0.5)
	return points


## 3. Pictogrammes : un seul signe par ville et par palier ; sceaux, sites et marteaux réservés à
## la couche « Signes » et aux modes de carte.
func _test_signs(map: Node3D) -> void:
	var sim: Object = map.sim
	var layer: SettlementLayer = map.settlement_layer
	var limit := int(MapReadability.section("signs").get("max_per_town", 0))
	_check(limit == 1, "signs.max_per_town should be 1")
	MapReadability.signs_layer_on = false
	var seal_id := -1
	if sim.has_method("debug_offer_decision"):
		sim.call("set_chronicle_enabled", false)
		seal_id = int(sim.call("debug_offer_decision", "evt_crue", "prov_touraine"))
		map.refresh_all()
	for distance: float in TIERS:
		await _look(map, PARIS, distance)
		var extras := _extra_signs(map)
		# Lieux nommés dans l'écran (`screen_occupancy` : noms et écus affichés, sans marge).
		var occupancy: Dictionary = layer.screen_occupancy(map.camera)
		var kinds: Array = occupancy["kinds"]
		var owners: PackedInt32Array = occupancy["owners"]
		var signs_of := {}  # index de colonie → nombre de signes
		for n in owners.size():
			if not signs_of.has(owners[n]):
				signs_of[owners[n]] = 0
			if kinds[n] == "marker":
				signs_of[owners[n]] += 1
		var worst := 0
		for i: int in signs_of:
			var anchor: Vector2 = map.camera.unproject_position(layer._labels[i].global_position)
			for point in extras:
				if point.distance_to(anchor) < 48.0:
					signs_of[i] += 1
			worst = maxi(worst, int(signs_of[i]))
		var towns := signs_of.size()
		var shields := int(occupancy["markers"])
		_check(towns > 0, "some town names expected at distance %.0f" % distance)
		_check(worst <= limit, "at most %d sign per town at distance %.0f, got %d" % [limit, distance, worst])
		_check(extras.is_empty(), "no hammer, seal or encounter sign on the political map at distance %.0f" % distance)
		if distance >= 400.0:
			# Vue moyenne et large : l'écu est réservé aux lieux majeurs (rang 3 et 4).
			for i: int in signs_of:
				_check(int(signs_of[i]) == 0 or layer._marker_rank[i] >= 3, "minor place %s should lose its shield at distance %.0f" % [layer.data.settlements[i]["id"], distance])
		print("tb2 signs: distance %.0f -> %d places named on screen, %d shields, %d other signs, max %d per place" % [distance, towns, shields, extras.size(), worst])
	# Sceau d'incident : caché en mode politique (sauf dernier tour), montré par la couche et par
	# le mode Mécontentement.
	var incidents: IncidentMarkers = map.life.incidents if map.life != null else null
	var seal: Control = incidents.marker(seal_id) if incidents != null and seal_id > 0 else null
	if seal != null:
		await _look(map, Vector2(seal.get("world").x, seal.get("world").z), 400.0)
		var urgent := int(seal.call("turns_left")) <= 1
		_check(seal.visible == urgent, "incident seal hidden on the political map unless it expires this turn")
		map.map_modes.set_mode("unrest")
		await process_frame
		_check(seal.visible, "incident seal shown on the unrest map")
		map.map_modes.set_mode("political")
		MapReadability.signs_layer_on = true
		await process_frame
		_check(seal.visible, "incident seal shown by the signs layer")
		MapReadability.signs_layer_on = false
		await process_frame
	else:
		print("tb2 signs: incident staging unavailable (seal checks skipped)")
	_check(not MapReadability.sign_shown("construction", "political"), "construction hammers hidden on the political map")
	_check(MapReadability.sign_shown("construction", "wealth"), "construction hammers shown on the wealth map")
	_check(not MapReadability.sign_shown("encounter", "political"), "encounter sites hidden by default")
	_check(MapReadability.sign_shown("encounter", "political", false, true), "encounter sites shown while an army is selected")
	_check(MapReadability.sign_shown("encounter", "political", true), "a claimed or expiring encounter stays visible")
	_check(not map.construction_markers.visible, "construction markers node hidden on the political map")


## 4. Étiquettes : hiérarchie typographique, aucun nom coupé en bord d'écran, noms de région en
## vue moyenne.
func _test_labels(map: Node3D) -> void:
	var layer: SettlementLayer = map.settlement_layer
	var data: SettlementData = layer.data
	var paris := int(data.index_by_id.get("set_paris", -1))
	if _check(paris >= 0, "set_paris missing"):
		_check(layer._marker_rank[paris] == 4 and layer.label_is_small_caps(paris), "capital names should be set in small caps")
		_check(SettlementLayer.has_small_caps(layer._labels[paris].font), "EB Garamond should provide true small caps (smcp)")
	var roman := 0
	var minor := 0
	for i in data.settlements.size():
		if layer._marker_rank[i] < 4:
			minor += 1
			if not layer.label_is_small_caps(i):
				roman += 1
	_check(minor > 0 and roman >= minor - 3, "towns below capital rank should be set in roman (%d of %d)" % [roman, minor])
	var regions: RegionLabels = map.strategic.region_labels
	_check(regions != null, "StrategicView.region_labels missing")
	# Centres de vue : Paris, puis des points où des villes tombent près des bords (Manche, Loire).
	var spots: Array[Vector2] = [PARIS, PARIS + Vector2(-140.0, -90.0), PARIS + Vector2(60.0, 170.0)]
	var cut_total := 0
	var names_total := 0
	for distance: float in TIERS:
		for spot in spots:
			await _look(map, spot, distance)
			# Zone visible : l'écran moins le bandeau du haut. La passe de dé-encombrement exige
			# en plus la marge `labels.edge_margin_px`, qui absorbe le glissement des noms entre
			# deux passes (échelle du relief, décalage au-dessus de l'emprise).
			var safe: Rect2 = layer.label_safe_rect().grow(MapReadability.number("labels", "edge_margin_px", 16.0) - 1.0)
			var occupancy: Dictionary = layer.screen_occupancy(map.camera)
			var rects: Array = occupancy["rects"]
			var owners: PackedInt32Array = occupancy["owners"]
			var pinned: PackedInt32Array = layer._pinned_indices()
			for n in rects.size():
				if pinned.has(owners[n]):
					continue  # sélection, survol, capitale du joueur : toujours affichées
				names_total += 1
				if not safe.grow(1.0).encloses(rects[n]):
					cut_total += 1
					push_error("tb2_declutter: %s cut by the screen edge at distance %.0f: %s" % [data.settlements[owners[n]]["id"], distance, rects[n]])
			if regions != null:
				regions.queue_redraw()
				await process_frame
				await process_frame
				var screen: Rect2 = map.get_viewport().get_visible_rect()
				var town_rects: Array[Rect2] = layer.screen_label_rects(map.camera)
				for rect in regions.placed_rects:
					_check(screen.encloses(rect), "region name cut by the screen edge at distance %.0f" % distance)
					for other in town_rects:
						_check(not other.intersects(rect), "region name over a town name at distance %.0f" % distance)
				if spot == PARIS:
					print("tb2 labels: distance %.0f -> %d region names (%s), weight %.2f" % [distance, regions.placed_rects.size(), ", ".join(regions.placed_names.slice(0, 4)), regions.weight])
					if distance == 400.0:
						_check(regions.visible and regions.placed_rects.size() >= 2, "region names expected in the medium view, got %d" % regions.placed_rects.size())
						_check(MapReadability.number("region_labels", "alpha", 1.0) <= 0.6, "region names should stay discreet")
					elif distance == 90.0:
						_check(regions.placed_rects.is_empty(), "no region name in the close view")
	_failures += cut_total
	_check(names_total > 50, "label sample too small (%d)" % names_total)
	print("tb2 labels: %d names and shields checked over %d views, %d cut by the screen edge (placement rect %s)" % [names_total, TIERS.size() * spots.size(), cut_total, layer.label_safe_rect()])


## 5. Nuées et brumes : seulement sous une météo réelle, fines en vue moyenne, jamais sur la
## province sélectionnée.
func _test_clouds(map: Node3D) -> void:
	var view: CampaignWeatherView = map.weather_view
	if not _check(view != null and view.enabled, "campaign weather view missing"):
		return
	var terrain_material: ShaderMaterial = map.terrain.material
	var cloud_material: ShaderMaterial = view._cloud_material
	# Opacité par palier : rien de près, fin en vue moyenne, plein en vue large.
	var close := view.cloud_alpha_at(90.0)
	var medium := view.cloud_alpha_at(400.0)
	var wide := view.cloud_alpha_at(1100.0)
	_check(is_zero_approx(close), "no cloud plane in the close view")
	_check(medium <= 0.2 and medium < wide * 0.6, "clouds should be thinner in the medium view (%.2f vs %.2f wide)" % [medium, wide])
	# Météo réelle seulement : pas d'ombre de nuage par temps clair, nuées coupées sans météo.
	_check(is_zero_approx(view.shadow_amount_for("clear")) and is_zero_approx(view.shadow_amount_for("fog")), "no cloud shadow without real clouds")
	_check(view.shadow_amount_for("rain") > 0.0 and view.shadow_amount_for("rain") <= 0.15, "light cloud shadow under real rain")
	var saved: Dictionary = view.weather.duplicate(true)
	view.weather = {}
	view._upload_mask()
	_check(not bool(cloud_material.get_shader_parameter("weather_enabled")), "clear weather everywhere: cloud plane disabled")
	view.weather = saved
	view._upload_mask()
	# Province sélectionnée dégagée (nuées, ombres de nuées et nappes de brume).
	var paris: int = map.map_data.index_of_id("prov_ile_de_france")
	map.selected_index = paris
	await _look(map, PARIS, 400.0)
	_check(int(cloud_material.get_shader_parameter("weather_clear_id")) == paris, "clouds should clear the selected province")
	_check(int(terrain_material.get_shader_parameter("weather_clear_id")) == paris, "ground mist should clear the selected province")
	map.selected_index = 0
	await process_frame
	await process_frame
	_check(int(cloud_material.get_shader_parameter("weather_clear_id")) == 0, "no cleared province once the selection is dropped")
	print("tb2 clouds: alpha close %.2f, medium %.2f, wide %.2f ; cloud shadow clear %.2f, rain %.2f" % [close, medium, wide, view.shadow_amount_for("clear"), view.shadow_amount_for("rain")])


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb2_declutter: " + message)
	return condition
