# 0281 — Boucle du tour fluide et narrative (WH turn)

Statut : accepté (lot WH `turn`).

## Contexte
Écarts relevés contre Total War: Warhammer III (`docs/wip/wh/tour.md`) : sauvegarde auto tous les 4 tours sur 3 emplacements, missions visibles seulement dans le panneau O et sans suite, dilemmes sans option conditionnelle, rapport de saison modal à chaque saison, écran de fin sans bilan.

## Décision
- **Sauvegarde** : défaut `game/autosave_interval = 1` (chaque saison), 5 emplacements `auto_1..5` tournants, plus `auto_battle` écrit avant le dialogue d'une bataille (`SaveSlots.autosave_battle`, désactivé si l'intervalle est 0).
- **Dilemmes** : `EventOption.requires` (conditions existantes) + `requires_reason` ; `CampaignState::option_unavailable` est l'unique juge. `decision_views` expose `allowed/reason`, `choose_event_option` refuse (`ChronicleError::OptionUnavailable`). Plancher de trésor implicite : une option qui dépense plus que le trésor est grisée, sauf si aucune autre option n'est payable (jamais de décision bloquée). L'IA écarte les options dont `requires` échoue tant qu'il en reste une.
- **Missions de faction** : `MissionTemplate.faction/after/province/source`, cible `fixed`, `MissionsState.done` (gabarits réussis) ; un gabarit de faction n'est offert qu'à sa faction, une fois, et après son prédécesseur. 8 missions sourcées (France 3, Angleterre 3, Bourgogne 2). Choix retenu : l'offre reste imposée (comme les missions génériques), pas un dilemme accepter/refuser — à trancher plus tard.
- **Suivi** : `MissionTracker` (haut droite, repliable, réglage `interface/mission_tracker_collapsed`) ; pastille de cloche `mission_due` quand `turns_left <= 1`.
- **Rapport de saison** : `interface/season_report = always | auto | off` (défaut `auto`) ; en `auto`, fenêtre seulement si perte, prise ou bataille du joueur, sinon un toast d'une ligne. L'ancien booléen migre (vrai : auto, faux : off).
- **Bilan de fin** : `CampaignState::campaign_report` (module `campaign_stats`, compteurs sérialisés `stats`, `#[serde(default)]`) + `CampaignSim.get_campaign_report` ; le score est décomposé en terres / objectifs / prestige / trésor, le reste en « autres » pour rester exact si la formule de `victory.rs` (lot RX) change.

## Conséquences
- `victory.rs` intact ; le bilan ne lit que `campaign_score`, `objectives` et `outcome`.
- Un événement annoté `requires` est une donnée : 16 options annotées, d'autres suivront sans code.
- Les parties en cours gardent leurs compteurs de bilan à zéro avant la sauvegarde (départ pris au premier tour rechargé).
