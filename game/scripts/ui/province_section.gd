class_name ProvinceSection
extends PanelSection

## Base des sections du panneau de province : la province affichée, si le joueur la tient
## et la simulation (celle de `SimFacade` par défaut). Patron de méthode : `show_for` fixe ce
## contexte puis appelle `_render(data)`, que la fille surcharge ; `data` : ce que le panneau
## fournit hors simulation (ville, colonies…). Filles : `ProvinceChoiceSection` (La Table, Édit
## régional), `ClassesSection`, `SettlementsSection`. Aucune règle ici.

var province_id: String = ""
var is_player_owner: bool = false


## Remplit la section pour `province` ; `sim` : `SimFacade.sim` si null.
func show_for(province: String, player_owned: bool, sim: Object = null, data: Dictionary = {}) -> void:
	province_id = province
	is_player_owner = player_owned
	_sim = _resolve_sim(sim)
	_render(data)


## À surcharger : reconstruit la section depuis `_sim`, `province_id` et `data`.
func _render(_data: Dictionary) -> void:
	pass
