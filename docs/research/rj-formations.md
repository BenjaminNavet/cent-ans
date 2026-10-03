# RJ-a — formations historiques des régiments (sources et réserves)

Lot RJ-a, ADR 0174. Données : `data/rules/unit_formations.json`. Ce document dit d'où viennent les noms et ce qui reste incertain ; les chiffres de jeu (rangs, modificateurs, durées) sont des choix d'équilibrage, pas des valeurs historiques.

Règle suivie : ne rien affirmer qui ne soit pas couramment admis ; quand une interprétation est débattue, l'infobulle le dit.

## Formations retenues

| Clé | Nom en jeu | Troupes | Fondement | Degré de certitude |
|---|---|---|---|---|
| `line` | Rangés en bataille | toutes | Formule courante des chroniques (« ordonnés et rangés », « se ranger en bataille ») pour un front de quelques rangs. | Sûr pour l'expression ; le nombre de rangs est un choix de jeu. |
| `thin_line` | En haie | pied, tireurs, cavaliers | « En haye » : front mince. L'expression est surtout attestée pour les gens d'armes des compagnies d'ordonnance (fin XVe-XVIe s.) ; son emploi pour l'infanterie du XIVe s. est une extension. | Moyen ; anachronisme léger signalé dans l'infobulle. |
| `battle` | Bataille serrée | pied (hommes d'armes) | « Bataille » = corps principal d'une armée (Froissart : les trois batailles). Hommes d'armes démontés en masse profonde : Crécy 1346, Poitiers 1356, Azincourt 1415 (Curry, *Agincourt: A New History*, 2005 ; Sumption, *The Hundred Years War*, t. I-II). | Sûr pour le principe ; profondeur (« huit rangs et plus ») approximative selon les récits d'Azincourt. |
| `column` | En ordre de marche | pied, tireurs, cavaliers | Marche « en route », bannière après bannière. Le terme « colonne » est moderne (XVIIe s.), d'où le nom choisi. | Sûr. |
| `square` | Schiltron | pied | Formation de piques écossaise : Falkirk 1298 (défaite face aux archers), Bannockburn 1314 (victoire) ; Halidon Hill 1333 et Neville's Cross 1346 montrent sa vulnérabilité aux archers (Barrow, *Robert Bruce*, 1965 ; Prestwich, *Armies and Warfare in the Middle Ages*, 1996). | Sûr pour le nom et l'usage ; « cercle » ou « carré » : les deux sont attestés selon les récits. |
| `herse` | Herse | tireurs à pied | Froissart (Crécy) : archers disposés « en manière d'une herse ». Interprétations concurrentes : quinconce dans la ligne, ou saillants triangulaires sur les ailes et entre les batailles (Burne, *The Crécy War*, 1955 ; discussion chez Bradbury, *The Medieval Archer*, 1985 ; Strickland et Hardy, *The Great Warbow*, 2005). | **Débattu** ; l'infobulle le dit. Le jeu représente la lecture en quinconce. |
| `wedge` | Coin | cavaliers | *Cuneus* dans les chroniques latines ; l'existence d'un vrai coin de cavalerie lourde en Occident est discutée (Verbruggen, *The Art of Warfare in Western Europe during the Middle Ages*, 2e éd. 1997, qui lit souvent *cuneus* comme simple « troupe »). | **Débattu** ; l'infobulle le dit. Formation conservée (existait déjà, IA de charge). |
| `conroi` | En conroi | cavaliers | Conroi : petit corps de chevaliers chargeant serrés derrière une bannière (Verbruggen, *op. cit.*). Terme surtout XIIe-XIIIe s. ; au XIVe s. on parle plutôt de « bannières » et de « routes ». | Moyen ; période légèrement antérieure, signalée (« XIIe-XIVe siècle »). |

## Écartées (et pourquoi)

- **Haie de pieux** : déjà présente sous forme d'ordre (pieux plantés des archers, `stakes_planted`, capacité `stakes`). Une formation dédiée ferait doublon.
- **Ordre lâche / tirailleurs** : déjà couvert par le mode escarmouche (K, `unit_modes.json`).
- **Hérisson / « ordre en rond » de piquiers suisses** : XVe s. tardif, hors de la plupart des batailles du jeu ; le schiltron couvre l'usage.
- **Wagenburg (chariots hussites)** : réel (années 1420) mais demande des chariots en bataille ; hors périmètre.

## Points à confirmer par l'historien du projet

1. Libellé de la ligne ordinaire : « Rangés en bataille » est une formule, pas un terme technique ; « En ordonnance » serait une alternative.
2. « En haie » appliqué aux archers et à l'infanterie du XIVe s.
3. Profondeur de la « bataille » (8 rangs en jeu).
