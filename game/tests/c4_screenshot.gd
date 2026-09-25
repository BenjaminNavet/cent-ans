extends SceneTree

## Captures du lot C4 (TW) : sélecteur d'édit régional (section du panneau de province, édit
## « Levée de la milice » en attente, infobulle d'effets) et liste de construction triée par
## chaîne (Agen, 1337 : maison des métiers, collégiale, moulin à eau…).
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/c4_screenshot.gd
## Écrit `docs/img/c4-edits/edicts.png` et `docs/img/c4-edits/chains.png`.

const FACTION_ID := "fac_france"
const PROVINCE_ID := "prov_berry"


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img/c4-edits").simplify_path()
	DirAccess.make_dir_recursive_absolute(out_dir)
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var facade: Node = root.get_node("/root/SimFacade")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	sim.call("new_campaign", data_dir, FACTION_ID, 1337)
	facade.set("sim", sim)

	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var sheet := PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.93, 0.88, 0.76)
	style.border_color = Color(0.45, 0.33, 0.18)
	style.set_border_width_all(2)
	style.set_content_margin_all(12)
	sheet.add_theme_stylebox_override("panel", style)
	sheet.position = Vector2(40, 40)
	sheet.custom_minimum_size = Vector2(460, 0)
	root.add_child(sheet)

	# 1. Édits : choix de la milice (délai d'un tour), sélecteur ouvert, infobulle.
	var section := EdictSection.new()
	sheet.add_child(section)
	section.show_for(PROVINCE_ID, true, sim)
	section.request_edict("edict_militia_levy")
	section.options_box.visible = true
	for _i in 4:
		await process_frame
	var tip := RichTooltip.make_panel(section._tooltip(section._option("edict_feudal_aid")))
	root.add_child(tip)
	tip.position = Vector2(sheet.position.x + sheet.size.x + 20, 120)
	for _i in 4:
		await process_frame
	_save(out_dir.path_join("edicts.png"))
	tip.queue_free()
	section.queue_free()

	# 2. Chaînes : options de construction de la cité, triées par catégorie puis rang.
	var box := VBoxContainer.new()
	sheet.add_child(box)
	var header := Label.new()
	header.text = "Constructions possibles — Agen"
	header.add_theme_font_size_override("font_size", 16)
	box.add_child(header)
	var list := VBoxContainer.new()
	box.add_child(list)
	var city := "set_agen"
	var detail: Dictionary = sim.call("settlement_detail", city)
	var built: Array = detail.get("buildings", [])
	# Même rendu que `PanelWidgets.fill_buildable` (non chargeable ici : `IconLibrary` est un
	# autoload absent du mode --script), même tri catégorie puis rang.
	var rows: Array = (sim.call("settlement_buildable", city) as Array).filter(func(r: Dictionary) -> bool: return not built.has(str(r.get("building", ""))))
	rows.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var da := GameCatalog.building(str(a.get("building", "")))
		var db := GameCatalog.building(str(b.get("building", "")))
		if str(da.get("category", "")) != str(db.get("category", "")):
			return str(da.get("category", "")) < str(db.get("category", ""))
		return int(da.get("tier", 1)) < int(db.get("tier", 1)))
	for row in rows:
		var definition := GameCatalog.building(str(row.get("building", "")))
		var line := HBoxContainer.new()
		var button := Button.new()
		button.text = "%s (rang %d) — %d ℔ / %d tour(s)" % [str(row.get("name", "")), int(definition.get("tier", 1)), int(row.get("cost", 0)), int(row.get("turns", 1))]
		button.disabled = not bool(row.get("available", false))
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		line.add_child(button)
		if button.disabled:
			var reason := Label.new()
			reason.text = str(row.get("reason", ""))
			reason.add_theme_font_size_override("font_size", 13)
			reason.add_theme_color_override("font_color", Color(0.55, 0.20, 0.15))
			line.add_child(reason)
		list.add_child(line)
	for _i in 6:
		await process_frame
	sheet.size = Vector2.ZERO
	await process_frame
	_save(out_dir.path_join("chains.png"))
	quit(0)


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("c4 screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
