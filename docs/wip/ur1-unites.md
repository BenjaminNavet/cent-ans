# WIP — UR1 : variété des unités (roster à la Medieval II)

Branche : `worktree-agent-a213f64dfca3b3a18`.

## État

| Lot | État |
|---|---|
| Données : 14 types, schéma (`available_from`, `available_until`, `figure`), doctrines, `enables_units` | fait |
| Cœur : refus de recrutement hors époque (`orders.rs::recruit_blocker`), tests `sim-campaign/tests/ur1_units.rs` | fait |
| Équilibre : sondes `matrix` et `campaign` | à faire |
| Figurines (pipeline V2) | à faire |
| Cartes d'unité (illustrations, icônes) | à faire |
| Captures `docs/audit/captures/ur1/` | à faire |

## Nouveaux types

| Id | Nom | Accès | Figurine |
|---|---|---|---|
| unit_welsh_spearmen | Lanciers gallois | culture galloise | infantry_3 |
| unit_hobelars | Hobelars | cultures anglaise, galloise, irlandaise ; jusqu'en 1400 | cavalry_6 |
| unit_english_retinue | Hommes d'armes des retenues | Angleterre, tech. hommes d'armes à pied | infantry_7 |
| unit_scottish_spearmen | Schiltron écossais | culture écossaise | infantry_4 |
| unit_goedendag_militia | Milice au goedendag | cultures flamande, néerlandaise | infantry_5 |
| unit_coutiliers | Coutiliers | France, Bretagne, Bourgogne ; tech. compagnies d'ordonnance ; dès 1445 | infantry_6 |
| unit_francs_archers | Francs-archers | France ; tech. francs-archers ; dès 1448 | archer_3 |
| unit_ordonnance_gendarmes | Gendarmes des compagnies d'ordonnance | France ; tech. ; dès 1445 | cavalry_3 |
| unit_routiers | Routiers des Grandes Compagnies | mercenaires, 1356-1395 | infantry_8 |
| unit_ecorcheurs | Écorcheurs | mercenaires, cultures française, occitane, allemande ; 1435-1445 | cavalry_4 |
| unit_jinetes | Jinetes | cultures castillane, andalouse, galicienne | cavalry_5 |
| unit_breton_knights | Chevaliers bretons | culture bretonne | cavalry_0 (chevaliers) |
| unit_gascon_crossbowmen | Arbalétriers gascons | cultures occitane, basque, navarraise | archer_4 |
| unit_culveriners | Couleuvriniers | tech. couleuvrines, arsenal ; dès 1380 | archer_5 |

## Prochaine étape

Sonde `matrix` puis `campaign 200 1..8` ; figurines ; illustrations.
