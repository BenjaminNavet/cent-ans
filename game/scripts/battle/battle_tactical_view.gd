class_name BattleTacticalView
extends Node

## CB3 : vue tactique (Tab, spec § « Vue tactique », plan § CB3). Caméra du dessus cadrée sur
## tout le champ (inclinaison quasi verticale, `BattleCamera.manual_pitch`), terrain assombri par
## un calque semi-transparent posé dans le HUD — pas de `CanvasModulate` (qui ne module que les
## `CanvasItem` 2D, pas la scène 3D du champ) ni de changement de shader de terrain (un autre
## chantier, PO, y touche). Les bannières sont forcées en pastilles B7 (regroupées) et les
## ennemis non `spotted` (cœur, champ dérivé CB3) sont masqués ; les contours CB-M restent
## actifs. La simulation continue (Espace la met en pause comme d'habitude) ; ordres, glisser,
## trajets et curseur restent actifs. Tab ou Échap restaure la caméra précédente. En déploiement,
## les contraintes de zone restent appliquées : elles vivent dans `DeploymentController`,
## indépendant de la caméra, donc rien à faire ici.

## Assombrissement du calque (alpha) : assez marqué pour lire l'ensemble sans noyer les repères.
const OVERLAY_COLOR := Color(0.03, 0.03, 0.05, 0.55)
## Marge (m) ajoutée à la plus grande dimension du champ pour tout voir depuis le dessus.
const FRAME_MARGIN_M := 120.0
## Inclinaison (degrés) : la borne haute de `BattleCamera` (quasi vertical sans l'être tout à
## fait, pour garder une caméra en perspective valide).
const PITCH_DEG := 85.0

## La scène, assignée par elle avant `add_child` (comme `BattleInput`).
var scene: BattleScene = null
var active: bool = false

var _saved_target: Vector3 = Vector3.ZERO
var _saved_distance: float = 0.0
var _saved_yaw: float = 0.0
var _saved_manual_pitch: bool = false
var _saved_manual_pitch_deg: float = 0.0
var _overlay: ColorRect = null


func _ready() -> void:
	name = "BattleTacticalView"


## Bascule (touche Tab) : entre en vue tactique si elle est fermée, en sort sinon.
func toggle() -> void:
	if active:
		exit()
	else:
		enter()


## Mémorise la caméra courante, puis cadre tout le champ du dessus, assombri.
func enter() -> void:
	if active or scene == null or scene.camera_rig == null:
		return
	active = true
	var cam := scene.camera_rig
	_saved_target = cam.target
	_saved_distance = cam.distance
	_saved_yaw = cam.yaw
	_saved_manual_pitch = cam.manual_pitch
	_saved_manual_pitch_deg = cam.manual_pitch_deg
	var center := scene.terrain.field_center() if scene.terrain != null else Vector2(cam.target.x, cam.target.z)
	var extent: float = FRAME_MARGIN_M
	if scene.terrain != null:
		extent = maxf(scene.terrain.FIELD_W, scene.terrain.FIELD_D) + FRAME_MARGIN_M
	var span := clampf(extent, cam.min_distance, cam.max_distance)
	cam.look_at_point(Vector3(center.x, 0.0, center.y), span, 0.0)
	cam.manual_pitch = true
	cam.manual_pitch_deg = PITCH_DEG
	_show_overlay()


## Restaure la caméra mémorisée par `enter` (touche Tab ou Échap).
func exit() -> void:
	if not active:
		return
	active = false
	var cam := scene.camera_rig
	cam.look_at_point(_saved_target, _saved_distance, _saved_yaw)
	cam.manual_pitch = _saved_manual_pitch
	cam.manual_pitch_deg = _saved_manual_pitch_deg
	_hide_overlay()


func _show_overlay() -> void:
	if scene == null or scene.hud == null or scene.hud.root == null:
		return
	_overlay = ColorRect.new()
	_overlay.name = "TacticalOverlay"
	_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_overlay.color = OVERLAY_COLOR
	_overlay.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	scene.hud.root.add_child(_overlay)
	# Sous tout le reste du HUD (cartes, journal, minicarte…) : seul le champ 3D est assombri.
	scene.hud.root.move_child(_overlay, 0)


## Un `Node` retiré de l'arbre n'est pas libéré tout seul : `queue_free` (pas seulement
## `remove_child`), et un nœud neuf est recréé à la prochaine entrée.
func _hide_overlay() -> void:
	if _overlay != null and is_instance_valid(_overlay):
		_overlay.queue_free()
	_overlay = null


## Régiments à montrer en vue tactique (repères) : les siens toujours, les ennemis seulement
## s'ils sont `spotted` (cœur, champ dérivé CB3). Un ennemi sans le champ (rétrocompatibilité —
## rejeu enregistré avant CB3, ou pont pas encore à jour) reste affiché. Fonction pure, testée
## hors scène.
static func filter_spotted(units: Array, player_side: String) -> Array:
	var out: Array = []
	for unit in units:
		if str(unit.get("side", "")) == player_side or bool(unit.get("spotted", true)):
			out.append(unit)
	return out
