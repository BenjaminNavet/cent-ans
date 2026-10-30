# CAMP-GEN (lot E, ADR 0831)

Spec : `docs/audits/2026-10-30-juge-jeu-iter3.md` (M1, M2, M5, « Lot E »).

## État
- `data/missions.json` § `scale` (+ schéma, `MissionScale` dans data-model) : seuils de proportion.
- `sim-campaign/src/mission_scale.rs` : puissance de la maison (hommes, or), force de la cible, filtre des conquêtes / batailles / levées ; `probe_offers` pour les tests.
- `missions.rs` : `candidates_with(scaled)` ; les rancunes sont filtrées aussi ; une levée est plafonnée à ce que le trésor paie.
- Tests : `tests/campaign_life/missions_scaled.rs` (8 maisons, 1337 ; armée de campagne conservée 4 saisons).

## Prochaine étape
- Leviers de départ (Navarre, Bretagne, Flandre) via `data/rules/starting_armies.json` si le test d'armée échoue.
- ADR 0831 + INDEX.

## Points ouverts
- Affaires « Armée ennemie près de X (200 hommes) » : dans `agenda.rs` (lot XP-SOBRE), non touché.
