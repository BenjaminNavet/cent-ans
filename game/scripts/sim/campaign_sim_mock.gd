class_name CampaignSimMock
extends RefCounted

## Reste minimal : `campaign_map.gd` (gelé) l'instancie encore dans `_ensure_*_capable_sim`.
## À supprimer avec ces fonctions après le dégel ; `new_campaign` échoue, donc elles sortent sans effet.


func new_campaign(_data_dir: String, _faction: String, _seed: int) -> bool:
	return false
