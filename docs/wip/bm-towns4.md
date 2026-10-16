# bm-towns4 : LOD des maquettes de lieux (client Bevy)

Branche `bm/towns4` (base : towns2). Dossier `client/src/campaign/markers/` (+ une ligne de shader dans `towns/atlas`).

## Fait
- `markers/lod.rs` :
  - Fondu de portée tramé (Godot `visibility_range_fade_mode = FADE_SELF` + `_update_fades`) : chaque maillage d'une maquette (et la
    bannière DN) porte un `VisibilityRange` Bevy dont la fin vaut `portée_du_type + (distance œil - distance du rig)` ; plein
    jusqu'à `portée - fade_margin`, nul à la portée, tramé entre les deux. Mesure sur la distance du rig comme Godot, tramage Bevy
    (pas de matériau transparent). Le chaînon `building_atlas.wesl` appelle `visibility_range_dither` (le matériau du kit
    de repli est un fragment personnalisé).
  - `shadow_range` (`town_maquettes.json`, château 120 / abbaye 100 / village 100) et `model_shadow_distance` du préréglage
    (`RenderQuality`) : au-delà, `NotShadowCaster` sur les maillages de la maquette.
  - Faces de bannière du kit : les glb des six familles ont une seule surface `Kit` (alpha de sommet 0 = bannière) ; matériau
    `KitMaterial` (`kit_banner.wesl`, ExtendedMaterial) dont le fragment remplace ces faces par la couleur héraldique du
    contrôleur (uniforme par matériau, un matériau par couleur et par source ; repeint quand le lieu change de mains),
    équivalent de `maquette_kit.gdshader`. Le primitif `Banner` des anciens glb multi-surfaces est traité aussi.
  - Instanciation : Bevy regroupe en un appel de dessin les maillages de même (maillage, matériau) ; les glb sont partagés
    entre lieux, donc `draw_batches` << `scene_meshes` (dump `campaign_markers.lod`). Pas de MultiMesh par tuile : rien à
    construire, le culling reste par entité.
- Hachage de variante / lacet / glb DN : déjà `String.hash` de Godot (towns2) ; tests ajoutés sur `variant_of` et `yaw_of`.
- Maquettes enfoncées sous les villes 1:1 : sans objet en style `maquette` (défaut, aucune couche 1:1, towns2 : 0 enfoncé à d=22..60) ;
  en style `real` le comportement VT (écrasement par le poids de la couche) est conservé.

## Preuve (dumps `campaign_markers.lod`, Paris xz 1651,2731)
- d=880 : 570 maillages en bande de fondu, 346/392 maquettes visibles, 392 sans ombre ; d=60 : 392 visibles, 0 sans ombre, 0 enfoncé.
- `CENT_ANS_DN_MODELS=0`, d=400 : 392 `Kit` repeints, 174 lots de dessin (couleur de contrôleur par matériau) ; 0 erreur de shader.
- DN : 392 lieux, 392 lots (un glb par lieu, pas de partage). Tests : 1037 réussis.

## Écarts
- Une maquette du kit = un matériau par couleur de contrôleur (Godot : donnée d'instance), donc plus de lots de dessin que de modèles.
- Le fondu Godot est un `transparency` sur MultiMesh ; ici tramage Bevy (même aspect que `FADE_SELF`).
- Pas de découpage en tuiles (`tile_size`) : le culling et le regroupement sont ceux de Bevy.

## Restes
- Fondu croisé maquette du kit / glb DN (`near_distance` 1300 > toute portée : jamais visible tant que la table sert 392/392).
