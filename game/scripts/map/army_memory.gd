class_name ArmyMemory
extends RefCounted

## WH hover (ADR 0271) : mémoire des armées ennemies perdues de vue. Éphémère (UI seulement,
## hors sauvegarde). `note` retient la dernière situation connue d'une armée ennemie en vue ;
## `ghosts` rend celles qui ne sont plus en vue depuis au plus `ttl` saisons (tours), la durée
## venant de `StanceCues.fog_memory_turns()` (`data/map/stance_cues.json`). Une armée qui
## réapparaît n'est plus un fantôme (sa fiche est réécrite à chaque `note`). Ne consulte jamais
## la simulation : rien de ce que le joueur n'a pas vu.

## id → {id, faction, general_name, pos: Vector2 (px carte), men, turn, units}
var _seen: Dictionary = {}


## Armée ennemie en vue au tour `turn` (`army` : dictionnaire de `get_army`).
func note(army_id: String, army: Dictionary, turn: int, position: Vector2) -> void:
	var men := 0
	for unit: Dictionary in army.get("units", []):
		men += int(unit.get("strength", 0))
	_seen[army_id] = {
		"id": army_id,
		"faction": str(army.get("faction", "")),
		"general_name": str(army.get("general_name", "")),
		"pos": position,
		"men": men,
		"turn": turn,
	}


## Fantômes au tour `turn` : armées notées, absentes de `visible_now` (id → vrai), vues il y a
## au plus `ttl` saisons ; les plus anciennes sont oubliées. Chaque entrée porte `ago` (saisons).
func ghosts(turn: int, visible_now: Dictionary, ttl: int) -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for id in _seen.keys():
		var entry: Dictionary = _seen[id]
		if visible_now.has(id):
			continue
		var ago := maxi(turn - int(entry["turn"]), 0)
		if ago > ttl:
			_seen.erase(id)
			continue
		var ghost := entry.duplicate()
		ghost["ago"] = ago
		out.append(ghost)
	return out


## Oublie une armée (détruite à la vue du joueur, nouvelle partie).
func forget(army_id: String) -> void:
	_seen.erase(army_id)


func clear() -> void:
	_seen.clear()
