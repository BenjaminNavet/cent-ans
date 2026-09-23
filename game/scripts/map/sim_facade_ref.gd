class_name SimFacadeRef
extends RefCounted

## Accès à l'autoload `SimFacade` résolu à l'exécution (les scripts référencés par le smoke test
## sont compilés avant l'enregistrement des autoloads).


static func info(node: Node, faction_id: String) -> Dictionary:
	var facade: Node = node.get_node_or_null("/root/SimFacade")
	return facade.call("faction_info", faction_id) if facade != null else {}
