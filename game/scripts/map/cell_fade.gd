class_name CellFade
extends RefCounted

## Lot ZF-B : rampe d'apparition/disparition par cellule de décor (une valeur par MultiMeshInstance3D,
## jamais par instance). Le shader lit `instance uniform float cell_grow` (0-1) et fait « pousser »
## les objets (même mécanique que `vis`) : les matériaux restent opaques.


## Valeur de départ d'un nœud neuf (invisible jusqu'à la première rampe).
static func init_node(node: MultiMeshInstance3D) -> void:
	node.set_instance_shader_parameter("cell_grow", 0.0)


## Avance la rampe d'une cellule (`entry["grow"]`) vers 1 si `shown`, sinon vers 0. `seconds` <= 0 :
## saut immédiat. Met à jour les nœuds seulement si la valeur change. Renvoie vrai si la cellule doit
## rester affichée (voulue, ou encore en train de se replier).
static func step(entry: Dictionary, shown: bool, delta: float, seconds: float) -> bool:
	var grow := float(entry.get("grow", 0.0))
	var target := 1.0 if shown else 0.0
	var next := target if seconds <= 0.0 else move_toward(grow, target, delta / seconds)
	if not is_equal_approx(next, grow) or not entry.has("grow"):
		entry["grow"] = next
		var eased := smoothstep(0.0, 1.0, next)
		for node: MultiMeshInstance3D in entry["nodes"]:
			node.set_instance_shader_parameter("cell_grow", eased)
	return shown or next > 0.0


## Seuil à hystérésis pour le niveau de détail : `current` conservé tant que `distance` reste dans la
## bande ±`hysteresis` (fraction) autour des seuils.
static func lod_with_hysteresis(distance: float, thresholds: Array, current: int, hysteresis: float) -> int:
	var level := current
	while level < thresholds.size() and distance >= float(thresholds[level]) * (1.0 + hysteresis):
		level += 1
	while level > 0 and distance < float(thresholds[level - 1]) * (1.0 - hysteresis):
		level -= 1
	return level
