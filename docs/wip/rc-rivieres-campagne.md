# RC — Fleuves et rivières de la carte de campagne

ADR : `docs/decisions/0117-passages-de-riviere-en-campagne.md`. Branche : `feat/rivieres-campagne`.
Budget : section « RC » de `docs/budget.md`, plafond 5 $ (matières d'eau Nano Banana 2).

## Consigne de reprise
> Lis ce fichier et `git log --oneline feat/rivieres-campagne -20`, puis continue à la première
> case non cochée.

## Lots
- [x] RC0 — Squelette : `BattleSetup.crossing` (`BattleCrossing`, `CrossingStructure`),
      `data/rules/river_crossings.json` + schéma, `data/map/river_names.json` + schéma, ADR 0117.
- [ ] RC1 — Cœur campagne : chargement de `crossings_px.json`, détection du passage (rives
      opposées), coefficient d'assaillant / tireurs du défenseur, pronostic nommé, `setup.crossing`.
- [ ] RC2 — Bataille tactique : un seul passage de la structure demandée entre les deux lignes.
- [ ] RC3 — Rendu : largeur, contraste, rivières mineures plus loin ; étiquettes des noms (français).
- [x] RC4 — Densité : `rivers-render --fine-min-order` depuis la pyramide hydro fine (Mac du joueur).
      Outil fait et testé (pyramide synthétique). **À lancer sur le Mac** (pyramide présente),
      puis commiter `data/map/rivers_render.json`, `river_bed.png`, `crossings_px.json` :
      `uv run --project tools cent-ans geo rivers-render --fine-min-order 5`
      (ajuster avec `--fine-min-length-km`, 15 par défaut ; ordre 4 pour plus de densité).
- [ ] RC5 — Matières d'eau Nano Banana 2 (partiel) :
  - [x] pipeline : `data/art/water_materials.yaml`, `--config/--dry-run/--envelope`, section RC de
        `docs/budget.md` (5 $), `water_detail.gdshaderinc`, `WaterDetail.apply`, `data/fx/water_detail.json`.
  - [ ] génération sur le Mac (clé `OPENROUTER_API_KEY`, ~0,31 $ pour 4 images) :
        `uv run --project tools cent-ans assets materials --config data/art/water_materials.yaml --out game/assets/textures/water --sheet docs/research/rc5_water_sheet.png`
        (d'abord `--dry-run`) ; juger la planche, supprimer `<id>_raw.png` pour retenter.
  - [ ] brancher `#include "res://shaders/water_detail.gdshaderinc"` + `WaterDetail.apply()` dans les
        shaders mer / fleuve / rivière (session principale) ; régler `scale` à l'œil.
- [x] RC6 — Rivières infranchissables supplémentaires : Marne, Yonne, Vienne, Charente, Lot, Tarn,
      Allier, Cher, Moselle (`Mosel`), Severn, Trent dans `navgrid.MAJOR_RIVERS` (Oise et Aisne absentes
      de `rivers.geojson`) ; 40 ponts, gués et bacs médiévaux ajoutés à `crossings.json` ; `navgrid.png`,
      aperçu et `crossings_px.json` régénérés (`river_bed.png` laissé tel quel).

## Journal
- 2026-09-30 : RC0.
- 2026-09-30 : RC5 pipeline prêt (génération à faire sur le Mac).
- 2026-09-30 : RC4 (outil `--fine-min-order` ; données à régénérer sur le Mac).
- 2026-09-30 : RC6 (11 rivières de plus infranchissables hors ponts et gués, 40 passages).
