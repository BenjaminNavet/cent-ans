# 0123 — Budget de pixels de la mise à l'échelle 3D sur écran HiDPI

Date : 2026-09-29. Statut : acceptée. Complète l'ADR 0080 (MetalFX), sans la remplacer.

## Contexte
Le joueur signale des saccades sur la carte de campagne pendant les zooms et les déplacements. Sur son
écran Retina, la fenêtre fait 2624 × 1644 pixels physiques (4,31 M). Les échelles des préréglages de
l'ADR 0080 (Haute : MetalFX spatial 0,75) ont été mesurées en 1920 × 1080. En Retina, Haute
rend 2,43 M pixels, soit 2,1 fois ce qui avait été mesuré.

Les mesures du 29/09 ont été faites sur une machine chargée : charge 2 à 7, autres sessions
et programmes. Les voici à titre indicatif : panoramique p50 53-59 ms, scripts ~5 ms dans les pires
images. Le coût vient donc du rendu. La machine ne sera pas libre avant le lendemain. Le joueur a
demandé une **estimation** plutôt qu'une mesure.

## Estimation
Modèle linéaire en pixels rendus tiré des mesures de l'ADR 0080 (1080p, Haute, Metal) :
- d = 150 : 32,1 ms à 2,07 M px, 23,4 à 1,17 M, 17,9 à 0,52 M → ≈ 13,1 ms + 9,2 ms par M px ;
- d = 12 : ≈ 12,1 + 9,4 ms par M px ; d = 40 (Paris) : ≈ 26,6 + 5,0 ms par M px.

Pour l'écran du joueur (Haute 0,75 → 2,43 M px), ce modèle donne ≈ 35 ms à d = 150, ≈ 35 ms à
d = 12 et ≈ 39 ms à d = 40. Avec le budget de 1080p (1,17 M px, échelle 0,52), il donne
≈ 24, 23 et 32 ms, soit **−33 %, −34 % et −16 %**. Ces chiffres ne sont pas mesurés.

## Décision
- Les échelles des préréglages sont définies pour une référence de 1920 × 1080. Quand le choix
  « Automatique » s'applique et que le viewport 3D a plus de pixels, l'échelle devient
  `échelle × √(pixels de référence / pixels du viewport)`, avec un plancher de 0,5
  (`RenderQuality.budget_scale`).
- Les choix explicites du joueur (Désactivée, MetalFX qualité 75 %, performance 50 %) et Ultra
  (natif) ne changent pas.
- L'échelle est recalculée quand la fenêtre change de taille. Le libellé des Réglages affiche
  l'échelle réelle (« MetalFX spatial 52 % »).

## Conséquences
- En Retina 2624 × 1644, Haute rend 1364 × 855 pixels pour un affichage logique de 1312 × 822 :
  au moins un pixel rendu par point logique, soit la même netteté 3D qu'un écran 1080p non Retina
  de même taille. L'interface 2D reste en définition native.
- Le flou constaté à 0,5 en 1080p (ADR 0080 : étiquettes 3D, toits) ne se transpose pas ici :
  la densité de points est double. Il faudra quand même le vérifier à l'œil lors de la prochaine partie.
- La bataille partage le viewport racine et en profite aussi. Elle est surtout limitée par le
  processeur, donc le gain y sera plus faible.
- À faire sur une machine calme : mesurer le gain avec le banc `--bench-map` (fenêtre au premier plan,
  sinon le compositeur plafonne les images à 6,9 ms) et comparer à ces estimations.
