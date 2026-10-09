extends SceneTree

## Mesure du chargement du Codex (`CodexStore.reload`) : médiane de 5 rechargements à froid
## d'appel (cache disque chaud), plus une empreinte du contenu chargé (fiches, alias, entités,
## exclusions) pour vérifier qu'un changement de format de lecture ne change rien.
## Usage : godot --headless --path game --script res://tests/codex_load_bench.gd

const RUNS := 5


func _initialize() -> void:
	var store: Node = root.get_node("CodexStore")
	var times: Array = []
	for i in RUNS:
		var start := Time.get_ticks_usec()
		store.call("reload")
		times.append((Time.get_ticks_usec() - start) / 1000.0)
	times.sort()
	var snapshot := {
		"entries": store.entries,
		"aliases": store._aliases,
		"by_entity": store._by_entity,
		"exclusions": store._exclusions,
	}
	var digest := JSON.stringify(snapshot, "", true).sha256_text()
	print("codex_load_bench: dir=%s entries=%d median_ms=%.1f min_ms=%.1f max_ms=%.1f" % [
		store.codex_dir, store.entries.size(), times[RUNS / 2], times[0], times[RUNS - 1]])
	print("codex_load_bench: content_sha256=%s" % digest)
	quit(0)
