class_name BattleReplayController
extends RefCounted

## Rejeu d'après bataille. `--replay=<fichier>` (menu « Rejeux ») ou « Revoir la
## bataille » sur l'écran de fin : le cœur re-simule la bataille enregistrée, la scène la montre
## sans ordre. Extrait de `BattleScene`, qui garde l'état public (`replay_mode`, `replay_bar`,
## `replay_error`, `replay_saved_path`) et des raccourcis vers ces méthodes.

var _scene: BattleScene = null


func _init(scene: BattleScene) -> void:
	_scene = scene


## Rejeu du fichier `path` ; `replay_error` dit pourquoi en cas d'échec (autre format, bataille
## impossible à reconstruire).
func begin(path: String) -> bool:
	var scene := _scene
	if not ClassDB.class_exists("BattleSim"):
		scene.replay_error = "extension absente"
		return false
	scene.audio.silence_campaign()
	scene.battle = ClassDB.instantiate("BattleSim")
	var result: Dictionary = scene.battle.call("load_replay", path)
	if not bool(result.get("ok", false)):
		scene.replay_error = str(result.get("error", "?"))
		scene.battle = null
		return false
	scene.setup = scene.battle.call("get_setup")
	scene.padded = true  # hors campagne : rien à rapporter
	scene.replay_mode = true
	if not scene._build_scene():
		return false
	var info: Dictionary = scene.battle.call("get_replay")
	if str(info.get("title", "")) != "":
		scene._title_text = str(info["title"])
		scene.hud.set_title(scene._title_text, scene._weather_text, [scene.side_colors[scene.player_side], scene.side_colors[scene.enemy_side]])
	enter()
	return true


## Le rejeu ne peut être lu : message, puis retour au menu principal.
func failed() -> void:
	var dialog := AcceptDialog.new()
	dialog.title = "Rejeu"
	dialog.dialog_text = "Ce rejeu ne peut être revu : %s." % _scene.replay_error
	dialog.confirmed.connect(func() -> void: SceneFader.go("res://scenes/start_menu.tscn"))
	dialog.canceled.connect(func() -> void: SceneFader.go("res://scenes/start_menu.tscn"))
	_scene.hud.add_child(dialog)
	dialog.popup_centered()


## Passe la scène en rejeu (barre de rejeu, ordres, déploiement et vitesses de combat cachés).
func enter() -> void:
	var scene := _scene
	var hud := scene.hud
	scene.replay_mode = true
	scene.paused = false
	scene.speed = 1.0
	scene.selected.clear()
	if scene.deployment != null:
		scene.deployment.queue_free()
		scene.deployment = null
	if scene._leader_bar != null:
		scene._leader_bar.queue_free()  # ordres du chef : rien à ordonner pendant un rejeu
		scene._leader_bar = null
	if hud.withdraw_all_button != null:
		hud.withdraw_all_button.get_parent().visible = false  # ordres et retraite générale
	if not hud._speed_buttons.is_empty():
		hud._speed_buttons[0].get_parent().visible = false  # la barre de rejeu a ses vitesses
	if hud.toast_label != null:
		hud.toast_label.get_parent().visible = true
	scene.replay_bar = BattleReplayBar.new()
	hud.root.add_child(scene.replay_bar)
	var info: Dictionary = scene.battle.call("get_replay")
	scene.replay_bar.setup(scene._title_text, float(info.get("duration", 0.0)))
	scene.replay_bar.play_toggled.connect(toggle_play)
	scene.replay_bar.speed_chosen.connect(set_speed)
	scene.replay_bar.seek_requested.connect(seek)
	scene.replay_bar.quit_pressed.connect(scene._on_return)
	hud.add_events([{"time": float(scene.battle.call("get_elapsed")), "text_fr": "Rejeu de la bataille : on regarde, on ne commande pas."}])
	var divergence: Dictionary = info.get("divergence", {})
	if not divergence.is_empty():
		hud.add_events([{"time": 0.0, "text_fr": str(divergence.get("message", ""))}])
	update()


## État de la barre de rejeu ; pause d'elle-même à la fin de l'enregistrement.
func update() -> void:
	var scene := _scene
	if scene.replay_bar == null or scene.battle == null:
		return
	var info: Dictionary = scene.battle.call("get_replay")
	if bool(info.get("at_end", false)) and not scene.paused:
		scene.paused = true
	scene.replay_bar.show_state(float(scene.battle.call("get_elapsed")), scene.paused, scene.speed, info.get("divergence", {}))


## Lecture / pause ; « Lecture » à la fin repart du début.
func toggle_play() -> void:
	var scene := _scene
	if scene.paused and bool((scene.battle.call("get_replay") as Dictionary).get("at_end", false)):
		seek(0.0)
	scene.paused = not scene.paused
	update()


func set_speed(value: float) -> void:
	_scene.speed = value
	_scene.paused = false
	update()


## Saut dans la barre de temps (le cœur repart de l'instantané le plus proche). Un saut en
## arrière reconstruit les figurines et effets (sang, traits, corps) pour ne pas montrer l'avenir.
func seek(seconds: float) -> void:
	var scene := _scene
	if scene.battle == null or not scene.replay_mode:
		return
	var before := float(scene.battle.call("get_elapsed"))
	scene.battle.call("replay_seek", seconds)
	var after := float(scene.battle.call("get_elapsed"))
	if after < before - 0.05:
		reset_visuals()
	scene._refresh_view(true)
	scene.hud.add_events([{"time": after, "text_fr": "Rejeu : saut à %s." % BattleReplayBar.clock(after)}])
	update()


## Figurines, étendards, effets, sang et herbe couchée refaits à neuf (après un saut en arrière
## ou au début d'un rejeu lancé depuis l'écran de fin).
func reset_visuals() -> void:
	var scene := _scene
	scene.units = scene.battle.call("get_units")
	# Les imposteurs survivent au saut : atlas déjà cuits gardés, et une cuisson en cours (coroutine
	# sur ce nœud) ne reprend jamais sur une instance libérée.
	var kept_impostors: BattleImpostors = null
	var soldiers := scene.soldiers
	if soldiers != null and is_instance_valid(soldiers) and soldiers.impostors != null and is_instance_valid(soldiers.impostors):
		kept_impostors = soldiers.impostors
		soldiers.remove_child(kept_impostors)
		soldiers.impostors = null
	for node in [scene.soldiers, scene.standards, scene.effects, scene.engines_fx, scene.assault_fx]:
		if node != null and is_instance_valid(node):
			(node as Node).get_parent().remove_child(node)
			(node as Node).queue_free()
	scene.standards = null
	scene.effects = null
	scene.blood = null
	scene.engines_fx = null
	scene.assault_fx = null
	scene.grass_flatten = null
	scene._build_soldier_layers(kept_impostors)


## « Revoir la bataille » depuis l'écran de fin (résultat déjà appliqué à la campagne).
func start_in_place() -> bool:
	var scene := _scene
	if scene.battle == null:
		return false
	var result: Dictionary = scene.battle.call("start_replay")
	if not bool(result.get("ok", false)):
		push_warning("BattleScene: start_replay: %s" % result.get("error", "?"))
		return false
	if scene.result_screen != null:
		scene.result_screen.queue_free()
		scene.result_screen = null
	reset_visuals()
	enter()
	scene._refresh_view(true)
	return true


## Enregistre la bataille (dossier utilisateur, N derniers gardés par le cœur). Pas pendant un
## rejeu, ni en banc d'essai ou capture ; en mode sans affichage (tests) seulement si un dossier
## de test est imposé (`ReplaysMenu.dir_override`).
func save() -> void:
	var scene := _scene
	if scene.replay_mode or scene.capture.screenshot_path != "" or scene.battle == null:
		return
	if DisplayServer.get_name() == "headless" and ReplaysMenu.dir_override == "":
		return
	scene.replay_saved_path = str(scene.battle.call("save_replay", ReplaysMenu.replays_dir(), scene._title_text))
