class_name TurnWaitIndicator
extends PanelContainer

## Lot PB3d : cartouche enluminé affiché pendant que le cœur résout la fin de tour dans son fil
## (« Les cours d'Europe délibèrent… », sablier qui se retourne). Discret : n'apparaît qu'après
## un court délai (pas de clignotement pour une fin de tour rapide), en fondu, en haut de l'écran
## sous la barre supérieure ; il n'intercepte pas la souris (caméra et survol continuent).

const TEXT := "Les cours d'Europe délibèrent…"
## Délai avant l'apparition (s) et durée du fondu (s).
const SHOW_DELAY := 0.15
const FADE := 0.25
## Distance au bord haut de l'écran (px), sous la barre supérieure.
const TOP_OFFSET := 64.0

var _elapsed := 0.0
var _active := false
var _glass: _Hourglass
var _label: Label


func _init() -> void:
	name = "TurnWaitIndicator"
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_theme_stylebox_override("panel", HudStyle.illuminated_box(8))
	var row := HBoxContainer.new()
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_theme_constant_override("separation", 8)
	add_child(row)
	_glass = _Hourglass.new()
	row.add_child(_glass)
	_label = HudStyle.label(TEXT, HudStyle.FONT_BODY + 1, HudStyle.RUBRIC)
	_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(_label)
	modulate.a = 0.0
	visible = false
	set_process(false)


## Début de l'attente (l'indicateur apparaît après `SHOW_DELAY`).
func begin() -> void:
	_elapsed = 0.0
	_active = true
	modulate.a = 0.0
	visible = false
	set_process(true)


## Fin de l'attente : masqué aussitôt.
func end() -> void:
	_active = false
	visible = false
	set_process(false)


func is_shown() -> bool:
	return visible and modulate.a > 0.0


func _process(delta: float) -> void:
	if not _active:
		return
	_elapsed += delta
	if _elapsed < SHOW_DELAY:
		return
	if not visible:
		visible = true
		_place()
	modulate.a = clampf((_elapsed - SHOW_DELAY) / FADE, 0.0, 1.0)
	_glass.phase = _elapsed


func _place() -> void:
	reset_size()
	var view := get_viewport_rect().size
	position = Vector2(round((view.x - size.x) * 0.5), TOP_OFFSET)


## Sablier à l'encre : le sable s'écoule puis le verre se retourne (cycle de 2 s).
class _Hourglass:
	extends Control

	const CYCLE := 2.0
	var phase := 0.0:
		set(value):
			phase = value
			queue_redraw()

	func _init() -> void:
		custom_minimum_size = Vector2(16, 20)
		mouse_filter = Control.MOUSE_FILTER_IGNORE

	func _draw() -> void:
		var t := fmod(phase, CYCLE) / CYCLE
		var flow := clampf(t / 0.8, 0.0, 1.0)  # sable écoulé
		var turn := clampf((t - 0.8) / 0.2, 0.0, 1.0)  # retournement
		var c := size * 0.5
		draw_set_transform(c, turn * PI, Vector2.ONE)
		var w := 6.0
		var h := 8.0
		var ink := HudStyle.INK
		var sand := Color(0.78, 0.58, 0.25)
		# Sable : haut qui se vide, bas qui se remplit.
		var top := h * (1.0 - flow)
		if top > 0.3:
			var k := top / h
			draw_colored_polygon(PackedVector2Array([Vector2(-w * k, -top), Vector2(w * k, -top), Vector2(0, 0)]), sand)
		var bottom := h * flow
		if bottom > 0.3:
			var k2 := 1.0 - bottom / h
			draw_colored_polygon(PackedVector2Array([Vector2(-w, h), Vector2(w, h), Vector2(w * k2, h - bottom), Vector2(-w * k2, h - bottom)]), sand)
		# Verre et montures.
		draw_polyline(PackedVector2Array([Vector2(-w, -h), Vector2(w, -h), Vector2(0, 0), Vector2(w, h), Vector2(-w, h), Vector2(0, 0), Vector2(-w, -h)]), ink, 1.2, true)
		draw_line(Vector2(-w - 1.5, -h - 1.0), Vector2(w + 1.5, -h - 1.0), ink, 2.0)
		draw_line(Vector2(-w - 1.5, h + 1.0), Vector2(w + 1.5, h + 1.0), ink, 2.0)
		draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
