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
- Captures (6 sur 6 non dépassé, hors dépôt) : Paris d=60 avant/après : les facettes se lisent maintenant comme des parcelles douces aux bords courbes, lignes sombres discrètes ; d=100 idem (reste un peu polygonal, bruns encore marqués). d=25 / d=200 non jugés sur du labour (cadrage tombé en forêt).
- A/B alterné 3 paires, banc carte, ground_detail 0 vs 2 (machine à charge ~125) : p50 31,75 -> 33,52 (+1,8 ms), p95 72,8 -> 69,9 (bruit). Le surcoût de ce lot seul (2 bruits de warp + 2 de variation interne + lignes) est une fraction de cela.
- Tests : `cargo test --release --lib ground_detail`, clippy release, schémas verts.
- Reste : bruns / ploughed encore contrastés (baisser `tints.brown`), lanières visibles seulement au plancher caméra, vérif visuelle d=25 / d=200 sur zone de labour (Beauce), poids de bocage par région dépend de `hedge_k` de la rangée (pas de masque dédié).
