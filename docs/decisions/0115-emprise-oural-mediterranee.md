# ADR 0115 — Emprise Oural–Méditerranée

## Contexte
La carte couvre lon −11→16, lat 35→60 (4096² unités de 719 m, EPSG:3035). Le joueur veut l'Europe
jusqu'à l'Oural et tout le pourtour méditerranéen. Le moteur suppose un monde carré de 4096 unités
(quadtree de relief, caméra, shaders, grille de navigation) ; ADR 0082 interdit de changer l'échelle
(≈ 200 constantes liées à 719 m/px) ; la pyramide de relief fin (≈ 2,8 Go) est publiée (ADR 0077).

## Décision
- Même projection, même échelle (718,9765625 m par unité). Le monde devient un rectangle de
  **7168 × 6144 unités** = 28 × 24 tuiles racines de 256 unités.
- Bornes projetées : x 2 169 486 → 7 323 110 m, y 775 684 → 5 193 076 m. Bord ouest inchangé, bas
  abaissé de 3 tuiles racines, haut relevé de 5 : toute coordonnée pixel existante garde x et gagne
  **+1280 en y** (lignes comptées depuis le haut). Les tuiles de la pyramide existante restent
  valides avec un décalage d'origine `(0, 5)` tuiles racines inscrit dans le manifeste : ni
  recuisson, ni nouveau paquet hébergé.
- La taille du monde se lit dans `data/map/map.json` (`size_px`) ; aucune constante 4096 ne reste
  dans le code (Rust, GDScript, shaders). Le quadtree de relief garde des tuiles racines de 256
  unités, en grille rectangulaire.
- Textures de base à pleine résolution sur tout le monde ; aucun fichier versionné au-delà de 50 Mo
  (découpage en tuiles sinon).
- L'Est et le Sud n'ont que le relief de base (ETOPO 15″) en v1.
- Les terres éloignées de toute graine de province (déserts, steppe kazakhe, Sibérie) restent hors
  provinces et infranchissables.

## Conséquences
- Surface ×2,6 : mémoire graphique et temps de chargement en hausse, à mesurer (lot OM1).
- Sauvegardes antérieures incompatibles (refus explicite au chargement).
- Tous les artefacts géo sont régénérés une fois ; les données écrites en pixels sont migrées (+1280 y).
