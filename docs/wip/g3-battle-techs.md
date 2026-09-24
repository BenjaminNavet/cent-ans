# G3 — technologies dans la bataille 3D (limite connue G1)

## Tâche

Lever la limite connue G1 : « la bataille 3D n'applique toujours pas les bonus des
technologies (seuls ceux des bâtiments de la province de levée passent dans
`battle_setup`) ».

## État : terminé

- `core/crates/sim-campaign/src/battle_request.rs::side_setup` applique désormais
  `research::tech_unit_bonus(state, data, &army.faction, unit_type.category)` à
  chaque unité, exactement comme `movement::side_from_army` le fait pour la
  résolution automatique (moral, mêlée, tir, armure — `research::boosted`), en
  plus des bonus des bâtiments de la province de levée (`levy_armor`,
  `levy_ranged`) qui existaient déjà. Les deux sources s'additionnent (pas de
  double comptage : un seul et même calcul alimente `battle_setup`, l'autre
  chemin `movement::side_from_army` reste indépendant et sert uniquement à
  l'auto-résolution).
- Tests ajoutés/mis à jour dans `core/crates/sim-campaign/tests/g1_rules.rs` :
  - `technology_bonuses_reach_the_battle_setup_without_double_counting` (nouveau) :
    part d'un état sans aucune technologie, vérifie que `battle_setup` ne
    change pas l'armure de base, puis ajoute `tech_coat_of_plates` (+5
    armure infanterie/cavalerie, donnée du jeu) et un bonus de bâtiment
    (`levy_armor = 3`) et vérifie que `battle_setup` reflète bien la somme
    des deux (pas l'un ou l'autre, pas le double), et que ce total est
    identique à celui de `movement::side_from_army` (même formule, pas de
    divergence entre bataille 3D et auto-résolution).
  - `levy_bonuses_reach_the_auto_resolver_and_the_battle_setup` (existant) :
    l'assertion finale comparait `battle_setup` à `unit_type.stats.armor + 5`
    en ignorant les technologies de départ de la France en 1337 (qui donnent
    déjà un bonus d'armure) ; corrigée pour comparer `battle_setup` à
    `side_from_army` (`side.units[0].armor`), qui est la référence correcte
    puisqu'elle inclut déjà les mêmes technologies.
- `cargo fmt --all` et `cargo clippy --all-targets -- -D warnings` passent.
  `cargo test -p sim-campaign --test g1_rules` : 11/11 OK. Suite complète
  `cargo test` lancée en tâche de fond (au-delà de 120 s, résultat à
  confirmer avant de clore).

## Prochaine étape

- Confirmer que `cargo test` (suite complète du workspace) passe sans
  régression ailleurs (aucun autre test ne dépendait de l'ancien comportement
  d'après une recherche de `battle_setup`/`levy_armor`/`levy_ranged` dans
  `core/`, hors `g1_rules.rs`).
- Aucun changement au pont GDExtension (`godot-bridge`) : le format de
  `BattleSetup`/`UnitSetup` transmis à Godot est inchangé (mêmes champs,
  valeurs mises à jour uniquement) ; `core/build.sh` +
  `godot --headless --path game --script res://tests/smoke.gd` à rejouer par
  précaution si un doute subsiste sur le pont.
- Mettre à jour `docs/status.md` (ligne G1 des limites connues) — fait dans
  ce commit.
