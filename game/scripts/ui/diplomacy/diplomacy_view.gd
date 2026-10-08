class_name DiplomacyView
extends PanelSection

## Base des sous-vues de l'écran de diplomatie (`DiplomacyPanel`) : contexte commun (simulation,
## joueur, faction choisie et sa fiche), constructeurs de libellés du manuscrit et ordres
## remontés au panneau. La fille remplit sa vue dans `_render` ; aucune règle ici.

signal order_requested(order: Dictionary, success_text: String)

## FA5 : côté du sceau de cire réel (bouton « Proposer le traité », traités signés).
const TREATY_SEAL_SIZE := 28
const STATUS_LABELS := {
	"war": "En guerre", "truce": "Trêve", "peace": "Paix", "alliance": "Alliance",
	"vassal": "Notre vassal", "suzerain": "Notre suzerain",
}
const STATUS_COLORS := {
	"war": Color(0.62, 0.12, 0.10), "truce": Color(0.66, 0.50, 0.08), "peace": Color(0.35, 0.33, 0.30),
	"alliance": Color(0.15, 0.32, 0.62), "vassal": Color(0.42, 0.20, 0.55), "suzerain": Color(0.42, 0.20, 0.55),
}

var sim: Object = null
var player_faction: String = ""
## Identifiant de la faction choisie et sa fiche de `get_diplomacy` ({} : aucune).
var faction_id: String = ""
var entry: Dictionary = {}


## Affiche la vue pour la faction choisie `selected` et sa fiche `faction_entry` ({} : inconnue).
func show_for(simulation: Object, player: String, selected: String, faction_entry: Dictionary = {}) -> void:
	sim = simulation
	player_faction = player
	faction_id = selected
	entry = faction_entry
	_render()


## À surcharger : reconstruit la vue depuis `sim`, `faction_id` et `entry`.
func _render() -> void:
	pass


static func _rule() -> Control:
	var rule := ColorRect.new()
	rule.color = HudStyle.GOLD
	rule.custom_minimum_size = Vector2(0, 2)
	rule.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return rule


static func _section(text: String) -> Label:
	var label := _label(text, UiType.HEADING, HudStyle.RUBRIC)
	label.add_theme_constant_override("outline_size", 0)
	return label


## Libellé du manuscrit : `HudStyle.label` (police, couleur) puis une variation `UiType` (taille
## de la bible DA § 12.2, jamais moins de `Caption`).
static func _label(text: String, variation: String, color: Color = HudStyle.INK) -> Label:
	var node := HudStyle.label(text, UiType.size(variation), color)
	UiType.apply(node, variation)
	return node


## Blason de la faction (carré `side`), pastille de sa couleur à défaut.
static func _heraldry(faction: String, side: int, fallback: String) -> Control:
	var texture := PortraitLoader.heraldry_texture(faction)
	if texture == null:
		var swatch := ColorRect.new()
		swatch.color = Color.html(fallback)
		swatch.custom_minimum_size = Vector2(side * 0.8, side)
		swatch.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		swatch.mouse_filter = Control.MOUSE_FILTER_IGNORE
		return swatch
	var rect := TextureRect.new()
	rect.texture = texture
	rect.custom_minimum_size = Vector2(side, side)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return rect
