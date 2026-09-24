# ADR 0004 — Direction artistique semi-réaliste

Date : 2026-09-23. Statut : accepté. Remplace la ligne « Visuel » du document de conception
(« 3D stylisée low-poly réaliste »).

## Contexte

Après la v1, le rendu est jugé « très basique » : terrain coloré par altitude sans texture, frontières
crénelées, éclairage plat sans post-traitement, champ de bataille vide, soldats en pavés.
Le joueur demande un jeu **semi-réaliste** (référence : cartes de Total War, Crusader Kings III).

## Décision

- **Lumière d'abord** : tonemapping AgX, SSAO, ombres du soleil, ciel physique, brouillard de distance,
  anticrénelage (MSAA 4× + FXAA), étalonnage léger. Réglages exposés dans `game/scripts/visual/`.
- **Matériaux PBR** : textures CC0 de Poly Haven (albédo, normale, rugosité) stockées dans
  `game/assets/textures/`, mélangées dans les shaders par une **splatmap** générée hors ligne
  (`tools/geo`, depuis altitude, pente, terrain dominant des provinces, bruit).
- **Carte** : frontières par champ de distance signé (lisses à tout zoom), eau avec profondeur,
  reflets et écume, fleuves affinés, forêts en `MultiMesh`, « carte papier » seulement au dézoom maximal.
- **Batailles** : sol texturé, herbe en `MultiMesh` animée par le vent, ciel, figurines détaillées
  (casques, écus, lances), animation par shader de sommets (marche, balancement) compatible `MultiMesh`.
- L'interface garde son style manuscrit / parchemin.
- Coût : 0 $ (assets CC0, génération procédurale). Toute dépense éventuelle va dans `docs/budget.md`.

## Conséquences

Charge GPU plus élevée : effets en espace écran seulement, pas de SDFGI ni de lumière volumétrique sur la
carte ; un préréglage « qualité » dans les réglages. Les modèles `.glb` low-poly doivent être retexturés
pour rester cohérents. Les tests headless ne voient pas le rendu : chaque lot fournit des captures
avant/après dans `docs/img/visuel/`.
