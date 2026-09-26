class_name SettlementFit
extends RefCounted

## Lot DC6c (suites DC4, ADR 0082) : réduction et masquage des maquettes voisines à l'échelle
## effective. Fonctions pures (testées par `tests/dc6c_fit_scale_test.gd`).
##
## DC4 réduit une maquette trop proche d'une voisine (« place » `room` = part de l'écart qui lui
## revient, indépendante des rayons) et masque une maquette de faubourg ; SZ4b rétrécit ensuite
## chaque maquette de près vers son emprise réelle (`sigma` = échelle sans réduction,
## `MapPropScale.settlement_scale`). Rayon affiché = rayon d'origine × sigma × réduction, la
## réduction étant calculée sur le rayon déjà rétréci : de près, deux maquettes qui ne se
## recouvrent plus à l'échelle réelle retrouvent leur taille pleine.


## Réduction (≤ 1, ≥ `min_fit`) d'une maquette de rayon `radius` qui dispose de `room` unités.
static func fit_factor(room: float, radius: float, min_fit: float) -> float:
	return clampf(room / maxf(radius, 0.001), min_fit, 1.0)


## Échelle du porteur d'une maquette déjà réduite à la taille de carte (`fit_factor(room,
## base_radius)` appliqué à la maquette enfant) : rayon affiché = rayon réduit × ce facteur =
## `base_radius` × `sigma` × `fit_factor(room, base_radius × sigma)`. Vaut 1 à `sigma` = 1 et ne
## dépasse jamais 1.
static func zoom_scale(base_radius: float, room: float, sigma: float, min_fit: float) -> float:
	var map_fit := fit_factor(room, base_radius, min_fit)
	return sigma * fit_factor(room, base_radius * sigma, min_fit) / map_fit


## Clé d'une paire de voisines `a` < `b` (triée par `b` puis `a`).
static func pair_key(a: int, b: int) -> int:
	return (mini(a, b) & 0xFFFF) | (maxi(a, b) << 16)


## Masquage DC4 : `b` est masquée si son centre tombe à moins de `factor` × son rayon du bord
## d'une voisine prioritaire `a` < `b` non masquée, ou si les deux emprises se recouvrent de plus de
## `overlap_limit` × le plus petit rayon (réduction bloquée au plancher : Marmoutier sous Tours,
## ville emblématique qui ne se réduit pas). `pairs` : clés `pair_key` triées (croissantes),
## sur-ensemble des paires possibles ; `radius` : rayons affichés (0 : pas de maquette, ne masque
## rien) ; `protected` : 1 pour une maquette jamais masquée (ville emblématique, pas de maquette).
## Renvoie 1 par maquette masquée.
static func absorbed(pairs: PackedInt64Array, positions: PackedVector2Array, radius: PackedFloat32Array, protected: PackedByteArray, factor: float, overlap_limit: float) -> PackedByteArray:
	var result := PackedByteArray()
	result.resize(radius.size())
	result.fill(0)
	for key in pairs:
		var b := key >> 16
		var a := key & 0xFFFF
		if result[b] != 0 or protected[b] != 0 or result[a] != 0 or radius[a] <= 0.0:
			continue
		var d := positions[a].distance_to(positions[b])
		if d < radius[a] + factor * radius[b] or radius[a] + radius[b] - d > overlap_limit * minf(radius[a], radius[b]):
			result[b] = 1
	return result
