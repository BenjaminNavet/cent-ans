extends Control

## Écran minimal M0 : affiche la date de campagne et permet de finir le tour.
## Aucune règle de jeu ici : tout vient de CampaignSim (Rust).

@onready var date_label: Label = %DateLabel
@onready var end_turn_button: Button = %EndTurnButton

var campaign: CampaignSim


func _ready() -> void:
	campaign = CampaignSim.new()
	campaign.new_campaign(1337)
	end_turn_button.pressed.connect(_on_end_turn_pressed)
	_refresh()


func _on_end_turn_pressed() -> void:
	campaign.end_turn()
	_refresh()


func _refresh() -> void:
	date_label.text = "%s — tour %d" % [campaign.get_date_label(), campaign.get_turn()]
