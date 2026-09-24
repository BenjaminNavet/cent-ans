class_name BattleMinimap
extends Control

## Minicarte de bataille (F5b) : champ, régiments en points aux couleurs des camps, cadre de la
## caméra ; clic (ou glisser) = déplacer la caméra. Pure présentation.

signal clicked(world: Vector2)

var field_size: Vector2 = Vector2(1200, 800)
