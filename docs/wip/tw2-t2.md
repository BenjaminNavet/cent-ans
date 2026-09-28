# TW2-T2 — Reconstitution des armées et réserves de recrutement

Branche `feat/tw2-t2`. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T2. ADR 0102.

## Constat initial
Aucune reconstitution d'armée de campagne n'existait : seules les garnisons regagnent des hommes
(`economy.rs`, effet `garrison`), et les blessés soignés après bataille (`medicine.rs`, H4).

## Fait
- [x] Données : `data/rules/replenishment.json` + schéma `replenishment_rules.schema.json`,
  `data-model` `ReplenishmentRules` (défaut = fichier embarqué).
- [x] `sim-campaign/src/replenish.rs` : taux saisonnier (territoire, posture, place amie, hiver,
  vivres, intendance du chef, bâtiments), coût en or, `fought_turn` posé par `apply_outcome`.
- [x] `sim-campaign/src/recruit_pool.rs` : réserves par établissement × unité (millièmes), recharge
  saisonnière, consommées par `order_recruit`, refus « réserve épuisée ».
- [x] Tests Rust dédiés (`tests/tw2_t2_replenish.rs`) ; `g1_rules` vide la réserve entre deux recrues.
- [x] IA : respecte les réserves dans sa boucle de recrutement (`ai/src/campaign.rs`).
- [x] Pont : `get_army_replenishment`, champs `pool_*` de `get_recruitable`.
- [x] UI : sceau d'armée (taux + info-bulle), ligne de recrutement « N disponibles, +1 dans K saisons ».
- [x] Test headless Godot `game/tests/tw2_t2_replenish_test.gd` (OK).
- [x] Sonde IA finale (`ai/examples/t2_replenish_probe.rs`) → tableau de l'ADR 0102 ; smoke.

## Prochaine étape
Lot terminé (tests Rust, smoke, test headless verts). Reste : fusion dans main par l'orchestrateur.
