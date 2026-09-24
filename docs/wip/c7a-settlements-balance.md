# C7a — repli du perdant, IA sur les colonies, équilibrage 50 tours

Spec : `docs/design/2026-09-24-echelle-colonies.md` (§ 4.3-4.5, § 7), suite de C4
(`docs/wip/c4-core-settlements.md`). Branche : `worktree-agent-a2e37f1c8b64c0bf2` (depuis `main` cda86a9).
Périmètre : `core/` et `data/` uniquement.

## État : terminé (en attente de fusion ; `main` avec C5 déjà fusionné dans la branche)

- [x] Repli du perdant : règle C7a (`movement::retreat_target`), réglages `data/settlements/rules.json`
  § `retreat` (+ schéma), tests `sim-campaign/tests/c7a_retreat.rs`.
- [x] Sondes `core/crates/ai/examples/` : `settlements_probe` (50 tours × 8 graines, 3-6 s par graine en
  release, trop lent pour `cargo test` en debug), `reach_probe` (portée d'une saison), `econ_probe`
  (entretien par type de colonie).
- [x] Ordre `Order::GarrisonUnits { army, unit_indices }` (laisser des unités en garnison), plafonné par
  `rules.json` `garrison_cap` ; erreurs `SettlementBesieged`, `GarrisonFull`.
- [x] IA (`crates/ai`) : reprise des colonies perdues (`RECLAIM_TARGET_BONUS` = 35), une unité laissée en
  garnison des places prises ou frontalières menacées, retour au pays des armées oisives hors de leurs
  places, abandon des sièges de forteresse (niv. ≥ 4) sans issue après 12 tours, garnisons comptées
  comme coût fixe dans le budget militaire. `ai_minimal` : reprise et retour au pays.
- [x] Portée d'une saison réduite (demande C5), équilibrage de l'entretien (bâtiments hors cité,
  garnison de départ des villes), mesures ci-dessous.
- [x] fmt, clippy, `cargo test`, pytest (sauf l'échec connu des portraits), `build.sh`, smoke Godot et
  `c5_settlements_ui_test.gd` verts.

## Audit de l'IA (constats avant correction)

- Les deux IA prennent déjà villes, abbayes et villages (C4) ; les châteaux (2 bonnes unités, fort. 3)
  sont rarement assiégés (2-10 sièges par graine) : l'IA exige 1,5 × la défense. Gardé : c'est historique.
- Aucune préférence pour ses propres places perdues : la France perdait 10 colonies en moyenne en 50 tours
  (jusqu'à 40 dans la graine 2, au profit de Gênes).
- Armées « bloquées » (2,6 par tour) : surtout des armées oisives sur des places neutres après une paix
  (Florence à Lucques, Venise à Trévise pendant 50 tours) ; aucune n'était bloquée devant une forteresse
  de niveau 4 (0,2 par tour à proximité, aucun siège de niveau 4 de plus de 8 tours).
- Les places prises restaient vides (pas d'ordre pour laisser une garnison ; le recrutement exige d'être
  propriétaire) : les abbayes changeaient de main 50 fois par graine.
- Les garnisons (≈ 160-200 unités par couronne depuis C4) entraient dans la part militaire du revenu :
  l'Angleterre n'avait plus d'armée de campagne après 10 tours (0 unité contre 22 avant C4).

## Règle de repli (décision)

Ordre, déterministe (égalités départagées par l'id de colonie) :
1. colonie amie (à soi ou à un allié) sans armée ennemie, la plus proche sur le graphe, dans un rayon de
   `friendly_radius_steps` = 2 pas, par un chemin qui ne traverse aucune place ennemie ;
2. sinon colonie non tenue par un ennemi (neutre) sans armée ennemie dans un rayon de
   `neutral_radius_steps` = 1 pas ; les traînards coûtent `neutral_loss_percent` = 10 % ;
3. sinon **débandade** : `rout_loss_percent` = 50 % de pertes ; les survivants rejoignent la place amie la plus
   proche à toute distance (chemin sans place ennemie) si l'armée garde au moins
   `rout_dissolve_below_percent` = 30 % de ses effectifs, sinon elle se disperse (le général s'échappe).

Un attaquant battu revient toujours d'où il venait, sauf si une armée ennemie y est entre-temps.
Justification : une armée battue loin de ses places et cernée se dispersait (fuite de l'ost de Philippe VI
après Crécy, débandade après Poitiers, compagnies dispersées) ; une armée proche de ses places s'y
réfugiait ; le passage en terre neutre (Empire, Bretagne neutre) était courant mais coûtait des traînards.

## Portée d'une saison (demande C5 / orchestrateur)

Sonde : `cargo run --release -p ai --example reach_probe` (armée principale, départ 1337 ; les places
ennemies arrêtent la marche). Réglages `movement.season_scale` = 0,5 et `movement.road_cost_factor` = 0,75
(les arêtes routières du graphe C3, cuites à 0,5, sont remises à 0,75 au chargement).

| départ | saison | avant : points / colonies / provinces / étapes méd.-max | après |
|---|---|---|---|
| Paris (France) | été | 420 / 233 / 57 / 9-16 (Villeneuve-sur-Lot, 14 étapes) | 210 / 62 / 16 / 5-8 (Vaucouleurs) |
| Paris | hiver | 280 / 162 / 37 / 7-14 | 140 / 24 / 4 / 3-5 |
| Londres (Angl.) | été | 420 / 65 / 15 / 5-10 | 210 / 34 / 7 / 3-6 |
| Bordeaux (Angl.) | été | 420 / 48 / 14 / 3-6 (Lleida) | 210 / 34 / 9 / 3-5 (Lusignan) |
| Bordeaux, sans ennemis | été | 420 / 219 / 50 / 8-18 (Hesdin) | 210 / 48 / 11 / 3-5 |

Une saison couvre ~1,5 pas de province (210 km de plaine), ~2 sur route. `PLANNING_RANGE` de l'IA passe de
8 à 5 pas et `OFFENSIVE_RANGE` d'`ai_minimal` de 6 à 4 (objectifs à 3 saisons au plus).

## Paramètres changés (données)

| Paramètre (`data/settlements/rules.json`) | Avant | Après | Justification |
|---|---|---|---|
| `retreat.*` | — | 2 pas / 1 pas / 10 % / 50 % / 30 % | règle de repli ci-dessus |
| `garrison_cap` | — | cité 8, château 4, ville 3, abbaye 2, village 1 | empêche de loger une armée dans une petite place à entretien réduit |
| `building_upkeep_percent` | 100 partout (implicite) | cité 100, ville 40, château 50, abbaye 25, village 50 | murage des villes, châtellenie, temporel des abbayes ; l'entretien des bâtiments avait doublé avec C4 (Angleterre 2 860 → 6 690) |
| `starting_garrison.town` | milice + arbalétriers | milice | guet bourgeois ; garnisons anglaises 68 unités avant C4, 199 après, 161 maintenant |
| `movement.season_scale` | (1) | 0,5 | portée d'une saison, demande C5 |
| `movement.road_cost_factor` | (0,5 cuit dans le graphe) | 0,75 | idem |

Code : `PLANNING_RANGE` 8 → 5, `OFFENSIVE_RANGE` 6 → 4 (portée réduite), `RECLAIM_TARGET_BONUS` 35
(au-dessus des 30 d'une revendication de trône), `SIEGE_PATIENCE_TURNS` 12, `GARRISON_MIN_ARMY_UNITS` 3,
`RECLAIM_PREFERENCE_STEPS` 1,5 (`ai_minimal`). `TAX_EFFICIENCY` inchangé (0,082) : le revenu est au
niveau d'avant C4 (France 27 713 contre 28 558) ; c'est la dépense qui avait dérivé.

## Mesures (50 tours, graines 1-8, IA contre IA, France « joueur » pilotée par l'IA)

Sonde : `cargo run --release -p ai --example settlements_probe -- 50 1 2 3 4 5 6 7 8`. « Avant C4 » :
sonde réduite sur `main` 1293377 (provinces, pas de colonies). « Avant C7a » : `main` cda86a9.

| Indicateur | Avant C4 | Avant C7a | Après C7a |
|---|---|---|---|
| Trésor final France (moy. [min ; max]) | 67 956 | 103 303 [61 090 ; 133 236] | 80 338 [43 656 ; 109 043] |
| Trésor final Angleterre | 35 181 | 9 878 [45 ; 24 684] | 17 385 [2 209 ; 36 031] |
| Angleterre : trésor négatif à un moment | 0/8 | 4/8 | 1/8 |
| Banqueroutes France / Angleterre | 0 / 0 | 0 / 0 | 0 / 0 |
| Armée de campagne au tour 20 (graine 1, unités) FR / EN ; colonne du milieu : IA C7a sans l’équilibrage | 80 / 29 | 12 / 0 | 62 / 15 |
| Δ provinces France / Angleterre | +1,4 / −1,6 | −1,4 / −2,2 | +0,5 / +0,0 |
| Δ colonies France / Angleterre | — | −10,0 / −8,0 | +1,9 / +0,8 |
| Sièges en cours par tour : cités / autres places | 0,8 (provinces) | 1,9 / 2,9 | 1,0 / 3,5 |
| Prises par graine : cités / places secondaires | 13,6 (provinces) | 22,2 / 110,6 | 10,2 / 69,5 |
| Armées bloquées par tour (dont devant niv. 4) | — | 2,6 (0,2) | 0,2 (0,1) |
| Sièges de niv. 4 de plus de 8 tours / graine | — | 0 | 0 |
| Places frontalières secondaires sans garnison FR / EN | — | 75/543 / 9/497 | 24/539 / 28/541 |
| Débandades / graine (dispersées) | — | 0 | 0,9 (0,9) |
| Factions disparues | 0 | 0 | 0 |

Lecture : France et Angleterre solvables sur 50 tours, sans banqueroute ; ni effondrement ni conquête
éclair (± 2 provinces) ; les sièges de places secondaires sont 3,5 fois plus fréquents que ceux des cités.
L'Écosse reste fragile (trésor négatif 6/8, 1 banqueroute ; 23 avant) et perd des places dans 2 graines
sur 8 (graines 3 et 7).

### Siècle (century_probe, 464 tours, 8 graines)

| | Guerre FR-EN | en bande 55-75 % | Banq. / fac. / déc. | 4 majeures en 1400 | Prises / graine |
|---|---|---|---|---|---|
| avant C7a | 64 % [55-77] | 6/8 | 0,72 | 8/8 | ~1 360 |
| après C7a | 55 % [34-75] | 4/8 | 0,37 | 8/8 | ~520 |

La guerre franco-anglaise est un peu moins présente (marches plus lentes, moins de prises) : à surveiller
en C7b avec les réglages diplomatiques G5.

## Points ouverts

- Guerre FR-EN sur un siècle : 55 % (bande cible 55-75 % tenue par 4 graines sur 8 au lieu de 6).
- France : le trésor reste haut (80 000, 2,3 saisons de revenu brut) ; l'Angleterre frôle zéro vers le
  tour 35 dans la moitié des graines avant de remonter.
- Écosse : trésor presque toujours négatif (problème G2 antérieur, amélioré).
- Châteaux rarement assiégés (garnison de 2 bonnes unités) ; pas corrigé, historique.
- L'ordre `garrison_units` n'a pas encore de bouton dans l'UI (C5) ; le pont l'accepte via la
  désérialisation générique des ordres.
- La portée réduite rend l'aperçu C5 plus court : `c5_settlements_ui_test.gd` passe sans changement (cible
  la plus lointaine atteignable : Saint-Mihiel depuis Paris, 7 étapes).

## Prochaine étape

Fusion par l'orchestrateur.
