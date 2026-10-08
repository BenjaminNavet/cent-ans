extends SceneTree

## Chantier IB (ADR 0109), lot IB1 : infobulles en sections (spec § 2.1-2.3, critères § 5).
## - chaque type (unité en recrutement, unité d'armée, bâtiment, technique) produit les blocs
##   attendus, versions courte et complète ;
## - version courte ≤ `short_max_body_lines` lignes de corps pour toutes les unités de 1337 ;
## - aucune infobulle ne dépasse l'écran à 720p (largeur, hauteur bornée) ;
## - tailles de police issues de `UiType` (titre Heading, corps Body, pied Caption) ;
## - clé « ib:<kind>:<id> » dans `tooltip_text` reconstruite par `panel_for`, repli BBCode.
## Usage : godot --headless --path game --script res://tests/ib_layout_test.gd

const ENABLED := true
const YEAR := 1337

var _failures := 0
var _checks := 0


func _init() -> void:
	if not ENABLED:
		print("ib_layout_test: disabled (IB0 skeleton)")
		quit(0)
		return
	_run.call_deferred()


func _check(condition: bool, message: String) -> bool:
	_checks += 1
	if not condition:
		_failures += 1
		push_error("ib_layout_test: " + message)
		print("FAIL: " + message)
	return condition


func _run() -> void:
	await process_frame
	root.size = Vector2i(1280, 720)
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")
	await process_frame
	var style := TooltipView.style()
	_check(int(style.get("width_px", 0)) >= 240 and style.has("headline"), "tooltip_style.json should be loaded")
	await _check_kinds()
	await _check_short_units(int(style.get("short_max_body_lines", 8)))
	await _check_keys()
	_check_live()
	print("ib_layout_test: %d checks, %s" % [_checks, "OK" if _failures == 0 else "%d failure(s)" % _failures])
	quit(0 if _failures == 0 else 1)


## Blocs attendus par type, versions courte et complète ; tailles et place à l'écran.
func _check_kinds() -> void:
	var recruit := {"cost": 420, "upkeep": 40, "available": true}
	var refused := {"cost": 420, "upkeep": 40, "available": false, "reason": "aucune place de recrutement"}
	var army := {"strength": 96, "max_strength": 120, "morale": 62}
	var building_id := _building_with_effects()
	var tech_id := _technology_with_effects()
	var cases := [
		# [nom, spec, blocs requis en version courte, blocs requis en version complète, vedettes]
		["recrutement", RichTooltip.unit_spec("unit_longbowmen", recruit), ["Headline", "Traits"], ["Headline", "Stats", "Traits", "Flavour"], ["cost", "melee_or_ranged"]],
		["refus", RichTooltip.unit_spec("unit_longbowmen", refused), ["Headline", "Conditions"], ["Conditions"], ["cost", "melee_or_ranged"]],
		["armée", RichTooltip.unit_spec("unit_longbowmen", army), ["Headline"], ["Headline", "Stats"], ["strength", "morale"]],
		["bâtiment", RichTooltip.building_spec(building_id, {"cost": 300, "turns": 2, "available": true}), ["Headline"], ["Headline", "Conditions"], ["main_effect"]],
		["technique", RichTooltip.technology_spec(_tech_node(tech_id)), ["Headline", "Effects"], ["Headline", "Effects"], ["research_cost"]],
	]
	for case in cases:
		var spec: Dictionary = case[1]
		_check(not spec.is_empty() and str(spec.get("title", "")) != "", "%s: spec should have a title" % case[0])
		var keys: Array = []
		for item in spec.get("headline", []):
			keys.append(str(item.get("key", "")))
		_check(keys == case[4], "%s: headline %s, expected %s" % [case[0], keys, case[4]])
		for detailed in [false, true]:
			var panel := TooltipView.build(spec, detailed)
			root.add_child(panel)
			await process_frame
			await process_frame
			var blocks: PackedStringArray = panel.get_meta("ib_blocks", PackedStringArray())
			for block in (case[3] if detailed else case[2]):
				_check(blocks.has(block), "%s (%s): block %s missing in %s" % [case[0], "complète" if detailed else "courte", block, blocks])
			if not detailed:
				_check(not blocks.has("Stats") and not blocks.has("Flavour") and not blocks.has("Detail"), "%s: short version should not show %s" % [case[0], blocks])
			_check_sizes(panel, str(case[0]))
			_check_on_screen(panel, "%s (%s)" % [case[0], "complète" if detailed else "courte"])
			_check(TooltipHost.last_panel != null and TooltipHost.last_panel.get_ref() == panel, "%s: last_panel should be the view" % case[0])
			_check(TooltipHost.last_bbcode.contains(str(spec.get("title", ""))) or TooltipHost.last_bbcode.contains("[url="), "%s: last_bbcode should carry the title" % case[0])
			_check(TooltipHost.last_spec == spec, "%s: last_spec expected" % case[0])
			panel.free()
	# Prérequis ✓ (version complète) / ✗ ; avertissement ⚠ ; un effet par ligne ; avant → après.
	var spec := RichTooltip.building_spec(building_id, {"available": true, "before_after": {}})
	var met := {"id": "x", "kind": "building", "title": "Essai", "requires": [{"text": "Arsenal", "met": true}, {"text": "Arbalète", "met": false}], "warnings": ["Indisponible : test"],
		"effects": [{"key": "army_morale", "label": "Moral", "value": "+5", "text": "Moral +5", "sign": 1, "before": 60, "after": 65}, {"text": "Mécontentement +2", "sign": -1}]}
	var short := TooltipView.blocks_for(met, false)
	var full := TooltipView.blocks_for(met, true)
	var short_conditions := "\n".join(_block(short, "Conditions"))
	var full_conditions := "\n".join(_block(full, "Conditions"))
	_check(short_conditions.contains("✗") and not short_conditions.contains("✓") and short_conditions.contains("⚠"), "short: unmet ✗ and ⚠ only: %s" % short_conditions)
	_check(full_conditions.contains("✓") and full_conditions.contains("✗"), "full: ✓ and ✗: %s" % full_conditions)
	var effects := _block(short, "Effects")
	_check(effects.size() == 2 and effects[0].contains("60 → 65"), "one effect per line with before → after: %s" % effects)
	_check(not spec.is_empty(), "building spec with live expected")


## Version courte ≤ `limit` lignes de corps pour les unités recrutables en 1337.
func _check_short_units(limit: int) -> void:
	var count := 0
	var units := GameCatalog.definitions("unit_types")
	for unit_id in units:
		var definition: Dictionary = units[unit_id]
		var from := int(definition.get("available_from", 0))
		var until := int(definition.get("available_until", 0))
		if (from > 0 and from > YEAR) or (until > 0 and until < YEAR):
			continue
		count += 1
		for live in [{"cost": 500, "upkeep": 50, "available": true}, {"strength": 50, "max_strength": 100, "morale": 60}]:
			var panel := TooltipView.build(RichTooltip.unit_spec(str(unit_id), live), false)
			var lines := int(panel.get_meta("ib_body_lines", 99))
			_check(lines <= limit, "%s: short tooltip has %d body lines (> %d)" % [unit_id, lines, limit])
			panel.free()
	_check(count >= 15, "expected about 20 units in %d, got %d" % [YEAR, count])


## Clé « ib: » : `set_tooltip` pose clé + repli, `panel_for` reconstruit la vue ; `make_panel`
## ignore la clé ; `spec_for` sert les bulles verrouillées (IB3/IB4).
func _check_keys() -> void:
	var button := RichButton.new()
	TooltipHost.set_tooltip(button, "unit", "unit_longbowmen", {"cost": 420, "upkeep": 40, "available": true})
	_check(button.tooltip_text.begins_with("ib:unit:unit_longbowmen\n") and button.tooltip_text.contains("Coût"), "tooltip_text should carry the key and the BBCode fallback")
	var view: Control = button._make_custom_tooltip(button.tooltip_text)
	_check(view != null and view.name == "TooltipView", "RichButton should build a TooltipView from an ib: key")
	var footer := view.find_child("Hint", true, false) as Label
	_check(footer != null and footer.text == "Alt : explorer", "footer hint should announce Alt")
	view.free()
	var plain: Control = button._make_custom_tooltip("[b]Texte[/b] simple")
	_check(plain.name != "TooltipView", "plain BBCode keeps make_panel")
	plain.free()
	var fallback := TooltipHost.from_bbcode(button.tooltip_text)
	_check(not TooltipHost.last_bbcode.contains("ib:unit"), "make_panel should drop the key line")
	fallback.free()
	button.free()
	for key in ["ib:unit:unit_longbowmen", "ib:building:" + _building_with_effects(), "ib:technology:" + _technology_with_effects()]:
		var spec := RichTooltip.spec_for(key)
		_check(not spec.is_empty() and str(spec.get("kind", "")) == key.get_slice(":", 1), "spec_for(%s) expected" % key)
	_check(RichTooltip.spec_for("ib:nothing:x").is_empty() and RichTooltip.spec_for("cdx:x").is_empty(), "unknown keys give {}")
	var bbcode := RichTooltip.unit("unit_longbowmen", {"cost": 420, "upkeep": 40, "available": false, "reason": "test"})
	_check(bbcode.contains("Coût") and bbcode.contains("Indisponible : test") and bbcode.contains("Forces"), "BBCode fallback keeps its content: %s" % bbcode)


## IB5 : « avant → après » et état de chaque prérequis calculés par le core (`CampaignSim`) :
## une infobulle de bâtiment constructible montre une ligne « a → b », un prérequis manquant est ✗
## et un prérequis rempli ✓ ; mêmes données pour les techniques et le recrutement.
func _check_live() -> void:
	if not _check(ClassDB.class_exists("CampaignSim"), "CampaignSim missing (run core/build.sh)"):
		return
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(bool(sim.call("new_campaign", data_dir, "fac_france", YEAR)), "new_campaign failed"):
		return
	var capital := "prov_ile_de_france"
	var rows: Array = (sim.call("get_province_city", capital) as Dictionary).get("buildable", [])
	var arrow := false
	var unmet := false
	var met := false
	for row in rows:
		_check((row as Dictionary).has("requirements") and row.has("before_after"), "buildable row should carry requirements and before_after: %s" % [row.keys()])
		var spec := RichTooltip.building_spec(str(row.get("building", "")), row)
		var effects := "\n".join(_block(TooltipView.blocks_for(spec, false), "Effects"))
		if str(row.get("reason", "")) != "déjà construit" and not (row.get("before_after", {}) as Dictionary).is_empty() and effects.contains(" → "):
			arrow = true
		var conditions := "\n".join(_block(TooltipView.blocks_for(spec, true), "Conditions"))
		for requirement in row.get("requirements", []):
			if bool(requirement.get("met", true)) == false and conditions.contains("✗"):
				unmet = true
			if bool(requirement.get("met", false)) and conditions.contains("✓"):
				met = true
	_check(arrow, "a buildable building tooltip should show a « before → after » line (%s: %d rows)" % [capital, rows.size()])
	_check(unmet, "a missing building requirement should be ✗")
	_check(met, "a met building requirement should be ✓")
	# Techniques : prérequis et avant → après de faction.
	var tech_arrow := false
	var tech_state := false
	for node in sim.call("get_tech_tree", "fac_france"):
		var spec := RichTooltip.technology_spec(node)
		if " → " in "\n".join(_block(TooltipView.blocks_for(spec, false), "Effects")):
			tech_arrow = true
		for requirement in node.get("requirements", []):
			for item in spec.get("requires", []):
				if str(item.get("text", "")).contains(GameCatalog.display_name(str(requirement.get("id", "")))) and item.get("met") == bool(requirement.get("met")):
					tech_state = true
	_check(tech_arrow, "a technology tooltip should show a « before → after » line")
	_check(tech_state, "technology prerequisites should follow the core's state")
	# Recrutement : chaque ligne porte l'état de ses prérequis.
	var recruit: Array = sim.call("get_recruitable", capital)
	_check(not recruit.is_empty() and (recruit[0] as Dictionary).has("requirements"), "recruitable rows should carry requirements")
	for row in recruit:
		for requirement in row.get("requirements", []):
			if str(requirement.get("id", "")) == "enabling_building":
				var spec := RichTooltip.unit_spec(str(row.get("unit_type", "")), row)
				var states: Array = []
				for item in spec.get("requires", []):
					states.append(item.get("met"))
				_check(states.has(bool(requirement.get("met"))), "%s: enabling building state %s expected in %s" % [row.get("unit_type"), requirement.get("met"), states])


func _check_sizes(panel: Control, label: String) -> void:
	var title := panel.find_child("Title", true, false) as RichTextLabel
	_check(title != null and title.get_theme_font_size("normal_font_size") == UiType.size(UiType.HEADING), "%s: title should be Heading" % label)
	# Mise en page réelle : titre sur 1-2 lignes (pas replié lettre à lettre), chiffres vedettes lisibles.
	if title != null:
		_check(title.size.x >= 120.0 and title.size.y <= UiType.size(UiType.HEADING) * 3.5, "%s: title laid out %s" % [label, title.size])
	for badge in panel.find_children("Badge_*", "", true, false):
		var value := badge.find_child("Value", true, false) as Control
		_check(value != null and value.size.x >= 10.0 and value.size.y <= UiType.size(UiType.TITLE) * 2.5, "%s: headline value laid out %s" % [label, value.size if value != null else Vector2.ZERO])
	for name in ["Effects", "Traits", "Conditions"]:
		var body := panel.find_child(name, true, false) as RichTextLabel
		if body != null:
			_check(body.get_theme_font_size("normal_font_size") == UiType.size(UiType.BODY), "%s: %s should be Body" % [label, name])
	var hint := panel.find_child("Hint", true, false) as Label
	_check(hint != null and hint.get_theme_font_size("font_size") == UiType.size(UiType.CAPTION), "%s: footer hint should be Caption" % label)


func _check_on_screen(panel: Control, label: String) -> void:
	var screen := root.get_visible_rect().size
	var size := panel.get_combined_minimum_size()
	_check(size.x <= screen.x, "%s: width %.0f exceeds the screen (%.0f)" % [label, size.x, screen.x])
	_check(size.y <= screen.y and size.y <= TooltipView.max_height() + 1.0, "%s: height %.0f exceeds %.0f (screen %.0f)" % [label, size.y, TooltipView.max_height(), screen.y])


func _block(blocks: Array, name: String) -> PackedStringArray:
	for entry in blocks:
		if str(entry[0]) == name:
			return PackedStringArray(entry[1])
	return PackedStringArray()


func _building_with_effects() -> String:
	var buildings := GameCatalog.definitions("buildings")
	var ids := buildings.keys()
	ids.sort()
	for id in ids:
		if not ((buildings[id] as Dictionary).get("effects", []) as Array).is_empty() and (buildings[id] as Dictionary).has("required_building"):
			return str(id)
	return "bld_castle"


func _technology_with_effects() -> String:
	var techs := GameCatalog.definitions("technologies")
	var ids := techs.keys()
	ids.sort()
	for id in ids:
		if not ((techs[id] as Dictionary).get("effects", []) as Array).is_empty():
			return str(id)
	return ""


func _tech_node(id: String) -> Dictionary:
	var node := RichTooltip.technology_node(id)
	node["state"] = "available"
	return node
