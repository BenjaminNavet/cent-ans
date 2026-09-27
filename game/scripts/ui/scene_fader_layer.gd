class_name SceneFaderLayer
extends Node

## Chantier PO5 (ADR 0097, bible DA § 12.4) : nœud des transitions entre scènes, créé à la
## demande sous la racine (`/root/SceneFaderLayer`, survit aux changements de scène) par la
## façade statique `SceneFader` (`SceneFader.go(path)`), comme `UiSounds`. Pas un autoload :
## un identifiant d'autoload n'est pas déclaré au compilateur en mode `--script` (smoke.gd).
## - `go(path)` : fondu au noir parchemin (0,25 s), changement de scène, fondu d'entrée ;
## - `cover()` / `reveal()` : passages internes sans changement de scène (retour de bataille
##   vers la carte, la bataille étant ajoutée à la racine par la carte).
## Instantané en headless (tests, CI) : pas de `Tween`, le changement de scène est immédiat.
## « Réduire les animations » (`access/reduce_motion`) ramène chaque fondu à `REDUCED_SECONDS`.

## Noir parchemin : encre brune presque noire, jamais un noir pur.
const COVER_COLOR := Color(0.075, 0.056, 0.038, 1.0)
const FADE_SECONDS := 0.25
const REDUCED_SECONDS := 0.06
## Au-dessus de tout (écran de chargement AR1 : 110, infobulles, dialogues).
const LAYER := 128

## Vrai pendant un `go` (les appels suivants sont ignorés : double clic sur un bouton).
var busy: bool = false
## Dernière scène demandée à `go` (tests).
var last_path: String = ""

var _layer: CanvasLayer
var _rect: ColorRect
var _tween: Tween


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	_layer = CanvasLayer.new()
	_layer.name = "SceneFaderLayer"
	_layer.layer = LAYER
	add_child(_layer)
	_rect = ColorRect.new()
	_rect.name = "Cover"
	_rect.color = COVER_COLOR
	_rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_rect.modulate.a = 0.0
	_rect.visible = false
	_layer.add_child(_rect)


## Fondu au noir, `change_scene_to_file(path)`, puis fondu d'entrée sur la nouvelle scène.
## Rend l'erreur du changement de scène (`OK` si accepté).
func go(path: String) -> Error:
	if busy:
		return ERR_BUSY
	last_path = path
	var tree := get_tree()
	if instant():
		return tree.change_scene_to_file(path)
	busy = true
	await cover()
	var error := tree.change_scene_to_file(path)
	if error == OK:
		await tree.scene_changed
		# Une image de la nouvelle scène sous le voile (construction, premier rendu).
		await tree.process_frame
	await reveal()
	busy = false
	return error


## Voile la vue (fondu au noir parchemin) et bloque la souris ; rend la main une fois opaque.
func cover(seconds: float = FADE_SECONDS) -> void:
	if _rect == null:
		return
	_rect.visible = true
	_rect.mouse_filter = Control.MOUSE_FILTER_STOP
	if instant():
		_rect.modulate.a = 1.0
		return
	await _fade_to(1.0, seconds)


## Lève le voile ; rend la main une fois transparent.
func reveal(seconds: float = FADE_SECONDS) -> void:
	if _rect == null:
		return
	_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if instant():
		_rect.modulate.a = 0.0
		_rect.visible = false
		return
	await _fade_to(0.0, seconds)
	if _rect.modulate.a <= 0.0:
		_rect.visible = false


## Vrai si la vue est voilée (même partiellement).
func is_covered() -> bool:
	return _rect != null and _rect.visible and _rect.modulate.a > 0.0


## Vrai en headless : aucun fondu, tout s'applique tout de suite.
func instant() -> bool:
	return DisplayServer.get_name() == "headless"


func _fade_to(alpha: float, seconds: float) -> void:
	if _tween != null and _tween.is_valid():
		_tween.kill()
	var duration := REDUCED_SECONDS if Accessibility.reduce_motion() else seconds
	duration *= absf(alpha - _rect.modulate.a)  # repart d'un fondu interrompu sans à-coup
	if duration <= 0.0:
		_rect.modulate.a = alpha
		return
	var tween := create_tween()
	_tween = tween
	tween.set_pause_mode(Tween.TWEEN_PAUSE_PROCESS)
	tween.tween_property(_rect, "modulate:a", alpha, duration).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
	# Pas `await tween.finished` : un fondu interrompu (`kill`) ne l'émettrait jamais.
	while tween.is_valid() and tween.is_running():
		await get_tree().process_frame
