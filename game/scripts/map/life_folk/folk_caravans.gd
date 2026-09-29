class_name FolkCaravans
extends RefCounted

## Chantier FK, lot FK3 (`docs/design/2026-09-29-carte-vivante-folk.md` § 3.2). Squelette.
## Rendu seulement : aucune règle de jeu ; tout vient du pont.


func refresh(_sim: Object) -> void:
	pass


func populate(_pool: FolkPool, _focus: Vector2, _radius: float) -> void:
	pass
