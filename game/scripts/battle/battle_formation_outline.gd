class_name BattleFormationOutline
extends Node3D

## CB-M1 : contour de formation projeté sur le relief, un `Decal` par régiment (remplace l'anneau
## jaune de sélection). États (spec CB-M, « Contour de formation ») :
## - Sélectionnée : trait plein, couleur du camp ;
## - Survolée : trait pâle ;
## - Ennemie survolée : rouge, trait pointillé ;
## - Ennemie ciblée (cible `target` d'une unité sélectionnée) : rouge pulsé ;
## - En déroute, absente (détruite, sortie du champ, en réserve) : rien.

enum State { NONE, SELECTED, HOVERED, ENEMY_HOVERED, ENEMY_TARGETED }


## État du contour d'un régiment. `unit` : dictionnaire de `get_units()` ; `selected`, `hovered`,
## `targeted` : drapeaux calculés par l'appelant ; `player_side` : camp du joueur (« ennemie » =
## tout autre camp).
static func outline_state(unit: Dictionary, selected: bool, hovered: bool, targeted: bool, player_side: String) -> int:
	return State.NONE


func setup(side_colors: Dictionary, player_side: String) -> void:
	pass


## Met les décales à jour à partir de `get_units()`, de la sélection et des régiments survolés.
func update(units: Array, selected: Array, hovered: Array) -> void:
	pass
