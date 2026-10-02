extends SceneTree

## Test headless du lot TB2 (désencombrement de la carte de campagne, plan
## `docs/design/2026-10-02-campagne-tob.md` § 3) : un seul signe par ville et par palier de zoom,
## étiquettes entières dans l'écran, noms de région en vue moyenne, frontières discrètes hors
## sélection, brouillard de guerre en voile de parchemin, nuages réservés à la météo réelle.
## Usage : godot --headless --path game --script res://tests/tb2_declutter_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	if _failures == 0:
		print("tb2_declutter OK")
	quit(1 if _failures > 0 else 0)


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
	for i in 5:
		await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	_test_fog(map)
	_test_borders(map)
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


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb2_declutter: " + message)
	return condition
