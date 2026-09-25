# EQ1 — équilibre (trouble, Angleterre, banqueroutes, couleuvriniers)

Branche : `worktree-agent-a6c791c0cae04ee1a` (C4 + C5 + DP1 + main fusionnés). ADR :
`docs/decisions/0029-revoltes-et-couts-d-evenements.md`.

## État : terminé, main fusionné (cb553f64) ; fmt, clippy, 611 tests, build.sh, import, smoke (26 OK), pytest 419 verts. Prochaine étape : fusion par l’orchestrateur.

## Avant / après

Même machine, build release. « Avant » = C5R + DP1 + main (74ffeb1a), « après » = EQ1.

`balance_probe campaign 200 1-8` :

| Mesure | Avant | Après | Cible |
|---|---|---|---|
| Mécontentement moyen final | 9,2 | **17,9** | 15-35 |
| Mécontentement moyen (tous les 10 tours) | 10,7 | **19,8** | 15-35 |
| Révoltes / partie (hors passage aux rebelles) | 13,9 | **4,4** | 4-10 |
| Trouble 4 tours / 1 tour avant la révolte | 71 / 81 | 72 / 83 | élevé et visible |
| Révoltes « soudaines » (< 50 quatre tours avant) | 7 % | 0 % | — |
| Pire province (révoltes / partie) | Biscaye 12, Savoie 10 | ≤ 7 (Luxembourg) | — |
| Banqueroutes / faction / décennie | 1,21 | **0,25** | < 0,5 |
| Impôt « Haut » (échantillons) | 48 % | 45 % | < 40 % (E2) |
| Guerre FR-EN (200 tours) | 60 % | 54 % | 55-75 % sur le siècle |
| Milice / recrutements | 32,0 % | 31,1 % | < 40 % |
| Archers longs / recr. anglais | 60,1 % | 59,0 % | ≥ 25 % |
| Types recrutés (min) | 15 | 15 | ≥ 6 |
| Couleuvriniers (ordres / partie) | 1,2 | 7,5 (it. précédente 7,0) | — |
| Commerce / revenu total | — | 0,3 % | — (piste C5) |
| Édits (provinces) | Paix de Dieu 89 %, aucun 11 % | Paix de Dieu 72 %, Franchises 17 %, aucun 11 % | — (piste C4) |

`century_probe 464 1-5` :

| Mesure | Avant | Après | Cible |
|---|---|---|---|
| Guerre FR-EN moy. | 57 % [53-61], 3/5 | 58 % [41-69], 3/5 | 55-75 % |
| 4 majeures vivantes en 1400 | 5/5 | 5/5 | 5/5 |
| Banqueroutes / fac. / déc. | 0,98 [0,43-2,27] | **0,28 [0,21-0,35]** | < 0,5 |
| Couleuvriniers recrutés | 1/5 | **5/5** | ≥ 1 sur 2 |
| Ordonnance / francs-archers / coutiliers | 5/5, 4/5, 4/5 | 5/5, 5/5, 4/5 | — |

Angleterre, joueur inactif (`start_economy_probe 8 fac_england`) :

| Tour | Avant : solde / trésor | Après : solde / trésor |
|---|---|---|
| 0 | −940 / 45 000 | **+1 139 / 45 000** |
| 2 | −1 474 / 42 779 | +481 / 46 854 |
| 4 | −1 862 / 39 511 | +25 / 47 450 |
| 6 | −2 225 / 33 543 | −413 / 45 205 |

Matrice à budget égal (`balance_probe matrix`) : couleuvriniers 30 % de victoires en moyenne
(cible E8 : 20-80 %), stats inchangées : ils ne dominent pas.

## Diagnostic et correctifs
1. **Révoltes** : une province au-delà de 75 deux saisons se révoltait ensuite **à chaque
   saison** (compteur jamais remis à zéro). Correctif (données `data/rules/population.json`) :
   3 saisons, remise à zéro après chaque révolte, pas de révolte contre les rebelles ; la jauge
   de troubles de la province (prises, pillages, régence) entre dans la cible (poids 0,5, plafond
   20) ; impôt 100 → 130, occupation 35 → 20. IA : pas d'impôt Haut si une province est à moins
   de 10 points du seuil (plafond de mécontentement moyen pour Haut 18 → 30).
   **Visibilité** : le panneau de province affichait `province.unrest` (troubles), pas le
   mécontentement des classes qui déclenche la révolte ; il affiche désormais ce dernier et
   « révolte dans N saisons » (pont : clés `unrest`, `disorder`, `revolt_seasons`,
   `revolt_threshold`, `revolt_seasons_needed`).
2. **Angleterre au tour 1** : l'embargo sur la laine était compté deux fois (la Flandre
   « imposait » aussi un embargo à l'Angleterre dans `fac_flanders.json`) : −11 % de revenu
   (≈ −1 900 ₶). La relation flamande devient une paix. L'Angleterre démarre avec
   `tech_royal_taxation` (administration fiscale efficace, cf. sa description ; +10 % d'impôt,
   +3 de mécontentement bourgeois). Garnisons (48 % de l'impôt) et Guyenne (1 406 ₶, 3ᵉ province)
   inchangées. Le solde baisse ensuite de lui-même (la richesse de départ converge vers sa cible,
   −12 % de revenu en 6 tours pour toutes les factions) : l'Angleterre repasse à −400 au tour 6
   avec 45 000 ₶ en caisse, sans banqueroute ni licenciement.
3. **Banqueroutes** : les rebelles comptaient (323 saisons en graine 1) → hors de l'économie ;
   les mineures (Suisses, Grenade, Gueldre, Suède) passaient sous 0 par des événements à coût
   fixe (−1 000 à −2 500 ₶ pour 400-600 ₶ de revenu) → `data/rules/economy.json` : effet de trésor
   proportionnel au revenu sous 4 000 ₶ / saison, au moins 25 % du montant (infobulle et choix de
   l'IA sur le montant réel).
4. **Couleuvriniers** : exigeaient `bld_armoury`, jamais construit (il améliore
   `bld_muster_field`, jamais construit non plus). Désormais levés à la **maison des métiers**
   (`bld_guild_hall`, milices urbaines ; historiquement couleuvrines des villes), toujours
   `tech_handgonnes` et dès 1380.

## Pistes TW (orchestrateur C4/C5)
- (1) **Fait** : une armée ennemie en rase campagne à moins de `zoc_radius_km` (8 km) d'une
  colonie de la route la menace (`trade.rs::path_security`, `armies_near`).
- (2) Mesuré : le commerce pèse **0,3 %** du revenu total (50-220 ₶ par route) : sans lien avec
  les banqueroutes. Montants non changés (à relever si l'on veut que le commerce compte).
- (3) Mesuré : l'IA ne choisit **jamais** l'Aide féodale ; son score statique donnait la Paix de
  Dieu partout (89 %). Score rendu contextuel (`edicts.rs::ai_choose_edicts` : apaisement pondéré
  par le mécontentement local, impôt et levées doublés en guerre ou en déficit) → Paix de Dieu
  72 %, Franchises 17 %, aucun 11 %. L'Aide féodale reste à 0 % (−15 de mécontentement pour
  +20 % d'impôt : mauvais marché à ces niveaux). Paliers empilés des colonies de départ : non
  traité.

## Points ouverts
- Impôt « Haut » à 45 % des échantillons (cible E2 < 40 %).
- Graine 5 du siècle à 41 % de guerre FR-EN (3/5 graines dans la bande, comme avant).
- La jauge `ProvinceState::unrest` reste à 100 dans les provinces prises et reprises sans cesse
  (Lothian) ; son effet est plafonné, pas sa cause.
- ADR numéroté 0029 (0028 réservé à NV1) : vérifier l'absence de collision à la fusion.

## Outils
- `balance_probe campaign` : lignes EQ1 (trouble moyen dans le temps, révoltes et leur contexte,
  banqueroutes / faction / décennie, part du commerce, édits) ; détail JSON par révolte
  (province, faction, impôt, dévastation, troubles, garnison).
- `start_economy_probe [tours] [factions…]` : économie d'ouverture d'un joueur inactif ;
  `AI=<graine> STEP=<n>` : faction jouée par l'IA, décomposition des passages sous 0.
- Tests : `sim-campaign/tests/eq1_balance.rs` (5), `data-model/tests/real_data.rs`
  (`economy_rules_match_their_default`), `tools/tests/test_economy_rules_schema.py`.
