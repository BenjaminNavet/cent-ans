# 0291 — Cinq colonies au plus par province

Statut : accepté

## Contexte
La carte comptait 2 148 colonies (443 cités, 705 villes, 419 villages, 338 châteaux, 243 abbayes), jusqu'à 14 dans
une province (Bourgogne : 13). Trop de places à tenir, à lire et à équilibrer, et plus de villes que de villages, ce
qui ne ressemble pas à la France de 1337. Demande du joueur (2026-10-10, chantier CO, `docs/wip/co-colonies.md`) :
au plus 5 colonies par province, plus de villages que de villes, pas de nouveau type « bourg ».

## Décision
- Outil reproductible `cent-ans geo settlement-cap` (`tools/cent_ans_tools/settlement_cap.py`, réglages dans
  `data/map/settlement_cap_rules.json`, schéma `settlement_cap_rules.schema.json`). Règles, dans l'ordre :
  1. la cité est toujours gardée ;
  2. toute colonie citée par une donnée de jeu qui la désigne (ports de flottes et noms de navires, routes
     maritimes, arêtes maritimes forcées, hubs de commerce, base de croisade, places inrasables, monuments
     `data/landmarks*`, zones de détail, chemins de monuments, modèles d'eau) est gardée d'office ;
  3. les autres places sont classées par score = poids + 2 (château, abbaye) + 6 (port) + 4 (zone maritime) + 1 par
     niveau de fortification + 4 (citée par un test) ; on garde d'abord un village (province de 3 colonies ou plus),
     puis un château ou une abbaye de poids ≥ 8, puis les mieux classées jusqu'à 5 ;
  4. parmi les villes gardées, les plus mineures (poids < 25, sans port, non protégées) deviennent des villages
     jusqu'à ce que villages ≥ 2 × villes (hors cités) : fortification ramenée à 1 au plus, bâtiments que
     `settlement_kinds` n'autorise pas à un village retirés.
- Les colonies retirées sont écrites dans `data/map/former_settlements.json` (schéma `former_settlements.schema.json`) ;
  `cent-ans geo hamlets` (ou `--merge-former`, qui ne touche pas à la sélection GeoNames) les ajoute à la fin de
  `data/map/hamlets.json`, au même format que les hameaux décoratifs : le rendu existant (`settlement_layer.gd`,
  décor de `sim-battle`) les affiche sans code nouveau. Leurs ancrages fins sont déplacés de `settlements` à `hamlets`
  dans `fine_anchors.json` (même ordre que `hamlets.json`).
- Dérivés régénérés ou élagués : `settlement_graph.json`, `settlements_px.json`, `settlement_edge_paths.json`
  (`cent-ans geo settlements`), `towns_1340.json`, `town_footprint.json`, `fine_anchors.json`,
  `data/rules/starting_fit.json` (`CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p sim-campaign starting_fit`).
- `building_slot_cap` (`data/settlements/rules.json`) : cité 6, ville 4, château 3, abbaye 3, village 2 (avant
  10/5/4/4/3, ADR 0275).

## Conséquences
- Comptes : 2 148 → 1 656 colonies (cités 443, villes 705 → 269, villages 419 → 539, châteaux 338 → 289, abbayes
  243 → 116) ; 250 provinces à 3 colonies, 59 à 4, 134 à 5 ; 492 hameaux ajoutés, 281 villes rétrogradées.
- Mesures `campaign_probe` (120 tours, 6 graines, avant → après) : guerre FR-EN 64,8 % → 66,5 % en moyenne (bande
  55-75 %), révoltes 2,5 → 3,0 par partie (sous la bande 4-10, point ouvert connu), banqueroutes 109 → 117, revenu de la
  France à 120 tours 33,3 k → 28,5 k (plage WH 22-42 k). Aucun réglage de `rules.json` ajusté hormis `building_slot_cap`.
  Détail dans `docs/wip/co-colonies.md` § CO-A.
- Aucune règle n'est codée : tout se règle dans `settlement_cap_rules.json`. Relancer l'outil sur des données déjà
  plafonnées ne change rien. Pour repartir de zéro : restaurer `data/settlements/` depuis git.
- Un `geo hamlets` complet (sans `--merge-former`) rejoue la sélection GeoNames, qui ne redonne pas le fichier validé
  (il avait été généré avec d'autres positions) : il décale l'ordre des hameaux, donc les ancrages fins ; il faut alors
  relancer `geo anchors-fine`.
- Les villages retirés ne sont plus jouables ni sélectionnables ; ce sont des hameaux décoratifs.
