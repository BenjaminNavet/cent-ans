# FE — féodalité et petites factions (orchestration)

Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`. Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.
ADR : 0098. Worktree orchestrateur : `../game_project-fe` sur `feat/fe`.

## État (2026-09-28)
- **Vagues 1 et 2 dans `main`** (26bea252) : F0-F5 + registres F4a-F4e. 91 factions (dont `fac_rebels`),
  186 provinces. Vérif complète verte (159 binaires cargo, 980 pytest, smoke).
- F5 : IA féodale (`FeudalPolicy` installée par `ai::feudal::install`, ADR 0110), ordres `DeclareCommise`,
  `GrantTitle`, `Revolt`, `SwitchAllegiance` ; doctrine « survie d'abord » des comtés.
- Intégration vague 2 : artefacts géo régénérés une fois sur les données fusionnées ; JSON partagés
  (maisons, portraits, front-end, index horizon, titres) fusionnés à trois voies ; `campaign.rs`
  compare désormais l'état aux données (plus de compteurs figés). Écus et bannières générés pour toutes
  les factions et maisons (62 factions n'en avaient pas) ; moteur héraldique : sautoir, clef, chaudière,
  main, nef, croissant, chef, bœuf. Pavie : bordure de jeu (doublon Palatinat).
- Tests recalés sur une graine (trajectoire changée par les nouvelles factions, pas de règle en cause) :
  `cv3_ai_stances` (graine 1), `m4` régence (graine 11), `c7_retinue` (Bohun au lieu de Lancastre),
  `g4` (allié ancre pris aux Pays-Bas).
- Tous les worktrees FE d'agents supprimés. `../game_project-fe` (`feat/fe`) = worktree d'intégration.

## Reprise
- **Vagues 1 à 3 dans `main`** (f8c126d5) : F0-F8. F6 interface, F7 portraits (5,13 $), F8 équilibre
  (ADR 0114 : ost effectif, pas d'alliance suzerain-vassal, commise sur coalitions, félonie de 1337 ;
  banqueroutes divisées par deux ; rapport `docs/wip/fe8-equilibre.md`).
- **Reste : partie pilote du joueur.**
- Points ouverts à arbitrer :
  - part de guerre France-Angleterre (ADR 0085) dans la bande 55-75 % sur 6/10 graines seulement
    (moyenne 66 %, trajectoires très sensibles) ;
  - une ancienne petite faction finit à 11-29 provinces par partie (« aucune faction mineure n'explose »
    non tenu) ;
  - royaumes irlandais, Îles, Luna, Urbino : 50-80 livres de revenu, pas une unité de garnison payable
    (données économiques F4) ;
  - carte de choix de faction petite dans son cadre (F6) ;
  - difficultés des factions ajoutées, incertitudes sourcées (Grelle, Nicolas de Brno, Dietrich IX,
    blasons gaéliques et italiens), `virneburg` sans meuble, Bretagne/Flandre/Navarre non jouables.

## Entrées F8 (signalées par la session RS, `docs/wip/rs-m-c7a.md`, ADR 0113)
- **Ost impérial sans distance** : dès la vague 1, vers le tour 8 sur chaque graine, l'Empire s'allie au
  Hainaut (allié de l'Angleterre), déclare la guerre à la France (« défense d'un allié ») et `summon_host`
  convoque ses vassaux déduits, dont Milan, Savoie, Gênes, Vérone (`de_jure_liege: tit_empire`), qui prennent
  Lyonnais/Auvergne/Rouergue/Nîmois. Pas de notion de distance ni d'indépendance de fait ; l'Empire s'allie
  même à son propre vassal. Recettes France −92 000 sur 50 tours. Pistes : ost limité aux vassaux proches
  ou « effectifs » (seuil de loyauté/indépendance de fait pour l'Italie impériale), pas d'alliance avec son
  propre vassal. RS a recalibré c7a (plancher trésor France 40 000 → 15 000) : le remonter si l'ost est restreint.
- **Banqueroutes** : `century_probe` donne 1,09 banqueroute par faction et par décennie contre 0,05 avant FE
  (probablement les petites factions ; colonne à vérifier identique entre sondes).

## Points ouverts (à trancher en F5/F8)
- `ai/tests/g4.rs` (Brabant allié de l'Angleterre) en `#[ignore]` : un vassal peut-il s'allier hors de son suzerain ? (F5)
- Test 50 tours `m3_grid_ai` (`--ignored`) : rouge déjà sur main ; plus tôt avec les princes d'Empire vassaux (F8).
- Interprétations F3 à valider : conquête (§ 4.6), « plusieurs prétendants », lois de succession par titre (données à écrire).
- F6 : offres Protection/Arbitrage, ordre `ArbitratePrivateWar`, article `demand_title`, factions créées sans données d'affichage.
- F4a : Normandie en deux titres, héritiers manquants, couleurs héraldiques en doublon.
