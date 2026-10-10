class_name BattleHotkeys
extends RefCounted

## Table unique des raccourcis de bataille (schéma de touches de la spec
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

## Réglage où sont enregistrées les touches réaffectées : action → {key, mods}.
const SETTING_KEY := "input/battle_bindings"
## Lignes à touche non réaffectable : Échap et F1 (sorties de secours), I (barre des ordres du chef).
const FIXED: Array[String] = ["deselect", "help", "burn"]
## Modificateurs acceptables pour une touche réaffectée.
const VALID_MODS: Array[String] = ["", "ctrl", "alt", "shift", "ctrl_shift", "alt_shift"]
## Touches réservées par `battle_input.gd` (hors table) : on ne peut pas les prendre.
const RESERVED: Array[String] = ["", "shift"]

## Surcharges de l'utilisateur : action → {key: int, mods: String} (touche saisie, voir `capture`).
static var overrides: Dictionary = {}

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
	{"group": "orders", "action": "formation", "key": KEY_T, "dispatch": true, "help": "changer de formation (formation suivante permise ; le bouton Formation ouvre le menu des formations historiques)"},
	{"group": "orders", "action": "halt", "key": KEY_H, "dispatch": true, "help": "halte"},
	{"group": "orders", "action": "pursue", "key": KEY_P, "dispatch": true, "help": "poursuivre : la sélection (cavalerie surtout) pourchasse les fuyards ennemis les plus proches"},
	{"group": "orders", "action": "leader_orders", "label": "{orders}", "help": "ordres du chef"},
	{"group": "orders", "action": "burn", "key": KEY_I, "physical": true, "lot": "RS-F", "help": "incendier (siège) : la maison ou la porte la plus proche à portée de torche (bouton de la barre des ordres)"},
	{"group": "orders", "action": "queue", "label": "Maj + clic droit", "help": "ajouter un point de passage (ordres en file)"},
	{"group": "orders", "action": "walk_attack", "label": "Alt + clic droit", "lot": "BCTRL", "help": "sur un ennemi : attaquer au pas (sans courir)"},
	{"group": "orders", "action": "pivot", "label": "glisser droit sur place", "lot": "BCTRL", "help": "pivoter sur place : le front se tourne sans que l’unité bouge"},
	{"group": "orders", "action": "minimap_order", "label": "clic droit minicarte", "lot": "BCTRL", "help": "déplacer la sélection vers ce point de la minicarte (Maj : en file)"},
	# Modes (CB2).
	{"group": "modes", "action": "run", "key": KEY_R, "dispatch": true, "help": "course : tous les déplacements au pas de course"},
	{"group": "modes", "action": "guard", "key": KEY_G, "dispatch": true, "help": "garde : tenir sa position, sans poursuite"},
	{"group": "modes", "action": "skirmish", "key": KEY_K, "dispatch": true, "help": "escarmouche : les tireurs reculent devant la mêlée"},
	{"group": "modes", "action": "melee", "key": KEY_M, "dispatch": true, "help": "mêlée : les tireurs engagent au lieu de tirer"},
	{"group": "modes", "action": "breach", "label": "bouton", "help": "battre en brèche : les engins de siège ne tirent que sur les murs et les portes"},
	# Capacités (CB4) : aiguillées par `ability_slot` dans `battle_input.gd` (touches physiques).
	{"group": "abilities", "action": "abilities", "label": "Alt+1…4", "lot": "CB4", "help": "capacités des unités sélectionnées (tir tendu, pavois, bannière, rangs serrés, piques plantées ; boutons sous les cartes)"},
	# Sélection et groupes.
	{"group": "selection", "action": "select_all", "key": KEY_A, "mods": "ctrl", "dispatch": true, "help": "sélectionner toutes ses unités"},
	{"group": "selection", "action": "select_shooters", "key": KEY_A, "mods": "ctrl_shift", "dispatch": true, "lot": "BCTRL", "help": "sélectionner tous les tireurs"},
	{"group": "selection", "action": "select_cavalry", "key": KEY_C, "mods": "ctrl_shift", "dispatch": true, "lot": "BCTRL", "help": "sélectionner toute la cavalerie"},
	{"group": "selection", "action": "next_idle", "key": KEY_PERIOD, "physical": true, "dispatch": true, "lot": "BCTRL", "help": "unité suivante au repos (sélection et caméra)"},
	{"group": "selection", "action": "prev_idle", "key": KEY_COMMA, "physical": true, "dispatch": true, "lot": "BCTRL", "help": "unité précédente au repos"},
	{"group": "selection", "action": "groups", "label": "Ctrl+1…9 / 1…9", "help": "enregistrer / rappeler un groupe (deux fois : centrer la caméra)"},
	{"group": "selection", "action": "lock_group", "key": KEY_G, "mods": "ctrl", "dispatch": true, "help": "verrouiller le groupe (il garde sa forme et l’allure du plus lent)"},
	{"group": "selection", "action": "group_formation", "label": "Alt+Maj+1…6", "lot": "CB6", "help": "formation de groupe (placement proposé)"},
	{"group": "selection", "action": "deselect", "key": KEY_ESCAPE, "help": "désélectionner"},
	# Temps, caméra et affichage.
	{"group": "view", "action": "pause", "key": KEY_SPACE, "dispatch": true, "help": "pause (ordres possibles en pause)"},
	{"group": "view", "action": "speed", "label": "+ / −", "help": "vitesse de la bataille (×0,5 à ×4)"},
	{"group": "view", "action": "tactical_view", "key": KEY_TAB, "dispatch": true, "lot": "CB3", "help": "vue tactique"},
	{"group": "view", "action": "bookmarks", "label": "Ctrl+F2…F4 / F2…F4", "lot": "BCTRL", "help": "enregistrer / rappeler un signet de caméra"},
	{"group": "view", "action": "follow", "key": KEY_C, "dispatch": true, "help": "suivre la sélection (ou le général)"},
	{"group": "view", "action": "markers", "key": KEY_U, "dispatch": true, "help": "masquer / afficher les bannières"},
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
		var binding := effective(row)
		if code == int(binding["key"]) and str(binding["mods"]) == mods:
			return str(row["action"])
	return ""


## Touche et modificateurs en vigueur d'une ligne : surcharge de l'utilisateur, sinon défaut.
static func effective(row: Dictionary) -> Dictionary:
	var custom: Variant = overrides.get(str(row["action"]), null)
	if custom is Dictionary and (custom as Dictionary).has("key"):
		return {"key": int(custom["key"]), "mods": str(custom.get("mods", ""))}
	return {"key": int(row.get("key", 0)), "mods": str(row.get("mods", ""))}


static func row_of(action: String) -> Dictionary:
	for row in BINDINGS:
		if str(row["action"]) == action:
			return row
	return {}


## Lignes réaffectables : une touche unique, aiguillée par la table, hors `FIXED`.
static func reconfigurable() -> Array:
	var result: Array = []
	for row in BINDINGS:
		if row.has("key") and not FIXED.has(str(row["action"])) and not bool(row.get("pending", false)):
			result.append(row)
	return result


## Touche saisie `event` sous la forme d'une surcharge `{key, mods}` pour `row` ({} si invalide :
## modificateurs non pris en charge, touche modificatrice seule).
static func capture(row: Dictionary, event: InputEventKey) -> Dictionary:
	var mods := mods_of(event)
	if not VALID_MODS.has(mods):
		return {}
	var code: Key = event.physical_keycode if bool(row.get("physical", false)) else event.keycode
	if code == KEY_NONE or code in [KEY_SHIFT, KEY_CTRL, KEY_ALT, KEY_META]:
		return {}
	return {"key": int(code), "mods": mods}


## Autre ligne qui utilise déjà (`key`, `mods`) hors `action` ("" sans conflit). Les touches
## fixes comptent ; une ligne physique et une ligne non physique ne se comparent qu'au code.
static func action_using(key: int, mods: String, except: String) -> String:
	for row in BINDINGS:
		if not row.has("key") or str(row["action"]) == except or bool(row.get("pending", false)):
			continue
		var binding := effective(row)
		if int(binding["key"]) == key and str(binding["mods"]) == mods:
			return str(row["action"])
	return ""


## Réaffecte `action` à `binding` ({key, mods}). Si une autre ligne réaffectable l'utilisait,
## elle reçoit l'ancienne touche de `action` (échange) ; une ligne fixe refuse (renvoie
## `"!" + action`). Renvoie l'action échangée ("" sans conflit). Persiste dans `settings`.
static func rebind(action: String, binding: Dictionary, settings: Object = null) -> String:
	var row := row_of(action)
	if row.is_empty() or binding.is_empty() or FIXED.has(action):
		return ""
	var other := action_using(int(binding["key"]), str(binding["mods"]), action)
	var mine := effective(row)
	if other != "":
		if FIXED.has(other):
			return "!" + other
		overrides[other] = mine
	overrides[action] = {"key": int(binding["key"]), "mods": str(binding["mods"])}
	_prune()
	_save(settings)
	return other


## Rétablit les touches par défaut (toutes, ou `action` seule).
static func reset(action: String = "", settings: Object = null) -> void:
	if action == "":
		overrides = {}
	else:
		overrides.erase(action)
	_save(settings)


## Retire les surcharges identiques au défaut.
static func _prune() -> void:
	for action in overrides.keys():
		var row := row_of(str(action))
		var custom: Dictionary = overrides[action]
		if row.is_empty() or (int(custom["key"]) == int(row["key"]) and str(custom["mods"]) == str(row.get("mods", ""))):
			overrides.erase(action)


static func _save(settings: Object) -> void:
	if settings != null:
		settings.call("set_value", SETTING_KEY, overrides.duplicate(true))


## Applique les surcharges enregistrées (au démarrage) ; lignes inconnues, fixes ou invalides ignorées.
static func apply_saved(stored: Variant) -> void:
	overrides = {}
	if not stored is Dictionary:
		return
	for action in (stored as Dictionary):
		var row := row_of(str(action))
		var custom: Variant = stored[action]
		if row.is_empty() or FIXED.has(str(action)) or not row.has("key") or not custom is Dictionary:
			continue
		if int(custom.get("key", 0)) != 0 and VALID_MODS.has(str(custom.get("mods", ""))):
			overrides[str(action)] = {"key": int(custom["key"]), "mods": str(custom["mods"])}
	# Un fichier à la main peut créer un doublon : on repart des défauts plutôt que d'ambiguïser.
	var seen := {}
	for row in BINDINGS:
		if row.has("key"):
			var b := effective(row)
			var id := "%d/%s" % [int(b["key"]), str(b["mods"])]
			if seen.has(id):
				overrides = {}
				return
			seen[id] = true


## Emplacement de capacité (1…4) d'un appui Alt/Option + chiffre de la rangée (touche
## physique, AZERTY compris), 0 sinon.
static func ability_slot(key: InputEventKey) -> int:
	if mods_of(key) != "alt":
		return 0
	if key.physical_keycode >= KEY_1 and key.physical_keycode <= KEY_4:
		return int(key.physical_keycode - KEY_0)
	return 0


## Signet de caméra (2…4) d'un appui F2…F4, 0 sinon ; `save_mode` : Ctrl/Cmd + Fn (enregistrer),
## sans modificateur = rappeler. Autres modificateurs : 0.
static func bookmark_slot(key: InputEventKey, save_mode: bool) -> int:
	if key.keycode < KEY_F2 or key.keycode > KEY_F4:
		return 0
	if mods_of(key) != ("ctrl" if save_mode else ""):
		return 0
	return int(key.keycode - KEY_F1) + 1


## Modificateurs d'un appui : "", "ctrl", "ctrl_shift", "alt", "shift", "alt_shift" (autres : "other").
static func mods_of(key: InputEventKey) -> String:
	var ctrl := key.ctrl_pressed or key.meta_pressed
	if ctrl:
		if key.alt_pressed:
			return "other"
		return "ctrl_shift" if key.shift_pressed else "ctrl"
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
	var binding := effective(row)
	var code: Key = int(binding["key"])
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
	var prefix := {"ctrl": "Ctrl+", "alt": "Alt+", "shift": "Maj+", "alt_shift": "Alt+Maj+", "ctrl_shift": "Ctrl+Maj+"}
	return str(prefix.get(str(binding["mods"]), "")) + name


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
