class_name BattleTerrainTip
extends PanelContainer

const _Tooltip := preload("res://scripts/ui/rich_tooltip.gd")

## Étiquette du terrain sous le curseur en bataille (« Verger — couvert +x %, vitesse
## −y % »). Rendu seulement : les pourcentages viennent du cœur (`hover_context().decor`,
## `decor_hover_at`), rien n'est recalculé ici. Signe et flèche ▲/▼ en plus de la couleur.

const OFFSET := Vector2(20, 18)

var _label: Label


func _ready() -> void:
	name = "TerrainTip"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	add_theme_stylebox_override("panel", _box())
	_label = Label.new()
	_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	_label.add_theme_color_override("font_color", Color(0.15, 0.10, 0.05))
	add_child(_label)


func _box() -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = Color(0.93, 0.88, 0.76, 0.92)
	box.border_color = Color(0.35, 0.24, 0.12)
	box.set_border_width_all(1)
	box.set_content_margin_all(4)
	return box


## Texte de l'étiquette pour `decor` (dictionnaire du cœur) ; vide si pas de décor.
static func text_for(decor: Dictionary) -> String:
	if decor.is_empty():
		return ""
	var label := str(decor.get("label", ""))
	var parts := PackedStringArray()
	var cover := int(decor.get("cover_pct", 0))
	if cover != 0:
		parts.append("couvert %s" % _signed(cover))
	var foot := int(decor.get("foot_speed_pct", 0))
	var horse := int(decor.get("horse_speed_pct", 0))
	if foot != 0 or horse != 0:
		parts.append("vitesse %s" % (_signed(foot) if foot == horse else "%s à pied, %s à cheval" % [_signed(foot), _signed(horse)]))
	var defense := int(decor.get("defense_pct", 0))
	if defense != 0:
		parts.append("mêlée %s" % _signed(defense))
	if bool(decor.get("breaks_charge", false)):
		parts.append("brise les charges")
	var title := label.substr(0, 1).to_upper() + label.substr(1)
	return title if parts.is_empty() else "%s — %s" % [title, ", ".join(parts)]


static func _signed(percent: int) -> String:
	if percent == 0:
		return "0 %"
	return "%s%s%d %%" % [_Tooltip.mark(1 if percent > 0 else -1), "+" if percent > 0 else "−", absi(percent)]


func show_at(at: Vector2, decor: Dictionary) -> void:
	var text := text_for(decor)
	if _label == null or text == "":
		hide_tip()
		return
	_label.text = text
	position = at + OFFSET
	visible = true


func hide_tip() -> void:
	visible = false


## Texte affiché (tests).
func text() -> String:
	return _label.text if _label != null and visible else ""
