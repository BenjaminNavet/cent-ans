# CC C6 — échelle, normalisation, textes, équilibre (branche `cc/c6`)

## Plan
1. Échelle (ADR 0357) puis recalage points_per_step, max_muster_km, rivers.json, seuils km.
2. Renormalisation des champs additionnés à la fusion (règle écrite dans `cc-c3-donnees.md`).
3. Données : fac_navarre / title_check ; réécriture rejouable des textes (anciens noms).
4. Objectifs de victoire et scénarios (provinces disparues).
5. IA : filtre Attack vers cellule infranchissable ; arbitrage Abbeville/Meaux (c7a).
6. Lever les tests `CC C6: balance` (7 : jr4b, campaign.rs:714, cm2, om3, a6_l2, a6_forecast, rg10).
7. Seed bots rapide puis longue ; constats dans le rapport.

## État (wip 1)
Fait :
- ADR 0357 : `meters_per_px` nominal conservé ; points_per_step 250 -> 155, max_muster_km 300 -> 350, km_per_turn
  150 -> 260, rayons de repli 2/1 -> 3/2 pas, km des fleuves (script `cent-ans compose c6 rivers`).
- base_supply renormalisé (`compose c6 base-supply`, règle dans cc-c3-donnees.md § C6).
- Textes : table `TEXT_FIXES` (missions, sacre de Reims, tutoriel) ; bg_picardie -> prov_ponthieu.
  Audit (`compose c6 audit`) : ~1600 mentions d'anciens noms restent, 97 % en prose historique (maisons
  Valois/Luxembourg/Anjou/Foix, personnes, récits, codex) : volontairement non réécrites.
- fac_navarre : titres Angoulême/Mortain retirés.
- IA : `Attack` filtré par `find_path` (grid.rs).
- Tests recalés : c7a_retreat / review_tests (neutral 3 pas), campaign movement_spans (portée dérivée), starting_fit.

Fait (wip 2) : les 7 tests `CC C6: balance` sont levés, un par un :
- jr4b : le royaume aidé par le fit est cherché dans les données (Moscou : -552 -> +59) ; les Mamelouks sont en excédent (1529).
- campaign france_income : bande 20-30k -> 20-33k (main : France 28 422 ; cc 31 202, +9,8 % : population +5,5 %, bonus
  de province pleine partout ; sans bonus 28 535 contre 26 079) ; provinces 26 -> 10.
- om3 : province de test sans bonus de ravitaillement (Île-de-France a +5 : granges), pas de base_supply en cause.
- rg10 : `ai_min_buildings` 2 -> 3 (part de lieux spécialisés par l'IA 52 % -> 20 %).
- cm2_weather, a6_l2_siege_walls, a6_forecast : passent sans changement après le starting_fit regénéré.
- tw2_t5 : arrondi unique de l'allocation (+-1) ; rg10_vocations : jeu de 3 bâtiments de commerce.
- Objectifs de victoire : `compose c6 objectives` (seuils x ratio de provinces ; Ottomans +Macédoine ; Sicile 1 sur 2).

Fait (wip 3) : cargo test --workspace -j 4 : 1880 ok / 0 échec / 19 ignorés (lents, légitimes) ; pytest 2788 ok ; clippy/fmt/ruff verts ;
smoke.gd headless : « smoke OK: turn 10 » (erreurs de relief `_fail_relief` dans smoke_map : relief fin absent du worktree, domaine CD).
Seed bots : rapide 0 constat (partielle) ; longue complète (32 parties, 4488 tours, 791 s) : 8 avertissements, tous `bankruptcies_per_game`
(moyennes 7 à 48 contre bande [66,75 ; 365] de main) ; autres métriques dans leurs bandes ; 0 panique, 0 invariant, 0 ordre refusé (plus de NoPath).
Bandes NON recalées (écart attribuable à la carte, 400 lieux / 166 factions, mais non prouvé : à décider par le joueur).
