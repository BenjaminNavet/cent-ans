# MX — modèles Meshy de la vague 8 en campagne

## État (13/10)
- Villes et châteaux (31) montrés tels que générés : `e64631417`, ADR 0349. Validé par le joueur.
- Vague 8 (6 arbres, 3 roches, 4 champs, 3 cultures en gros plan) réingérée en mode fidèle et
  branchée : `d63521db9`.
  - Arbres : 40 essences → 6 modèles (`tree_species.yaml`), imposteurs recuits sans étalonnage,
    `tree_model_gains.json` `faithful` (pas de `model_chroma`). LOD1 fidèle = 6 000 triangles.
  - Roches : 21 affleurements → 3 modèles (`rock_outcrops.yaml` `faithful`) ; volcan, arche,
    mégalithes gardent leur modèle DN. LOD0 ≈ 31 000 triangles.
  - Champs : couche réactivée pour blé, orge, vigne, olivier (gain 1) ; `render.crop_style`
    field / closeup (`crops_closeup`, culture répétée 4 × 4, `min_lod` 1).
- Le joueur constate une **baisse de fps**.

## Banc (machine chargée, charge ≈ 44 : fps non fiables ; primitives fiables)
`--bench-map` 1440 × 900 :
- état actuel : 3,0 fps, 18,25 M primitives (p50), 953 appels ;
- champs coupés : 14,5 fps, 17,5 M primitives, 909 appels (champs ≈ 0,7 M primitives) ;
- arbres proches en lod2 (1 200 triangles) : banc interrompu à la demande du joueur.
Le joueur veut **reprendre le banc sur machine calme**. Ne pas relancer avant.

## Pistes (à mesurer, ne rien générer avant)
- Arbres proches : déjà en MultiMesh (un appel par essence et par tuile) ; le coût = triangles ×
  arbres × 2 (ombres) sur un rayon jusqu'à 40 unités (`generalised_model_radius_max`). Options :
  LOD intermédiaire ≈ 2 500-3 000 triangles, rayon réduit, ombres des modèles coupées.
- Idée du joueur : modèles de forêt (bosquets entiers) pour le milieu de zoom au lieu d'arbres
  individuels — à évaluer après le banc.
- Roches : LOD0 31 k sous 30 unités, plafond `max_visible_triangles` 450 000.
- Champs : LOD0 30 k sous 7 unités.

## Prochaine étape
1. Banc sur machine calme : actuel, champs coupés, arbres lod2, rochers LOD0 limité ; comparer à
   `e64631417~` si possible.
2. Captures (≤ 5) champ / gros plan, arbres, roches ; planche au joueur.
3. Release models (accord du joueur), `docs/budget.md`, mémoire.
