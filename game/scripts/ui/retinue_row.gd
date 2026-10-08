class_name RetinueRow
extends HFlowContainer

## Ligne de vignettes de la suite d'un général (lot C7) :
## un médaillon carré par compagnon (glyphe coloré selon la famille), infobulle riche (effets,
## obtention, transmission). Clic sur une vignette = `companion_pressed` (la fiche propose alors
## de confier le compagnon à un autre général). Aucune règle ici : tout vient de
## `CampaignSim.get_character(id).retinue` / `get_retinue_catalog()`.

signal companion_pressed(companion_id: String)

const VIGNETTE_SIZE := Vector2(58, 70)
## Couleur de la vignette, par famille de compagnon.
const CATEGORY_COLORS := {
	"military": Color(0.62, 0.13, 0.08), "court": Color(0.55, 0.40, 0.12),
	"faith": Color(0.36, 0.22, 0.45), "learning": Color(0.18, 0.36, 0.42),
	"commerce": Color(0.24, 0.34, 0.18), "intrigue": Color(0.25, 0.25, 0.28),
}
const CATEGORY_LABELS := {
	"military": "Guerre", "court": "Cour", "faith": "Foi", "learning": "Savoir",
	"commerce": "Négoce", "intrigue": "Renseignement",
}
const TRIGGER_LABELS := {
	"battle_won": "après une victoire", "battle_fought": "après une bataille",
	"siege_won": "après un siège victorieux", "raid_led": "après une chevauchée",
	"season_in_settlement": "saison passée dans une ville amie dotée de : %s",
	"ransom_received": "quand le souverain touche une rançon",
}


func _init() -> void:
	add_theme_constant_override("h_separation", 6)
	add_theme_constant_override("v_separation", 6)


## `companions` : entrées de `retinue` (`get_character`) ; `cap` : plafond ; `empty_slots` :
## dessine les emplacements libres jusqu'au plafond.
func show_retinue(companions: Array, cap: int, empty_slots: bool = true) -> void:
	for child in get_children():
		remove_child(child)
		child.queue_free()
	for companion in companions:
		var vignette := make_vignette(companion as Dictionary)
		var id := str((companion as Dictionary).get("id", ""))
		vignette.gui_input.connect(func(event: InputEvent) -> void:
			if event is InputEventMouseButton and (event as InputEventMouseButton).pressed \
					and (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
				companion_pressed.emit(id))
		add_child(vignette)
	if empty_slots:
		for _i in range(maxi(0, cap - companions.size())):
			add_child(_empty_slot())


## Vignette d'un compagnon (utilisée aussi par l'encyclopédie).
static func make_vignette(companion: Dictionary) -> PanelContainer:
	var category := str(companion.get("category", ""))
	var color: Color = CATEGORY_COLORS.get(category, HudStyle.INK_SOFT)
	var vignette := _Vignette.new()
	vignette.name = "Companion_" + str(companion.get("id", "?"))
	vignette.custom_minimum_size = VIGNETTE_SIZE
	vignette.mouse_filter = Control.MOUSE_FILTER_STOP
	vignette.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	var style := StyleBoxFlat.new()
	style.bg_color = color.lerp(HudStyle.PARCHMENT_LIGHT, 0.72)
	style.border_color = color
	style.set_border_width_all(2)
	style.border_width_top = 5
	style.set_corner_radius_all(4)
	style.set_content_margin_all(2)
	vignette.add_theme_stylebox_override("panel", style)
	var box := UiBuild.vbox(0)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var glyph := UiBuild.label(str(companion.get("glyph", "?")))
	glyph.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	UiType.apply(glyph, UiType.TITLE)  # P2a (ADR 0097) : glyphe de vignette, taille d'origine
	glyph.add_theme_color_override("font_color", color.darkened(0.2))
	glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_child(glyph)
	var label := UiBuild.label(_short_name(str(companion.get("name", ""))), 0, null, false, VIGNETTE_SIZE.x - 6)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.clip_text = true
	UiType.apply(label, UiType.CAPTION)  # P2a (ADR 0097) : plus petite taille du gabarit (14 px)
	label.add_theme_color_override("font_color", HudStyle.INK)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_child(label)
	vignette.add_child(box)
	vignette.tooltip_text = tooltip(companion)
	return vignette


static func _short_name(name: String) -> String:
	var first := name.replace("-", " ").split(" ", false)
	return first[0] if not first.is_empty() else name


func _empty_slot() -> Control:
	var slot := PanelContainer.new()
	slot.custom_minimum_size = VIGNETTE_SIZE
	var style := StyleBoxFlat.new()
	style.bg_color = Color(HudStyle.PARCHMENT_DARK, 0.35)
	style.border_color = Color(HudStyle.INK_SOFT, 0.45)
	style.set_border_width_all(1)
	style.set_corner_radius_all(4)
	slot.add_theme_stylebox_override("panel", style)
	TooltipHost.attach_plain(slot, "retinue_slot_free")
	return slot


## Infobulle riche d'un compagnon.
static func tooltip(companion: Dictionary) -> String:
	var category := str(companion.get("category", ""))
	var local := str(companion.get("local_name", ""))
	var lines: Array = ["[b]%s[/b]%s  [color=%s][i]%s[/i][/color]" % [
		str(companion.get("name", "?")), " [i](%s)[/i]" % local if local != "" else "",
		RichTooltip.MUTED, CATEGORY_LABELS.get(category, category)]]
	lines.append(RichTooltip._effects_block(companion.get("effects", [])))
	var ways := PackedStringArray()
	for rule in companion.get("acquisition", []):
		var trigger := str((rule as Dictionary).get("trigger", ""))
		var text := str(TRIGGER_LABELS.get(trigger, trigger))
		if trigger == "season_in_settlement":
			text = text % str((rule as Dictionary).get("building_name", (rule as Dictionary).get("building", "")))
		ways.append("%s (%d ‰)" % [text, int((rule as Dictionary).get("chance_permille", 0))])
	if not ways.is_empty():
		lines.append("Obtention : " + " ; ".join(ways))
	var factions: Array = Array(companion.get("factions", []))
	if not factions.is_empty():
		lines.append("Réservé à : " + ", ".join(PackedStringArray(factions)))
	if int(companion.get("min_battles", 0)) > 0:
		lines.append("Après %d batailles au moins" % int(companion.get("min_battles", 0)))
	lines.append("À la mort du maître : %s" % ("passe à son héritier" if bool(companion.get("inheritable", false)) else "quitte la maison"))
	var description := str(companion.get("description", ""))
	if description != "":
		lines.append("[color=%s][i]%s[/i][/color]" % [RichTooltip.MUTED, description])
	return RichTooltip._join(lines)


## Vignette à infobulle riche (BBCode).
class _Vignette:
	extends PanelContainer

	func _make_custom_tooltip(for_text: String) -> Object:
		return TooltipHost.bubble(for_text, self)
