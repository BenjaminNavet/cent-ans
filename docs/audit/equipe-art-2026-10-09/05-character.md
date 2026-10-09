# Character artist — état des lieux (09/10, lecture seule)

## 1. État actuel
**Trois générations de figurines coexistent**
- V1 (plus utilisée) : `game/assets/models/battle/` (9 glb, 2,2 Mo) encore référencée par `battle_meshes.gd` ; `army*.glb` encore cités par `model_library.gd`.
- V2 Quaternius : `battle_skinned/` (7,4 Mo), repli `--coarse-figures`.
- **Fines FG** (défaut, ADR 0089) : `battle_fine/` (49 Mo), 37 figurines, MakeHuman CC0 + cheval CC0 + équipement procédural ; LOD0 ~9,4-11,9 k tris (15-17 k monté), LOD1 ~1,3-2 k, LOD2 ~250-530, imposteurs > 300 m ; 2-4 variantes, 8 visages, atlas (ADR 0088), usure SR2 (ADR 0136).
- **GA3** (image → 3D) : `battle_ga3/` (36 Mo), 16 figurines fusionnées par-dessus (`_merge_ga3`, ADR 0140, 0211), 2 variantes de visage, ~0,16 $/unité.

**Couverture** : 35 unités sur 22 figurines. Partages : `cavalry_0` (chevaliers, bretons, teutoniques, serbes), `cavalry_3` (gendarmes, sergents montés, archers montés), `archer_3` (francs-archers, écossais, yaya), `infantry_4` (piquiers flamands, lanciers écossais), `cavalry_2` (mamelouks, steppe), etc. 177 factions distinguées seulement par livrée, armoiries et `unit_looks.json` (10 unités orientales).

**Campagne** : `army_figures.gd` réutilise le LOD1 ; général monté ×2,3 portant l'étendard + 2-6 soldats. Civils : villager_0-3 + 15 accessoires. Pas de modèle de seigneur dédié.

## 2. Forces
- Chaîne scriptée, reproductible, presque gratuite (CC0).
- Rig et clips partagés partout ; LOD mesurés (+2,8 % au banc).
- Équipement d'époque documenté ; trois types de montures, caparaçons, robes.
- GA3 nettement plus réaliste (`docs/img/ga3/l4_units.jpg`).

## 3. Faiblesses
1. **Écart GA3/fines dans une même bataille** : cavalerie orientale, archer_3/5, musiciens, étendard, équipages, villageois restent en fines → armées mameloukes ou lituaniennes moins finies que les anglaises.
2. **Infidélités par partage** : archers montés sur un gendarme à lance ; akinci à lance (fiche : arc) ; yaya en francs-archers ; teutoniques/serbes en chevalier français ; flamands en schiltron écossais.
3. Aucun vrai seigneur (harnois, cimier, roi) ; sur la carte, un gendarme agrandi.
4. Civils : 4 hommes seulement (ni femmes, enfants, clercs, marchands, bourgeois).
5. Variété GA3 limitée (2 variantes, couvre-chefs fixes), mains en moufle, figurines gonflées de 6 mm.
6. Possibles artefacts `l3c_knight.jpg` (plaque blanche sous le cheval, chevaux non texturés) — à vérifier.
7. Dette : V1, `army*.glb`, V2 gardés ; `mocap_trial`/`video_trial` versionnés ; atlas chargés en double.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Corriger les `figure` aberrantes (archers montés, akinci, yaya) | Fort | S | 0 | game design, historien |
| 2 | GA3 pour les 9 figurines restantes, cavalerie orientale d'abord | Fort | M | ~1,5-2 $ | tech art, animation |
| 3 | 6-8 nouvelles figurines (teutonique, serbe/byzantin, yaya, piquier flamand, almogavar, archer écossais) | Fort | M-L | 0 ou ~0,16 $/u. | historien, données |
| 4 | Figurine « seigneur » (harnois, heaume à cimier, tabard armorié, variante royale) | Fort | M | 0-0,3 $ | carte, DA1 |
| 5 | Civils élargis (femmes, enfants, moine, marchand, bourgeois, pèlerin) | Moyen-fort | M | 0 | animation, carte vivante |
| 6 | Variété GA3 (3-4 variantes, couvre-chefs, `unit_looks` sur 35 unités) | Moyen | M | ~0,1-0,2 $/var. | DA |
| 7 | QA des cavaliers GA3 en jeu | Moyen | S | 0 | QA |
| 8 | Nettoyage V1/V2/essais ; statuer sur `--coarse-figures` | Faible | S | 0 | tests |
| 9 | Distinctions régionales des fantassins | Moyen | M | 0 | historien |

Ordre : 1 et 7, puis 4 et 2, puis 3, 5, 6 ; 8 en fond.
