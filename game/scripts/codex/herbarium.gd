class_name Herbarium
extends RefCounted

## H9 — Herbier : les plantes (`herbs`, ids du Codex) des technologies acquises par le joueur
## sont marquées découvertes dans `CodexStore` (méta-progression, pas une règle de jeu). Les
## fiches encore absentes du Codex sont ignorées silencieusement.


## Marque découvertes les plantes des technologies `known` de `faction` ; renvoie les ids
## nouvellement découverts (vide si rien de neuf, si la simulation n'a pas d'arbre ou sans Codex).
static func sync(sim: Object, faction: String) -> Array:
	var found: Array = []
	var codex := CodexText.store()
	if sim == null or codex == null or faction == "" or not sim.has_method("get_tech_tree"):
		return found
	for node in sim.call("get_tech_tree", faction):
		if not (node is Dictionary) or str(node.get("state", "")) != "known":
			continue
		for herb in node.get("herbs", []):
			var id := str(herb)
			if bool(codex.call("has_entry", id)) and bool(codex.call("discover", id)):
				found.append(id)
	return found


## « Nouvelle plante dans l'herbier : Sauge, Rue » (titres du Codex).
static func message(ids: Array) -> String:
	var codex := CodexText.store()
	var names := PackedStringArray()
	for id in ids:
		names.append(str(codex.call("title", str(id))) if codex != null else str(id))
	return "%s dans l'herbier : %s" % ["Nouvelle plante" if ids.size() == 1 else "Nouvelles plantes", ", ".join(names)]
