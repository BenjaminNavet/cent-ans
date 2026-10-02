# ADR 0150 — Saisons de la carte : écart posé sur la carte de couleur, réglages dans `data/ui/`

Date : 2026-10-02. Lot TB1 (`docs/design/2026-10-02-campagne-tob.md` § 3).

## Contexte

- La carte de couleur satellite (ADR 0142) et les matières par biome (ADR 0143) sont d'une seule
  saison moyenne. Posées après les teintes saisonnières CV1, elles les remplaçaient (100 % en vue
  large et moyenne, 45 % de près) : seule la neige se voyait.
- Un étalonnage par saison existe déjà : `data/fx/atmosphere.json` (`seasons`, commun aux batailles,
  et `campaign.seasons.<saison>.grade`, étalonnage de carte en S ; ADR 0097). Le lot TB1 demande
  un étalonnage de saison plus franc pour la carte, avec ses valeurs dans `data/ui/`.
- La mer et les toits des villes 1:1 (ADR 0138) ne connaissaient pas la saison.

## Décision

1. **Écart saisonnier multiplicatif, pas quatre cartes.** `terrain.gdshader` calcule le rapport
   saison / été des teintes de base CV1 (prés, cultures, forêts, lande), pondéré par l'occupation du
   sol au pixel, et le passe à `sg_apply` (`season_k`, force `sg_season_strength`) : la carte donne
   la couleur moyenne, la saison la déplace. Dans `hb_apply`, prés et canopée reçoivent le même
   rapport ; une parcelle cultivée prend, hors été, la couleur de saison de sa culture
   (`season_field`) et ne garde de sa matière que le grain. La variante « quatre cartes de saison
   précalculées » de la spec n'est pas retenue : 4 × 60 Mo de BC1 et un outil à relancer pour un
   écart que le shader sait calculer ; elle reste possible si le rendu ne suffit pas.
2. **Un seul fichier de réglages de saison pour la carte : `data/ui/campaign_seasons.json`**
   (schéma `campaign_seasons_ui.schema.json`, lu par `SeasonLook`) : mer (`sea` : teinte et gris visé), tempête
   (`storm`), étalonnage (`grade`), neige des toits (`roof_snow`, `snow`).
3. **L'étalonnage TB1 est une surcouche, pas un remplacement.** `CampaignAtmosphere.resolve_preset`
   compose : saison commune (`atmosphere.json` `seasons`) → saison de carte (`SeasonLook.grade`) →
   étalonnage de carte en S (dernier, comme l'exige `po_grade_test`). Les batailles ne changent pas.
4. **Neige des toits par paramètre global `campaign_roof_snow`** (quantité, limites sud en px
   carte), publié par `CampaignLife` : les matériaux des villes 1:1 sont nombreux (un par ville et
   par niveau de détail) et `model_holder()` ne les expose plus ; un paramètre global évite de les
   recenser. Les shaders gardent leur uniform `snow` local (le plus fort des deux gagne).

## Conséquences

- Les teintes CV1 de `campaign_life.gdshaderinc` restent la source de la saison du sol ; régler une
  saison se fait là (ou par `sg_season_strength` / `hb_season_strength`), pas dans la carte de couleur.
- Deux fichiers portent un étalonnage de saison pour la carte (`fx/atmosphere.json` et
  `ui/campaign_seasons.json`) : le premier est la base commune, le second l'accent de la carte.
- Sans `campaign_seasons.json` (données de test), tout est neutre : rendu d'avant TB1.
- La suie des villes 1:1 saccagées (état par ville) n'est pas couverte par le paramètre global ;
  elle demandera un attribut par ville (lot TB suivant).
