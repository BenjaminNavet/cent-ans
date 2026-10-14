# 0355 — Carte composée façon Total War

Statut : accepté dans son principe (14/10, choix du joueur) ; la forme sera validée sur la
maquette 2D du lot C0 avant les lots lourds. Remplacera, à la fusion, la carte à géographie
réelle (ADR 0019, 0036, 0115-0116 pour l'emprise, 0138 et 0158 pour l'échelle 1:1 et le relief).
Spécification : `docs/design/2026-10-14-carte-composee.md`.

## Contexte
Après le relief exagéré (ADR 0353), le joueur veut aller jusqu'à une réinterprétation de la
géographie comme les cartes Total War : sélection de villes, repositionnement, simplification.
Choix : emprise Europe + Méditerranée (Oural et steppe lointaine retirés), ≈ 400 lieux jouables
(≈ 150-200 provinces), géographie reconnaissable mais caricaturée. Les batailles gardent le relief
réel de leur site.

## Décision
- La carte est **composée** : description vectorielle d'auteur (déformation, côtes, fleuves, lacs,
  crêtes, lieux) compilée en le même contrat que l'étape 0 actuelle de `geo build`
  (`heightmap.png`, `land_mask.png`, `rivers.geojson`, `coastline.geojson`, `map.json`) ; l'aval de
  la chaîne est conservé.
- **Déformation** réel → carte par cartogramme de diffusion sur la densité des lieux retenus
  (lieux régulièrement espacés à l'écran), corrigée par des épingles d'auteur ; nouveau CRS
  `composed` ; une seule fonction de projection par langage.
- **Relief procédural** sur des lignes de crête d'auteur (plaines plates, massifs, cols creusés) ;
  la pyramide de relief fin réel est abandonnée pour la campagne, détail rapproché procédural.
- **Rasters thématiques** (couverture, climat, forêts, zones humides, agraire) rééchantillonnés par
  la déformation inverse puis généralisés.
- Chaque lieu garde sa **vraie latitude/longitude** pour les règles qui en dépendent (météo, peste,
  sécheresse) et pour les batailles (relief et horizon du site réel).
- **Identifiants conservés** quand une province ou un lieu survit ; table ancien → nouveau et outil
  de réécriture automatique des références.
- Sauvegardes incompatibles ; développement sur la branche longue `cc-carte-composee`.

## Conséquences
- Perte de la fidélité kilométrique, du relief fin réel rapproché (pyramide, LiDAR, fleuves fins),
  d'≈ 1 250 lieux mineurs et d'≈ 20 factions de l'Est.
- Bandes seed bots, `points_per_step`, km des fleuves, objectifs de victoire et tests à
  coordonnées à recalibrer (lot C6).
- Les chantiers carte parallèles (GR, CV…) doivent se coordonner avec la branche.

## Addendum du 14/10 (lot CD) — relief : MNT réel déformé + contraintes d'auteur, à la place du procédural
Retour du joueur sur les captures en jeu : « les montagnes sont vraiment pas réalistes, on voit les
grilles, c'est très pixelisé ». Le relief procédural des massifs (C2/C2b) est abandonné comme source de
forme ; il ne reste qu'en CONTRAINTE.

**Décision.**
- Pour chaque pixel composé, la déformation inverse (`warp.json`) donne la position réelle ; la hauteur est lue
  dans le cache de relief réel (pyramide E0-E4 : ETOPO, Copernicus GLO-90/30, re-téléchargeable par `geo pyramid`)
  et passée en bicubique. L'étage composé L lit l'étage réel L + 1 (un pixel composé couvre ≈ 0,85 pixel réel).
  Commande `cent-ans geo compose-relief-real` (`compose/relief_real.py`, réglages `compose/relief_real.json`).
- Les hauteurs écrites sont VRAIES : l'exagération verticale façon RE (ADR 0353) reste celle du profil
  `relief_exaggeration.tres` à l'exécution, et la règle `massif_impassable` lit les mêmes hauteurs vraies
  (`geo navgrid` à rejouer).
- La géométrie d'auteur reste une contrainte, appliquée comme champs au relief réel fin (le détail réel est
  conservé partout ailleurs) : plancher le long des crêtes `massif` (le mur reste continu là où le réel est bas ;
  jamais au-dessus de 85 % du sommet réel voisin, ou 0,8 x hauteur d'auteur plafonnée à 1 700 m pour garder la
  règle), cols de `crossings.json` en encoches (plafond local), rampe côtière et côtes du masque composé, lacs
  plats, plateformes sous les lieux.
- Pyramide fine composée E1-E3 (180, 90, 45 m par pixel composé) régénérée dans le cadre composé, au format
  existant (`data/map/pyramid/`, hors git, manifeste `relief_pyramid.json`) ; le jeu la charge si le manifeste existe
  (`terrain_builder.gd`), sinon détail procédural C5.
- L'érosion simulée (`geo compose-erode`, `compose/erode.py`) reste disponible comme complément éventuel
  (zones étirées par le warp) ; elle n'est plus la voie principale.
- Limite des neiges en données (`data/ui/campaign_seasons.json` `snow.line_m`), calée sur les hauteurs vraies.

**Conséquences.** Vraies vallées ramifiées, arêtes et cirques ; plus de grille de 719 m de près. `heightmap.png`
et `height/` changent : rejouer `geo navgrid`, `geo splat`, `geo colormap`. Les massifs que le réel ne rend pas
assez hauts (Carpates, Atlas) ne sont infranchissables que par le plancher de crête d'auteur.
