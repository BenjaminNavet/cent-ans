# PO4 — Bataille (chantier PO, `feat/po4-battle`)

Plan : `docs/superpowers/plans/2026-09-27-po-polish.md` § PO4. Bible DA § 12.6, ADR 0097.

## État

- [x] Squelette : `game/tests/po4_shot.gd`, ce fichier.
- [x] 1. Heure du jour : `time_of_day` (morning 15°, midday 36°, evening 12°) dans
  `data/fx/atmosphere.json` ; soleil sorti de `battle_atmosphere.gd` (plus aucune valeur de
  soleil dans `PRESETS`) ; facteurs par météo `battle.<météo>.sun_energy_scale`,
  `sun_elevation_scale`, `sun_tint` (reproduisent les anciennes valeurs à midi).
  `po_grade_test.gd` partie bataille active (48 contextes). `tools/tests/test_time_of_day_presets.py`.
- [x] 2. Sol : Grass Path 2 (Poly Haven CC0, 2k, 5,3 Mo) dans `battle_ground.gdshader` (`near_detail_*`, < 32 m, lié à DA6).
- [ ] 3. Herbe en touffes.
- [ ] 4. Rangs : décalage de rendu ±0,15 m, ±4°.
- [ ] 5. Horizon : arbres « sucette ».
- [ ] 6. Contours de formation : liseré or pâle.
- [ ] 7. smoke, banc PB1, captures.

## Écart à la spec (étape 1)

Le cœur a déjà une heure de bataille (EP8, ADR 0055 : `BattleSim.get_time_of_day()`, tirée par la
campagne, affichée par le HUD, avec un effet de règle sur la visibilité) et `BattleTimeOfDay`
anime la lumière en facteurs sur la base « midi ». Tirer l'heure sur la graine aurait contredit
l'horloge du HUD. Donc :
- avec l'heure du cœur, le préréglage est celui dont `phases` contient la phase de départ
  (dawn/morning → morning, midday/afternoon → midday, dusk/night → evening) ; la lumière reste
  animée par EP8 depuis le soleil `midday` (base neutre), le préréglage apporte son étalonnage ;
- sans heure du cœur ou avec `--no-daytime` : soleil et ciel du préréglage ; sans heure du cœur,
  tirage de rendu `hash(seed) % n`, `midday` exclu par temps couvert.
- Ciel HDRI selon l'heure : `time_of_day.<clé>.sky.clear` (panorama à soleil bas, winter_clear
  18°) ; l'élévation est abaissée au soleil peint des ciels ensoleillés comme avant.
- `battle_scene.gd` : 3 lignes pour passer l'heure (hors liste de fichiers du lot, inévitable).

## Banc PB1 bataille (Vulkan, 1280×720, GPU ms, 3 tours)

Avant : large 11,34 / 11,28 / 9,67 (médiane 11,28) ; rapproché 11,48 / 11,43 / 11,60 (médiane 11,48).

## Prochaine étape

Étape 3 (herbe en touffes). Machine chargée (GPU partagé) : le banc final doit alterner avant/après dans le même créneau.
