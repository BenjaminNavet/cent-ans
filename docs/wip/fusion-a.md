# FUSION-A — fusion des lots ECO-R1c, IA-PLAN-2, PLAQUES-2, XP-R3a, CAMP-R6, CAMP-R6b

État : fusionné dans `fusion-a` par cherry-pick de chaque branche (équivalent d'un rebase), puis ff-only dans `main`.

## Ordre et ADR
1. ECO-R1c — ADR 0815
2. IA-PLAN-2 — ADR 0816 (fixture auto-resolve régénérée dans la branche, ADR 0807 ; aucun conflit sur data/ depuis)
3. PLAQUES-2 — pas d'ADR propre ; 4 fichiers HUD non commités = pur `cargo fmt`, commités dans la branche (`style: cargo fmt campaign HUD`)
4. XP-R3a — ADR 0818 (a réécrit/retrié `docs/decisions/INDEX.md` : conservé tel quel)
5. CAMP-R6 — ADR 0817
6. CAMP-R6b — pas d'ADR propre

Seuls conflits : `docs/decisions/INDEX.md` (résolus par union).

## Check (out/check-fusion-a/)
- core fmt + clippy -D warnings : OK ; core tests : OK
- client clippy : OK ; client tests : 1542 OK, 1 échec d'environnement : `campaign::markers::dn::tests::real_table_and_glbs`
  (les GLB `game/assets/models/dn/` sont gitignorés et absents d'un worktree neuf ; le test passe dans le checkout principal).
- Snapshot `faction_assembly` : n'a pas échoué dans fusion-a (aucune mise à jour nécessaire).
- smoke (CENT_ANS_MUTE=1) : OK.
- pytest : 4102 OK, 1 échec corrigé : `data/map/settlement_markers.json` (PLAQUES-2, `declutter.scene_height_ratio`) absent du schéma → ajouté à `settlement_markers.schema.json`.
- seed bots `run --quick` : exit 0 (rapport dans `core/target/seed-bots/2026-10-30_085657/report.md`).

## Points ouverts
- Part armée de l'IA (ECO-R1c) à surveiller aux mesures longues.
- Test `real_table_and_glbs` : dépend d'assets gitignorés, ne passe que dans le checkout principal.
