class_name TileJobPool
extends RefCounted

## File de tâches de tuiles dans le `WorkerThreadPool` : une entrée par clé
## (tuile, cellule, page…), un identifiant de tâche et une charge (le « job » ou le dictionnaire
## de résultat que la tâche remplit). Les résultats sont repris sur le fil principal, qui attend
## chaque tâche exactement une fois (un identifiant attendu n'est plus valide).
##
## Usage : `submit(key, payload, work)` ; chaque image `for key in ready_keys(): take(key)` ;
## `wait_all()` à la sortie de l'arbre. Une clé déjà soumise est refusée (`has`).

## Clé des files à tâche unique (un calcul à la fois).
const SINGLE := 0

# key → {"task": int, "payload": Variant}
var _entries: Dictionary = {}


## Lance `work` (appelable sans argument) pour `key` ; `payload` est rendu tel quel par `take`.
## Renvoie `payload`. No-op (renvoie null) si la clé est déjà en cours.
func submit(key: Variant, payload: Variant, work: Callable, label: String = "tile job") -> Variant:
	if _entries.has(key):
		return null
	_entries[key] = {"task": WorkerThreadPool.add_task(work, false, label), "payload": payload}
	return payload


func has(key: Variant) -> bool:
	return _entries.has(key)


func size() -> int:
	return _entries.size()


func is_empty() -> bool:
	return _entries.is_empty()


func keys() -> Array:
	return _entries.keys()


## Charge de `key` (tâche éventuellement en cours : le dictionnaire est partagé avec elle).
func payload(key: Variant) -> Variant:
	return (_entries[key] as Dictionary)["payload"] if _entries.has(key) else null


## Tâche terminée (non bloquant).
func is_done(key: Variant) -> bool:
	return _entries.has(key) and _completed(_entries[key])


## Clés dont la tâche est terminée.
func ready_keys() -> Array:
	var ready: Array = []
	for key: Variant in _entries:
		if _completed(_entries[key]):
			ready.append(key)
	return ready


## Attend la tâche de `key`, retire l'entrée, rend la charge.
func take(key: Variant) -> Variant:
	var entry: Dictionary = _entries[key]
	_wait(entry)
	_entries.erase(key)
	return entry["payload"]


## Attend toutes les tâches mais garde les entrées (à reprendre ensuite par `take`).
func wait_done() -> void:
	for entry: Dictionary in _entries.values():
		_wait(entry)


## Attend toutes les tâches et rend les charges (ordre de soumission), file vidée.
func take_all() -> Array:
	var payloads: Array = []
	for key: Variant in _entries.keys():
		payloads.append(take(key))
	return payloads


## Attend toutes les tâches et vide la file (sortie de l'arbre, `clear`).
func wait_all() -> void:
	wait_done()
	_entries.clear()


static func _completed(entry: Dictionary) -> bool:
	return entry.has("waited") or WorkerThreadPool.is_task_completed(int(entry["task"]))


## Attend une seule fois (un identifiant attendu n'est plus valide).
static func _wait(entry: Dictionary) -> void:
	if not entry.has("waited"):
		WorkerThreadPool.wait_for_task_completion(int(entry["task"]))
		entry["waited"] = true
