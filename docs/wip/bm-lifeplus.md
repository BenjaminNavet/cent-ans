# bm-lifeplus : suite de la vie de la carte (client Bevy)

Dossier : `client/src/campaign/life/`. Branche `bm/lifeplus`. Point de départ : `docs/wip/bm-life.md`.

## Fait
- Table `PROPS`/`ROLES`/`ACTIVITIES` de `FolkModels` déplacée dans `data/rules/folk_props.json` (schéma
  `data/schemas/folk_props.schema.json`), lue par `props.rs` ; emplacements et attelages calculés sur les bornes des glb
  (`glb_bounds`, accesseurs POSITION), comme `_slots_from_bounds` / `team` de Godot.
- Figurines (`figures.rs`) : maillages `CAM1` des figures de bataille cuits en pose de repos (variante, couleurs de
  livrée du rôle, tenues et boucliers écartés) à la place des capsules ; repli capsule si le fichier manque.
- Scènes de province FK4 (`scenes.rs`, `scene_layers.rs`) : 10 types, 4 archétypes, effets hors réservoir
  (cheminées éteintes, fumées de bûcher et d'émeute). `CENT_ANS_SCENES=<province>:<kind>,...` force des scènes.
- Incidents FK5 (`incidents.rs`) : sceau (UI) par décision `map`, clic sur le sceau ou sur la scène -> décision +
  caméra ; `CENT_ANS_FOLK_OFF=incidents` les coupe.
- Vie marine : sillage des navires en mer (`boats.rs`, `data/fx/sea_life.json` `wake`).
- Mouvement animal (`motion.rs`, `data/fx/animal_motion.json`) : bascule du corps des bêtes attelées, cahot et roulis
  des charrettes (lignes mesurées), broutage des troupeaux.
- Seuils de zoom via `ZoomView` (bateaux : `strategic`, oiseaux : `near`) ; LOD de la faune 0/1/2 avec hystérésis
  (`fauna::update_lods`).

## Écarts / reste
- Figurines non animées (pas de texture d'os) : marche = rebond du pied, poses de travail absentes ; atlas peint non
  utilisé (couleurs de sommet + livrée).
- Mouvement animal : pas d'animation de pattes, cou, queue, roues (shader de sommets de Godot) ; seulement ce qu'une
  transformation permet.
- Mer : les paramètres du shader de mer (fonds clairs, crêtes, estrans) de `sea_life.json` relèvent du lot water.
- Scènes : graine différente de Godot (ordre des types trié), disques des villes emblématiques remplacés par le rayon
  de maquette de `Places`, champs sans ouvriers de la disette seulement comptés (`idle_provinces`).
- Sceau : disque de cire + nombre (pas la gravure `HudStyle`), toujours visible (filtre `MapReadability` absent).
- Chemins de commerce du pont, cheminées 1:1, bannières de maquette, biomes pour faune et oiseaux : inchangés.

## Preuve
`--dump-state` (section `campaign_life`), Paris d=40 avec `CENT_ANS_SCENES=prov_ile_de_france:plague,...:fair` : 44 figurines cuites
(0 capsule, 35 maillages), 2 scènes (fair 28 + plague 19 éléments, 34 figurants, 13 accessoires), cheminées éteintes 1, fumée de bûcher 1,
3 charrettes attelées, `prop_geometry` 14 glb mesurés / 4 attelages ; Rouen d=60 : celebration 19 + flood 20 éléments. Faune (LOD) et sillage
couverts par tests seulement (pas de faune ni de navire à ces vues). Incidents : 0 décision `map` au tour 0 (tests unitaires).
