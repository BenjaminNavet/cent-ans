# VIS-R7 — végétation de plaine

Spec : `docs/audits/2026-10-29-visuel-campagne.md` table (d) R7, constat G2-7. ADR : `docs/decisions/XXXX-vis-r7-vegetation-de-plaine.md`.

## État
Fait : arbres isolés coupés au-delà de d 150 ; bosquets 0,5 → 0,1 ; `forest_masses.value_lift` (teinte seulement) ; canopée
`value` 0,88 → 0,6 et `balance` plus vert ; couleurs d'automne par classe en données (`seasons.autumn`), `winter.bare`
brun-violet. Non fait : haies du bocage en lignes sombres (le style généralisé ne dessine aucune haie ; motif de sol de R3).

## Mesures (CENT_ANS_WEATHER=clear, dump `open_forest`, boîtes HSV sur camp_0600)
| | avant | après |
|---|---|---|
| arbres isolés gardés, camp_0600 (d 600) | 2 979 / 48 169 | 0 |
| arbres isolés gardés, camp_guyenne (d 300) | 1 850 / 25 740 | 0 |
| bosquets gardés, camp_0600 | 5 492 / 10 864 | 1 031 |
| forêt IdF, valeur V (camp_0600) | 0,408 | 0,356 |
| forêt Alençon / Paris-S, V | 0,348 / 0,374 | 0,283 / 0,311 |
| champs (3 boîtes), V | 0,371 / 0,413 / 0,434 | 0,377 / 0,409 / 0,439 |
| teinte forêt IdF (H) | 74 (jaune) | 83 |

`forest_masses.value_lift` seul n'a pas bougé la forêt vue (la masse vue est la canopée des imposteurs, pas la carte de couleur) ;
c'est `canopy.value` qui a fait la hiérarchie. Captures (scratchpad) : `vr7_final/{camp_0600,camp_guyenne,autumn_0300}.png`,
avant : `base/`.

## Points ouverts
- terrain.wesl : seule `dark_masses` modifiée (+ `macro_amp.z`). À signaler à VIS-R1.
- Printemps des feuillus (×1.10, 1.15, 0.80 dans `foliage_season`) reste codé en dur : jaune ; à passer en données par classe.
- Haies du bocage : R3.
