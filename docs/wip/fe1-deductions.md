# FE1 — déductions, obligations, loyauté (`feat/fe1-deductions`)

Plan : `docs/superpowers/plans/2026-09-28-feodalite.md` § F1. ADR 0098.

## État
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

## Prochaine étape
- Remplir les 4 tests ignorés de `feudal_deductions.rs`, lancer la suite complète, vérifier
  l'équilibre.
