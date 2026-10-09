class_name ArmyPlate
extends RefCounted

## Plaque d'effectif d'une armée : construction et style du cartouche parchemin 2D
## (écu de la faction, glyphe d'hostilité, nombre d'hommes, état). Le placement à l'écran est
## fait par `ArmyPlateLayout`.

const PLATE_FONT_SIZE := 15
const INK := Color(0.16, 0.10, 0.05)
const PARCHMENT := Color(0.94, 0.89, 0.76, 0.94)
const PLAYER_BORDER := Color(0.85, 0.66, 0.2)
const OTHER_BORDER := Color(0.30, 0.20, 0.12)
const STATUS_TEXT := {"moving": "»", "siege": "siège", "embarked": "à bord", "idle": "inactive"}


## A6-C3 : facteur de taille des plaques (`map.plate_scale` de `data/ui/campaign_map.json`, 0,8 = −20 %).
static func plate_scale() -> float:
	return clampf(float(ArmyFigures.map_settings().get("plate_scale", 1.0)), 0.3, 1.5)


static func _ps(value: float) -> int:
	return maxi(1, roundi(value * plate_scale()))


static func build(marker: ArmyMarker, selected: bool = false) -> PanelContainer:
	var plate := PanelContainer.new()
	plate.name = "Plate_" + marker.army_id
	plate.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var row := HBoxContainer.new()
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_theme_constant_override("separation", _ps(4.0))
	plate.add_child(row)
	var heraldry := ArmyMarker._heraldry(marker.faction_id)
	if heraldry != null:
		var icon := TextureRect.new()
		icon.texture = heraldry
		icon.custom_minimum_size = Vector2(_ps(18.0), _ps(18.0))
		icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		icon.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(icon)
	else:
		var swatch := ColorRect.new()
		swatch.color = marker.faction_color
		swatch.custom_minimum_size = Vector2(_ps(12.0), _ps(16.0))
		swatch.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(swatch)
	var glyph_text := StanceCues.plate_glyph(marker.cue)
	if glyph_text != "":  # EN : marque des armées ennemies, lisible sans la couleur
		var glyph := Label.new()
		glyph.name = "EnemyGlyph"
		glyph.text = glyph_text
		glyph.add_theme_font_size_override("font_size", _ps(PLATE_FONT_SIZE - 2.0))
		glyph.add_theme_color_override("font_color", StanceCues.plate_border(marker.cue, OTHER_BORDER, 1)["color"])
		glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(glyph)
	var count := Label.new()
	count.text = format_men(marker.men)
	count.tooltip_text = FrText.count(marker.unit_count, "unité")
	count.add_theme_font_size_override("font_size", _ps(float(PLATE_FONT_SIZE)))
	count.add_theme_color_override("font_color", INK)
	count.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(count)
	var status_text: String = STATUS_TEXT.get(marker.status, "")
	if status_text != "":
		var status := Label.new()
		status.text = status_text
		status.add_theme_font_size_override("font_size", _ps(PLATE_FONT_SIZE - 3.0))
		status.add_theme_color_override("font_color", Color(0.45, 0.12, 0.08) if marker.status == "siege" else INK.lightened(0.25))
		status.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(status)
	apply_style(plate, marker, selected)
	return plate


## `hovered` (SA, ADR 0160) : plaque de l'armée sous le curseur, fond clair et liseré doré épais.
static func apply_style(plate: PanelContainer, marker: ArmyMarker, selected: bool, hovered: bool = false) -> void:
	var style := StyleBoxFlat.new()
	style.bg_color = PARCHMENT if not (selected or hovered) else PARCHMENT.lightened(0.5 if hovered else 0.35)
	# EN : bordure selon la relation avec le joueur (rouge réservé aux ennemis) ; au survol
	# (SA) elle garde sa couleur de relation et s'épaissit, le doré reste à la sélection.
	var border := StanceCues.plate_border(marker.cue, PLAYER_BORDER if marker.is_player else OTHER_BORDER, 2 if marker.is_player else 1)
	style.border_color = border["color"] if not selected else Color(1.0, 0.82, 0.3)
	style.set_border_width_all(3 if hovered else (2 if selected else int(border["width"])))
	style.set_corner_radius_all(3)
	style.content_margin_left = _ps(6.0)
	style.content_margin_right = _ps(6.0)
	style.content_margin_top = 1
	style.content_margin_bottom = 1
	style.shadow_color = Color(0, 0, 0, 0.35)
	style.shadow_size = 3
	style.shadow_offset = Vector2(1, 2)
	plate.add_theme_stylebox_override("panel", style)
	plate.z_index = 2 if hovered else (1 if selected else 0)


static func format_men(men: int) -> String:
	var text := str(men)
	if men >= 10000:
		text = "%s %03d" % [men / 1000, men % 1000]
	elif men >= 1000:
		text = "%d %03d" % [men / 1000, men % 1000]
	return text
