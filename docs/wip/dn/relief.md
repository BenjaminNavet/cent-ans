# DN-RELIEF : roches générées sur la carte de campagne

Branche worktree-agent-ae2942e4f7bfcd2df. ADR provisoire : 0217 (`docs/decisions/0217-roches-dn-campagne.md`), à renuméroter à la fusion.

## Décision de départ
La couche existe déjà : `RockOutcrops` (HB5, ADR 0143), catalogue `data/art/rock_outcrops.yaml`,
schéma `art_rock_outcrops.schema.json`. Pas de couche concurrente : on étend le catalogue avec des
entrées pointant vers les modèles DN (`dn/rocks/*`, en mètres réels) et on remplace les 6 modèles HB.

## Plan
1. Schéma : champs `model` (stem DN), `model_length_m` (normalisation), `coast` (distance à la côte,
   roche de côte par géologie `coast_types.json`).
2. Code : chargement DN (normalisation du maillage à 1 m), terme côte dans `suitability`.
3. Données : entrées DN par biome / altitude / pente / côte / mégalithes.
4. Tests : `game/tests/dn_relief_test.gd`, smoke, mesure perf.

## État
- [ ] squelette
