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

## Prochaine étape
Build `core/build.sh`, smoke, puis mesures `--benchmark --units=63` (15 000) et `--units=105`.
