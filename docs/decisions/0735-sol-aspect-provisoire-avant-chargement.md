# 0735 — Sol de campagne : aspect provisoire avant le chargement du sol texturé

Statut : accepté (lot SOL, boucle juge 10-29).

## Contexte
Le juge de direction artistique a vu des patchs rectangulaires roses et mauves en damier sur le sol de
campagne au lancement Angleterre, absents au lancement France. Ce n'était ni une texture manquante ni du
KTX2 : le sol régional texturé (`ground::load_ground`, quatre paquets BC1 encodés sur le CPU) se charge sur
un fil de travail ; en qualité de texture « haute » (variantes locales 2048), le chargement dure de 2 s à
5,6 s selon la charge de la machine. Une capture ou un dump à 60 images tombe donc avant la fin, et le
sol n'est alors que la carte de couleur cuite, qui contient les parcelles agricoles en gros pixels (bords
en escalier, tons rosés). France était sombre (brume locale) et masquait le défaut. Preuve : dump
`campaign_terrain.material.regional_ground.bound = false` et `close_patchwork.parcels_bound = false` sur la
capture fautive, `true` ensuite. Reproductible : `CENT_ANS_SETTINGS_PATH=out/juge2/settings.json`,
`CENT_ANS_CAMERA="13.405,52.52,70"`, `--frames 60`, charge machine élevée.

## Décision
- Tant que le sol texturé n'est pas lié (`ground.bg.x < 0.5`), `terrain.wesl` affiche la carte de couleur
  floue (mip + 2) et non la carte brute : plus de damier, un ton régional continu.
- Gardes : `ground_pack_manifests_have_no_missing_or_pink_layer` (indices de couches uniques, aucune couche
  de couleur moyenne rose, variante 2048 cohérente avec la base) et `parcels_pack_loads_in_both_qualities`
  (le paquet de parcelles charge dans les deux qualités, drapeau `tex_flags.z`). L'échec de chargement du
  paquet de parcelles passe de `warn!` à `error!`.
- Tons des stades de culture (`data/fx/ground_detail.json`) assourdis : brun ocre au lieu de mauve.

## Conséquences
- Les outils de capture doivent attendre `regional_ground.bound = true` (sonder le dump) avant de juger le
  sol ; 60 images ne suffisent pas en qualité haute.
- Pas de bascule d'écran de chargement : le sol apparaît quelques secondes après l'entrée en campagne.
