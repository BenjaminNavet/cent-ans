# ADR 0081 — Fin de tour dans un fil

Date : 2026-09-26. Statut : accepté. Chantier PB3 (`docs/wip/pb3-performance.md`), lot PB3d
(`docs/wip/pb3d-fin-de-tour.md`).

## Contexte

La fin de tour de la carte (`CampaignMap._on_end_turn`) appelait `CampaignSim.end_turn()` dans le
fil principal : 225-390 ms de sim et d'IA pendant lesquels l'image est figée (caméra, animations,
son), puis `refresh_all()` relisait chaque province par `get_province_state` — un dictionnaire
complet (garnison, gouverneur, siège) par province, dans cinq boucles au moins (couleurs
politiques, frontières, parchemin, hameaux, vie des campagnes, alertes de la cloche).

## Options

- **A. Fil de travail sur un clone de l'état** : `begin_end_turn()` clone l'état et le résout dans
  un `std::thread` (données partagées par `Arc<GameData>`), `poll_end_turn()` installe le résultat.
  L'interface lit l'état d'avant pendant le calcul ; tout ce qui modifierait l'état est refusé.
- **B. État déplacé dans le fil** (sans clone) : pas de copie, mais aucune lecture possible
  pendant le calcul — la carte, les panneaux et les infobulles n'ont plus rien à lire.
- **C. `WorkerThreadPool` de Godot** en GDScript : le `CampaignSim` serait partagé entre fils
  sans garde côté Rust (`&mut self` depuis deux fils) : exclu.
- **D. Découper le tour en tranches** exécutées image par image : invasif pour la sim (ordre
  fixe spec § 1.3, IA par faction), déterminisme plus difficile à garantir.

## Décision

**Option A.**

- Pont (`godot-bridge`) : `turn_job.rs` (Rust pur, testé) porte `resolve_turn` — la seule
  résolution de fin de tour du pont, utilisée par la voie synchrone et par le fil, ce qui les rend
  identiques par construction — et `TurnJob` (fil nommé, pile de 64 Mio pour l'IA).
  `campaign_sim_turn.rs` expose `begin_end_turn() -> bool`, `poll_end_turn() -> Variant` (`null`
  tant que le fil tourne, puis les événements au format de `end_turn`), `is_end_turn_pending()`.
- `end_turn()` reste synchrone (tests, captures, mode headless, `campaign_sim_mock.gd`) ; appelé
  pendant un calcul en cours, il l'attend et renvoie ses événements.
- Pendant le calcul, chaque méthode qui modifie l'état commence par
  `refuse_while_turn_pending` : ordres et réponses → `{ok: false, error: « la fin de tour est en
  cours »}`, réglages et outils de debug → ignorés avec un avertissement. `new_campaign` et
  `load_from_string` abandonnent le calcul (son résultat n'est jamais installé).
- Compteur `get_state_revision()` augmenté par toute méthode qui peut changer l'état : les
  lectures groupées de la carte sont mises en cache par lui.
- Lecture groupée : `get_provinces_snapshot(ids)` (propriétaire, contrôleur, dévastation,
  population, siège en `Packed*Array`) et `get_settlements_live()` ; côté carte, la classe
  `ProvinceSnapshot` partage un instantané entre tous les calques et n'est relue que si la
  révision change ; couleurs politiques et parchemin ne sont refaits que si les propriétaires ont
  changé.
- Carte : `_on_end_turn(true)` (bouton, raccourci, confirmation) passe par le fil ; cloche
  désactivée, ordres refusés (toast), cartouche enluminé « Les cours d'Europe délibèrent… » avec
  sablier après 150 ms ; caméra, survol et animations continuent. La suite (journal, rejeu IA,
  diplomatie, victoire, rapport, sauvegarde automatique, batailles) est inchangée, après
  installation du nouvel état. Les appels directs `_on_end_turn()` restent synchrones.

## Conséquences

- Déterminisme : le fil résout un clone identique avec les mêmes données ; le test
  `turn_job::tests::threaded_turn_matches_synchronous_turn` compare état sérialisé et événements
  sur trois tours. La sim n'a pas d'état global (les `thread_local` de navigation et les caches de
  l'IA sont des brouillons par appel).
- Coût : un clone de l'état par fin de tour (quelques ms, dans le fil principal).
- Nouvelle méthode mutante du pont : elle doit commencer par `refuse_while_turn_pending`, sinon
  sa modification serait écrasée par l'état résolu.
- PB3f (IA parallèle avec rayon) peut paralléliser l'intérieur de `resolve_turn` sans changer
  cette API.
