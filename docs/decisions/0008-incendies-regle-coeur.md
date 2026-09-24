# 0008 — Les incendies de siège sont une règle du cœur

Date : 2026-09-24 (lot S2, `docs/design/s2-incendies.md`)

## Contexte
Les batailles de siège gagnent des incendies : traits et pots à feu des assiégeants, feu qui court de
maison en maison, maisons brûlées qui s'effondrent, porte en bois qui cède, faubourgs incendiés par
la garnison. Le feu change le jeu (pertes et moral près des foyers, fumée qui gêne le tir, ruines qui
ouvrent de nouveaux passages, porte ouverte) : il doit être déterministe, rejouable et testable sans
Godot, comme le reste de la bataille (ADR 0001).

## Décision
- L'état du feu (par maison : `intact → burning(intensité, combustible) → burnt`, et la porte),
  l'allumage, la propagation, le vent, la météo et les effets vivent dans `sim-battle`
  (`src/fire.rs` pour les données, `src/sim/fire.rs` pour le tick). Godot ne fait que l'afficher
  (`game/scripts/battle/siege_fire_fx.gd`) à partir de `BattleSim.get_siege()` ; aucune décision de
  jeu côté GDScript (pas de tirage, pas de minuterie de combustion).
- Tous les tirages du feu viennent d'un flux `BattleRng` dédié, dérivé de la graine de la bataille :
  même graine et mêmes commandes, même incendie ; le feu ne décale pas les tirages des autres règles.
- Les nombres de la règle sont dans `data/rules/siege_fire.json` (schéma
  `siege_fire_rules.schema.json`), **embarqués à la compilation** (`include_str!`,
  `FireRules::bundled()`) ; ceux du rendu dans `data/fx/siege_fire.json`, lus par Godot au lancement.
  `BattleSim::set_fire_rules` permet de les remplacer (tests, sonde) ou de couper le feu.
- Une maison brûlée n'est plus un obstacle : ni pour l'arrêt des régiments, ni pour la grille A*
  (le cache des chemins est invalidé quand le nombre de ruines change).

## Conséquences
- Toute la mécanique est couverte par `cargo test` (`sim-battle/tests/fire.rs`) et mesurable par la
  sonde (`probe -- siege …` avec `WEATHER=` et `FIRE=off`).
- Régler le feu impose de recompiler le cœur (les règles ne passent pas par `GameData` ni par
  `BattleSetup`) ; si un jour la bataille reçoit ses règles de la campagne, `FireRules` pourra
  voyager dans le `BattleSetup` sans changer la simulation.
- Le rendu peut évoluer librement (particules, lumières, modèles de ruines) sans toucher aux règles ;
  il doit seulement lire `houses[i].fire`, `gate_fire` et `wind`.
