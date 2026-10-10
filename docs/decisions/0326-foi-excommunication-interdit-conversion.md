# 0326 — Foi : excommunication élargie, interdit, conversion des provinces

Statut : accepté.

## Contexte
Lot TW `m2b` (critique campagne vs Medieval II, `docs/wip/tw/campagne-m2.md` top4 et top6). L'excommunication n'avait qu'un déclencheur (un catholique à faveur < 10 en attaque un autre), il n'y avait pas d'interdit, et la foi d'une province ne changeait jamais (seule une hérésie s'y superposait ; une province de foi étrangère coûtait seulement un malus d'ordre `foreign_religion`).

## Décision
- Données dans `data/rules/religion.json` (schéma `religion_rules`, `data_model::ReligionRules`, `GameData::religion_rules`, optionnel : sans le fichier tout est inerte).
- Excommunication : en plus de la règle existante, rupture de trêve ou de pacte contre un catholique (faveur après pénalité < `perjury_favor_below`) ; exécution d'un captif catholique par un catholique (`ransom::execute_captive` : -`captive_favor_loss` de faveur, excommunication sous `captive_favor_below`). Réutilise `religion::excommunicate`.
- Interdit : `FactionState.interdict_until`. Posé avec l'excommunication si la faveur est sous `interdict.favor_below` ou si le souverain est déjà excommunié. Effet : `interdict.unrest` de troubles plats dans chaque province du royaume (`CampaignState::interdict_unrest`, lu par `population.rs`). Levé par un don à l'Église d'au moins `lift_donation` livres, par la médiation papale (`request_papal_mediation`), à la levée de l'excommunication, ou au bout de `turns` saisons. L'IA qui rachète déjà la faveur du pape (don de 2 000 livres) le fait donc aussi contre l'interdit.
- Conversion : `ProvinceState.faith_override` (foi convertie ; `None` = foi des données), `conversion_progress` (0-100), `conversion_to`. `CampaignState::province_faith` remplace la foi des données partout (ordre public, pont, carte des religions). Phase `conversion::resolve_conversion` après `resolve_heresy` : un seigneur d'une autre foi (obédiences d'une même Église exclues) fait progresser la province de `base_per_season` + piété du gouverneur (ou du souverain) / `piety_divisor` + bâtiments religieux pondérés, ×`kindred_percent` pour une Église apparentée, ×`occupier_percent` si la province n'est qu'occupée, bloqué si l'ordre est sous `stall_unrest`. À 100 la foi change (troubles `unrest_on_conversion`). Sans objet, le progrès retombe de `decay_per_season`. L'action `preach` du prédicateur ajoute `preacher_gain` + `preacher_per_level` par niveau (effet `conversion` dans `data/rules/agents.json`) ; l'IA prêcheur vise aussi les provinces à convertir.
- UI minimale : ligne « INTERDIT » dans la chancellerie (`diplomacy_panel.gd`), conversion dans l'infobulle de la carte des religions (`get_province_religion` : `conversion_progress`, `conversion_to_name`, `conversion_speed`).

## Conséquences
- Sauvegardes anciennes : tous les champs en `serde(default)`.
- Le mode de conversion est symétrique : un seigneur musulman convertit aussi une province catholique qu'il tient.
- Écart à la spec : « refus d'obédience au schisme » n'est pas un déclencheur (refuser une obédience laisse le choix précédent, il n'y a rien à punir) ; la « médiation » ne lève l'interdit que pour un royaume non excommunié (la médiation exige de ne pas l'être), le cas courant est le don.
