# 0217 — Roches générées DN sur la carte de campagne (numéro provisoire)

Statut : accepté. Numéro provisoire du lot DN-RELIEF, à renuméroter à la fusion.

## Contexte
La carte de campagne avait déjà une couche de roches à l'échelle du paysage : `RockOutcrops`
(HB5, ADR 0143), 6 modèles génériques (chaîne GA3) et une règle de pose par biome / pente /
altitude / crête / lande dans `data/art/rock_outcrops.yaml`. Le paquet de modèles DN (ADR 0212)
fournit désormais ~40 roches et reliefs minéraux nommés (granit breton, craie, schiste, éboulis,
moraine, karst, hamada, gypse, tuf, basalte, mégalithes, arche marine…), à taille réelle en mètres.

## Décision
- Pas de seconde couche : `RockOutcrops` consomme les modèles DN. Une entrée du catalogue porte
  `model` (`dn/rocks/<nom>`) et `model_length_m` (plus grand côté horizontal du manifeste) ; le
  maillage est ramené à 1 m au chargement (copie mise en cache), le reste de la couche (pose,
  `size_m`, LOD, shader) est inchangé. Les entrées HB sans `model` restent possibles (aiguilles
  alpines conservées : aucun équivalent DN).
- 26 entrées DN remplacent 5 des 6 entrées HB : falaises (calcaire, basalte, grès rouge), barres de
  schiste, chaos granitique, éboulis, moraine, karst, terrains secs (hamada, gypse, grès, tuf),
  blocs erratiques, mégalithes (rares), cône volcanique.
- Roches de côte : champ `coast` (`max_px`, `weight`, `rock`, `cliff_only`). Terme d'aptitude propre
  au littoral (distance au rivage `coast_dist.png`), filtré par la géologie de côte
  (`data/map/coast_types.json` via `CoastLook.region_at`) et, au besoin, par la règle falaise /
  plage de `CoastLook.kind_at`. Une entrée `coast` ignore pente, crête et lande (sinon elle se
  poserait partout). Craie à Caux / Douvres, granit en Bretagne / Cornouailles / Galice.
- Coût : modèles DN de 300 à 3 000 triangles (LOD0) au lieu de ≤ 400 ; `max_visible_triangles`
  900 k → 450 k et `lod_distances` 45/160 → 30/120 pour garder l'enveloppe de dessin de HB5.

## Conséquences
- Tout est en données : ajouter une roche = une entrée validée par `art_rock_outcrops.schema.json`.
- Appels de dessin : un par modèle et par tuile (≈ 58 à d = 60, ≈ 230 à d = 400 sur les Alpes).
- Les modèles DN sont gitignorés (paquet de release) : sans eux, les entrées DN sont sautées.
