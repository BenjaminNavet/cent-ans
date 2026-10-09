class_name RedrawOnDemand
extends RefCounted

## Drapeau « à redessiner » réutilisable (SC PF-09/MB7) : une couche 2D coûteuse ne se redessine
## que si elle a été marquée sale (`mark_dirty`) ou si la signature de sa vue a changé
## (`needs_redraw`). Aucun redessin périodique : plus d'animation de fond.

var _dirty := true
var _signature: Variant = null
## Nombre de redessins demandés (mesure).
var count := 0


func mark_dirty() -> void:
	_dirty = true


## Vrai (et consomme le drapeau) si `signature` diffère de la précédente ou si la couche est sale.
func needs_redraw(signature: Variant) -> bool:
	if not _dirty and signature == _signature:
		return false
	_dirty = false
	_signature = signature
	count += 1
	return true
