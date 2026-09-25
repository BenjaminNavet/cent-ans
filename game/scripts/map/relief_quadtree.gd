class_name ReliefQuadtree
extends Node3D

## Quadtree de relief streamé au-dessus des morceaux E0 de `TerrainBuilder` (chantier ZG, lot ZG2,
## ADR 0036) : sélection des nœuds par erreur à l'écran, patchs fixes déplacés dans le vertex
## shader depuis des pages de hauteurs (`Texture2DArray`), décodage hors fil principal, hauteurs
## gardées côté processeur pour `surface_height_at`. Remplace à terme le relief fin de
## `FineTerrainJob`.

signal surface_changed(rect: Rect2)

var pyramid: ReliefPyramid


func setup(_pyramid: ReliefPyramid, _terrain: Node) -> void:
	pyramid = _pyramid  # lot ZG2


func update_view(_camera: Camera3D) -> void:
	pass  # lot ZG2


## Hauteur monde de la surface affichée la plus fine chargée, NAN si aucune tuile de la pyramide.
func surface_height_at(_x: float, _y: float) -> float:
	return NAN  # lot ZG2
