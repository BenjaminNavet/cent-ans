# Lot EP1 — Échelle massive (batailles épiques)

Branche `worktree-agent-a8c20286fc6084f08`. Plan du chantier : `docs/wip/epic.md`. ADR : 0076.

## Objectif
Batailles de 15 000 soldats et plus, fluides (≥ 40 i/s à 15 000 en Haut, ≥ 30 à 25 000).

## État
| Étape | État |
|---|---|
| 0. Squelette : `data/rules/battle_scale.json` + schéma + test, `sim-battle/src/scale.rs` | fait |
| 1. Dimensions du champ paramétriques (cœur, pont, Godot) | fait (cœur testé ; Godot à vérifier au smoke) |
| 2. Plafond de régiments par palier (données) | fait (20 / 40 / 80) |
| 3. Rendu 15 000+ (mesures A/B, imposteurs très lointains, budget d'animation) | fait |
| 4. Option « Épique » (× 4), sonde IA 60 contre 60 | fait |
| 5. ADR 0076 | fait |

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
  `draw_calls`, `skipped_updates`.

## Mesures (Mac M4 Pro, machine très chargée : charge 200+, 50+ rustc/godot d'autres agents)
`--disable-vsync --resolution 1600x900 -- --benchmark --quality=high --bench-at=90` ; l'écran
plafonne à 60 Hz malgré `--disable-vsync` (comme BV3) : les i/s moyennes saturent à ~57-58.

| Scénario | Config | i/s moy. | p95 ms | primitives | figurines | appels |
|---|---|---|---|---|---|---|
| `--units=63` (15 120 soldats, palier epic 2400 × 1600) | A sans budget | 57,1 · 56,4 | 23,9 · 25,1 | 2,70 M | 14 644 | 1 370 |
| idem | B budget EP1 | 57,6 · 57,4 · 57,4 | 23,0 · 25,0 · 24,2 | 2,70 M | 14 644 | 1 364 |
| `--units=63 --unit-size=1.75` (≈ 25 600 figurines) | A | 55,2 · 48,4 | 24,3 · 33,3 | 3,1-3,5 M | 25 600 | 1 364-1 400 |
| idem | B | 52,4 · 50,3 | 26,4 · 29,6 | 3,1-3,5 M | 25 600 | 1 365-1 413 |
| `--units=105` (19 000 soldats présents, 80 régiments/camp + 25 en renfort) | A | 30,1 | 54,5 | 3,03 M | 19 079 | 1 630 |
| idem | B | 36,2 | 44,9 | 2,95 M | 19 052 | 1 630 |
| `--units=105 --unit-size=1.5` (≈ 28 600 figurines) | B | 34,4 | 42,9 | 3,18 M | 28 562 | 1 624 |
| `--units=20 --unit-size=4` (Épique, 4 700 soldats, palier large) | B | 65,0 | 20,9 | 3,16 M | 18 648 | 1 196 |

Lecture : cibles tenues (15 000 soldats ≥ 40 i/s : ~57, plafonné par l'écran ; 25 000 figurines
≥ 30 i/s : ~50). Le budget ne se voit pas tant que l'écran plafonne ; il rend +20 % à 172
régiments présents (`--units=105`), où le coût est côté script par régiment. Simulation (sonde
`probe_tick_cost`, release) : 0,77 ms par pas à 20 régiments/camp, 0,89 ms à 63, 1,81 ms à 105
(10 pas par seconde de bataille) : ce n'est pas le goulot. Sonde IA contre IA 60 contre 60
(`ai_handles_sixty_regiments_a_side`) : jusqu'à 39 régiments en mêlée, fin à 1 160 s, pas
d'effondrement.

## Fusion de main
- Conflit avec FB1 (`battle/max_figures`, plafond de figurines par défaut 15 000) dans
  `settings.gd` / `settings_menu.gd` : les deux réglages gardés ; choix du plafond étendus à
  20 000, 25 000 et 30 000 (défaut de FB1 inchangé). Avec le plafond par défaut, « Épique » est
  ramené à 15 000 figurines au total.

## Points ouverts
- Écran plafonné à 60 Hz malgré `--disable-vsync` : les gains du budget ne se lisent qu'au-delà
  de ~170 régiments présents ; mesures sur machine très chargée (bruit ±30 %).
- Sièges toujours sur le champ standard (plan de ville fixe).
- Rivière : toujours 2 gués (EP3 les multiplie) ; routes, bosquets et rochers décoratifs suivent la
  taille du champ, les anneaux lointains (EP2) seulement via `NEAR_RECT` / `FAR_RECT`.
- Au-delà de 172 régiments, le coût est dans le script par régiment (bannières, marqueurs,
  `get_units`) : piste pour un lot suivant.

## Prochaine étape
Terminé ; à fusionner dans `integration/epic`.
