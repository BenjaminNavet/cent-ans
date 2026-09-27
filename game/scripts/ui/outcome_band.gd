class_name OutcomeBand
extends PanelContainer

## Lot CV3-4 : bandeau de classe de résultat d'une bataille (« Victoire héroïque », « Désastre »…),
## libellé du cœur et couleur par classe. Posé sur l'écran de fin de bataille (B2) et sur la
## notice d'auto-résolution de la carte. Aucune règle : la classe et son libellé viennent de
## `CampaignSim.get_last_battle_outcome()` / `resolve_battle().outcome` (`battle_outcome.rs`).

## Couleur de fond par classe (présentation seulement).
const CLASS_COLORS := {
	"heroic": Color(0.72, 0.53, 0.12),
	"decisive": Color(0.22, 0.42, 0.18),
	"victory": Color(0.30, 0.44, 0.22),
	"pyrrhic": Color(0.62, 0.40, 0.14),
	"honourable_defeat": Color(0.30, 0.34, 0.46),
	"defeat": Color(0.52, 0.16, 0.10),
	"disaster": Color(0.30, 0.06, 0.05),
}
const DEFAULT_COLOR := Color(0.36, 0.26, 0.16)
const TEXT_COLOR := Color(0.98, 0.93, 0.80)

## Clé de classe (`heroic`…) et libellé affiché.
var outcome_class: String = ""
var label_text: String = ""
var label: Label


## Couleur de fond de la classe `key` (repli brun pour une classe inconnue).
static func color_for(key: String) -> Color:
	return CLASS_COLORS.get(key, DEFAULT_COLOR)


## `{class, label}` vus par le joueur dans un dictionnaire de résultat du cœur
## (`player_class`/`player_label`, sinon ceux du camp `side` : "attacker" / "defender").
static func pick(outcome: Dictionary, side: String = "") -> Dictionary:
	var key := str(outcome.get("player_class", ""))
	var text := str(outcome.get("player_label", ""))
	if key == "" and side != "":
		key = str(outcome.get(side + "_class", ""))
		text = str(outcome.get(side + "_label", ""))
	return {"class": key, "label": text}


## Bandeau prêt à insérer ; `font_size` règle la hauteur.
static func create(key: String, text: String, font_size: int = 22) -> OutcomeBand:
	var band := OutcomeBand.new()
	band.name = "OutcomeBand"
	band.set_outcome(key, text, font_size)
	return band


func set_outcome(key: String, text: String, font_size: int = 22) -> void:
	outcome_class = key
	label_text = text
	var style := StyleBoxFlat.new()
	style.bg_color = color_for(key)
	style.border_color = Color(0.85, 0.68, 0.30)
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.set_corner_radius_all(3)
	style.content_margin_left = 18
	style.content_margin_right = 18
	style.content_margin_top = 3
	style.content_margin_bottom = 4
	style.shadow_color = Color(0, 0, 0, 0.35)
	style.shadow_size = 4
	add_theme_stylebox_override("panel", style)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	if label == null:
		label = Label.new()
		label.name = "ClassLabel"
		label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		label.add_theme_color_override("font_outline_color", Color(0.1, 0.04, 0.02))
		label.add_theme_constant_override("outline_size", 4)
		add_child(label)
	label.text = text
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", TEXT_COLOR)
	visible = text != ""
