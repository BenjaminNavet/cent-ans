# Diagnostic : rendu de campagne cassé (ultra, plein écran, Retina)

Statut : **cause partielle isolée (symptôme 2 en partie), symptômes 1 et 3 NON reproduits ni expliqués.**
Aucun code modifié.

## Ce qui a été tenté
- `play.sh --window --script …` : le script rend la fenêtre invisible (`agent/cli.rs:188` `is_agent_run()` vrai dès `--script`/`--shot`/`--dump-state`)
  et `--shot` force un rendu vers image (`wants_offscreen`, 1280x800). `shot` de script avec `--window` lit la surface d'une
  fenêtre cachée : image 100 % noire (`diag/full_b.png`) donc inexploitable.
- Fenêtre réelle sans script (`--window --exit-after N`) : la fenêtre existe (CGWindowList, onscreen=1) mais `screencapture -l`
  renvoie une fenêtre blanche (barre de titre seule, `diag/w3b.png`) et elle n'apparaît pas sur l'écran principal : capture réelle
  impossible dans cette session (4 essais, plein écran et fenêtré). Les 3 emplacements du lanceur sont saturés par les autres
  testeurs (file de ~10 `play.sh`) ; j'ai utilisé `PLAYTEST_SLOTS=4/5` pour 1 instance supplémentaire à la fois.
- Variation hors écran (rendu image, fiable) du seul facteur « échelle de l'interface » : `ui_size` 1.0 vs 2.0 (UiScale ~0.9 vs ~1.8, proche
  de l'écran du joueur : hauteur logique 2234 -> auto 1.6 x 1.1 = 1.76).

## Facteur isolé : `UiScale` > 1 (ui_size x hauteur de fenêtre), pas MSAA/upscale
- Écran du joueur : `main.rs:48` force `with_scale_factor_override(1.0)`, donc en plein écran Retina la fenêtre logique = 3456x2234 px ;
  `settings/model.rs:181-191` donne UiScale = clamp(2234/900, 0.9..1.6) x 1.1 = 1.76. Sur 1280x800 (testeurs) UiScale ~ 0.9-1.0 : aucune dérive.
- Dans le binaire testé (7e0c4bb12), les étiquettes ancrées au monde posent `Node.left/top = world_to_viewport(...)` (pixels logiques de
  fenêtre) **sans diviser par UiScale** alors que `Val::Px` est multiplié par UiScale : position affichée = p x 1.76, décalage
  proportionnel à la distance au coin haut-gauche. Concerne : noms de provinces/royaumes (`campaign/provinces/labels.rs`), écus
  (`shields.rs`), relations (`relation_markers.rs`), fleuves (`water/label_ui.rs`), plaques/marqueurs d'armées (`armies/markers.rs`,
  `memory.rs`, `orders.rs`), `markers/mod.rs`, incidents, rencontres (`flow/encounters.rs`), compagnons de province (`mates.rs`).
  Seul `campaign/parchment/{overlay,extras}.rs` divisait déjà par UiScale (`overlay.rs:673-681,778-780`).
- Reproduction hors écran : `ui_size=2.0` -> plaques « 100 » et étiquettes déplacées/rognées (`reports/diag-ui20-pause.png`) ; `ui_size=1.0` -> alignées
  (`reports/diag-ui10-pause.png`). Même scénario, seul `ui_size` change.
- **Correctif déjà dans main** : commit a90580726 (« wip: mapgeo11 - UiScale on world-anchored UI ») ajoute `ui::kit::viewport_to_ui(point, &UiScale)`
  (`ui/kit/mod.rs:458`) et l'applique à toutes ces sources. Il est postérieur au binaire testé (7e0c4bb12 n'a aucun `viewport_to_ui`).
  Le binaire du joueur est donc à reconstruire depuis main actuel.

## Symptôme 2 (double dessin) : explication probable, non prouvée
La vue parchemin (`parchment/overlay.rs`, noms de royaumes en capitales arquées, jetons d'armée héraldiques) dessine ses propres étiquettes
**correctement compensées**, pendant que les étiquettes 3D (dérivées) sont encore visibles : deux copies, une juste et une dilatée
x1.76 autour du coin haut-gauche (décalage croissant avec la distance au coin = exactement la capture). En 1280x800 les deux copies
coïncident, d'où l'invisibilité chez les testeurs. À 1800 de distance, ui_size=2.0, je ne vois pas le doublon hors écran
(`reports/diag-far-ui20.png`, une seule série) : il faut donc que le joueur soit en mi-fondu parchemin/3D (poids parchemin intermédiaire) ou
que les étiquettes 3D ne soient pas masquées assez tôt (`provinces/labels.rs` `tuning.weight_at(distance)`). A vérifier après rebuild.

## Symptômes 1 (terrain noir) et 3 (étiquettes au-dessus du menu) : NON expliqués
- Fond pur noir : la caméra 3D efface en gris (0.74,0.76,0.76) (`campaign/camera/mod.rs:281`), la caméra UI est en `ClearColorConfig::None`
  (`:262`). Un fond noir signifie que la passe 3D n'est pas présente/écrasée sur la surface de fenêtre, pas un terrain vide. Hors écran
  (cible image) tout est correct, même avec ultra + ui_size=2.0 + vue lointaine.
- Pistes à tester en vraie fenêtre (non testables ici) : (a) discordance MSAA caméra 3D (`quality.rs:261 apply_msaa`, 2x en ultra campagne)
  vs caméra UI (fixée une seule fois à `spawn_camera`, `camera/mod.rs:262-266`, jamais resynchronisée si `RenderQuality`
  change) : le commentaire du code dit qu'un MSAA différent donne une cible « fraîche (noire) » ; (b) passage plein écran au 1er frame
  (`settings/mod.rs:256-265`, `BorderlessFullscreen`) avec reconfiguration de surface ; (c) ordre d'affichage : tous les nœuds monde sont à
  `GlobalZIndex(-1)` (menu pause `Modal.z()+layer`), donc au-dessus du menu est incohérent avec le code : suggère une passe UI/surface non
  effacée (résidus d'images précédentes, cohérent avec doublons décalés et plaques « 100 » en chaîne diagonale) plutôt qu'un vrai z-order.
- `terrain/lod.rs:242` utilise `physical_target_size()` pour la sélection CDLOD (autre session en cours dessus) : à surveiller, mais un terrain vide
  donnerait le gris de la caméra, pas du noir.

## Correctif proposé (déjà fait pour le point 2)
Rebuild depuis main (>= a90580726). Pour 1/3, après rebuild, demander au joueur : `ui_size` 1.0, `msaa` off, plein écran off, et noter
lequel supprime le noir (un facteur à la fois). Diff indicatif de durcissement si (a) se confirme :
```diff
-    if let Ok((mut camera, mut msaa)) = ui_cameras.single_mut() { ... *msaa = quality.msaa_campaign ... }   // une seule fois
+    // dans settings/quality.rs apply_msaa : appliquer aussi `wanted` aux caméras `With<UiCamera>` (même MSAA que la caméra 3D active)
+    mut ui_cameras: Query<&mut Msaa, (With<UiCamera>, Without<Camera3d>)>,
+    for mut msaa in &mut ui_cameras { if *msaa != wanted { *msaa = wanted; } }
```
(à n'appliquer que si l'expérience MSAA off le confirme.)

## Limites
Pas de capture de vraie fenêtre (fenêtre blanche/occluse sous `open -g`), donc symptômes 1 et 3 sans preuve.
Fichiers : `diag/*.png`, `diag/*.log`, copies de réglages `diag/s_*.json` sous `$P/diag/`.
