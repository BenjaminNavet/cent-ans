class_name CrusadeSection
extends VBoxContainer

## JR3 — section « Ferveur » du panneau de faction (faction croisée seulement), construite en
## code : jauge 0-100 avec ses seuils, aumônes, état (élan / moral / débandade), bouton « Prêcher
## le passage » et contingents attendus. Aucune règle ici : tout vient de
## `CampaignSim.get_crusade` / `submit_order` (spec JR § 4 et 5).

signal passage_preached

var crusade: Dictionary = {}
var last_result: Dictionary = {}
var _sim: Object = null


func _init() -> void:
	name = "CrusadeSection"


## Remplit la section ; masquée si le joueur n'est pas la faction croisée.
func show_for(_player_owned: bool = true, _sim_override: Object = null) -> void:
	hide()


## Ordre `preach_passage` ; en cas de refus, message de la simulation affiché en rouge.
func request_preach() -> Dictionary:
	return {}
