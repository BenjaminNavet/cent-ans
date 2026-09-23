extends Node

## Autoload `MapPaths` : résolution des chemins vers `data/`.
##
## `res://../data` ne survit pas à un export ; on calcule donc un chemin absolu
## au démarrage, surchargeable par la variable d'environnement `CENT_ANS_DATA_DIR`
## (utilisée par le smoke test pour pointer sur `game/tests/fixtures`).

const ENV_VAR := "CENT_ANS_DATA_DIR"

var data_dir: String


func _init() -> void:
	data_dir = default_data_dir()


static func default_data_dir() -> String:
	var from_env := OS.get_environment(ENV_VAR)
	if from_env != "":
		return from_env.simplify_path()
	return project_root().path_join("data")


## Racine du dépôt (le dossier parent de `game/`).
static func project_root() -> String:
	return ProjectSettings.globalize_path("res://").path_join("..").simplify_path()


func map_dir() -> String:
	return data_dir.path_join("map")
