# PB3f — rayon : fin de tour parallèle, déterminisme gardé

Branche `perf/pb3f-rayon` (worktree agent `agent-a27afc73c1888787a`, partie de `main` a7877ac6).
Plan général : `docs/wip/pb3-performance.md`. ADR réservé : `docs/decisions/0091-parallelisme-de-la-fin-de-tour.md`.

**EN PAUSE (demande du joueur, 26/09).** Aucun code de production modifié ni commité ; rien de
parallélisé encore.

## État
- [x] Profil release (graine 1, 20 tours, machine chargée, `pb3f_profile`) — voir ci-dessous.
- [ ] Choix des cibles parallélisables (lecture seule, sans RNG partagé).
- [ ] rayon en dépendance de workspace, pool limité aux cœurs performants (`hw.perflevel0.physicalcpu` = 10).
- [ ] Test bit à bit séquentiel/parallèle sur N tours, plusieurs graines.
- [ ] Mesures turn_perf / pb1_turns avant-après (A/B alternés, médianes de 3).
- [ ] ADR 0091.

## Profil mesuré (release, par tour, moyenne de 20 tours, graine 1)
Deux exécutions (charge variable) : total 120 / 167 ms ; IA des factions 107 / 153 ms dont
**planificateur `ai::plan_turn` 94 / 138 ms (≈ 80 % du cœur)** ; phases `resolve_*` ≈ 13 ms seulement
(économie 4,4-4,9 ; commerce 2,5-2,8 ; population 1,3-1,9 ; le reste < 1 ms).
→ Paralléliser les phases par province ne rapporterait que quelques ms : la cible est le planificateur.

Dans `plan_turn` (2e exécution, 138 ms) : `plan_economy` 66 ms, `Context::new` 28 ms
(ancres de toutes les armées + `GridPlanner::new` + `faction_income_effective` appelé deux fois),
`plan_armies` 10, `agents::plan_agents` 5,8, `ransom` 3,2, `chivalry` 3,1, `diplomacy_eval` 2,2.
Par faction : Angleterre 14 ms, Empire 11, Castille 6,5, le reste 3-5 ms.

## Pistes (pas encore évaluées finement)
- Les factions jouent en séquence et chaque planificateur lit presque tout l'état modifié par les
  précédentes (armées, trésors, guerres) : un pré-calcul inter-factions exact paraît très limité.
- Piste principale exacte : paralléliser **à l'intérieur** de `plan_turn` les boucles pures en
  lecture seule, puis réassembler dans l'ordre : options de construction (`state.buildable` +
  `building_value` par colonie possédée), menaces des provinces frontières, ancres des armées,
  `GridPlanner::new` (voisinage des ennemis). La boucle de recrutement reste séquentielle (budget).
- Avant cela : profil fin avec samply pour savoir ce qui coûte dans `plan_economy` et `Context::new`
  (peut-être un simple gain séquentiel : `faction_income_effective` calculé deux fois).
- Si le gain parallèle mesuré < 15 % du cœur : le dire et proposer de ne pas fusionner.

## Outils de reprise
- Instrumentation temporaire NON commitée dans le worktree (turn.rs, campaign.rs,
  exemple `core/crates/ai/examples/pb3f_profile.rs`), sauvegardée dans
  `docs/wip/pb3f-profil-temporaire.patch` (`git apply` pour la remettre ; à retirer avant tout
  commit de code, ne jamais la fusionner).
- `CARGO_TARGET_DIR=<worktree>/core/target cargo run --release -p ai --example pb3f_profile -- 20 1`
- Binaire turn_perf de référence (avant) : `<scratchpad>/turn_perf_base` (scratchpad de session,
  peut avoir disparu : le reconstruire depuis a7877ac6).
- samply installé dans `<scratchpad>/samply/bin/samply` (idem).
- Cible cargo privée `<worktree>/core/target` conservée (à supprimer à la fin du lot).
- Lien symbolique `data/map/pyramid` → dépôt principal déjà en place.

## Prochaine étape
1. `samply record` sur `pb3f_profile` (release) → coût détaillé de `plan_economy` et `Context::new`.
2. Ajouter rayon (workspace), pool global limité aux cœurs performants ; paralléliser les boucles
   pures de `plan_turn` en préservant l'ordre des résultats.
3. Test bit à bit (état sérialisé + événements, 3 graines × N tours) séquentiel vs parallèle.
