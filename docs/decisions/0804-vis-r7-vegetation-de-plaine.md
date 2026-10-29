# 0804 — Végétation de plaine (VIS-R7)

Numéro attribué à la fusion. Amende les ADR 0161 (arbres généralisés), 0710 (« une forêt se lit comme une masse sombre ») et
0770 ; reprend le constat G2-7 de `docs/audits/2026-10-29-visuel-campagne.md` (refonte R7).

## Contexte

En vue moyenne, la plaine est criblée de points sombres réguliers (arbres isolés des haies et bosquets, gardés à 6 % à d 260 et
exemptés près du regard), et les massifs de l'Île-de-France sont des taches vert-jaune plus claires que les champs : le plancher
`forest_masses.floor 0.17` relève la valeur des masses de la carte de couleur au-dessus des cultures assombries. La bible (c)
demande la hiérarchie de valeur eau < forêt < pré < labour, et des saisons par classe (forêts rousses et or en automne, brun-violet
en hiver).

## Décision

- **Arbres isolés** (`data/art/campaign_forest_look.json`, `open`) : tous dessinés jusqu'à d 60, fondu jusqu'à d 150, plus aucun
  au-delà (`far_share 0`). L'exemption « près du regard » (`focus_keep_units`) s'efface avec la même rampe de distance caméra
  (`focus_cut 1`), sinon un disque d'arbres isolés resterait au centre de la vue lointaine. Les bosquets (`groves.far_share`)
  passent de 0,5 à 0,1 : quelques taches compactes, plus de trame de points. Les forêts ne sont pas touchées (`Role::Forest`).
- **Canopée** (`canopy.value` 0,88 → 0,6, `balance` 0,82 / 1 / 0,95) : la masse forestière vue en plaine est la canopée des
  imposteurs. Elle passe sous les cultures (valeur forêt Île-de-France 0,41 → 0,36 contre 0,38 à 0,44 pour les champs) et vire du
  jaune (H 74) au vert (H 83).
- **Masses forestières** (`data/ui/campaign_map.json`, `forest_masses.value_lift`, `terrain.wesl::dark_masses`) : le plancher
  relève toujours la teinte (la chroma de la couleur pleinement relevée est conservée, donc pas de noir grisâtre), mais la valeur
  n'est relevée que de `value_lift` (0,3 ; 1 = ancien comportement). Les massifs restent plus sombres que les cultures.
- **Saisons par classe** (`seasons.autumn` du même fichier) : la couleur d'automne de chaque classe (chêne, hêtre, résineux,
  haie, or) passe du shader aux données ; teintes plus rousses et dorées. Hiver : `winter.bare` tiré vers le brun-violet.
- **Haies du bocage** : le style généralisé ne dessine aucune haie alignée (`drop_hedges`). Les lignes sombres du bocage sont un
  motif de sol, livré par le parcellaire de R3 ; R7 ne l'implémente pas.

## Conséquences

- Moins d'imposteurs en vue moyenne (gain de coût) ; la « vie » de la plaine vient du parcellaire de R3.
- `terrain.wesl` est touché dans la seule fonction `dark_masses` (utilise `macro_amp.z`, jusqu'ici libre). Le bloc de paramètres
  `billboard.wesl::Params` gagne 5 `vec4` (couleurs d'automne par classe).
- Réglages en données avec schéma : `art_campaign_forest_look.schema.json`, `campaign_map_ui.schema.json`.
- Mesures avant/après : `docs/wip/vis-r7.md`.
