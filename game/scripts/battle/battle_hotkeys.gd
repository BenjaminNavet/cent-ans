class_name BattleHotkeys
extends RefCounted

## CB2 : table unique des raccourcis de bataille (schéma de touches de la spec
## `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`, « Raccourcis »). Elle sert
## à la fois à l'aiguillage des touches (`action_for`, lignes `dispatch`), aux indications des
## boutons (`key_label`) et à l'aide F1 (`help_bbcode`). Ajouter un raccourci = ajouter une
## ligne ici :
## - `dispatch: true` : `BattleInput.handle_action(action)` le traite ;
## - `dispatch: false` : traité ailleurs (autre lot, souris, `match` de `battle_input.gd`), la
##   ligne ne sert qu'à l'aide ;
## - `label` remplace le libellé calculé (plages « Alt+1…4 », touches de souris) ;
## - `physical: true` : touche physique (libellé selon la disposition, AZERTY compris) ;
## - `pending: true` : lot pas encore fusionné, hors de l'aide ;
## - `lot` : lot d'origine (pour les fusions).
## Modificateurs (`mods`) : "" (aucun), "ctrl" (Ctrl ou Cmd), "alt" (Alt ou Option), "shift",
## "alt_shift".

const GROUPS := [
	["orders", "Ordres"],
	["modes", "Modes (bascule, sur la sélection)"],
	["abilities", "Capacités"],
	["selection", "Sélection et groupes"],
	["view", "Temps, caméra et affichage"],
]

const BINDINGS := [
	# Ordres.
	{"group": "orders", "action": "fire_at_will", "key": KEY_F, "dispatch": true, "help": "tir à volonté ou tir retenu"},
	{"group": "orders", "action": "formation", "key": KEY_T, "dispatch": true, "help": "changer de formation (ligne, colonne, schiltron, coin)"},
	{"group": "orders", "action": "halt", "key": KEY_H, "help": "halte"},
	{"group": "orders", "action": "leader_orders", "label": "{orders}", "help": "ordres du chef"},
	{"group": "orders", "action": "burn", "key": KEY_I, "physical": true, "lot": "RS-F", "help": "incendier (siège) : la maison ou la porte la plus proche à portée de torche (bouton de la barre des ordres)"},
	{"group": "orders", "action": "queue", "label": "Maj + clic droit", "help": "ajouter un point de passage (ordres en file)"},
	# Modes (CB2).
	{"group": "modes", "action": "run", "key": KEY_R, "dispatch": true, "help": "course : tous les déplacements au pas de course"},
	{"group": "modes", "action": "guard", "key": KEY_G, "dispatch": true, "help": "garde : tenir sa position, sans poursuite"},
	{"group": "modes", "action": "skirmish", "key": KEY_K, "dispatch": true, "help": "escarmouche : les tireurs reculent devant la mêlée"},
	{"group": "modes", "action": "melee", "key": KEY_M, "dispatch": true, "help": "mêlée : les tireurs engagent au lieu de tirer"},
	{"group": "modes", "action": "breach", "label": "bouton", "help": "battre en brèche : les engins de siège ne tirent que sur les murs et les portes"},
	# Capacités (CB4) : aiguillées par `ability_slot` dans `battle_input.gd` (touches physiques).
	{"group": "abilities", "action": "abilities", "label": "Alt+1…4", "lot": "CB4", "help": "capacités des unités sélectionnées (tir tendu, pavois, bannière, rangs serrés, piques plantées ; boutons sous les cartes)"},
	# Sélection et groupes.
	{"group": "selection", "action": "select_all", "key": KEY_A, "mods": "ctrl", "help": "sélectionner toutes ses unités"},
	{"group": "selection", "action": "groups", "label": "Ctrl+1…9 / 1…9", "help": "enregistrer / rappeler un groupe (deux fois : centrer la caméra)"},
	{"group": "selection", "action": "lock_group", "key": KEY_G, "mods": "ctrl", "dispatch": true, "help": "verrouiller le groupe (il garde sa forme et l'allure du plus lent)"},
	{"group": "selection", "action": "group_formation", "label": "Alt+Maj+1…6", "lot": "CB6", "help": "formation de groupe (placement proposé)"},
	{"group": "selection", "action": "deselect", "key": KEY_ESCAPE, "help": "désélectionner"},
	# Temps, caméra et affichage.
	{"group": "view", "action": "pause", "key": KEY_SPACE, "help": "pause (ordres possibles en pause)"},
	{"group": "view", "action": "speed", "label": "+ / −", "help": "vitesse de la bataille (×0,5 à ×4)"},
	{"group": "view", "action": "tactical_view", "key": KEY_TAB, "lot": "CB3", "help": "vue tactique"},
	{"group": "view", "action": "follow", "key": KEY_C, "help": "suivre la sélection (ou le général)"},
	{"group": "view", "action": "markers", "key": KEY_U, "help": "masquer / afficher les bannières"},
	{"group": "view", "action": "help", "key": KEY_F1, "help": "aide"},
]


## Action `dispatch` de la touche pressée (clé d'action, ou "" si la table ne la traite pas).
## Les modificateurs doivent correspondre exactement (Ctrl et Cmd confondus, Alt et Option).
static func action_for(key: InputEventKey) -> String:
	var mods := mods_of(key)
	for row in BINDINGS:
		if not bool(row.get("dispatch", false)) or not row.has("key"):
			continue
		var code: Key = key.physical_keycode if bool(row.get("physical", false)) else key.keycode
		if code == int(row["key"]) and str(row.get("mods", "")) == mods:
			return str(row["action"])
	return ""


## CB4 : emplacement de capacité (1…4) d'un appui Alt/Option + chiffre de la rangée (touche
## physique, AZERTY compris), 0 sinon.
static func ability_slot(key: InputEventKey) -> int:
	if mods_of(key) != "alt":
		return 0
	if key.physical_keycode >= KEY_1 and key.physical_keycode <= KEY_4:
		return int(key.physical_keycode - KEY_0)
	return 0


## Modificateurs d'un appui : "", "ctrl", "alt", "shift", "alt_shift" (autres : "other").
static func mods_of(key: InputEventKey) -> String:
	var ctrl := key.ctrl_pressed or key.meta_pressed
	if ctrl:
		return "ctrl" if not key.alt_pressed and not key.shift_pressed else "other"
	if key.alt_pressed:
		return "alt_shift" if key.shift_pressed else "alt"
	return "shift" if key.shift_pressed else ""


## Libellé du raccourci de `action` (« F », « Ctrl+G »…), "" si aucune ligne.
static func key_label(action: String) -> String:
	for row in BINDINGS:
		if str(row["action"]) == action:
			return row_label(row)
	return ""


static func row_label(row: Dictionary) -> String:
	if row.has("label"):
		return str(row["label"])
	var code: Key = int(row["key"])
	var name := OS.get_keycode_string(code)
	if bool(row.get("physical", false)) and DisplayServer.get_name() != "headless":
		var shown := DisplayServer.keyboard_get_label_from_physical(code)
		if shown != KEY_NONE:
			name = OS.get_keycode_string(shown)
	match code:
		KEY_ESCAPE:
			name = "Échap"
		KEY_SPACE:
			name = "Espace"
	var prefix := {"ctrl": "Ctrl+", "alt": "Alt+", "shift": "Maj+", "alt_shift": "Alt+Maj+"}
	return str(prefix.get(str(row.get("mods", "")), "")) + name


## Lignes d'aide des raccourcis, une par groupe (« • Ordres : F tir à volonté · T … »).
## `labels` remplace des gabarits de libellé (`{orders}` : touches des ordres du chef).
static func help_bbcode(labels: Dictionary = {}) -> String:
	var lines: PackedStringArray = []
	for group in GROUPS:
		var items: PackedStringArray = []
		for row in BINDINGS:
			if str(row["group"]) != str(group[0]) or bool(row.get("pending", false)):
				continue
			var label := row_label(row)
			for name in labels:
				label = label.replace("{%s}" % name, str(labels[name]))
			items.append("[b]%s[/b] %s" % [label, str(row["help"])])
		if not items.is_empty():
			lines.append("• %s : %s." % [str(group[1]), " · ".join(items)])
	return "\n".join(lines)
