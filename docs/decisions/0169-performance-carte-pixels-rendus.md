# 0169 — Performance de la carte : densité du relief en pixels rendus, pas de MSAA sur la carte

Date : 2026-10-03. Statut : acceptée. Complète l'ADR 0123 (budget de pixels HiDPI) et la PF1.

## Contexte
Le joueur trouve la carte de campagne « un peu moyenne » en images par seconde. Les mesures ont
été faites sur une machine très chargée (charge 40 à 200 : autres sessions, compilations, Godot).
Des bancs lancés l'un après l'autre donnaient des écarts de 2 pour 1 sur une même configuration
(base à 64 ms puis à 122 ms). Le banc `--bench-map` sait désormais alterner des configurations
toutes les 12 images dans un même processus (`--bench-ab=a;b;…`). Toutes les configurations
subissent alors la même charge, et les écarts deviennent reproductibles à ±1 ms.

Panoramique à d = 30, Haute « Automatique » (MetalFX spatial 50 %, ADR 0123), fenêtre Retina :
- image p50 : 59 à 62 ms ;
- shader de fragment du terrain court-circuité : 17 ms. Le fragment du terrain coûte donc
  **≈ 42 ms** ;
- rivières, colonies, vie, routes, tuiles fines, végétation masquées une à une : 0 à 1 ms
  chacune (bruit) ; ombres du soleil coupées : −2,5 ms ;
- vingt blocs du shader du terrain coupés un à un : aucun ne dépasse 3 ms, et les vingt ensemble
  font 27 ms. Le coût est diffus.

Le seuil du quadtree de relief (ZG2) était exprimé en pixels **affichés** : 7,5 px par sommet en
Haute. Sous l'échelle 50 % de l'ADR 0123, cela donne ~3,75 px rendus par sommet, soit des
triangles de ~2 px² que le GPU ombre par quads de 2 × 2 : le très lourd shader du terrain tourne
plusieurs fois par pixel. Le MSAA 2× ajoute encore des exécutions aux bords des triangles.

## Décision
1. **Densité du relief en pixels rendus** (`ReliefQuadtree._prepare_camera`) : la hauteur de vue
   utilisée pour la sélection CDLOD est multipliée par `scaling_3d_scale`. Les valeurs
   `relief_vertex_px` des préréglages sont donc des pixels rendus. À l'échelle 1, rien ne change ;
   à 50 %, Haute passe à 15 px affichés par sommet.
   Mesure (A/B en processus) : **61,7 → 40,5 ms** (−34 %) ; primitives 4,45 → 3,62 M. Captures
   à d = 30 : écart moyen 1/255, aucune différence visible. Les normales viennent des pages de
   hauteurs, une par pixel.
2. **Pas de MSAA sur la carte** : nouvelle clé `map_msaa` des préréglages, appliquée en contexte
   « campaign » (`RenderQuality.msaa_for`). Basse, Moyenne et Haute : désactivé ; Ultra : 2× ;
   `legacy` : 2×. Les batailles gardent `msaa`. FXAA (project.godot) reste actif.
   Mesure : **−11 ms** (39 → 28 ms) ; capture de Paris à d = 30 sans différence visible
   (MetalFX et FXAA lissent déjà les bords).
3. Hors rendu, deux correctifs de scripts :
   - les maillages des bâtiments hors les murs (TB3) sont préparés sur un fil de travail dès le
     chargement ; avant, chaque maquette était chargée et découpée en pièces sur le fil
     principal à sa première apparition (pics de 40 à 85 ms) ;
   - le dé-encombrement des noms ignore les colonies dont l'ancre est loin hors de l'écran, sans
     recaler leur `Label3D` ; seules celles à l'écran ou en bordure sont traitées.

## Conséquences
- Au zoom rapproché sur l'écran du joueur, l'image passe d'environ 60 ms à environ 28 ms
  (≈ ×2) sur machine chargée. Machine calme : à remesurer.
- Le shader du terrain reste le premier poste, avec un coût diffus. Les pistes sont dans
  `docs/wip/fps-carte.md`.
- Ne plus comparer deux bancs lancés séparément sur une machine partagée : utiliser
  `--bench-ab`.
