# GR — la carte de campagne suit la géographie

Mandat (13/10) : « ok pour tout, tu es en autonomie ». Enchaîner les lots sans redemander ;
pas de crédit Meshy sans accord ; pas de push.

## Lots
1. **Couleur lointaine par culture** : chaque culture (blé, seigle, vigne, olivier, pré, rizière,
   essart…) a sa couleur d'été et sa classe de saison (`crop_far` de `ground_biome_mix.json`) ;
   la sole tire sa culture avec les poids du paysage régional, proche et lointain cohérents.
2. **Relief et eau** : prés de fauche en fond de vallée près des rivières, vigne sur les coteaux
   exposés au sud, cultures qui reculent sur les pentes fortes et en altitude (estives).
3. **Régions historiques vers 1340** (agent historien, sources) : `agri_landscapes.json`,
   recuisson de `agri_regions.png`.
4. **Habitat selon le paysage** : groupé en openfield, dispersé en bocage, perché au sud.
5. **Planche régionale** : 8 régions × 2 hauteurs (Beauce, Bretagne, Provence, Bourgogne,
   Flandre, Guyenne, Castille, Angleterre).
6. **Textures de près** (Z-Image local ; Meshy seulement avec accord).

## État
- 13/10 : squelette ; lots 1+2 faits (5b442c2ca, ADR 0352).
- Planche 1 (lot 5) : Beauce, Bretagne, Flandre lisibles ; Bourgogne, Bordelais, Castille,
  Oxfordshire centrés sur une ville → points décalés vers la campagne.
- Lot 3 fusionné (aa9e9c1b5) : 91 régions, 8 paysages (+ élevage d'altitude, châtaigneraie), sarrasin
  et riz lombard retirés ; sources et incertitudes `docs/histoire/gr-paysages-1340.md`.
- Lot 4 fusionné : tests verts sur le checkout principal ; `dn_pays_test` à 95 appels de dessin / 100
  en vue bocage d=30 (aussi avec les vrais glb) — marge faible.

## Lot 4 — habitat selon le paysage (13/10, fusionné)
- Filtres de règle `landscapes` / `exclude_landscapes` (ids de `agri_landscapes.json`), lus dans
  `agri_regions.png` au point de pose (`CountrysideLayer.landscape_ok`, aussi pour les règles `village`).
- Règles : `bocage_fermes_isolees`, `bocage_haies_denses` (bocage) ; `mas_sud`, `cabanons_de_vigne`,
  `murets_de_terrasses` (olivier + vigne) ; `fermes_de_clairiere` (essarts) ; `bocage_enclos` exclut l'openfield
  (habitat groupé, aucune ferme isolée). Modèles DN existants seulement, aucune génération.
- Modèles : farm_norman_timber_courtyard, env_bocage_hedge_bank, farm_southern_stone_mas, vineyard_hut,
  terrain_dry_stone_wall_limestone, farm_baltic_longhouse, farm_polish_log_cabin ; schéma : dossier `rocks/` admis.
- Test : `game/tests/gr_habitat_test.gd` (OK) ; `dn_pays_test` OK mais 95 appels de dessin sur 100 en vue bocage d=30
  (mesure sans les glb DN du worktree : retester sur le checkout principal). Restes : œil sur les densités en jeu.

- Planche 2 : Bordelais/Provence « vides » = brouillard de guerre de la planche (vue France), pas les
  champs ; planche sans brouillard (`--fog` pour le garder), mélanges sud rééquilibrés, pentes de
  cession 0,38-0,70 (aaca2e519, addendum ADR 0352). Collines provençales grises (garrigue, roche) : voulu.

## Prochaine étape
Lot 6 (textures de près, Z-Image local) ; avis du joueur sur la planche.
