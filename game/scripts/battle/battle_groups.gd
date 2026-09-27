class_name BattleGroups
extends RefCounted

## Regroupements d'interface du HUD de bataille (F5b) : les trois « batailles » de l'ost
## (avant-garde, corps de bataille, arrière-garde) qui rangent les cartes d'unités, et les
## groupes de sélection Ctrl+1..9 (enregistrer) / 1..9 (rappeler). Pure présentation : aucune
## règle, les ordres passent toujours par `BattleSim.issue_command`.
##
## Assignation par défaut selon le rôle : cavalerie → avant-garde ; tireurs et infanterie →
## corps de bataille ; réserve (clé `reserve` si la simulation l'expose) et engins de siège →
## arrière-garde.

const ORDER := ["vanguard", "main", "rear"]
const LABELS := {"vanguard": "Avant-garde", "main": "Bataille", "rear": "Arrière-garde"}
const MAX_GROUPS := 9

## numéro de groupe (1..9) → ids d'unités
var groups: Dictionary = {}


static func default_battle(unit: Dictionary) -> String:
	if bool(unit.get("reserve", false)):
		return "rear"
	match str(unit.get("category", "")):
		"cavalry":
			return "vanguard"
		"siege":
			return "rear"
	if ["ram", "tower"].has(str(unit.get("render", ""))):
		return "rear"
	return "main"


## Enregistre la sélection sous le numéro `number` (1..9) ; une sélection vide efface le groupe.
func save(number: int, ids: Array) -> void:
	if number < 1 or number > MAX_GROUPS:
		return
	if ids.is_empty():
		groups.erase(number)
		return
	var copy: Array[int] = []
	for id in ids:
		copy.append(int(id))
	groups[number] = copy


## Ids du groupe `number` encore présents sur le champ ([] si le groupe est vide).
func recall(number: int, units: Array) -> Array[int]:
	var result: Array[int] = []
	var ids: Array = groups.get(number, [])
	for unit in units:
		if ids.has(int(unit["id"])) and bool(unit["present"]):
			result.append(int(unit["id"]))
	return result


## Numéros des groupes contenant `unit_id` (pour les pastilles des cartes).
func numbers_of(unit_id: int) -> Array[int]:
	var result: Array[int] = []
	for number in groups:
		if (groups[number] as Array).has(unit_id):
			result.append(int(number))
	result.sort()
	return result


# ----- CB1 : groupes verrouillés (Ctrl/Cmd+G) -------------------------------------------------
# Un groupe verrouillé garde sa forme : un ordre donné au groupe devient des ordres `move`
# individuels (translation et rotation rigides des décalages relevés au verrouillage), à
# l'allure du plus lent (`match_speed`) et sous une même étiquette (`group_tag`). Une unité en
# déroute, détruite ou sortie du champ quitte le groupe ; un groupe réduit à une unité se défait.

## étiquette (1, 2, …) → {ids: Array[int], shape: FormationDrag.lock_shape(...)}
var locks: Dictionary = {}
var _next_tag := 1


## Verrouille la sélection `ids` (unités de `units`), ou la déverrouille si elle forme déjà
## exactement un groupe verrouillé. Renvoie l'étiquette du nouveau groupe, 0 si déverrouillé
## (ou si moins de deux unités disponibles).
func toggle_lock(ids: Array, units: Array) -> int:
	var tag := locked_group_for(ids)
	if tag != 0 and (locks[tag]["ids"] as Array).size() == ids.size():
		locks.erase(tag)
		return 0
	var members: Array = []
	for unit in units:
		if ids.has(int(unit["id"])) and _available(unit):
			members.append(unit)
	for id in ids:
		_leave(int(id))
	if members.size() < 2:
		return 0
	var copy: Array[int] = []
	for unit in members:
		copy.append(int(unit["id"]))
	tag = _next_tag
	_next_tag += 1
	locks[tag] = {"ids": copy, "shape": FormationDrag.lock_shape(members)}
	return tag


## CB6 : verrouille les régiments de `places` ({id, x, z, facing} : places d'une formation de
## groupe) dans la forme de ces places, en les retirant de leurs groupes précédents. Renvoie
## l'étiquette du nouveau groupe (0 : moins de deux régiments).
func lock_as(places: Array) -> int:
	for place in places:
		_leave(int(place["id"]))
	if places.size() < 2:
		return 0
	var copy: Array[int] = []
	for place in places:
		copy.append(int(place["id"]))
	var tag := _next_tag
	_next_tag += 1
	locks[tag] = {"ids": copy, "shape": FormationDrag.lock_shape(places)}
	return tag


## Étiquette du groupe verrouillé qui contient toutes les unités `ids` (0 sinon).
func locked_group_for(ids: Array) -> int:
	if ids.is_empty():
		return 0
	var tag := lock_of(int(ids[0]))
	if tag == 0:
		return 0
	for id in ids:
		if lock_of(int(id)) != tag:
			return 0
	return tag


## Étiquette du groupe verrouillé de `unit_id` (0 : aucun).
func lock_of(unit_id: int) -> int:
	for tag in locks:
		if (locks[tag]["ids"] as Array).has(unit_id):
			return int(tag)
	return 0


func is_locked(unit_id: int) -> bool:
	return lock_of(unit_id) != 0


## Ids du groupe verrouillé `tag` ([] s'il n'existe pas).
func lock_ids(tag: int) -> Array[int]:
	var out: Array[int] = []
	if locks.has(tag):
		out.assign(locks[tag]["ids"])
	return out


## Places des membres du groupe `tag` déplacé en `point` (x, z), tourné vers `facing` (NaN :
## orientation du verrouillage) : voir `FormationDrag.rigid_places`.
func lock_places(tag: int, point: Vector2, facing: float) -> Array:
	if not locks.has(tag):
		return []
	return FormationDrag.rigid_places(locks[tag]["shape"], locks[tag]["ids"], point, facing)


## Retire des groupes verrouillés les unités en déroute, détruites ou hors du champ.
func prune_locks(units: Array) -> void:
	if locks.is_empty():
		return
	for unit in units:
		if not _available(unit):
			_leave(int(unit["id"]))


static func _available(unit: Dictionary) -> bool:
	return bool(unit["present"]) and str(unit["state"]) != "routing" and not bool(unit.get("left_field", false))


func _leave(unit_id: int) -> void:
	var tag := lock_of(unit_id)
	if tag == 0:
		return
	var ids: Array = locks[tag]["ids"]
	ids.erase(unit_id)
	(locks[tag]["shape"]["members"] as Dictionary).erase(unit_id)
	if ids.size() < 2:
		locks.erase(tag)
