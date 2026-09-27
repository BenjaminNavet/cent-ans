class_name BattleInput
extends Node

## CB0 : entrées de bataille (clics, glisser, touches, groupes, sélection rapide), extraites de
## `battle_scene.gd` (même comportement, seul l'emplacement change). Nœud enfant de
## `BattleScene` : lit l'état partagé sur `scene` (sélection, unités, caméra, HUD, pont) et
## notifie ses décisions par signal ; la scène connecte ces signaux à `issue()`, au rendu et au
## pont. `game/tests/smoke.gd` continue d'appeler `scene.issue` et `scene.handle_group_key`
## (délégations fines gardées sur la scène).
##
## Squelette (CB0 étape 1) : API et signaux posés, implémentation à l'étape suivante
## (`refactor: extract battle input`).

signal command_requested(command: Dictionary)
signal selection_changed(ids: Array)
signal camera_focus_requested(point: Vector3)
signal pause_toggled
signal speed_step(delta: int)
signal help_toggled
signal markers_toggled
signal screenshot_requested

const DOUBLE_CLICK_MS := 350

## La scène (`BattleScene`), assignée par elle avant `add_child`.
var scene: Node = null
