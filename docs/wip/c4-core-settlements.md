# C4 — refonte du cœur (colonies)

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 2, 4.2-4.6. ADR 0005. Suit C1
(`docs/wip/c1-settlements-skeleton.md`).

## État

- [x] Données : `settlement_kinds` des bâtiments (schéma + 24 fichiers), section `movement` de
  `data/settlements/rules.json` (+ schéma), types `data-model` (`Building::allowed_in`,
  `MovementRules`).
- [ ] `sim-campaign` : `ProvinceState` allégé, contrôle dérivé, économie, siège, recrutement,
  construction par colonie.
- [ ] Mouvement sur le graphe des colonies (graphe chargé ou repli).
- [ ] Sauvegarde v5.
- [ ] IA (`ai_minimal`, `crates/ai`).
- [ ] Pont Godot + adaptations minimales `game/`.
- [ ] Tests § 7, fmt/clippy/test, build.sh, smoke.

## Décisions

- Unité de mouvement : le km de plaine. `points_per_step` = 150 (distance moyenne 157 km,
  médiane 144 km, entre cités de provinces voisines, 302 paires) ; une saison = 3 pas (2 en hiver)
  × 150 = 450 points. Coût d'arête de repli = distance × coût du terrain de la province d'arrivée.

## Prochaine étape

Refonte de `state.rs` puis des modules de règles (le crate ne compile pas entre-temps).
