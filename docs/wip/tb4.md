# TB4 — conséquences visibles de la guerre et des fléaux (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB4 ». Branche `feat/tb4`, worktree
`/Users/jean_hubert/dev/gp-tb4`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb4_scars_test.gd`.
- [x] 1. Brûlis (`terroir_burn`) par-dessus la carte de couleur (SS2) et les matières (HB3) : appel déplacé après `sg_apply` / `hb_apply` dans `terrain.gdshader` ; couverture moyenne au loin (`campaign_life.gdshaderinc`) ; masque de terroir relu à son échelle (voir « Écarts »).
- [x] 2. Peste : `WarScars.set_plague_sites` (scènes `plague` résolues par `FolkScenes.edge_frame`) : fosses et charrette des morts au bord de la colonie, portes marquées d'une croix sur les façades du plan de la ville 1:1 (`CampaignLife._town_plan`), échelle 1:1, sous `plague.max_distance`.
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
- `peste` : première fosse au bord de Paris `(2220.0, 3200.8)` à 0,5 (5 fosses, charrette) ;
- `portes` : première porte marquée `(2212.8, 3204.2)` à 0,12 (14 portes dans Paris) ;
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

## Mesures du brûlis (02/10)
`godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd -- --stats --season=summer
--distances=1100,400,90,40,15 --hide=Clouds --param=weather_enabled=false --param=cloud_shadow_amount=0
--devastate=prov_ile_de_france:100` (Paris, moyenne RVB du tiers bas ; sans `--devastate` : témoin).

| Distance | Témoin (non dévasté) | Dévasté, brûlis sous la carte (avant) | Dévasté, brûlis par-dessus (après) |
|---|---|---|---|
| 90 | 95 79 48 | 112 92 59 | 71 59 41 |
| 40 | 94 79 48 | 111 92 59 | 73 62 48 |
| 15 | 95 77 48 | 113 91 58 | 61 53 44 |

À 1100 et 400, le tiers bas de l'image sort de l'Île-de-France : pas d'écart (88 79 46 / 95 85 46).
« Avant » : même masque corrigé, ancien ordre (le brûlis ne noircissait rien : la zone dévastée
ressortait plus claire, champs remplacés par la friche). Avant tout correctif, dévasté = témoin
(95 78 48 à 90) : le masque était décalé (voir ci-dessous) et le brûlis recouvert.

## Écarts à la spec
- **Masque de terroir décalé (défaut antérieur, corrigé ici)** : `TerroirMask` peint un masque carré
  à la même échelle sur les deux axes (1024 / plus grand côté), mais `terrain.gdshader` le lisait
  avec `uv = p / map_size`. Depuis la carte 7168 × 6144 (OM), finages, vignes et brûlis étaient
  décalés vers le nord d'un sixième de l'ordonnée (Paris : ≈ 457 px). Lecture corrigée
  (`p / max(map_size)`) : sans cela, aucun brûlis n'apparaît là où la province est dévastée.
  Conséquence visible sur toute la carte : les finages (champs autour des colonies) reviennent
  autour de leur colonie ; les chiffres `--stats` de TB1 près de Paris changent (40 : 112 93 59 →
  94 79 48).

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
- Jugement visuel non fait (captures écrites, non lues) : tailles (`battlefield.radius` 8,
  engins 4 / 5 / 7,5 dans le repère des figurines où un homme fait 4,3 et le camp 13 de large),
  places des engins au bord du camp (x ≈ 12,5-13), lisibilité des portes (vantail de 1,15 m posé
  à 15 cm de la façade, au milieu de celle-ci : peut mordre un encorbellement).
- Portes marquées seulement quand la ville 1:1 est chargée ; villages sans plan : fosses et
  charrette seulement.
- Fosses posées sans regarder l'occupation du sol (forêt, eau, autre colonie proche).
- La charrette des morts arrêtée s'ajoute à celle, en marche, de la scène FK4. Les sites de peste
  viennent de `FolkScenes` : avec `--no-folk` ou `--folk-off=scenes`, ni fosses ni portes.
- Beaucoup de pestes dès 1337 (3 à 15 colonies) : réglage du cœur (`map_scenes`), hors lot.
- Erreurs déjà présentes dans `smoke` et sans rapport : `region_labels.gd:95` (Rect2 négatif,
  TB2, des milliers de lignes), `hb_ground.gd:33` (JSON vide).
- La première exécution de `ss_shot.gd --stats` après un changement de shader peut donner des
  chiffres faux (cache de pipelines froid : 73 76 88 au lieu de 88 78 46) : relancer une fois.
