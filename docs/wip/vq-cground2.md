# VQ cground2 — parcelles moins « cristal », plus champs ouverts

## Défaut
À d=60 les parcelles (Voronoï) se lisent comme de grands polygones anguleux à fort saut de luminance.

## Fait (code, à valider visuellement)
- Contraste de luminance 0,9 -> 0,4, saturation 0,45 -> 0,25, jitter 0,34 -> 0,2, teintes de stade rapprochées, tons de lanières réduits.
- Variation lente dans la parcelle (`inner`, 700 m) pour qu'aucune cellule ne soit plate.
- `gd_warp` : déformation de domaine supplémentaire de la recherche de parcelle (un appel ajouté dans `terrain.wesl` avant `hb_voronoi`) : frontières courbes.
- `gd_edges` : le facteur de parcelle revient vers le neutre près de la frontière (transition douce, largeur >= 2 pixels), puis ligne sombre fine (haie / balk) pondérée par `hedge_k` de la rangée (bocage) ; énergie conservée et disparition avec `fade` quand sous-pixel.
- Données : `edges`, `inner` dans `data/fx/ground_detail.json` + schéma.

## Reste / mesures
Voir bas de fichier une fois mesuré.
