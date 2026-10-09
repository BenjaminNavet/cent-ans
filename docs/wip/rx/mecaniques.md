# RX — rapport `mecaniques` (économie, féodalité, victoire, rythme)

Méthode : parties IA contre IA avec une sonde jetable écrite hors dépôt (crate Cargo dans le scratchpad, dépend de `ai`, `sim-campaign`, `data-model` par chemin, cible `release` propre). Les sondes `century_probe` / `balance_probe` / `ai_duel_probe` ont été supprimées par `c2308153d` (SC) : il n'existe plus d'outil de mesure de campagne dans le dépôt (seuls `playthrough`, `pb1_core`, `turn_*` restent). Mesures : France pilotée par l'IA (ordres du joueur = `ai::plan_turn`), graines 1-6, 464 tours (1337-1453), difficulté normale, 177 factions de départ. Tout chiffre ci-dessous vient de ces runs sauf mention.

## 1. Verdict
- Forces : la simulation tient 464 tours × 177 factions sans plantage ; peu de banqueroutes (0 à 4 trésors négatifs par instantané sur 110-160 factions vivantes) ; guerre FR-EN présente (61-82 %) ; 28 types d'effets de bâtiments/techs, tous réellement lus par le cœur (aucune donnée morte de ce côté).
- Faiblesse majeure 1 : la **victoire est atteignable en 5 à 11 ans de jeu**, sans effort, pour au moins 13 factions jouables et, avec l'IA aux commandes, pour la France dans 3 graines sur 6 (tours 45-49, soit 1348). La campagne de 116 ans n'a donc pas de rythme garanti.
- Faiblesse majeure 2 : plus aucun outil de mesure de campagne dans le dépôt, alors que les ADR 0085/0100/0148 et les wip (`eq6`, `fe8`, `ia-nuit`) s'y réfèrent.
- Faiblesses moyennes : trésors thésaurisés sans débouché en fin de partie, arbre technologique court (45 techs) saturé par les petites factions, ordre public quasi invisible à l'échelle de la carte, élimination de 35-38 % des factions.

## 2. Constats

### [bloquant] [conception/bug] Victoire « premier vassal » acquise en 20 saisons sans rien faire
**Constat** : douze factions jouables (Athènes, Holstein-Kiel, Îles, Lara, Ordre livonien, Moscou, Raguse, Ribagorce, Samtskhé, Slavonie, Tripoli, Naples ; Bohême et Naples selon la graine) remportent « Victoire ! X s'est imposé comme premier vassal du royaume » exactement au tour 20 (printemps 1342) : elles partent déjà premier vassal de leur couronne et la série `first_vassal` atteint `ascension_turns` = 20. Le joueur d'une de ces factions voit l'écran de victoire après 5 ans, sans avoir joué. Sur 150 factions jouables, 1 autre (Vidin) gagne par « indépendance » à ~tour 10 et Varsovie par objectifs de titre avant le tour 40.
**Preuve** : `core/crates/sim-campaign/src/feudal/objectives.rs:116-146` (`generic_victory`, `streaks.first_vassal >= ascension_turns`), `data/rules/feudal.json` (`ascension_turns: 20`, `independence_turns: 20`), `victory.rs` (branche `else if let Some(path) = feudal` sans durée minimale ni condition de puissance absolue). Sondes : 13 factions jouables en victoire générique à t40 ; parties conduites par l'IA en tant que joueur : `fac_athens`, `fac_moscow`, `fac_naples` → `OUTCOME t20 Victory`. Sortie `vic` : « first_vassal>=20 at t30: 13 ».
**Correction** : (a) `first_vassal` ne compte qu'à partir d'une date d'ouverture (ex. 1380) ou exige en plus d'avoir gagné au moins une province/une guerre depuis le départ ; (b) augmenter `ascension_turns` à 60-80 et exiger un seuil de puissance relative au suzerain (`faction_power ≥ 0,5 × suzerain`) ; (c) ne pas afficher l'écran de victoire avant l'`end_year` pour ces voies, seulement un jalon. Fichiers : `feudal/objectives.rs`, `data/rules/feudal.json`, test dans `tests/feudal`.
**Coût** : S (paramètres) à M (condition de puissance + test).

### [majeur] [équilibrage/conception] France : la moitié des objectifs sont remplis au tour 0, victoire en 11 ans
**Constat** : `obj_fr_crown` (2/2 provinces) et `obj_fr_burgundy` (« soumission obtenue », la Bourgogne étant vassale au départ) sont vrais au tour 0 ; restent `obj_fr_expel` (3 provinces anglaises) et `obj_fr_guyenne` (2 provinces). L'IA pilotant la France les remplit vers le tour 26-30, tient 20 saisons (`hold_turns`) et gagne aux tours 46 (graine 1), 45, 49, 45 (graines 1, 3, 5 sur 6 avec la sonde `mprobe`). Victoire historique en 1348 : la guerre de Cent Ans est « gagnée » avant Crécy.
**Preuve** : sortie `obj fac_france 1` : `t0 obj_fr_crown=true obj_fr_burgundy=true ... t30 streak=4 ... OUTCOME t46 Victory`. `data/factions/fac_france.json:137` (`hold_turns: 20`, `end_year: 1453`).
**Correction** : objectifs de la France dépendants de dates (expulsion d'Angleterre de tous les ports + Calais, Guyenne entière hors Bordeaux anglais, comme en 1453) ; `obj_fr_burgundy` doit demander « Bourgogne fidèle (loyauté > X) » ou être retiré (déjà acquis) ; `hold_turns` ≥ 40 pour que la victoire exige la durée. Même revue pour l'Angleterre (aucun objectif acquis au départ, OK) et la Bourgogne (OK : 0/3). Fichiers : `data/factions/fac_france.json`.
**Coût** : S.

### [majeur] [refonte/outillage] Aucune sonde de campagne dans le dépôt
**Constat** : `century_probe`, `balance_probe`, `ia_quality_probe`, `ai_duel_probe` (ADR 0148) ont été supprimés (`c2308153d`, SC) ; ADR 0085, 0100, 0113, `docs/wip/eq6-guerre-toutes-difficultes.md`, `fe8-equilibre.md`, `docs/status.md` y renvoient encore. Les cibles d'équilibrage (guerre FR-EN 55-75 %, révoltes 4-10, banqueroutes < 0,5) ne sont donc plus vérifiables par une commande.
**Preuve** : `git show c2308153d --stat` ; `ls core/crates/ai/examples` ne contient plus que `playthrough`, `pb1_core`, `turn_*`.
**Correction** : conserver une seule sonde (≈ 100 lignes : parties N graines × T tours, compte d'événements par `EventKind`, guerre FR-EN, factions vivantes, revenus/entretien/trésor du top 5, séries de victoire). La mienne tient en `main.rs` (scratchpad `…/probe/src/main.rs`) et se compile en ≈ 1 min ; l'ajouter à `core/crates/ai/examples/balance_probe.rs` et mettre à jour les renvois des ADR/wip.
**Coût** : S.

### [majeur] [équilibrage] Trésors thésaurisés, aucun débouché des grosses fortunes
**Constat** : la France gagnante finit à 675 444 livres (3 revenus de saison) en 1453, avec 218 730 de revenu ; le trésor a été multiplié par 9 de 1362 à 1453 sans que l'argent serve (graine 1 : 75 970 → 127 526 → 204 007 → 422 278 → 675 444). Même constat pour de petites puissances : Arborea 88 878 livres (revenu 43 181), Castille 72 965, Autriche 76 089, Chypre 192 741. Les dépenses possibles sont bornées (30 bâtiments, 45 technologies, recrutement plafonné à 3 levées/tour ; l'impôt d'opulence à 20 % au-delà de 6 saisons de revenu ne suffit pas). Pour un joueur : après ~1400 plus rien à acheter.
**Preuve** : lignes `t100…t464` des sorties 1, 3, 6 ; `data/rules/economy.json` (`opulence_seasons: 6`, `opulence_percent: 20`, `administration_max: 0.28`).
**Correction** : puits d'argent en fin de partie (prestige : fondations, universités par province, mécénat, rachat de rançons, corruption d'agents ; coût de ré-entretien indexé sur la taille de l'armée ; nouvelles chaînes de bâtiments tier 4-5) et opulence progressive (20 % puis 35 % au-delà de 12 saisons). Données : `data/buildings/`, `data/rules/economy.json`.
**Coût** : M.

### [majeur] [équilibrage] Fin de campagne trop vidée : 35-38 % des factions disparaissent
**Constat** : 177 factions au départ, 109-118 vivantes en 1453 (`FactionDestroyed` 59-68 par partie). Des petites puissances deviennent des empires improbables : Chypre 36 provinces (1437, graine 6), Galicie-Volhynie 24, Beloozero 18, Arborea 15 avec 88 k de trésor. La France finit à 17-36 provinces selon la graine, l'Angleterre à 22-37 : la partie est bimodale (France 32/30/28 provinces dans les graines 1, 3, 6 ; 19/17/21 dans 2, 4, 5 avec l'Angleterre devant).
**Preuve** : ligne `t464` de chaque sortie ; `FactionDestroyed=59/61/62/59/66/68`.
**Correction** : faire coûter l'annexion totale (prestige, ligue anti-hégémon, loyauté des vassaux) ; plafonner le gain de provinces par traité (`war_score`) ; vérifier le facteur « 3 guerres déclarées par tour » (cf. ci-dessous) qui alimente la consommation. Fichiers : `data/ai/diplomacy.json`, `sim-campaign/src/negotiation`.
**Coût** : M.

### [majeur] [équilibrage] Guerre permanente : une faction sur deux est en guerre en tout temps
**Constat** : 1 092 à 1 603 déclarations de guerre et 1 053 à 1 576 paix par partie de 464 tours (≈ 3 par tour) ; 34 à 64 factions sur ~120 vivantes sont en guerre à chaque instantané (jusqu'à 64/129). L'alternance guerre/paix est si dense que la paix n'est jamais un état stable ; `Attrition` 685-1 289 événements. Des alliances se forment et se rompent en continu (341-487 formées, 64-148 rompues).
**Preuve** : colonnes `at_war=` et `events:` des sorties (ex. graine 4 : `WarDeclared=1603 PeaceSigned=1576`).
**Correction** : allonger les trêves et durcir la déclaration (légitimité : revendication, casus belli datés, `war_weariness` post-paix), mesurer le nombre de guerres actives par faction. Fichiers : `data/ai/diplomacy.json` (`war.*`, trêves), `negotiation.rs`.
**Coût** : M.

### [majeur] [équilibrage] Guerre FR-EN hors bande (55-75 %) sur 2 graines sur 6
**Constat** : 77 %, 73 %, 82 %, 73 %, 70 %, 61 % (sondes `mprobe`, normale) ; les graines 1 et 3 dépassent 75 %. EQ6 annonçait 10/10 en bande avant les cartes OM/FE ; ADR 0100 / `fe8-equilibre.md` donnent 56-72 %. Écart faible mais pas de mesure récente reproductible (cf. constat sonde).
**Preuve** : en-têtes `== seed N … FR-EN war X %` des 6 sorties.
**Correction** : après restauration de la sonde, 10 graines × 464 tours ; si l'écart se confirme, `claim_war_ignores_kinship` (déjà en place) ou la fatigue de guerre (`data/ai/diplomacy.json`).
**Coût** : S (mesure) / M (réglage).

### [mineur] [équilibrage] Arbre technologique court : saturé par les petites factions
**Constat** : 45 technologies (16 militaires, 15 civiles, 14 médecine, coûts 100 à 1 350) ; 3 618-3 766 `TechnologyResearched` par partie, soit ≈ 8 par tour ; 4 259 techs possédées pour 177 factions en 1453 (≈ 24 sur 45 par faction, y compris duchés de 14 provinces). Peu de différenciation entre grandes et petites puissances, et rien à rechercher dans la dernière moitié du siècle.
**Preuve** : `techs=` des instantanés (1 807 à t100 → 4 258 à t464) ; `ls data/technologies | wc -l` = 45 ; `BASE_RESEARCH_POINTS = 5` (`research.rs:39`).
**Correction** : tiers 4-5 historiques (artillerie de siège, bombardes mobiles, ordonnances de 1445), coûts croissants avec le nombre de technologies déjà acquises, bonus de recherche rares (universités) pour que les petites factions n'épuisent pas l'arbre.
**Coût** : M (données + équilibrage).

### [mineur] [conception] Ordre public quasi invisible à l'échelle de la carte
**Constat** : 5 à 32 `Revolt` par partie de 464 tours (moyenne 16, soit 7 par 200 tours : dans la bande 4-10 de l'ADR 0100) sur 118-177 factions et 443 provinces : une révolte tous les 29 tours pour toute la carte. Le joueur qui gère 15-30 provinces en voit rarement ; le levier « Impôt Haut / garnison / édit / église » ne se paie presque jamais. La bande 4-10 a été fixée sur une carte de 186 provinces (EQ1) et jamais rapportée au nombre de provinces.
**Preuve** : `Revolt=16/16/11/18/32/5` ; `data/rules/population.json` (seuil 75, 2 saisons) ; ADR 0100 tableau « Mesures ».
**Correction** : mesurer en révoltes par province-décennie (ex. 0,1-0,3) et viser une fréquence perceptible pour le joueur : abaisser le seuil à 70 pour le joueur seulement (`difficulty.player_unrest`) ou ajouter des révoltes de ville (pas seulement de province).
**Coût** : S (données), M (si règle).

### [mineur] [bug] Journal : `Income` et `BuildingCompleted` ne couvrent que le joueur
**Constat** : `Income=464-469` (un par tour : seulement la faction du joueur, `economy.rs:642`). Les décomptes `BuildingCompleted` (124-1 540) varient d'un facteur 12 d'une graine à l'autre pour la même IA, signe d'un journal partiel ou d'une grande dépendance au sort ; toute sonde qui compte ces événements est biaisée.
**Preuve** : `core/crates/sim-campaign/src/economy.rs:642-650` ; sorties 1 et 2 (`BuildingCompleted=1540` contre `137`).
**Correction** : si la dispersion est réelle, vérifier la boucle de construction de l'IA (`ai/src/campaign`) ; sinon documenter que le journal est filtré. Pas de correction de règle nécessaire avant mesure.
**Coût** : S.

### [mineur] [conception] Ordre de victoire du joueur pas signalé aux factions sans bloc `victory`
**Constat** : seules 4 factions (France, Angleterre, Bourgogne, Croisés) ont `data/factions/*.json` `victory` (objectifs + année de fin) ; les 146 autres jouables n'ont ni `end_year` ni écran « Fin de campagne » (la branche `Ended` suppose `end_year`), seulement les objectifs de titre (130 titres) et les voies génériques. Une partie en Lituanie, par exemple, ne finit jamais.
**Preuve** : `victory.rs` (`end_year = victory.map(|v| v.end_year)`), script Python sur `data/factions` : 4 blocs `victory`.
**Correction** : année de fin globale par défaut (1453) quand le bloc manque, avec score d'`Ended` ; `data/rules/` ou `victory.rs`.
**Coût** : S.

## 3. À ne surtout pas changer
- La structure de budget (impôt × classe, entretien d'armée, administration plafonnée à 28 %, opulence) : peu de banqueroutes, pas d'effondrement économique (trésors négatifs : 0-4 par instantané).
- La pondération de la garnison et des bâtiments par province (ADR 0100) et les constantes d'économie en données (`data/rules/economy.json`) : réglables sans toucher au code.
- Les 28 familles d'effets de bâtiments/techs, toutes câblées dans `EffectKind` ; les 153 événements (167 effets de trésor, 215 de prestige, 169 d'agitation) : un seul vocabulaire d'effets, tous implémentés.
- Le garde-fou `weary_stay_out` et `claim_war_ignores_kinship` (ADR 0085) : la guerre FR-EN reste présente à tous les niveaux.
- La protection du journal d'événements ; la structure des titres (FE) : aucun plantage en 6 × 464 tours × 177 factions.
