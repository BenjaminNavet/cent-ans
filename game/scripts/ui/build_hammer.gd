class_name BuildHammer
extends Button

## RG5 : petit marteau « on peut bâtir ici » à côté d'une colonie. Allumé si le cœur dit
## `can_build` ; grisé, avec le motif en infobulle, sinon. Un clic émet `activated` (c'est à
## l'appelant d'ouvrir la vue des bâtiments). Aucune règle ici : verdict et motif viennent du cœur.

signal activated(settlement_id: String)

const SIZE := Vector2(22, 22)
const LIT := Color(1, 1, 1, 1)
const DIM := Color(0.5, 0.5, 0.5, 0.45)

var settlement_id: String = ""
var can_build: bool = false


func _init() -> void:
	name = "BuildHammer"
	flat = true
	focus_mode = Control.FOCUS_NONE
	custom_minimum_size = SIZE
	expand_icon = true
	icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
	mouse_filter = Control.MOUSE_FILTER_STOP
	pressed.connect(func() -> void:
		if can_build:
			activated.emit(settlement_id))


## `verdict` : `{ok, reason}` (`settlement_can_build`) ou une ligne de `get_holdings_overview`
## (`can_build`, `can_build_reason`).
func setup(id: String, ok: bool, reason: String) -> BuildHammer:
	settlement_id = id
	can_build = ok
	var texture: Texture2D = IconLibrary.get_icon("glyph_hammer")
	if texture != null:
		icon = texture
	else:
		text = "⚒"
	modulate = LIT if ok else DIM
	tooltip_text = "Construction possible ici : cliquer pour bâtir." if ok else (reason if reason != "" else "Construction impossible ici.")
	return self


## Marteau pour une ligne du résumé des colonies (`get_holdings_overview`).
static func for_holding(settlement: Dictionary) -> BuildHammer:
	return BuildHammer.new().setup(str(settlement.get("id", "")), bool(settlement.get("can_build", false)), str(settlement.get("can_build_reason", "")))
