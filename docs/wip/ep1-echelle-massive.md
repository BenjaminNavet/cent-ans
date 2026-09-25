# Lot EP1 — Échelle massive (batailles épiques)

Branche `worktree-agent-a8c20286fc6084f08`. Plan du chantier : `docs/wip/epic.md`. ADR : 0031.

## Objectif
Batailles de 15 000 soldats et plus, fluides (≥ 40 i/s à 15 000 en Haut, ≥ 30 à 25 000).

## État
| Étape | État |
|---|---|
| 0. Squelette : `data/rules/battle_scale.json` + schéma + test, `sim-battle/src/scale.rs` | fait |
| 1. Dimensions du champ paramétriques (cœur, pont, Godot) | en cours |
| 2. Plafond de régiments par palier (données) | à faire |
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

## Prochaine étape
Remplacer `FIELD_WIDTH`/`FIELD_DEPTH`/`*_LINE_Z` dans field.rs, relief.rs, site.rs, ai.rs, sim.rs.
