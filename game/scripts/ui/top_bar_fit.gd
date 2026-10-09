class_name TopBarFit
extends RefCounted

## UX2 (audit A3, C9) : adaptation de la barre du haut de la carte à la largeur de l'écran.
## Libellés partout si la barre a la place, sinon repli des boutons en icône seule (ordre
## `TOP_COLLAPSE_ORDER`), puis mode compact (textes courts du trésor, du solde et de la date).
## Extrait de `MapUI`, qui garde les points d'entrée publics et délègue ici.

## Ordre de repli en icône seule quand la place manque (le premier se replie d'abord).
const TOP_COLLAPSE_ORDER := ["Colonies", "Unités", "Agents", "Objectifs", "Codex", "Techniques", "Cour", "Diplomatie", "Chronique"]
## Marge laissée à droite de la barre (bord, respiration).
const TOP_BAR_SLACK := 4.0
const RESEARCH_WIDTH := 190.0
const RESEARCH_WIDTH_COMPACT := 100.0
## États du bouton dont la marge droite s'élargit pour le cartouche de touche.
const KEYCAP_PAD_STATES := ["normal", "hover", "pressed", "hover_pressed", "disabled", "focus"]
## Écart entre la fin du libellé et le cartouche.
const KEYCAP_GAP := 4.0

## Q6 : barre compacte (écran étroit, grande taille d'interface) : libellés courts du trésor, du
## solde et de la date (le détail reste en infobulle), nom de faction masqué, recherche étroite.
var compact := false

var _host: Control
var _top_bar: PanelContainer
var _bar: HBoxContainer  # rangée de boutons (parent du bouton Techniques)
var _faction_label: Label
var _research_box: Control
var _research_label: Label
## Boutons libellés : `{button, label, glyph, labelled}`.
var _labels: Array[Dictionary] = []
var _fit_queued := false
var _texts: Dictionary = {}  # Label → [texte complet, texte compact]


func _init(host: Control, top_bar: PanelContainer, bar: HBoxContainer, faction_label: Label,
		research_box: Control, research_label: Label) -> void:
	_host = host
	_top_bar = top_bar
	_bar = bar
	_faction_label = faction_label
	_research_box = research_box
	_research_label = research_label


## Inscrit `button` dans la barre adaptative : `label` quand la place le permet, sinon l'icône
## (ou `glyph` pour les boutons sans icône). Nombre en attente : méta `count` (Chronique).
func register_label(button: Button, label: String, glyph: String = "") -> void:
	if button == null:
		return
	for entry in _labels:
		if entry["button"] == button:
			return
	UiType.apply(button, UiType.CAPTION)
	button.set_meta("top_label", label)
	var entry := {"button": button, "label": label, "glyph": glyph, "labelled": true}
	_labels.append(entry)
	if glyph == "" and button.icon == null:
		entry["glyph"] = label.left(1)
	_apply_label(entry, true)
	queue_fit()


func _apply_label(entry: Dictionary, labelled: bool) -> void:
	var button: Button = entry["button"]
	if not is_instance_valid(button):
		return
	entry["labelled"] = labelled
	var count := int(button.get_meta("count", 0))
	var glyph := str(entry["glyph"])
	var parts := PackedStringArray()
	if glyph != "":
		parts.append(glyph)
	if labelled:
		parts.append(str(entry["label"]))
	var text := " ".join(parts)
	if count > 0:
		text = ("%s (%d)" % [text, count]) if labelled else ("%s %d" % [text, count]).strip_edges()
	button.text = text
	pad_for_keycap(button, labelled)
	if button.has_meta("tooltip"):  # infobulle d'état fournie par le propriétaire (Chronique)
		button.tooltip_text = str(button.get_meta("tooltip"))


## Bouton libellé portant un cartouche (coin bas droit) : marge droite du style élargie de la
## largeur du cartouche + `KEYCAP_GAP`, pour que le cartouche ne morde plus la dernière lettre.
## La largeur minimale du bouton l'inclut, donc `fit` en tient compte. Icône seule :
## styles du thème (le cartouche se loge dans le coin, hors de l'icône).
func pad_for_keycap(button: Button, labelled: bool) -> void:
	var cap := button.get_node_or_null("Keycap") as Label
	for state: String in KEYCAP_PAD_STATES:
		if button.has_theme_stylebox_override(state):
			button.remove_theme_stylebox_override(state)
	var medallion := bool(button.get_meta("medallion", false))
	var padded_caps := cap != null and cap.visible and labelled
	if not padded_caps and not medallion:
		return
	var cap_width := cap.get_combined_minimum_size().x + KEYCAP_GAP if padded_caps else 0.0
	for state: String in KEYCAP_PAD_STATES:
		var base := button.get_theme_stylebox(state)
		if base == null:
			continue
		var padded: StyleBox = base.duplicate() as StyleBox
		# DA5 : au repos, le médaillon se pose sur le bandeau sans cadre plat.
		if medallion and state in ["normal", "disabled", "focus"]:
			padded = StyleBoxEmpty.new()
			for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
				padded.set_content_margin(side, maxf(base.get_margin(side), 0.0))
		padded.content_margin_right = maxf(base.get_margin(SIDE_RIGHT), 0.0) + cap_width
		button.add_theme_stylebox_override(state, padded)


## Réapplique le libellé de `button` (après un changement de méta `count` ou `tooltip`).
func refresh_button(button: Button) -> void:
	for entry in _labels:
		if entry["button"] == button:
			_apply_label(entry, bool(entry["labelled"]))
			queue_fit()
			return


## Vrai si le bouton `label` (« Cour », « Techniques »…) porte son libellé en ce moment.
func is_labelled(label: String) -> bool:
	for entry in _labels:
		if str(entry["label"]) == label:
			return bool(entry["labelled"])
	return false


func queue_fit() -> void:
	if _fit_queued or not _host.is_inside_tree():
		return
	_fit_queued = true
	fit.call_deferred()


## Libellés partout si la barre a la place, sinon repli en icône seule dans l'ordre de
## `TOP_COLLAPSE_ORDER` jusqu'à ce que la barre tienne dans la largeur de l'écran.
func fit() -> void:
	_fit_queued = false
	# Largeur de l'écran (la barre, ancrée, s'élargit au-delà quand son contenu déborde).
	var available := _host.get_viewport().get_visible_rect().size.x
	for entry in _labels:
		_apply_label(entry, true)
	set_compact(false)
	for label in TOP_COLLAPSE_ORDER:
		if bar_width() <= available - TOP_BAR_SLACK:
			break
		for entry in _labels:
			if str(entry["label"]) == label:
				_apply_label(entry, false)
	if bar_width() > available - TOP_BAR_SLACK:
		set_compact(true)


## Texte d'un libellé de la barre (trésor, solde, date, ferveur) : complet ou compact.
func set_text(label: Label, full: String, short: String) -> void:
	_texts[label] = [full, short]
	label.text = short if compact else full


## Oublie le texte à double version de `label` (Ferveur masquée).
func forget_text(label: Label) -> void:
	_texts.erase(label)


func set_compact(value: bool) -> void:
	compact = value
	for label: Label in _texts:
		label.text = str(_texts[label][1 if value else 0])
	_faction_label.visible = not value
	_research_box.custom_minimum_size.x = RESEARCH_WIDTH_COMPACT if value else RESEARCH_WIDTH
	_research_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	_research_label.clip_text = true


## Largeur minimale de la barre (contenu et marges), calculée sur les enfants visibles.
func bar_width() -> float:
	var width := 0.0
	var count := 0
	for child in _bar.get_children():
		var control := child as Control
		if control == null or not control.visible or control.top_level:
			continue
		width += control.get_combined_minimum_size().x
		count += 1
	width += float(maxi(count - 1, 0) * _bar.get_theme_constant("separation"))
	var style := _top_bar.get_theme_stylebox("panel")
	if style != null:
		width += style.get_margin(SIDE_LEFT) + style.get_margin(SIDE_RIGHT)
	return width
