# 0496 — Géographie de la carte : lacs au vrai niveau, côte lissée, maquettes hors de la mer

Statut : accepté (17/10, lot mapgeo des correctifs post-playtest).

## Contexte
Le playtest du 17/10 a relevé une baie du Mont-Saint-Michel noire, un Léman flottant avec un trou noir, des côtes en escalier, des villes débordant sur l'eau, des rivières noires hachées. Causes mesurées : la retouche de côte de mapgeo11 n'avait jamais atteint la pyramide E1-E3 (cache hors git) ; `flatten_lakes` cale un lac au 10e centile de sa rive (607 m pour le Léman, 366 m réels) ; `coast_dist.png` vient d'une EDT sur masque binaire (contour en marches d'un pixel carte) ; aucune contrainte terre/mer sur la pose des maquettes.

## Décision
- `compose lake-patch` : un lac dont l'anneau de rive porte un palier plat réel nettement plus bas (>= 8 m, >= 30 px) reprend ce niveau dans E0, E1-E3, `heightmap*.png` et `lakes.json` (`level_m`, `level_patched_from`). Rejouable ; une chaîne de lacs voisins se cale en quelques passes. `compose coast-patch` sur la pyramide complète le travail de mapgeo11.
- `compute_coast_dist` : distance au contour lissé (gaussienne 1,3 px, `erfinv`) à moins de 4 px de la côte, exacte au-delà ; `coast_dist.png` régénéré.
- Les maquettes de villes sont tirées vers la terre (`placement::shore_fit`, <= 1,25 rayon) quand plus de 4 % de leur empreinte est sur l'eau ; la position de simulation ne change pas.
- Rivières lointaines : berges sombres proportionnelles à la largeur écran, sommets glissés vers l'oeil le long du rayon de vue.
- Les noms de villes publient leur position préférée (`RegionLabelObstacles.names`) ; plaques d'armée, noms de régions et de rivières les évitent.

## Conséquences
- Les données dérivées lourdes (pyramide) doivent être retouchées par les deux commandes ci-dessus sur chaque machine de cache.
- La Tamise grise de la maquette de Londres est cuite dans la texture du glb généré : à traiter par régénération du modèle, pas côté client.
