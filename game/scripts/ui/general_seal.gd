class_name GeneralSeal
extends Control

## Sceau du chef (HUD de campagne, bas à gauche ; lot F10a) : médaillon de cire aux armes
## de la faction, ou portrait découpé en disque quand il existe (`PortraitLoader`), pastille
## des points de compétence non dépensés, rang sous le sceau ; à droite, un cartouche avec
## nom, titre, compétences (Cdt / Gouv / Cour), posture, ravitaillement et mouvement restant.
##
## Aucune règle de jeu : `set_general(character, army)` reçoit `CampaignSim.get_character(id)`
## (`{id, name, title, faction, skills{command, governance, court}, skill_points,
## skills_learned[]}`) et `CampaignSim.get_army(id)` (`{stance, supply, movement_points}`).
## Clic n'importe où sur le composant = `general_requested(character_id)`.

## Demande d'ouvrir la fiche du personnage.
signal general_requested(character_id: String)
## Posture choisie dans la rangée de boutons (armée du joueur seulement, `can_change_stance`).
signal stance_selected(stance: String)

const SEAL_RADIUS := 58.0
const PLATE_WIDTH := 212.0
## Lot CV3-4 : postures proposées, dans l'ordre des boutons (`StanceBar`).
const STANCES := StanceBar.STANCES

## Vrai pour une armée du joueur : une rangée de boutons de posture (`StanceBar`, lot CV3-4)
## émet `stance_selected` ; `army.stance_options` (`get_stance_options`) grise les refus.
@export var can_change_stance: bool = true

var character: Dictionary = {}
var army: Dictionary = {}
var faction_id: String = ""

var _portrait: Texture2D
var _heraldry: Texture2D
var _hover := false
var _plate: PanelContainer
var _name_label: Label
var _title_label: Label
var _skills_label: Label
var _status_row: HBoxContainer
## Lot CV3-4 : boutons de posture (armée du joueur).
var stance_bar: StanceBar


func _ready() -> void:
	custom_minimum_size = Vector2(SEAL_RADIUS * 2.0 + 14.0 + PLATE_WIDTH, SEAL_RADIUS * 2.0 + 24.0)
	mouse_filter = Control.MOUSE_FILTER_STOP
	mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	mouse_entered.connect(func() -> void:
		_hover = true
		queue_redraw())
	mouse_exited.connect(func() -> void:
		_hover = false
		queue_redraw())

	_plate = PanelContainer.new()
	_plate.add_theme_stylebox_override("panel", HudStyle.panel_box(8))
	_plate.mouse_filter = Control.MOUSE_FILTER_IGNORE
	# Le sceau (dessiné par ce nœud) passe devant le cartouche.
	_plate.show_behind_parent = true
	_plate.position = Vector2(SEAL_RADIUS * 2.0 - 6.0, 18.0)
	_plate.custom_minimum_size = Vector2(PLATE_WIDTH + 20.0, 0)
	add_child(_plate)
	# Le sceau chevauche le bord gauche du cartouche : marge intérieure plus large.
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 18)
	margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_plate.add_child(margin)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 1)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	margin.add_child(box)
	_name_label = HudStyle.label("", HudStyle.FONT_TITLE)
	_name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_name_label.custom_minimum_size = Vector2(PLATE_WIDTH - 4.0, 0)
	box.add_child(_name_label)
	_title_label = HudStyle.label("", HudStyle.FONT_SMALL, HudStyle.RUBRIC)
	_title_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_title_label.custom_minimum_size = Vector2(PLATE_WIDTH - 4.0, 0)
	box.add_child(_title_label)
	_skills_label = HudStyle.label("", HudStyle.FONT_BODY, HudStyle.INK_SOFT)
	box.add_child(_skills_label)
	_status_row = HBoxContainer.new()
	_status_row.add_theme_constant_override("separation", 10)
	_status_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_child(_status_row)
	stance_bar = StanceBar.new()
	stance_bar.stance_selected.connect(func(stance: String) -> void: stance_selected.emit(stance))
	box.add_child(stance_bar)
	_refresh()


## Affiche le chef. `army` (facultatif) fournit posture, ravitaillement et mouvement ;
## `faction` remplace `character.faction` / `army.faction` pour l'écu.
## `character.portrait` (chemin res://) force un portrait ; sinon `PortraitLoader` cherche
## `assets/portraits/<id>.png`.
func set_general(new_character: Dictionary, new_army: Dictionary = {}, faction: String = "") -> void:
	character = new_character.duplicate(true)
	army = new_army.duplicate(true)
	faction_id = faction
	if faction_id == "":
		faction_id = str(character.get("faction", army.get("faction", "")))
	_portrait = null
	var portrait_path := str(character.get("portrait", ""))
	if portrait_path != "":
		_portrait = PortraitLoader.load_texture(portrait_path)
	elif str(character.get("id", "")) != "":
		_portrait = PortraitLoader.portrait_texture(str(character["id"]))
	_heraldry = PortraitLoader.house_heraldry_texture(str(character.get("house", "")), faction_id)  # DA1
	_refresh()


## Rang affiché : `character.rank` s'il existe, sinon 1 + compétences apprises.
func rank() -> int:
	if character.has("rank"):
		return int(character["rank"])
	return 1 + (character.get("skills_learned", []) as Array).size()


## Points de compétence non dépensés (pastille rouge si > 0).
func unspent_points() -> int:
	return int(character.get("skill_points", 0))


func _refresh() -> void:
	if _plate == null:
		return
	var has_general := not character.is_empty()
	var name := str(character.get("name", ""))
	var epithet := str(character.get("epithet", ""))
	if epithet != "" and not name.contains(epithet):
		name = "%s %s" % [name, epithet]
	_name_label.text = name if has_general else "Sans chef"
	var title := str(character.get("title", ""))
	_title_label.text = title if has_general else "Nommez un chef depuis la cour"
	_title_label.visible = _title_label.text != ""
	var skills: Dictionary = character.get("skills", {})
	_skills_label.visible = has_general
	_skills_label.text = "Cdt %d · Gouv %d · Cour %d" % [
		int(skills.get("command", 0)), int(skills.get("governance", 0)), int(skills.get("court", 0))]
	_skills_label.tooltip_text = "Commandement, gouvernement, cour"
	_skills_label.mouse_filter = Control.MOUSE_FILTER_PASS
	for child in _status_row.get_children():
		child.queue_free()
	stance_bar.visible = not army.is_empty() and can_change_stance
	if not army.is_empty():
		var stance := str(army.get("stance", "normal"))
		if can_change_stance:
			stance_bar.set_state(stance, army.get("stance_options", {}))
		else:
			_add_status("stance_" + stance, StanceBar.stance_name(stance), "Posture")
		var supply := int(army.get("supply", 0))
		_add_status("supply", "%d %%" % supply, "Ravitaillement", HudStyle.gauge_color(supply / 100.0))
		# TW2-T2 : reconstitution saisonnière (taux et facteurs calculés par le cœur).
		var replenishment: Dictionary = army.get("replenishment", {})
		if not replenishment.is_empty():
			_add_status("infantry", replenishment_text(replenishment),
				str(replenishment.get("tooltip", "Reconstitution")), replenishment_color(replenishment))
		var moves := int(army.get("movement_points", 0))
		_add_status("movement", str(moves), "Mouvement restant : %s" % FrText.count(moves, "point"))
	tooltip_text = _tooltip_text()
	_fit_plate.call_deferred()
	queue_redraw()


## Lot CV3-4 : le sceau s'agrandit pour contenir le cartouche (rangée des postures comprise).
func _fit_plate() -> void:
	if _plate == null:
		return
	var needed := maxf(SEAL_RADIUS * 2.0 + 24.0, _plate.position.y + _plate.get_combined_minimum_size().y + 2.0)
	if not is_equal_approx(custom_minimum_size.y, needed):
		custom_minimum_size.y = needed


## TW2-T2 : « +20 % » (part des hommes manquants regagnée en fin de saison), « 0 % » en terre ennemie.
## Le taux reste affiché quand rien ne revient (effectifs au complet, bataille) : grisé.
static func replenishment_text(replenishment: Dictionary) -> String:
	var percent := int(replenishment.get("percent", 0))
	return "+%d %%" % percent if percent > 0 else "0 %"


static func replenishment_color(replenishment: Dictionary) -> Color:
	if str(replenishment.get("blocked", "")) != "":
		return HudStyle.INK_FADED
	return HudStyle.GOOD


func _add_status(glyph: String, text: String, tip: String, color: Color = HudStyle.INK) -> HBoxContainer:
	var item := HBoxContainer.new()
	item.add_theme_constant_override("separation", 3)
	item.mouse_filter = Control.MOUSE_FILTER_PASS  # infobulle, clic transmis au sceau
	var icon := GlyphIcon.new()
	icon.glyph = glyph
	icon.color = color
	icon.custom_minimum_size = Vector2(18, 18)
	item.add_child(icon)
	var label := HudStyle.label(text, HudStyle.FONT_SMALL + 1, color)
	item.add_child(label)
	item.tooltip_text = tip
	_status_row.add_child(item)
	return item


func _tooltip_text() -> String:
	if character.is_empty():
		return "Armée sans chef"
	var lines := PackedStringArray([_name_label.text])
	var points := unspent_points()
	if points > 0:
		lines.append("%s de compétence à dépenser" % FrText.count(points, "point"))
	lines.append("Clic : fiche du personnage")
	return "\n".join(lines)


func _gui_input(event: InputEvent) -> void:
	var click := event as InputEventMouseButton
	if click != null and click.pressed and click.button_index == MOUSE_BUTTON_LEFT:
		general_requested.emit(str(character.get("id", "")))
		accept_event()


func _draw() -> void:
	var center := Vector2(SEAL_RADIUS + 4.0, SEAL_RADIUS + 4.0)
	var seed_value := hash(faction_id) % 97
	var wax := HudStyle.WAX_LIGHT if _hover else HudStyle.WAX
	var inner := HudStyle.draw_wax_seal(self, center, SEAL_RADIUS, wax, seed_value)
	if _portrait != null:
		HudStyle.draw_texture_disc(self, _portrait, center, inner - 2.0)
		draw_arc(center, inner - 2.0, 0.0, TAU, 64, HudStyle.GOLD, 2.0, true)
	else:
		draw_circle(center, inner - 2.0, HudStyle.PARCHMENT_LIGHT)
		draw_arc(center, inner - 2.0, 0.0, TAU, 64, HudStyle.GOLD, 1.5, true)
		if _heraldry != null:
			HudStyle.draw_texture_fit(self, _heraldry, center + Vector2(0, 1), inner * 1.35)
		else:
			HudStyle.draw_glyph(self, "infantry", center, inner, HudStyle.INK_SOFT)
	# Rang sur un phylactère sous le sceau.
	if not character.is_empty():
		var font := get_theme_default_font()
		var text := "Rang %d" % rank()
		var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, 12)
		var band := Rect2(center + Vector2(-text_size.x * 0.5 - 10.0, SEAL_RADIUS - 4.0), Vector2(text_size.x + 20.0, 18.0))
		draw_rect(Rect2(band.position + Vector2(1, 2), band.size), HudStyle.SHADOW)
		draw_rect(band, HudStyle.PARCHMENT)
		draw_rect(band, HudStyle.INK_SOFT, false, 1.0)
		draw_string(font, Vector2(band.position.x + 10.0, band.position.y + 14.0), text, HORIZONTAL_ALIGNMENT_LEFT, -1, 12, HudStyle.INK)
	# Pastille des points de compétence non dépensés.
	var points := unspent_points()
	if points > 0:
		var badge := center + Vector2(cos(-PI * 0.25), sin(-PI * 0.25)) * SEAL_RADIUS * 0.95
		draw_circle(badge + Vector2(1, 2), 13.0, HudStyle.SHADOW)
		draw_circle(badge, 13.0, HudStyle.RUBRIC)
		draw_arc(badge, 13.0, 0.0, TAU, 24, HudStyle.GOLD_PALE, 1.5, true)
		var font := get_theme_default_font()
		var text := str(points)
		var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14)
		draw_string(font, badge + Vector2(-text_size.x * 0.5, 5.0), text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, HudStyle.PARCHMENT_LIGHT)


## Petit pictogramme (IconLibrary `hud_<id>` sinon dessin au trait).
class GlyphIcon:
	extends Control

	var glyph: String = ""
	var color: Color = HudStyle.INK

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_IGNORE

	func _draw() -> void:
		var texture := HudStyle.icon("hud_" + glyph, "hud")
		var center := size * 0.5
		if texture != null:
			HudStyle.draw_texture_fit(self, texture, center, minf(size.x, size.y))
		else:
			HudStyle.draw_glyph(self, glyph, center, minf(size.x, size.y) - 2.0, color)
