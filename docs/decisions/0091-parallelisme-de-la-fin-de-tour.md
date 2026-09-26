# ADR 0091 — Parallélisme de la fin de tour

Date : 2026-09-26. Statut : accepté. Chantier PB3 (`docs/wip/pb3-performance.md`), lot PB3f
(`docs/wip/pb3f-rayon.md`). Suite de l'ADR 0081 (fin de tour dans un fil).

## Contexte

Après PB3d, la fin de tour tourne dans un fil, mais sur un seul cœur : cœur Rust ≈ 95-115 ms en
release, pendant que 9 des 10 cœurs performants du M4 Pro attendent. Le profil (release, 20 tours,
`sample` macOS) :

- planificateur de l'IA `ai::plan_turn` ≈ 80 % du cœur ; phases `resolve_*` ≈ 13 ms au total
  (économie 4-5, commerce 2,5-3, population 1,3-1,9, le reste < 1 ms) ;
- dans `plan_turn` : `plan_economy` 45 % (`buildable` et `recruitable` pour chaque colonie,
  chacun recalculant l'approvisionnement de tout le royaume `free_supply` et la vitesse de
  construction pour chaque bâtiment), `Context::new` + `GridPlanner::new` 20 % (ancres de toutes
  les armées, terres interdites, `faction_income_effective` appelé trois fois), `plan_armies`
  14 % (tables de Dijkstra), les planificateurs « d'État » (subsides, agents, rançons, monnaie,
  chevalerie, diplomatie) 20 %.

Contraintes : le déterminisme bit à bit prime (sauvegardes, rejeux, tests d'équilibre) ; les
factions jouent l'une après l'autre et chacune voit l'état laissé par la précédente (spec § 3.4).

## Options

- **A. Planifier toutes les factions en parallèle sur l'état de début de tour** puis appliquer :
  change le jeu (une faction ne verrait plus les coups des précédentes) — exclu.
- **B. Planification spéculative avec invalidation** : presque tout ce que lit le planificateur
  (armées, trésors, guerres, sièges) change avec les ordres des factions précédentes ;
  l'invalidation exacte reviendrait à tout recalculer — rejeté.
- **C. Paralléliser les phases par province** (`par_iter_mut`) : au plus quelques ms à gagner,
  et plusieurs phases tirent le RNG partagé dans l'ordre des provinces — pas fait.
- **D. Paralléliser à l'intérieur du planificateur d'une faction** le travail en lecture seule
  de `&CampaignState`, puis réassembler dans l'ordre séquentiel — retenu, avec quelques calculs
  répétés supprimés.

## Décision

**Option D**, dans le crate `ai` :

- `ai::parallel` : pool rayon dédié (pas le pool global), `hw.perflevel0.physicalcpu` fils
  (10 sur M4 Pro ; ailleurs `available_parallelism`, plafonné à 16), QoS
  `USER_INITIATED` (sinon cœurs d'efficacité, cf. ADR 0081), pile de 64 Mio comme le fil de fin
  de tour. `Mode::{Sequential, Parallel}` : `join` et `map` (résultats dans l'ordre des
  entrées) ; en séquentiel, le même code tourne sur le fil appelant.
- `plan_turn` = `plan_turn_in(Mode::Parallel)` ; `plan_turn_sequential` = référence.
  1. En parallèle : `Context::new` (lui-même : `GridPlanner::with_mode` ‖ ancres des armées
     + revenu brut calculé une seule fois) ‖ planificateurs qui ne lisent que l'état, en quatre
     groupes. Les ordres sont réassemblés dans l'ordre d'origine ; le fief-rente et les subsides
     sont déduits du trésor du contexte comme avant ; le filtre de dévaluation est appliqué après.
  2. `plan_economy` : options de recrutement de chaque site et options de construction
     (valeur comprise) de chaque colonie calculées en parallèle, puis la boucle de budget
     séquentielle inchangée ; `free_supply` calculé une fois (`recruitable_with_supply`,
     `buildable_with_supply` dans `sim-campaign`), vitesse de construction une fois par colonie.
  3. `plan_armies` : les tables de routes que la boucle demandera sont pré-calculées en
     parallèle (`GridPlanner::prefetch_tables`) ; les caches du planificateur (`Mutex`, `Arc`)
     sont des mémos purs : leur ordre de remplissage ne change aucun résultat. La boucle des
     armées (ensembles `defended`/`targeted`) reste séquentielle.
- Rien ne change dans `sim-campaign/src/turn.rs` : les factions jouent toujours l'une après
  l'autre, les phases `resolve_*` restent séquentielles.

## Conséquences

- Résultats identiques bit à bit : test `ai/tests/pb3f_parallel_plan.rs` (3 graines × 12 tours :
  ordres de chaque faction, événements et état sérialisé, séquentiel contre parallèle) et
  `threaded_turn_matches_synchronous_turn` ; exemple `turn_digest` (empreinte état + événements
  par tour) identique à la base a7877ac6 sur 3 graines × 30 tours, en séquentiel et en parallèle.
- Gains mesurés (release, machine chargée, A/B alternés, médianes de 3) : voir
  `docs/wip/pb3f-rayon.md`. Environ −35 % sur le cœur de fin de tour, dont environ la moitié
  vient des calculs répétés supprimés (aussi en séquentiel).
- Les sondes multi-fils (`balance_probe`, `century_probe`) partagent le même pool de 10 fils :
  pas de sur-souscription au-delà de leurs propres fils.
- Toute nouvelle étape du planificateur qui lit l'état peut entrer dans un des groupes de
  `state_plans` ; une étape qui dépend des ordres déjà émis (comme `plan_armies` avec les
  licenciements de `plan_economy`) doit rester après eux.
- Pistes non prises : phases `resolve_*` (≈ 13 ms, RNG partagé), boucle des armées, marches
  (`continue_ai_marches`).
