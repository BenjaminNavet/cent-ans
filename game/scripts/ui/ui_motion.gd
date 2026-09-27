class_name UiMotion
extends RefCounted

## Chantier PO2 (ADR 0097, bible DA § 12.4) : ouverture et fermeture des panneaux — fondu de
## 0,12 s avec un léger glissement de 8 px (par `Tween` sur `modulate:a` et `position`), son
## `ui_open` à l'ouverture. « Réduire les animations » (`Settings` : `access/reduce_motion`)
## supprime le glissement, garde le fondu. Rien en headless (ni tests, ni CI) : l'état final est
## appliqué tout de suite, sans `Tween` ni son.

const DURATION := 0.12
const SLIDE_PX := 8.0


## Fondu et glissement d'entrée de `control` (déjà dans l'arbre, visible). Joue `ui_open`.
static func fade_in(control: CanvasItem, duration: float = DURATION) -> void:
	if control == null:
		return
	if _instant():
		control.modulate.a = 1.0
		return
	var slide := _slide_offset()
	var start_pos: Vector2 = control.position + slide
	control.modulate.a = 0.0
	control.position = start_pos
	var tween := control.create_tween()
	tween.set_parallel(true)
	tween.tween_property(control, "modulate:a", 1.0, duration)
	tween.tween_property(control, "position", start_pos - slide, duration)
	UiSounds.play("ui_open")


## Fondu et glissement de sortie de `control`. `free` : libère le nœud une fois invisible
## (sinon seulement `hide()`, pour un panneau réutilisé).
static func fade_out(control: CanvasItem, duration: float = DURATION, free: bool = false) -> void:
	if control == null:
		return
	if _instant():
		control.modulate.a = 0.0
		control.visible = false
		if free and is_instance_valid(control):
			control.queue_free()
		return
	var slide := _slide_offset()
	var end_pos: Vector2 = control.position + slide
	var tween := control.create_tween()
	tween.set_parallel(true)
	tween.tween_property(control, "modulate:a", 0.0, duration)
	tween.tween_property(control, "position", end_pos, duration)
	tween.chain().tween_callback(func() -> void:
		if not is_instance_valid(control):
			return
		control.visible = false
		if free:
			control.queue_free())


## Vrai en headless (tests, CI) : pas de `Tween`, l'état final s'applique tout de suite.
static func _instant() -> bool:
	return DisplayServer.get_name() == "headless"


## Glissement de 8 px (bas → haut à l'ouverture) ; nul si « Réduire les animations ».
static func _slide_offset() -> Vector2:
	var tree := Engine.get_main_loop() as SceneTree
	var settings := tree.root.get_node_or_null("Settings") if tree != null else null
	if settings != null and bool(settings.call("get_value", "access/reduce_motion")):
		return Vector2.ZERO
	return Vector2(0.0, SLIDE_PX)
