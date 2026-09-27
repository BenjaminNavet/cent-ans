class_name SceneFader
extends RefCounted

## Chantier PO5 (ADR 0097, bible DA § 12.4) : façade statique des transitions entre scènes.
## - `SceneFader.go(path)` : fondu au noir parchemin (0,25 s), changement de scène, fondu
##   d'entrée ; remplace `get_tree().change_scene_to_file(path)` partout dans le jeu ;
## - `SceneFader.cover()` / `SceneFader.reveal()` : passages internes sans changement de scène
##   (retour de bataille vers la carte).
## Instantané en headless ; « Réduire les animations » raccourcit les fondus. Le travail est fait
## par `SceneFaderLayer`, créé à la demande sous la racine.

const NODE_NAME := "SceneFaderLayer"


## Nœud de transition (créé sous la racine au premier appel).
static func instance() -> SceneFaderLayer:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null or tree.root == null:
		return null
	var node := tree.root.get_node_or_null(NODE_NAME) as SceneFaderLayer
	if node == null:
		node = SceneFaderLayer.new()
		node.name = NODE_NAME
		tree.root.add_child(node)
	return node


## Fondu, `change_scene_to_file(path)`, fondu d'entrée. Rend l'erreur du changement de scène.
static func go(path: String) -> Error:
	var node := instance()
	if node == null:
		return ERR_UNAVAILABLE
	return await node.go(path)


static func cover() -> void:
	var node := instance()
	if node != null:
		await node.cover()


static func reveal() -> void:
	var node := instance()
	if node != null:
		await node.reveal()


static func is_covered() -> bool:
	var node := instance()
	return node != null and node.is_covered()
