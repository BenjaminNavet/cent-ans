extends TestCase

## Base commune du smoke test : constantes, état partagé (autoloads, dossier `user://` dédié) et aides `_check` / `_fail`.
## Découpage de `smoke.gd` (SC GT7) : les sections se chaînent par héritage et partagent l'état.
## Ne se lance pas seul : point d'entrée `res://tests/smoke.gd`.

const EXPECTED_LABEL := "Automne 1339"
const EXPECTED_TURN := 10
const PICK_PROVINCE_INDEX := 3
const SMOKE_SAVE := "smoke"

var _fixtures_dir: String
## Autoloads obtenus dynamiquement : un script `--script` est compilé avant l'enregistrement des
## singletons, donc `SimFacade.x` / `MapPaths.x` (membres d'instance) y sont interdits.
var facade: Node
var paths: Node
## T2 : dossier `user://` dédié à cette exécution (PID + horodatage), pour que plusieurs smoke
## tests lancés en parallèle (plusieurs worktrees d'agents partagent le même `user://` Godot,
## dérivé du nom du projet et non du chemin sur disque) ne se marchent pas dessus sur les
## sauvegardes / réglages / découvertes du Codex. Nettoyé en fin d'exécution (`_cleanup_test_dir`).
var _test_root: String
var _test_settings_path: String
var _test_codex_path: String
var _test_saves_dir: String


## T2 : supprime le dossier `user://smoke_<pid>_<horodatage>` créé pour cette exécution
## (réglages, découvertes du Codex, sauvegardes). Best effort : une erreur ne fait pas
## échouer le smoke test.
func _cleanup_test_dir() -> void:
	_remove_dir_recursive(_test_root)


func _remove_dir_recursive(path: String) -> void:
	var dir := DirAccess.open(path)
	if dir == null:
		return
	dir.include_hidden = true
	for file_name in dir.get_files():
		dir.remove(file_name)
	for sub_dir in dir.get_directories():
		_remove_dir_recursive(path.path_join(sub_dir))
	DirAccess.remove_absolute(path)


static func _project_root() -> String:
	return ProjectSettings.globalize_path("res://").path_join("..").simplify_path()


func _check(condition: bool, message: String) -> bool:
	return check(condition, message)


func _fail(message: String) -> void:
	check(false, message)
