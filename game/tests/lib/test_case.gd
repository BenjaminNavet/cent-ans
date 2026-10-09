class_name TestCase
extends SceneTree
## Base commune des tests headless : `extends TestCase` à la place de `extends SceneTree`.
## Lancement : godot --headless --path game --script res://tests/<nom>_test.gd
## Un test appelle `check` / `check_eq` pour chaque vérification, attend avec `wait_frames`,
## puis termine par `finish()` (résumé + code de sortie 0 si aucun échec, 1 sinon).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

## Nombre de vérifications échouées ; un test peut le lire (ex. pour sortir plus tôt).
var failures := 0
## Nombre total de vérifications exécutées.
var checks := 0


## Nom du test (nom du fichier sans extension), préfixe des messages d'échec.
func test_name() -> String:
	return get_script().resource_path.get_file().get_basename()


## Vérifie `condition` ; en cas d'échec, compte l'échec et journalise `message`. Retourne `condition`.
func check(condition: Variant, message: String) -> bool:
	# Variant : un test peut passer une valeur véridique (objet, tableau) sans conversion.
	var ok: bool = true if condition else false
	checks += 1
	if not ok:
		failures += 1
		push_error("%s: %s" % [test_name(), message])
	return ok


## Vérifie `actual == expected` et ajoute les deux valeurs au message en cas d'échec.
func check_eq(actual: Variant, expected: Variant, message: String) -> bool:
	return check(actual == expected, "%s (obtenu %s, attendu %s)" % [message, str(actual), str(expected)])


## Attend `frames` images de rendu.
func wait_frames(frames: int) -> void:
	for i in frames:
		await process_frame


## Affiche le résumé et quitte : code 0 sans échec, 1 sinon.
func finish() -> void:
	print("%s: %s" % [test_name(), "OK" if failures == 0 else "%d failure(s)" % failures])
	quit(1 if failures > 0 else 0)


## Démarre une partie de campagne (vraie simulation, graine 1337) pour `faction` et retourne
## la carte ajoutée à la racine, après `frames` images. Réglages de test : fichier dédié,
## sans sauvegarde auto ni tutoriel, surchargeables par `settings_overrides` ({clé: valeur}).
## Le test vérifie ensuite `map.load_ok and map.sim != null`.
func start_campaign_map(faction := "fac_france", settings_overrides := {}, frames := 2) -> Node3D:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		var values := {"game/autosave_interval": 0, "tutorial/enabled": false}
		values.merge(settings_overrides, true)
		for key in values:
			settings.call("set_value", key, values[key], false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = faction
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await wait_frames(frames)
	return map
