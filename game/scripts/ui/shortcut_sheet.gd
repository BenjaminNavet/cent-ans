class_name ShortcutSheet
extends RefCounted

## Lot U7 (audit A3, C9 et M5) : fiche des raccourcis générée depuis l'`InputMap` (touches
## réelles du projet, libellées selon la disposition du clavier) et libellés de touches pour
## les boutons. Disposition : réglage `input/layout` (« auto », « azerty », « qwerty ») ; en
## « auto », celle que signale le système. Aucune règle de jeu.

const LAYOUTS: Array[String] = ["auto", "azerty", "qwerty"]
const LAYOUT_LABELS: Array[String] = ["Automatique (selon le système)", "AZERTY (français)", "QWERTY"]
## Touche physique (position QWERTY) → lettre gravée sur un clavier AZERTY.
const AZERTY_FROM_QWERTY := {
	KEY_Q: "A", KEY_A: "Q", KEY_W: "Z", KEY_Z: "W", KEY_M: ",", KEY_SEMICOLON: "M",
	KEY_1: "&", KEY_2: "É", KEY_3: "\"", KEY_4: "'", KEY_5: "(", KEY_6: "-", KEY_7: "È", KEY_8: "_",
	KEY_9: "Ç", KEY_0: "À",
}
## Noms français des touches spéciales.
const KEY_NAMES := {
	KEY_ENTER: "Entrée", KEY_KP_ENTER: "Entrée", KEY_ESCAPE: "Échap", KEY_SPACE: "Espace",
	KEY_UP: "↑", KEY_DOWN: "↓", KEY_LEFT: "←", KEY_RIGHT: "→", KEY_TAB: "Tab", KEY_BACKSPACE: "Retour",
	KEY_SHIFT: "Maj", KEY_CTRL: "Ctrl", KEY_ALT: "Alt", KEY_DELETE: "Suppr",
}

## Actions de la carte de campagne, par rubrique : [action, description]. Les touches viennent
## de l'`InputMap` (project.godot), jamais de ce tableau.
const CAMPAIGN_SECTIONS := [
	{"title": "Caméra", "actions": [
		["map_pan_up", "Avancer"], ["map_pan_left", "Aller à gauche"], ["map_pan_down", "Reculer"],
		["map_pan_right", "Aller à droite"], ["map_rotate_left", "Tourner à gauche"],
		["map_rotate_right", "Tourner à droite"], ["map_toggle_edge_pan", "Défilement par les bords (oui / non)"]]},
	{"title": "Fenêtres", "actions": [
		["map_toggle_court", "Cour et personnages"], ["map_toggle_tech", "Technologies"],
		["map_toggle_diplomacy", "Diplomatie et religion"], ["map_toggle_objectives", "Objectifs"],
		["map_toggle_agents", "Agents"], ["map_toggle_units", "Mes unités (armées et agents)"],
		["map_toggle_holdings", "Colonies (revenus, chantiers, menaces)"], ["codex_open", "Codex (histoire et règles)"],
		["encyclopedia_open", "Codex, onglet Règles"], ["help_open", "Aide et raccourcis"]]},
	{"title": "Filtres de carte", "actions": [
		["map_filters_menu", "Menu des filtres (richesse, population, loyauté, ravitaillement…)"],
		["map_toggle_unrest", "Mécontentement"], ["map_mode_diplomacy", "Carte diplomatique"],
		["map_mode_religion", "Carte religieuse"], ["map_toggle_trade", "Routes commerciales"]]},
	{"title": "Partie", "actions": [
		["campaign_end_turn", "Finir la saison"], ["campaign_pause", "Fermer la fenêtre du dessus, puis menu pause"],
		["quick_save", "Sauvegarde rapide"], ["quick_load", "Chargement rapide"],
		["map_screenshot", "Capture d'écran"], ["codex_pin_tooltip", "Maintenir ouverte la bulle ou l'infobulle"]]},
]
## Commandes à la souris (hors `InputMap`).
const MOUSE_LINES := [
	["Clic gauche", "Sélectionner une armée, une province ou une colonie"],
	["Clic droit", "Armée sélectionnée : marcher vers ce lieu"],
	["Molette", "Zoom"],
]


static func _settings() -> Node:
	var tree := Engine.get_main_loop() as SceneTree
	return tree.root.get_node_or_null("/root/Settings") if tree != null else null


## Disposition effective : « azerty » ou « qwerty ».
static func layout() -> String:
	var settings := _settings()
	var chosen := str(settings.call("get_value", "input/layout")) if settings != null else "auto"
	if chosen == "azerty" or chosen == "qwerty":
		return chosen
	if DisplayServer.get_name() != "headless" and DisplayServer.keyboard_get_label_from_physical(KEY_Q) == KEY_A:
		return "azerty"
	return "qwerty"


## Libellé d'une touche physique (position QWERTY) sur la disposition effective.
static func physical_label(physical: Key) -> String:
	if KEY_NAMES.has(physical):
		return str(KEY_NAMES[physical])
	var chosen := _layout_setting()
	if chosen == "auto" and DisplayServer.get_name() != "headless":
		var label := DisplayServer.keyboard_get_label_from_physical(physical)
		if label != KEY_NONE:
			return str(KEY_NAMES.get(label, OS.get_keycode_string(label)))
	if layout() == "azerty" and AZERTY_FROM_QWERTY.has(physical):
		return str(AZERTY_FROM_QWERTY[physical])
	return OS.get_keycode_string(physical)


static func _layout_setting() -> String:
	var settings := _settings()
	return str(settings.call("get_value", "input/layout")) if settings != null else "auto"


## Libellé d'un évènement clavier de l'`InputMap` (avec Ctrl / Maj).
static func event_label(event: InputEvent) -> String:
	var key := event as InputEventKey
	if key == null:
		return ""
	var text := ""
	if key.physical_keycode != KEY_NONE:
		text = physical_label(key.physical_keycode)
	elif key.keycode != KEY_NONE:
		text = str(KEY_NAMES.get(key.keycode, OS.get_keycode_string(key.keycode)))
	if text == "":
		return ""
	if key.shift_pressed:
		text = "Maj+" + text
	if key.ctrl_pressed:
		text = "Ctrl+" + text
	return text


## Touches d'une action (« Z / ↑ »), vide si l'action n'existe pas.
static func action_keys(action: String) -> String:
	if not InputMap.has_action(action):
		return ""
	var labels := PackedStringArray()
	for event in InputMap.action_get_events(action):
		var label := event_label(event)
		if label != "" and not labels.has(label):
			labels.append(label)
	return " / ".join(labels)


## Première touche d'une action (« K »), pour les boutons « Codex (K) ».
static func first_key(action: String) -> String:
	var keys := action_keys(action)
	return keys.get_slice(" / ", 0)


## Rubriques de la fiche : `[{title, lines: [[touches, description]]}]`, touches lues dans
## l'`InputMap` (les actions absentes sont omises).
static func sections() -> Array:
	var result: Array = []
	for section in CAMPAIGN_SECTIONS:
		var lines: Array = []
		for pair in section["actions"]:
			var keys := action_keys(str(pair[0]))
			if keys != "":
				lines.append([keys, str(pair[1])])
		if not lines.is_empty():
			result.append({"title": section["title"], "lines": lines})
	result.append({"title": "Souris", "lines": MOUSE_LINES.duplicate()})
	return result


## Fiche en BBCode (aide F1).
static func bbcode() -> String:
	var parts := PackedStringArray()
	for section in sections():
		parts.append("[b]%s[/b]" % section["title"])
		for line in section["lines"]:
			parts.append("• [b]%s[/b] : %s" % [line[0], line[1]])
		parts.append("")
	return "\n".join(parts).strip_edges()
