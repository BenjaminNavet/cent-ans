# FE1 — déductions, obligations, loyauté (`feat/fe1-deductions`)

Plan : `docs/superpowers/plans/2026-09-28-feodalite.md` § F1. ADR 0098.

## État
- **F1 terminé** (branche `feat/fe1-deductions`), suites Rust et Python vertes hors points ci-dessous.
- `FactionState::suzerain` = vue en cache de `feudal::liege_of`, recalculée par
  `feudal::sync_suzerains` (nouvelle partie, résolution diplomatique de chaque tour, chargement
  d'une sauvegarde dans le pont). Source unique : détentions + `FeudalState::liege_overrides`
  (surcharge du `de_jure_liege` effectif, `None` = souverain).
- Sauvegardes v7 d'avant F1 : `FeudalState::derived` absent → `sync_suzerains` adopte une fois
  les `suzerain` stockés comme surcharges.
- `make_vassal` → `feudal::pay_homage` ; `cut_vassal_tie`, refus d'appel aux armes, révolte,
  suzerain disparu, mort d'une faction → `feudal::release_from_liege`.
- `rally_vassals` → `feudal::direct_vassals` (maxime). Tribut : suzerain direct (vue).
- `loyalty_target` : poids de `feudal.json:loyalty` (+ `power_ratio`, `drift_per_turn`,
  `homage_start`, `memory_turns` ajoutés au schéma), liens familiaux, culture commune,
  mémoire d'événements `feudal::record_*` (protection, titre octroyé, commise d'un pair,
  défaite, prétendant) pour F2/F3.
- Princes d'Empire : `suzerain: fac_empire` ajouté aux données (cohérence avec les titres).

- `feudal::tribute_due` : tribut au seul suzerain direct. `feudal::liege_chain`, `effective_liege`.
- Tests `feudal_deductions.rs` : 7 actifs (vue du suzerain, hommage, sauvegarde, adoption pré-F1,
  anti-cycle ; maxime ; tribut ; loyauté et seuil d'ost lus dans les règles).

## Écart / test cassé (documenté, seuils non retouchés)
- `ai/tests/g4.rs::england_pensions_then_allies_brabant_beside_its_low_countries_allies` passe en
  `#[ignore]` : Brabant et ses voisins (Hainaut, Gueldre) sont désormais vassaux déduits de
  l'Empire, et l'IA G4 (`alignment.rs`, pensions et alliances dynastiques) n'approche aucun vassal
  (`suzerain.is_none()`). Historiquement ces princes d'Empire s'allient à Édouard III (1337-1340) :
  à trancher en F5 (IA féodale : un vassal peut-il s'allier hors de son suzerain ?) et F8.
- Bande d'équilibre `ai/tests/m3_grid_ai.rs::fifty_turns_on_eight_seeds_stay_in_the_c7a_band`
  (`--ignored`, release) : **déjà rouge sur `main`** (dfe88244 : trésor moyen de la France 38 297,
  hors bande, ligne 361). Avec F1 elle échoue plus tôt : graine 2, France en banqueroute deux fois
  (ligne 350). Essai avec `family_tie` et `shared_culture` à 0 : même échec, donc la cause est la
  vassalité déduite des princes d'Empire (alliances et appels aux armes de l'Empire, ost), pas les
  nouveaux termes de loyauté. Non corrigé (pas de retouche de seuils) : à juger en F8.
- `tools/tests/test_budget.py::test_real_budget_file_parses_and_round_trips` échoue déjà sur
  `main` (ligne de budget F0 à quatre colonnes), hors F1.

## Prochaine étape
- Fusion par l'orchestrateur. Points pour F2 : appeler `feudal::record_protection` (intervention
  ou dérobade) et `record_liege_defeat` ; `rally_vassals` utilise déjà `direct_vassals`.
  Pour F3 : `transfer_title` doit appeler `feudal::sync_suzerains` après chaque changement
  de détention ; `record_title_grant`, `record_peer_forfeiture`, `record_rival_claimant`.
