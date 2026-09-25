# Lot EP1 — Échelle massive (batailles épiques)

Branche `worktree-agent-a8c20286fc6084f08`. Plan du chantier : `docs/wip/epic.md`. ADR : 0031.

## Objectif
Batailles de 15 000 soldats et plus, fluides (≥ 40 i/s à 15 000 en Haut, ≥ 30 à 25 000).

## État
| Étape | État |
|---|---|
| 0. Squelette : `data/rules/battle_scale.json` + schéma + test, `sim-battle/src/scale.rs` | fait |
| 1. Dimensions du champ paramétriques (cœur, pont, Godot) | fait (cœur testé ; Godot à vérifier au smoke) |
| 2. Plafond de régiments par palier (données) | fait (20 / 40 / 80) |
| 3. Rendu 15 000+ (mesures A/B, imposteurs très lointains, budget d'animation) | à faire |
| 4. Option « Épique », sonde IA 40 régiments | à faire |
| 5. ADR 0031 | à faire |

## Conception
- Paliers par effectif total (soldats simulés des deux camps, réserves comprises) :
  escarmouche ≤ 4 000 → 1200 × 800 m, 20 régiments/camp ; grande ≤ 8 000 → 1800 × 1200, 40 ;
  rangée au-delà → 2400 × 1600, 80. Sièges : toujours le premier palier.
- `FieldSize` (largeur, profondeur, écart des lignes, profondeur des zones) ; lignes centrées
  (`depth/2 ∓ gap/2`), 250/550 au palier standard ; toutes les valeurs dérivées valent exactement
  les anciennes constantes au palier standard (graines inchangées).

## API pour EP2 / EP3 / EP6 (à lire, ne jamais supposer 1200 × 800)
- Cœur : `Battlefield::width/depth`, `Battlefield::size` (`FieldSize` : `center_x()`,
  `attacker_line_z()`, `defender_line_z()`, `line_half()`, `sx()`, `area_ratio()`) ;
  `Battlefield::generate_sized` / `generate_site_sized` (les anciens `generate` / `generate_site`
  = champ standard) ; `site::Occupied::size` ; `Coast::field_width` ; `River::polyline(step, width)`.
  `FIELD_WIDTH`, `FIELD_DEPTH`, `ATTACKER_LINE_Z`, `DEFENDER_LINE_Z` ne décrivent plus que le champ
  standard (tests, sièges).
- `BattleSim::scale()`, `max_on_field()`, `new_scaled(setup, seed, scale)`.
- Pont : `get_terrain()` a `width`, `depth`, `attacker_line_z`, `defender_line_z` ;
  `get_scale()`, `get_field_size()`, `set_scale_tier(key)` (avant `setup`).
- Godot : `BattleTerrain.FIELD_W/FIELD_D/SPLAT_RECT/NEAR_RECT/FAR_RECT` sont des variables
  (fixées dans `build` par `_set_field_size`), `field_center()`, `field_scale_x()`.
  `battle.tscn -- --scale=<skirmish|large|epic>` force le palier.

## Rendu (étape 3)
- `battle_soldiers.gd` : budget d'animation par distance (au-delà de 450 m, mise à jour une image
  sur 2 ; au-delà de 800 m, une sur 3, décalée selon l'id ; jamais quand l'effectif dessiné change) ;
  cadavres envoyés au GPU une fois par image et par cellule (`_flush_corpses`) au lieu d'une copie
  complète de la cellule à chaque mort ; imposteurs à demi-densité au-delà de 700 m (`thin_out`
  du shader : une figurine sur deux effacée, les autres élargies ×1,6).
- `--no-ep1-budget` coupe les trois (A/B). Banc : `tools/bench_ep1.sh <étiquette> --units=63 …`.
- JSON du banc : `scale`, `field_w`, `on_field_soldiers`, `figures`, `primitives_m`,
  `draw_calls`, `skipped_updates`, `process_ms`.

## Mesures (Mac M4 Pro, machine très chargée : charge 200+, 50+ rustc/godot d'autres agents)
`--disable-vsync --resolution 1600x900 -- --benchmark --quality=high --bench-at=90` ; l'écran
plafonne à 60 Hz malgré `--disable-vsync` (comme BV3) : les i/s moyennes saturent à ~57-58.

| Scénario | Config | i/s moy. | p95 ms | primitives | figurines | appels |
|---|---|---|---|---|---|---|
| `--units=63` (15 120 soldats, palier epic 2400 × 1600) | A sans budget | 57,1 · 56,4 | 23,9 · 25,1 | 2,70 M | 14 644 | 1 370 |
| idem | B budget EP1 | 57,6 · 57,4 · 57,4 | 23,0 · 25,0 · 24,2 | 2,70 M | 14 644 | 1 364 |

## Prochaine étape
Mesures à 25 000 figurines (`--unit-size=1.75`) et au-delà ; ADR ; fusion de main.
