extends SceneTree

## Test headless du lot FR1 (frontières de faction lumineuses) sur la vraie simulation :
##  1. crochet `fr1_borders` dans les deux shaders du terrain (3D et parchemin seul), réglages
##     posés sur le matériau partagé du terrain ;
##  2. `refresh` (via `refresh_all`) construit propriétaires et contrôleurs ; l'Île-de-France est
##     à la France, qui a son index de palette ; un second rafraîchissement sans changement ne
##     reconstruit rien ;
##  3. changement de propriétaire (conquête simulée) et occupation : texture mise à jour tout de
##     suite, drapeau joueur suivi ;
##  4. filtres de carte (MF1) : visibles en politique, encre neutre en religion, masquées en
##     ravitaillement ; retour à la politique ;
##  5. zoom : effacées sous ~200 m, pleines au zoom moyen ; vue parchemin : toujours branchées ;
##  6. qualité Basse : halo et hachures coupés ; `set_enabled(false)` met l'opacité à 0.
## Usage : godot --headless --path game --script res://tests/fr1_borders_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PARIS := "prov_ile_de_france"

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("fr1_borders_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("fr1_borders_test: " + message)
	return condition


func _run() -> void:
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
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	var borders: FactionBorders = map.faction_borders
	if not _check(borders != null, "CampaignMap.faction_borders missing"):
		map.queue_free()
		return

	# 1. Crochet dans le shader du terrain, uniformes posés sur son matériau partagé.
	_check(not borders.tuning.is_empty(), "data/map/faction_borders.json not loaded")
	_check(borders.material() == map.terrain.material, "borders should drive the shared terrain material")
	var terrain_material: ShaderMaterial = map.terrain.material
	_check(TerrainBuilder.TERRAIN_SHADER.code.contains("fr1_borders("), "terrain.gdshader should call the fr1_borders hook")
	_check((load("res://shaders/terrain_parchment.gdshader") as Shader).code.contains("fr1_borders("), "terrain_parchment.gdshader should call the fr1_borders hook")
	_check(terrain_material.get_shader_parameter("fr1_realm_glow_px") != null, "tuning not applied to the terrain material")

	# 2. Propriétaires depuis la simulation.
	map.refresh_all()
	var data: MapData = map.map_data
	var paris := -1
	for index in range(1, data.province_count + 1):
		if str(data.get_province(index).get("id", "")) == PARIS:
			paris = index
	if not _check(paris > 0, "Île-de-France not found in the raster"):
		map.queue_free()
		return
	var france := borders.faction_index("fac_france")
	_check(france > 0, "France has no palette index")
	_check(borders.ownership_of(paris) == Vector2i(france, france), "Paris should be owned and held by France, got %s" % borders.ownership_of(paris))
	_check(not borders.refresh(), "a refresh without change should not rebuild the textures")

	# 3. Conquête et occupation simulées : mise à jour immédiate.
	var owners: PackedStringArray = borders._owners.duplicate()
	var controllers: PackedStringArray = borders._controllers.duplicate()
	owners[paris - 1] = "fac_england"
	controllers[paris - 1] = "fac_england"
	_check(borders.set_ownership(owners, controllers, "fac_france"), "owner change should rebuild")
	var england := borders.faction_index("fac_england")
	_check(england > 0 and borders.ownership_of(paris) == Vector2i(england, england), "Paris should now be English")
	controllers[paris - 1] = "fac_france"
	_check(borders.set_ownership(owners, controllers, "fac_france"), "occupation should rebuild")
	_check(borders.ownership_of(paris) == Vector2i(england, france), "Paris should be English, occupied by France")
	var info: ImageTexture = terrain_material.get_shader_parameter("fr1_info")
	var pixel := info.get_image().get_pixel(paris, 0)
	_check(roundi(pixel.b * 255.0) == 255, "player flag expected on a province the player occupies")
	borders.refresh()
	_check(borders.ownership_of(paris) == Vector2i(france, france), "refresh should restore the simulation's owner")

	# 4. Filtres de carte.
	var modes: Node = map.map_modes
	borders.update_view(300.0)
	_check(borders.visible_now() and is_equal_approx(borders.effective_alpha, 1.0), "political map at medium zoom: fully visible, got %f" % borders.effective_alpha)
	modes.set_mode("religion")
	borders.update_view(300.0)
	_check(borders.mode == "religion" and bool(terrain_material.get_shader_parameter("fr1_neutral")), "religion map should use neutral ink")
	_check(borders.visible_now(), "borders stay visible (neutral) on the religion map")
	modes.set_mode("supply")
	borders.update_view(300.0)
	_check(not borders.visible_now(), "borders hidden on the supply map")
	modes.set_mode("political")
	borders.update_view(300.0)
	_check(borders.visible_now() and not bool(terrain_material.get_shader_parameter("fr1_neutral")), "back to political: faction colours")

	# 5. Zoom et parchemin.
	var mpp: float = data.meters_per_px
	borders.update_view(150.0 / mpp)
	_check(not borders.visible_now(), "borders should fade out below 200 m")
	borders.update_view(900.0 / mpp)
	_check(borders.effective_alpha > 0.0 and borders.effective_alpha < 1.0, "borders half faded around 900 m")
	map.strategic.update_view(3000.0)
	borders.update_view(3000.0)
	_check(borders.visible_now(), "borders stay visible in the parchment view")
	map.strategic.update_view(300.0)
	borders.update_view(300.0)

	# 6. Qualité et interrupteur.
	RenderQuality.override_level = "low"
	borders.apply_render_quality(RenderQuality.preset())
	_check(not bool(terrain_material.get_shader_parameter("fr1_glow")), "low quality should drop the glow")
	RenderQuality.override_level = ""
	borders.apply_render_quality(RenderQuality.preset())
	_check(bool(terrain_material.get_shader_parameter("fr1_glow")), "glow back at default quality")
	borders.set_enabled(false)
	_check(float(terrain_material.get_shader_parameter("fr1_alpha")) == 0.0 and not borders.visible_now(), "disabled borders should zero fr1_alpha")
	borders.set_enabled(true)
	_check(borders.visible_now(), "re-enabled borders should show again")

	map.queue_free()
	await process_frame
