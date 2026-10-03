# Arbres qui clignotent sur la carte de campagne (en pause, 2026-10-03)

## Symptôme (joueur)
Déplacement clavier/bord, zoom moyen : des **paquets d'arbres** clignotent d'un coup.
Arbres généralisés (HC1, `map.tree_style = generalised`), imposteurs au-delà de 60.

## Outil
`game/tests/tree_flicker_probe.gd` (fenêtre réelle) : panoramique, journal par image des
changements de palier, ombres, visibilité des parties, niveaux de relief, recalages, écart
pied/sol (relatif à la hauteur d'arbre), tuiles installées déjà visibles (`POP`), métrique
« blink ». Options : `--step`, `--frames`, `--yaw`, `--keys` (vraie touche `map_pan_right`,
sans snap), `--views=x,z,d;…`.

## Écarté (mesuré)
- Caméra fixe : 0 clignotement. Pas sous-pixel (`--step=0.01`) : arbres ≈ fond sans arbres.
- La métrique blink à grand pas confond déplacement de petits objets et clignotement : ne
  pas s'y fier au-delà d'un pas sous-pixel.
- Tramage Bayer entre vues, bruit de feuillage, quads opaques : sans effet.
- Plafond de mip `max_lod = 2.5` : réduit le scintillement à 640 px seulement, effet faible à
  1280 px, et le lever aplatit les forêts → **ne pas changer**.
- Graines triées décroissant dans tous les tampons (OK).
- Recalage au sol après changement de niveau du relief : 1 image de retard, écart ≤ 22 % de
  la hauteur d'arbre → peu probable.
- Tuiles semées tardivement : apparaissent au bord du fondu (d ≈ fade_end), arbres de taille
  ~0 → invisibles. NB : `tile_prefetch_distance` (120) < `fade_end` (≈ 312-360) : préchargement
  sans effet en généralisé.
- Avec `--keys` : distance du rig constante (120), densité 1, `fade_end` constant.

## Piste en cours
Avec `--keys`, la sonde avance de ~19 unités par image (images lentes) : le vrai mouvement
fait de grands sauts par image. Prochaines étapes :
1. Bascules **d'ombres par partie entière** (`_apply_lod` : `cast_shadow` on/off quand une
   partie franchit `generalised_mesh_distance`) — vu 4 fois en 150 images à rig 60 ; vérifier
   si `tree_shadow_limit()` (70) couvre le zoom moyen du joueur.
2. Bascules de visibilité des parties (`vis`) : vérifier qu'aucune partie ne se coupe avec
   des arbres encore de taille > 0 (shader : distance + |dy|·0,5 à `view_origin`, CPU : sans dy).
3. Si rien : capture d'une séquence d'images pendant `--keys` et recherche du paquet à l'œil
   (budget captures : 2 lues sur 5).
