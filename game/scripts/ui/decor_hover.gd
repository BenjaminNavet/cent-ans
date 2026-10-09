class_name DecorHover
extends Node

## NA (ADR 0219) : bulle codex différée au survol du décor naturel (arbres, troupeaux, rochers).
## Surveille la souris : immobile (≤ `still_px`) pendant `delay` secondes, hors interface et
## sans objet prioritaire (`blocker`), elle interroge `provider` (point écran → {kind, species}),
## traduit par `CodexStore.entry_for_decor` et ouvre une bulle codex ordinaire près du curseur
## (`CodexBubbles.open`). Un seul essai par immobilité : rien ne se répète avant que la souris
## ait bougé. La bulle se referme quand la souris s'éloigne (> `leave_px`) sauf si elle la survole
## ou si le joueur l'a épinglée (T, clic droit). Coût nul hors attente : `_process` n'est actif
## que pendant le décompte. Délai : réglage `interface/decor_hover_delay` (-1 = défaut des
## données, 0 = désactivé), défaut et choix dans `data/ui/tooltip_style.json` (bloc `decor_hover`).
## Présentation pure : aucune règle de jeu.

signal bubble_shown(entry_id: String)

const SETTING_KEY := "interface/decor_hover_delay"
const DEFAULT_DELAY_S := 1.5  # sans autoload Settings (tests) ; le défaut joueur vit dans Settings.DEFAULTS
const STYLE_FILE := "ui/tooltip_style.json"
const FALLBACK := {"decor_hover": {"still_px": 3.0, "leave_px": 12.0, "choices": [0.75, 1.5, 3.0, 0.0]}}

enum State { IDLE, WAITING, FIRED, SHOWN }

static var _style := JsonLookup.new(STYLE_FILE, FALLBACK)

## Callable(screen_position: Vector2) -> Dictionary {kind, species} ({} : rien sous le curseur).
var provider: Callable
## Callable(screen_position: Vector2) -> bool : vrai si un objet prioritaire (armée, ville,
## unité) est sous le curseur ; facultatif.
var blocker: Callable
## Tests : délai imposé (≥ 0 ; -1 : réglage du joueur).
var delay_override := -1.0
## Tests : ignore le survol d'un contrôle d'interface.
var ignore_gui := false
var state: State = State.IDLE
var shown_id := ""

var _anchor := Vector2(-1000.0, -1000.0)
var _elapsed := 0.0
var _bubble: PanelContainer = null


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	set_process(false)


## Paramètre `key` du bloc `decor_hover` des données.
static func style_value(key: String) -> Variant:
	return _style.section("decor_hover").get(key, (FALLBACK["decor_hover"] as Dictionary).get(key))


## Délai effectif (s) : 0 = désactivé.
func delay() -> float:
	if delay_override >= 0.0:
		return delay_override
	var settings := get_node_or_null("/root/Settings")
	return float(settings.call("get_value", SETTING_KEY)) if settings != null else DEFAULT_DELAY_S


func _input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		note_mouse((event as InputEventMouseMotion).position)
	elif event is InputEventMouseButton and (event as InputEventMouseButton).pressed:
		_interrupted((event as InputEventMouseButton).position)
	elif event is InputEventKey and (event as InputEventKey).pressed and not (event as InputEventKey).echo:
		_interrupted(_anchor)


func _process(delta: float) -> void:
	tick(delta)


## La souris est en `position` (écran). Relance le décompte si elle a bougé de plus de `still_px` ;
## ferme la bulle si elle s'est éloignée.
func note_mouse(position: Vector2) -> void:
	if state == State.SHOWN:
		if not _bubble_alive() or _bubble.get_meta("pinned", false):
			_begin_wait(position)  # fermée, ou épinglée par le joueur : elle ne nous appartient plus
		elif position.distance_to(_anchor) > float(style_value("leave_px")) and not _near_bubble(position):
			_close()
			_begin_wait(position)
		return
	if position.distance_to(_anchor) > float(style_value("still_px")):
		_begin_wait(position)


## Avance le décompte de `delta` secondes ; ouvre la bulle une fois le délai écoulé.
func tick(delta: float) -> void:
	if state != State.WAITING:
		return
	_elapsed += delta
	var wait := delay()
	if wait <= 0.0:
		set_process(false)
		return
	if _elapsed >= wait:
		_fire()


func is_shown() -> bool:
	return state == State.SHOWN and _bubble_alive()


func _interrupted(position: Vector2) -> void:
	if state == State.SHOWN and _bubble_alive() and not _near_bubble(position):
		_close()
	if state != State.SHOWN:
		_begin_wait(position)


func _begin_wait(position: Vector2) -> void:
	_anchor = position
	_elapsed = 0.0
	state = State.WAITING
	shown_id = ""
	_bubble = null
	set_process(delay() > 0.0)


func _fire() -> void:
	state = State.FIRED
	set_process(false)
	if not provider.is_valid():
		return
	if not ignore_gui and is_inside_tree() and get_viewport().gui_get_hovered_control() != null:
		return
	if blocker.is_valid() and bool(blocker.call(_anchor)):
		return
	var hit: Dictionary = provider.call(_anchor)
	if hit.is_empty():
		return
	var codex := CodexText.store()
	if codex == null:
		return
	var id := str(codex.call("entry_for_decor", str(hit.get("kind", "")), str(hit.get("species", ""))))
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if id == "" or bubbles == null:
		return
	var bubble: PanelContainer = bubbles.call("open", id, _anchor)
	if bubble == null:
		return
	bubble.set_meta("decor_hold", true)  # CodexBubbles ne la referme pas à la grâce : c'est ici
	_bubble = bubble
	shown_id = id
	state = State.SHOWN
	bubble_shown.emit(id)


func _bubble_alive() -> bool:
	return _bubble != null and is_instance_valid(_bubble) and _bubble.is_inside_tree()


## Souris sur la bulle, ou entre le point d'ancrage et elle (marge `leave_px`).
func _near_bubble(position: Vector2) -> bool:
	if not _bubble_alive():
		return false
	return _bubble.get_global_rect().grow(float(style_value("leave_px"))).has_point(position)


func _close() -> void:
	if _bubble_alive():
		var bubbles := get_node_or_null("/root/CodexBubbles")
		if bubbles != null and not _bubble.get_meta("pinned", false):
			bubbles.call("close_unpinned")
	_bubble = null
	shown_id = ""
