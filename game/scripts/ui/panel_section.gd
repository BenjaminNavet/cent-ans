class_name PanelSection
extends VBoxContainer

## Base des sections des panneaux de faction et de province (Monnaie, Chevalerie, Ferveur,
## Féodalité, Table, Édit). Elle porte le squelette commun : nom et espacement, simulation (celle
## de `SimFacade` par défaut), message de refus en rouge, texte riche aux liens du Codex, ordre
## soumis puis section rafraîchie. La fille construit ses widgets dans son `_init` (après
## `super(...)`) et remplit la section dans son `show_for` / `refresh` ; aucune règle ici.

const ERROR_COLOR := Color(0.55, 0.20, 0.15)
const MUTED_COLOR := Color(0.42, 0.33, 0.20)

var last_result: Dictionary = {}
var read_only := false
var error_label: Label
var _sim: Object = null
## Textes riches dont les mots du Codex deviennent cliquables à l'entrée dans l'arbre.
var _codex_labels: Array = []


func _init(section_name: String = "", separation: int = 4, expand: bool = true) -> void:
	name = section_name
	add_theme_constant_override("separation", separation)
	if expand:
		size_flags_horizontal = Control.SIZE_EXPAND_FILL


func _ready() -> void:
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		for label in _codex_labels:
			bubbles.call("attach", label)


## Simulation à utiliser : `sim` si fournie, sinon celle de `SimFacade` (null hors jeu).
func _resolve_sim(sim: Object) -> Object:
	if sim != null:
		return sim
	var facade := get_node_or_null("/root/SimFacade") if is_inside_tree() else null
	if facade == null:
		var loop := Engine.get_main_loop() as SceneTree
		facade = loop.root.get_node_or_null("/root/SimFacade") if loop != null else null
	return facade.get("sim") if facade != null else null


## Crée `error_label` (masqué, retour à la ligne) et l'ajoute à la section.
func _add_error_label(min_width: float = 0.0, color: Color = ERROR_COLOR) -> Label:
	error_label = Label.new()
	error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	if min_width > 0.0:
		error_label.custom_minimum_size = Vector2(min_width, 0)
	UiType.apply(error_label, UiType.CAPTION)
	error_label.add_theme_color_override("font_color", color)
	error_label.hide()
	add_child(error_label)
	return error_label


## Texte riche (BBCode) à retour à la ligne, couleur d'encre ; `font_size` pose les trois graisses.
## Si `codex`, ses mots du Codex sont cliquables dès l'entrée dans l'arbre.
func _rich_text(font_size: int, min_width: float = 160.0, codex: bool = false) -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.custom_minimum_size = Vector2(min_width, 0)
	label.add_theme_color_override("default_color", RichTooltip.INK)
	for key in ["normal_font_size", "bold_font_size", "italics_font_size"]:
		label.add_theme_font_size_override(key, font_size)
	if codex:
		_codex_labels.append(label)
	return label


## Soumet `order` à la simulation (ou à `via` si fourni), mémorise `last_result`, appelle
## `refresh` puis affiche le refus en rouge. Renvoie vrai si l'ordre est accepté.
func _submit(order: Dictionary, refresh: Callable, via: Callable = Callable()) -> bool:
	last_result = via.call(order) if via.is_valid() else _sim.call("submit_order", order)
	var ok := bool(last_result.get("ok", false))
	refresh.call()
	error_label.visible = not ok
	error_label.text = "Refusé : %s" % str(last_result.get("error", "?")) if not ok else ""
	return ok


## Option d'id `id` dans `options` (`{}` si absente).
static func find_option(options: Array, id: String) -> Dictionary:
	for option in options:
		if str(option.get("id", "")) == id:
			return option
	return {}
