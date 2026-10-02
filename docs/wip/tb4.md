# TB4 — conséquences visibles de la guerre et des fléaux (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB4 ». Branche `feat/tb4`, worktree
`/Users/jean_hubert/dev/gp-tb4`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb4_scars_test.gd`.
- [x] 1. Brûlis (`terroir_burn`) par-dessus la carte de couleur (SS2) et les matières (HB3) : appel déplacé après `sg_apply` / `hb_apply` dans `terrain.gdshader` ; couverture moyenne au loin (`campaign_life.gdshaderinc`) ; masque de terroir relu à son échelle (voir « Écarts »).
- [x] 2. Peste : `WarScars.set_plague_sites` (scènes `plague` résolues par `FolkScenes.edge_frame`) : fosses (rectangle de terre sombre, grande croix de bois) groupées au bord de la colonie, charrette des morts sur la route qui en sort (`CampaignLife._road_from`). Grossissement = `plague.scale_per_distance` (2,5) × distance de caméra, borné par `max_scale`, jamais sous 1:1 : taille à l'écran tenue jusqu'à `plague.max_distance` (60). Test : à 15 / 20 / 40, plus petit élément 15,7 / 15,8 / 16,0 px en 900p (charrette ; fosse ≈ 19,5 px), tous dans le champ.
- [x] 2 bis. Portes marquées : gardées dans les villes 1:1 ordinaires, croix sur toute la hauteur du vantail (2,2 m, 8,8 px au plancher de caméra 0,30 en 900p) ; **supprimées dans les villes emblématiques** (plancher 2,6 : ≈ 1 px).
- [x] 3. Champ de bataille : `WarScars.refresh` lit les événements `battle` (`get_events`, `get_pending_events`), marque à la position de l'armée (repli : centre de la province) ; tertre `battlefield.turns` tours, corbeaux `crow_turns`, débris `debris_turns` (`data/ui/war_scars.json`) ; taille des figurines d'armée, caché sous le brouillard de guerre et sur le parchemin.
- [x] 4. Siège : engins de `get_assault_odds(armée).engines` posés au bord du camp (enfants des figurines) : charpente et tas de bois, maquette sous échafaudage (`siege.almost_ready_turns`), engin prêt ; maquettes `assets/models/siege/*_lod.glb` (lecture seule), échelles procédurales.
- [x] Réglages `data/ui/war_scars.json`, schéma `data/schemas/war_scars_ui.schema.json`, pytest `tools/tests/test_war_scars_ui_schema.py`.
- [x] `game/tests/tb4_scars_test.gd` (4 points), `game/tests/tb4_shot.gd` (captures écrites, non lues), ADR 0155.

## Prochaine étape
Lot livré (4 points). Reste à la session principale : capture de contrôle, puis réglage à l'œil
des tailles et des places (`data/ui/war_scars.json`), des couleurs (`WarScarMeshes`).

Captures : `godot --path game --resolution 1600x900 --script res://tests/tb4_shot.gd --
--out=<dossier> [--only=brulis,peste,portes,bataille,siege]` écrit `tb4-<scène>.png` :
- `brulis` : Paris `(2213.2, 3203.9)` à 40, Île-de-France dévastée à 100 % ;
- `peste` : groupe des fosses et de la charrette au bord de Paris, cadré à 20 (tailles à l'écran
  imprimées : ligne `TB4 shot plague`) ;
- `portes` : ville 1:1 ordinaire la plus proche de Paris, première porte marquée au plancher de la
  caméra (taille de la croix imprimée : ligne `TB4 shot doors`) ;
- `bataille` : champ frais `(2227.2, 3209.9)` à 22 ;
- `siege` : première armée du joueur `(2215.8, 3196.7)` à 30, échelles prêtes et bélier presque prêt.
En jeu : `--scene=prov_ile_de_france:plague --devastate=prov_ile_de_france:100` (options de `CampaignLife`).

## Vérifications (02/10)
- `tb4_scars_test`, `smoke`, `fk_folk_test`, `fk5_incidents_test`, `tb1_seasons_test`,
  `cv1_campaign_life_test`, `tb2_declutter_test` : OK ; pytest `test_war_scars_ui_schema` : 4 OK.
- 16 tours réels (France, graine 1337, sans rendu) : 3 à 15 colonies pestiférées (≈ 4 fosses
  chacune), jusqu'à 6 champs marqués à la fois, effacés 4 tours après la dernière bataille de la
  province (Vérone : tours 5, 7, 9 → partie au tour 13) ; sièges : échelles en chantier puis
  prêtes, bélier en charpente, « presque prêt » à 1 tour, prêt, puis beffroi en charpente.

## Mesures du brûlis (02/10, après retouches)
`godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd -- --stats --season=summer
--distances=90,40,15 --hide=Clouds,Fires,Chimneys --param=weather_enabled=false
--param=cloud_shadow_amount=0 --param=hb_debug=9 --devastate=prov_ile_de_france:100` (Paris, moyenne
et écart-type RVB du tiers bas ; sans `--devastate` : témoin). **`--param=hb_debug=9` est
nécessaire** : il sort l'albédo du sol juste après le brûlis. Sans lui, la même commande donne deux
séries de chiffres selon les lancements (≈ ×1,2 sur la luminance, cause en aval de la sonde : ombres
de nuées ou météo reposées par la carte) ; avec lui, deux lancements donnent les mêmes chiffres.

| Distance | Témoin (σ) | Dévasté 50 % (σ) | Dévasté 100 % (σ) | Assombrissement à 100 % |
|---|---|---|---|---|
| 90 | 96 78 41 (22 16 15) | 89 72 39 (26 21 16) | 81 65 36 (28 23 17) | −16 % |
| 40 | 95 79 41 (19 15 14) | 87 71 38 (25 21 15) | 77 62 34 (28 23 15) | −19 % |
| 15 | 96 77 41 (19 13 13) | 76 60 33 (32 23 15) | 70 54 31 (29 21 14) | −27 % |

Écart-type du vert à 100 % : +44 % / +53 % / +62 % (parcelles carbonisées, roussies, intactes).
À 400, la zone mesurée sort de la province (96 86 39, inchangé).

Brûlis par parcelles (`terroir_burn`, `campaign_life.gdshaderinc`) : `hb_apply` rend la parcelle du
pixel (centre, tirage, poids) ; la dévastation est lue au centre de la parcelle, modulée par un
bruit large (la zone n'est plus un disque), et la parcelle entière est carbonisée (part
`burn_charred_share` 0,36), roussie (`burn_singed_share` 0,33) ou intacte. Brun-noir chaud
(`burnt_color`, ≈ 40 31 24 à l'écran), 22 % du sol d'origine gardé sous le noir
(`burn_ground_keep` : relief et sillons lisibles). Au loin (parcellaire HB éteint) : moyenne des
trois états. `burn_active` (posé par `CampaignLife`) coupe le calcul quand aucune province n'est
dévastée. Fumerolles : la couche `Fires` de `LifeEffects` en pose déjà (dévastation ≥ 25, un
hameau sur trois ; 7 en Île-de-France à 100 %) : rien ajouté, planches de bataille non touchées.

**Chiffres retirés** : le tableau précédent (71 59 41 contre 95 79 48, « avant » 112 92 59) et
« 112 93 59 → 94 79 48 après le correctif du masque » venaient de mesures sans la sonde, donc
mêlées à cette bascule de luminance. Le correctif du masque reste établi par le test
(`_check_burn` : le texel lu avec `uv` n'est pas brûlé, celui lu à l'échelle du masque l'est).

## Écarts à la spec
- **Masque de terroir décalé (défaut antérieur, corrigé ici)** : `TerroirMask` peint un masque carré
  à la même échelle sur les deux axes (1024 / plus grand côté), mais `terrain.gdshader` le lisait
  avec `uv = p / map_size`. Depuis la carte 7168 × 6144 (OM), finages, vignes et brûlis étaient
  décalés vers le nord d'un sixième de l'ordonnée (Paris : ≈ 457 px). Lecture corrigée
  (`p / max(map_size)`) : sans cela, aucun brûlis n'apparaît là où la province est dévastée.
  Conséquence visible sur toute la carte : les finages (champs autour des colonies) reviennent
  autour de leur colonie (effet sur les chiffres de TB1 non mesuré proprement, voir ci-dessus).

- **Mémoire des champs de bataille dans le rendu** (ADR 0155) : le pont ne donne ni position ni
  historique des batailles. La marque est posée à la position de l'armée de l'événement à la fin du
  tour (pas au point exact du combat) et vieillie par le rendu ; une partie rechargée ne retrouve
  que les batailles du dernier tour. Pour mieux : exposer dans `core/` un historique (tour,
  position).
- Un commit par point, mais les points 2 à 4 partagent `war_scars.gd` : le module entier est entré
  avec le commit de la peste, les commits 3 et 4 branchent le champ de bataille (`refresh`, `reset`)
  puis le siège (`ArmyMarkers.marker_ids` / `marker_of`).
- ADR numéroté 0155 (0154 = rotation musicale sur main) : à renuméroter si un autre lot TB l'a pris.
- Fichiers hors lot touchés, par petites additions : `army_markers.gd` (deux accesseurs),
  `life_folk/folk_scenes.gd` (`edge_frame`), `campaign_life.gd` (branchement, `--life-off=scars`).

## Points ouverts
- Retouches du 02/10 (brûlis par parcelles, peste lisible à 15-40, portes) non jugées à l'œil :
  captures écrites, non lues.
- Fosses grossies : à 40 de caméra, une fosse fait ≈ 650 m de long sur la carte et le groupe ≈ 3 km ;
  il peut recouvrir un hameau ou un bois voisin.
- Portes marquées seulement dans une ville 1:1 ordinaire chargée ; ni villes emblématiques ni
  villages sans plan. Le vantail est posé à 15 cm de la façade, en son milieu : peut mordre un
  encorbellement.
- `ss_shot.gd --stats` sans `--param=hb_debug=9` n'est pas répétable (bascule ≈ ×1,2) : cause non
  cherchée, hors lot ; les chiffres de TB1 pris ainsi sont à revoir.
- Fosses posées sans regarder l'occupation du sol (forêt, eau, autre colonie proche).
- La charrette des morts arrêtée s'ajoute à celle, en marche, de la scène FK4. Les sites de peste
  viennent de `FolkScenes` : avec `--no-folk` ou `--folk-off=scenes`, ni fosses ni portes.
- Beaucoup de pestes dès 1337 (3 à 15 colonies) : réglage du cœur (`map_scenes`), hors lot.
- Erreurs déjà présentes dans `smoke` et sans rapport : `region_labels.gd:95` (Rect2 négatif,
  TB2, des milliers de lignes), `hb_ground.gd:33` (JSON vide).
- La première exécution de `ss_shot.gd --stats` après un changement de shader peut donner des
  chiffres faux (cache de pipelines froid : 73 76 88 au lieu de 88 78 46) : relancer une fois.
