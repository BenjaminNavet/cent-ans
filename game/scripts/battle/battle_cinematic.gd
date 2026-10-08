class_name BattleCinematic
extends Node

## EP8 : plan cinématique facultatif au premier contact. Quand deux lignes se heurtent pour la
## première fois (régiments au corps à corps, `min_soldiers_in_contact` soldats au moins), la
## caméra quitte le joueur quelques secondes (`duration_s`) : elle tourne lentement autour du
## point de choc, à hauteur d'homme, bandes noires en haut et en bas, ralenti éventuel
## (`slowmo_scale`, réglage « Ralenti du plan »). Échap, Espace ou un clic passent le plan ; la
## caméra rend ensuite la main exactement où le joueur l'avait laissée. Rien n'est imposé :
## réglage « Plan cinématique au premier choc » (`battle/cinematic`), jamais en banc d'essai,
## en capture ou quand l'IA joue les deux camps.
## Paramètres : `data/fx/battle_staging.json` (`cinematic`).

signal started(focus: Vector3)
signal finished

var cfg: Dictionary = {}
var enabled: bool = true
var slowmo: bool = true
var active: bool = false
var done: bool = false
## Facteur de temps de la bataille pendant le plan (1 hors ralenti) : lu par `BattleScene`.
var time_scale: float = 1.0
var focus: Vector3 = Vector3.ZERO
var yaw: float = 0.0

var _scene: Node = null
var _rig: Node3D = null
var _camera: Camera3D = null
var _t: float = 0.0
var _saved_fov: float = 70.0
var _saved_transform: Transform3D
var _bars: Array[ColorRect] = []
var _hint: Label = null
var _hidden: Array = []  # calques d'interface masqués pendant le plan


func setup(p_cfg: Dictionary, scene: Node, rig: Node3D, camera: Camera3D) -> void:
	cfg = p_cfg
	name = "Cinematic"
	_scene = scene
	_rig = rig
	_camera = camera
	process_mode = Node.PROCESS_MODE_ALWAYS


## Au plus une fois par bataille : lance le plan si un vrai choc a lieu (`units` : `get_units`).
## `shot` : `BattleScene._closeup_shot(units)` (point de choc et lacet de profil).
func check(units: Array, shot_of: Callable) -> void:
	if done or active or not enabled:
		return
	var contact := 0
	for unit in units:
		if bool(unit["present"]) and str(unit["state"]) == "melee":
			contact += int(unit["soldiers"])
	if contact < int(cfg.get("min_soldiers_in_contact", 60)):
		return
	var shot: Dictionary = shot_of.call(units)
	start(shot.get("focus", Vector3.ZERO), float(shot.get("yaw", 0.0)))


func start(p_focus: Vector3, p_yaw: float) -> void:
	if _camera == null or _rig == null:
		return
	done = true
	active = true
	_t = 0.0
	focus = p_focus
	yaw = p_yaw
	if _rig.get("height_at") is Callable:
		focus.y = float((_rig.get("height_at") as Callable).call(focus.x, focus.z))
	_saved_fov = _camera.fov
	_saved_transform = _camera.global_transform
	_rig.set_process(false)
	_rig.set_process_unhandled_input(false)
	time_scale = float(cfg.get("slowmo_scale", 0.35)) if slowmo else 1.0
	_show_bars(true)
	_apply(0.0)
	started.emit(focus)


## Passe le plan (touche, clic, fin du temps).
func skip() -> void:
	if not active:
		return
	active = false
	time_scale = 1.0
	_camera.fov = _saved_fov
	_camera.global_transform = _saved_transform
	_rig.set_process(true)
	_rig.set_process_unhandled_input(true)
	_show_bars(false)
	finished.emit()


func _process(delta: float) -> void:
	if not active:
		return
	_t += delta
	if _t >= float(cfg.get("duration_s", 6.0)):
		skip()
		return
	_apply(_t)


func _unhandled_input(event: InputEvent) -> void:
	if not active:
		return
	var skip_key: bool = event is InputEventKey and event.pressed and not event.echo and ((event as InputEventKey).keycode in [KEY_ESCAPE, KEY_SPACE, KEY_ENTER])
	var click: bool = event is InputEventMouseButton and event.pressed
	if skip_key or click:
		skip()
		get_viewport().set_input_as_handled()


## Fige le plan à l'instant `t` (captures).
func pose_at(t: float) -> void:
	if active:
		_t = t
		_apply(t)


## Orbite lente autour du choc, à hauteur d'homme, avec une entrée en fondu de la focale.
func _apply(t: float) -> void:
	var duration := float(cfg.get("duration_s", 6.0))
	var k := clampf(t / duration, 0.0, 1.0)
	var orbit := deg_to_rad(float(cfg.get("orbit_deg", 50.0)))
	var angle := yaw - orbit * 0.5 + orbit * k
	var dist := float(cfg.get("distance_m", 30.0)) * lerpf(1.15, 0.9, k)
	var eye := focus + Vector3(sin(angle), 0.0, cos(angle)) * dist
	var ground := focus.y
	if _rig.get("height_at") is Callable:
		ground = float((_rig.get("height_at") as Callable).call(eye.x, eye.z))
	eye.y = maxf(ground, focus.y) + float(cfg.get("height_m", 5.5))
	var blend := clampf(t / maxf(float(cfg.get("blend_in_s", 0.6)), 0.01), 0.0, 1.0)
	var aim := focus + Vector3(0, 1.8, 0)
	var target := Transform3D(Basis(), eye).looking_at(aim, Vector3.UP)
	_camera.global_transform = _saved_transform.interpolate_with(target, blend * blend * (3.0 - 2.0 * blend))
	_camera.fov = lerpf(_saved_fov, float(cfg.get("fov_deg", 52.0)), blend)


func _show_bars(visible: bool) -> void:
	if _bars.is_empty() and visible:
		var layer := CanvasLayer.new()
		layer.name = "CinematicBars"
		layer.layer = 20
		add_child(layer)
		for top in [true, false]:
			var bar := ColorRect.new()
			bar.color = Color(0, 0, 0, 1)
			bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
			bar.anchor_left = 0.0
			bar.anchor_right = 1.0
			bar.anchor_top = 0.0 if top else 0.9
			bar.anchor_bottom = 0.1 if top else 1.0
			layer.add_child(bar)
			_bars.append(bar)
		_hint = Label.new()
		_hint.text = "Espace ou Échap : passer le plan"
		_hint.modulate = Color(1, 1, 1, 0.75)
		_hint.anchor_left = 0.75
		_hint.anchor_right = 0.98
		_hint.anchor_top = 0.92
		_hint.anchor_bottom = 0.98
		_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		layer.add_child(_hint)
	for bar in _bars:
		bar.visible = visible
	if _hint != null:
		_hint.visible = visible
	# Interface de bataille (HUD, barre des ordres…) masquée pendant le plan, rendue ensuite.
	if _scene == null:
		return
	if visible:
		_hidden.clear()
		for layer in _scene.find_children("*", "CanvasLayer", true, false):
			if (layer as CanvasLayer).visible and not is_ancestor_of(layer):
				(layer as CanvasLayer).visible = false
				# Figé le temps du plan : certaines barres se réaffichent d'elles-mêmes.
				layer.set_meta("cinematic_mode", layer.process_mode)
				layer.process_mode = Node.PROCESS_MODE_DISABLED
				_hidden.append(layer)
	else:
		for layer in _hidden:
			if is_instance_valid(layer):
				(layer as CanvasLayer).visible = true
				layer.process_mode = layer.get_meta("cinematic_mode", Node.PROCESS_MODE_INHERIT)
		_hidden.clear()
