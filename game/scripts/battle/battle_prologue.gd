class_name BattlePrologue
extends Node

## NT4 — bataille-prologue guidée (« Didacticiel de bataille », depuis « Batailles historiques »
## du menu principal). Petite escarmouche de 1337 construite comme une bataille personnalisée
## (`BattleSim.setup_custom`, NT2) ; les étapes viennent de `data/tutorial/battle_prologue.json`
## et s'affichent dans le parchemin du tutoriel de campagne (`TutorialOverlay`) : texte du
## conseiller, objectif, surlignage de l'élément d'interface visé. Chaque étape passe quand sa
## condition est remplie sur l'état observé de la bataille (`snapshot` → `evaluate`, fonctions
## pures testées sans scène) : caméra déplacée, sélection faite, régiment arrivé, front changé,
## charge ou tir en cours, pause, victoire. L'ennemi reste passif (IA du cœur coupée) jusqu'à
## l'étape marquée `enemy_ai`. Rendu et interface seulement : les règles restent au cœur.

signal step_changed(index: int)
signal finished

const DATA_FILE := "tutorial/battle_prologue.json"
const TUTORIAL_SCENE := "res://scenes/ui/tutorial.tscn"
const BATTLE_SCENE := "res://scenes/battle/battle.tscn"
const CHECK_INTERVAL := 0.25
const DONE_PAUSE := 0.8

var data: Dictionary = {}
var steps: Array = []
var step_index := -1
## Scène de bataille (`BattleScene`) ; null dans les tests d'évaluation pure.
var scene: Node = null
var overlay: TutorialOverlay = null
## État observé à l'entrée de l'étape (positions, fronts, caméra).
var baseline: Dictionary = {}
var done := false
var enemy_ai_enabled := false
## Défaite affichée (texte d'adieu, « Recommencer »).
var defeated := false
var restart_button: Button = null
## Faux en test : `battle_prologue/done` n'est pas enregistré.
var persist_progress := true
## Étape dont l'objectif vient d'être rempli (bref retour avant la suivante).
var _done_timer := -1.0
var _check_timer := 0.0


# --- Données -------------------------------------------------------------------------------


static func data_path() -> String:
	return DataFile.data_dir().path_join(DATA_FILE)


## Contenu de `data/tutorial/battle_prologue.json` ({} s'il manque ou est illisible).
static func load_data(path: String = "") -> Dictionary:
	var file_path := path if path != "" else data_path()
	var text := FileAccess.get_file_as_string(file_path)
	if text == "":
		return {}
	var parsed: Variant = JSON.parse_string(text)
	return parsed if parsed is Dictionary else {}


## Configuration de bataille personnalisée du prologue (le JSON lit les nombres en flottants ;
## le cœur attend des entiers pour les budgets, la fortification et la graine).
static func battle_config(prologue: Dictionary) -> Dictionary:
	var out: Dictionary = (prologue.get("battle", {}) as Dictionary).duplicate(true)
	for side in ["attacker", "defender"]:
		if out.get(side) is Dictionary:
			out[side]["budget"] = int(out[side].get("budget", 0))
	for key in ["fortification", "seed"]:
		if out.has(key):
			out[key] = int(out[key])
	return out


## Lance la bataille-prologue par le chemin des batailles personnalisées ; false sans données.
static func launch(tree: SceneTree) -> bool:
	var prologue := load_data()
	if prologue.is_empty() or not prologue.has("battle"):
		return false
	BattleScene.custom_config = battle_config(prologue)
	BattleScene.prologue_data = prologue
	var audio := tree.root.get_node_or_null("/root/AudioDirector")
	if audio != null and audio.has_method("stop_all"):
		audio.call("stop_all")
	SceneFader.go(BATTLE_SCENE)
	return true


# --- Évaluation (pure) ---------------------------------------------------------------------


## État observé de la bataille de `p_scene` : sélection, régiments, pause, caméra, issue.
static func snapshot(p_scene: Node) -> Dictionary:
	var battle: Object = p_scene.get("battle")
	var rig: Node = p_scene.get("camera_rig")
	var finished := battle != null and bool(battle.call("is_finished"))
	var winner := ""
	if finished:
		winner = str((battle.call("get_outcome") as Dictionary).get("winner", ""))
	return {
		"player_side": str(p_scene.get("player_side")),
		"selected": Array(p_scene.get("selected")),
		"units": Array(p_scene.get("units")),
		"paused": bool(p_scene.get("paused")),
		"camera": {"target": rig.get("target"), "distance": float(rig.get("distance")), "yaw": float(rig.get("yaw"))} if rig != null else {},
		"finished": finished,
		"winner": winner,
	}


## Référence d'une étape : position, front et formation de chaque régiment, caméra.
static func baseline_of(state: Dictionary) -> Dictionary:
	var units := {}
	for unit in state.get("units", []):
		units[int(unit["id"])] = {
			"x": float(unit.get("x", 0.0)), "z": float(unit.get("z", 0.0)),
			"width": float(unit.get("width", 0.0)), "formation": str(unit.get("formation", "")),
		}
	return {"units": units, "camera": (state.get("camera", {}) as Dictionary).duplicate()}


## Régiments présents du joueur (de la catégorie `category` si non vide).
static func _own_units(state: Dictionary, category: String = "") -> Array:
	var out := []
	var side := str(state.get("player_side", ""))
	for unit in state.get("units", []):
		if str(unit.get("side", "")) != side or not bool(unit.get("present", true)):
			continue
		if category != "" and str(unit.get("category", "")) != category:
			continue
		out.append(unit)
	return out


## La condition `condition` d'une étape est-elle remplie sur `state` (référence `base`) ?
static func evaluate(condition: Dictionary, state: Dictionary, base: Dictionary) -> bool:
	var category := str(condition.get("category", ""))
	match str(condition.get("type", "manual")):
		"manual":
			return false
		"camera_moved":
			var now: Dictionary = state.get("camera", {})
			var before: Dictionary = base.get("camera", {})
			if now.is_empty() or before.is_empty():
				return false
			var moved := (now["target"] as Vector3).distance_to(before["target"] as Vector3)
			var zoom := absf(float(now["distance"]) - float(before["distance"]))
			var turned := absf(angle_difference(float(now["yaw"]), float(before["yaw"])))
			return moved >= float(condition.get("min_distance", 40.0)) \
				or zoom >= float(condition.get("min_zoom", 30.0)) \
				or turned >= float(condition.get("min_yaw", 0.4))
		"selection":
			var count := 0
			var own_ids := _own_units(state, category).map(func(u: Dictionary) -> int: return int(u["id"]))
			for id in state.get("selected", []):
				if own_ids.has(int(id)):
					count += 1
			return count >= int(condition.get("min_count", 1))
		"moved":
			var units: Dictionary = base.get("units", {})
			for unit in _own_units(state, category):
				var before: Dictionary = units.get(int(unit["id"]), {})
				if before.is_empty():
					continue
				var dist := Vector2(float(unit["x"]), float(unit["z"])).distance_to(Vector2(float(before["x"]), float(before["z"])))
				if dist >= float(condition.get("min_distance", 25.0)):
					return true
			return false
		"formation_changed":
			var units: Dictionary = base.get("units", {})
			for unit in _own_units(state, category):
				var before: Dictionary = units.get(int(unit["id"]), {})
				if before.is_empty():
					continue
				if str(unit.get("formation", "")) != str(before["formation"]):
					return true
				var width := float(before["width"])
				if width > 0.0 and absf(float(unit.get("width", width)) - width) / width >= float(condition.get("min_ratio", 0.15)):
					return true
			return false
		"unit_state":
			var states: Array = condition.get("states", [])
			for unit in _own_units(state, category):
				if states.has(str(unit.get("state", ""))):
					return true
			return false
		"paused":
			return bool(state.get("paused", false))
		"victory":
			return bool(state.get("finished", false)) and str(state.get("winner", "")) == str(state.get("player_side", ""))
	return false


# --- Déroulement ---------------------------------------------------------------------------


## Prépare le prologue `p_data` (étapes) ; `attach` le branche ensuite sur une scène.
func setup(p_data: Dictionary) -> void:
	data = p_data
	steps = Array(data.get("steps", []))


## Branche le prologue sur la scène de bataille : parchemin, ennemi passif, première étape.
func attach(p_scene: Node) -> void:
	scene = p_scene
	var hud: Node = scene.get("hud")
	overlay = (load(TUTORIAL_SCENE) as PackedScene).instantiate()
	overlay.name = "PrologueOverlay"
	overlay.hide()
	if hud != null:
		hud.add_child(overlay)
	else:
		scene.add_child(overlay)
	overlay.later_button.visible = false  # pas de « Plus tard » en bataille
	overlay.continue_pressed.connect(advance)
	overlay.skip_step_pressed.connect(advance)
	overlay.skip_all_pressed.connect(finish)
	overlay.step_chosen.connect(go_to)
	var titles := PackedStringArray()
	for step in steps:
		titles.append(str(step.get("title", "")))
	overlay.set_steps(titles)
	if bool(data.get("enemy_passive", false)):
		_set_enemy_ai(false)
	go_to(0)


func current_step() -> Dictionary:
	return steps[step_index] if step_index >= 0 and step_index < steps.size() else {}


func current_step_id() -> String:
	return str(current_step().get("id", ""))


## Index de l'étape `id` (-1 si absente).
func index_of(id: String) -> int:
	for i in steps.size():
		if str(steps[i].get("id", "")) == id:
			return i
	return -1


func go_to(index: int) -> void:
	if defeated or index < 0 or index >= steps.size():
		return
	step_index = index
	_done_timer = -1.0
	_check_timer = 0.0
	var step: Dictionary = steps[index]
	if scene != null:
		baseline = baseline_of(snapshot(scene))
	if bool(step.get("enemy_ai", false)):
		_set_enemy_ai(true)
	if overlay != null:
		overlay.show_step(_overlay_step(step), index, steps.size())
	step_changed.emit(index)


func advance() -> void:
	if done:
		return
	if defeated or step_index >= steps.size() - 1:
		finish()
	else:
		go_to(step_index + 1)


## Fin du guide (dernière étape ou « Passer le tutoriel ») : l'ennemi reprend la main. Hors
## défaite, le didacticiel compte comme fait (plus d'invite au premier lancement).
func finish() -> void:
	if done:
		return
	done = true
	_set_enemy_ai(true)
	if overlay != null:
		overlay.hide()
		overlay.set_target({})
		if restart_button != null:
			restart_button.visible = false
	if not defeated and persist_progress:
		var settings := get_node_or_null("/root/Settings")
		if settings != null:
			settings.call("set_value", BattlePrologueInvite.DONE_KEY, true)
	finished.emit()


## Défaite : texte d'adieu du conseiller, « Recommencer » (relance l'escarmouche) ou « Fermer ».
func show_defeat() -> void:
	if done or defeated:
		return
	defeated = true
	_done_timer = -1.0
	var defeat: Dictionary = data.get("defeat", {})
	if overlay == null:
		return
	overlay.show_step(_overlay_step({
		"id": "defeat",
		"title": str(defeat.get("title", "Défaite")),
		"text": str(defeat.get("text", "")),
		"objective": "",
		"condition": {"type": "manual"},
	}), step_index, steps.size())
	overlay.set_target({})
	overlay.continue_button.text = "Fermer"
	overlay.skip_step_button.visible = false
	if restart_button == null:
		restart_button = Button.new()
		restart_button.name = "RestartButton"
		restart_button.text = "Recommencer"
		restart_button.pressed.connect(restart)
		overlay.continue_button.add_sibling(restart_button)
		overlay.continue_button.get_parent().move_child(restart_button, overlay.continue_button.get_index())
	restart_button.visible = true


## Relance la bataille-prologue depuis le début.
func restart() -> void:
	done = true
	if not Engine.is_editor_hint() and is_inside_tree():
		BattlePrologue.launch(get_tree())


func _overlay_step(step: Dictionary) -> Dictionary:
	var condition: Dictionary = step.get("condition", {})
	var advisor := str(data.get("advisor", ""))
	var text := str(step.get("text", ""))
	if advisor != "":
		text = "[i]%s :[/i] %s" % [advisor, text]
	return {
		"id": str(step.get("id", "")),
		"title": str(step.get("title", "")),
		"text": text,
		"objective": str(step.get("objective", "")),
		"advice": str(step.get("advice", "")),
		"target": str(step.get("target", "")),
		"manual": str(condition.get("type", "manual")) == "manual",
	}


## IA ennemie coupée = camp tenu (NT11, option du cœur `set_hold`) : l'ennemi garde sa place et
## ne recule pas de lui-même ; il peut encore se débander sous une forte pression. L'étape
## `enemy_ai` (victoire) rend l'IA et retire le camp tenu.
func _set_enemy_ai(enabled: bool) -> void:
	enemy_ai_enabled = enabled
	if scene == null:
		return
	var battle: Object = scene.get("battle")
	if battle != null:
		var side := str(scene.get("enemy_side"))
		battle.call("set_ai", side, enabled)
		if battle.has_method("set_hold"):
			battle.call("set_hold", side, not enabled)


## Contrôle ou point à surligner pour la cible `target` ({} : rien).
func resolve_target(target: String) -> Dictionary:
	if scene == null or target == "":
		return {}
	var hud: Node = scene.get("hud")
	match target:
		"cards":
			var cards: Control = hud.get("cards_box") if hud != null else null
			if cards != null and cards.is_visible_in_tree():
				return {"rect": cards.get_global_rect()}
		"pause":
			var buttons: Array = hud.get("_speed_buttons") if hud != null else []
			if not buttons.is_empty() and (buttons[0] as Control).is_visible_in_tree():
				return {"rect": (buttons[0] as Control).get_global_rect()}
		"minimap":
			var minimap: Control = hud.get("minimap") if hud != null else null
			if minimap != null and minimap.is_visible_in_tree():
				return {"rect": minimap.get_global_rect()}
		"enemy":
			for unit in Array(scene.get("units")):
				if str(unit["side"]) == str(scene.get("enemy_side")) and bool(unit["present"]):
					return _unit_point(unit)
		_:
			if target.begins_with("unit:"):
				var state := snapshot(scene)
				for unit in _own_units(state, target.trim_prefix("unit:")):
					return _unit_point(unit)
	return {}


func _unit_point(unit: Dictionary) -> Dictionary:
	var point: Vector2 = scene.call("_unit_screen", unit)
	if point.x < -1e5:
		return {}
	return {"point": point}


func _process(delta: float) -> void:
	if done or defeated or scene == null or step_index < 0:
		return
	overlay.set_target(resolve_target(str(current_step().get("target", ""))))
	if _done_timer >= 0.0:
		_done_timer -= delta
		if _done_timer < 0.0:
			advance()
		return
	_check_timer -= delta
	if _check_timer > 0.0:
		return
	_check_timer = CHECK_INTERVAL
	check_now()


## Vérifie l'étape courante sur l'état de la bataille (appelé toutes les `CHECK_INTERVAL` s).
func check_now() -> bool:
	if done or defeated or scene == null:
		return false
	var state := snapshot(scene)
	# La bataille est finie avant l'étape de victoire (l'ennemi s'est débandé plus tôt) : on y va.
	if bool(state["finished"]):
		var victory := index_of("victory")
		if str(state["winner"]) != str(state["player_side"]):
			show_defeat()
			return false
		if victory > step_index:
			go_to(victory)
	var condition: Dictionary = current_step().get("condition", {})
	if str(condition.get("type", "manual")) == "manual":
		return false
	if evaluate(condition, state, baseline):
		if overlay != null:
			overlay.mark_done()
		_done_timer = DONE_PAUSE
		return true
	return false
