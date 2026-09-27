extends SceneTree

## Chantier PO (ADR 0097, bible DA § 12.6) : critère C4. Chaque contexte de la tranche résout un
## préréglage complet d'heure du jour et d'étalonnage depuis `data/fx/atmosphere.json`, sans valeur
## de soleil codée en dur :
## - campagne (PO3) : chaque saison ;
## - bataille (PO4) : météo × saison × heure (`morning`, `midday`, `evening`).
## PO0 : désactivé (PO3 et PO4 activent leur partie).
## Usage : godot --headless --path game --script res://tests/po_grade_test.gd


func _init() -> void:
	print("po_grade_test: désactivé (PO0)")
	quit(0)
