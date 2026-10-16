# bm-batmo : atmosphère de bataille (V12, V13)

Lot `batmo` de la migration Bevy (spécification : `docs/wip/bm-portaudit.md`, lignes V12, V13).
Branche `bm/batmo`, worktree `gp-bm-batmo`.

## Fait
- Ciel HDRI visible : déjà en `Skybox` (bvis3) ; ajout du **disque solaire** (hdri_sky.gdshader) cuit
  dans le cube de fond seulement (le cube de lumière reste sans), ciels `sunny` et `battle.<météo>.sun_disc`
  (`battle_lighting.json` `sun_disc`). `lighting.rs` (`SunDisc`, `sky_radiance`).
- **LUT lift/gain** : passe `FullscreenMaterial` après le tonemapping (`scene/grade.wesl`, composant
  `BattleGrade`) = formule de la LUT Godot (balance, lift/gain/gamma par canal, virages ombres/hautes
  lumières, contraste S, saturation, saturation des ombres), en sRGB, sans quantification. `ColorGrading`
  de Bevy reste neutre en mode `lut` ; `grade.mode = "bevy"` rend l'ancienne approximation.
- **Pluie et neige** : `decor/precipitation.rs` + `.wesl` ; un maillage statique d'un quad par goutte,
  placé par le shader (graine + temps), boîte qui suit la caméra (émetteur Godot à `(0, 30, -45)` en
  repère caméra), effacement près de l'objectif et aux faces de la boîte.
- **Brouillard volumétrique** : `decor/volumetric.rs` ; `VolumetricFog` sur la caméra, `VolumetricLight`
  sur le soleil, boîte « AirVeil » (2 x 320 m autour de la caméra) et nappe « GroundMist » par brouillard
  (texture 3D de densité : chute avec la hauteur, fondu des bords). Seuil de qualité de
  `render_quality.json` (`volumetric` : off / weather / always), forçable par `CENT_ANS_VOLUMETRIC`.
- **Fumées d'horizon** : `decor/horizon_smoke.rs` + `.wesl` (battle_horizon_smoke.gdshader), 1-4
  colonnes à 2,6-8,5 km (`horizon.json` `silhouettes`), aucune sous pluie ou neige, teinte du soleil,
  brouillard de distance.
- Données : `data/fx/battle_weather_fx.json` (+ schéma `fx_battle_weather_fx.schema.json`),
  `battle_lighting.json` (`grade.mode`, `sun_disc`).
- Crochet dev `CENT_ANS_BATTLE_WEATHER=rain|snow|fog|clear` (`scene/setup.rs`, bataille par défaut).
- Section de dump `battle_weather` ; `battle_lighting` gagne `grade_mode`, `grade_pass`,
  `cameras_with_grade_pass`, `sun_disc`.

## Écarts assumés
- Pas de particules GPU dans Bevy : gouttes ancrées dans le monde qui bouclent dans la boîte (pas de
  durée de vie) ; flocons ronds, traits adoucis et effilés (Godot : quads pleins).
- Bevy ajoute l'ambiance du volumétrique sans la pondérer par la densité : `ambient_intensity` 0 (Godot
  injectait 0,35).
- Sites des fumées tirés ici (les silhouettes de villages de l'horizon ne sont pas portées).

## Reste
- `add_shower` (averse sous un ciel qui n'est pas celui de la pluie, carte historique).
- Silhouettes de villages et châteaux de l'horizon (battle_horizon.gd `_build_silhouettes`) : les fumées
  devraient alors partir de leurs sites.
- Brouillard de hauteur de Godot (`fog_height` 12 m par brouillard) dans le brouillard de distance.

## Preuves (16/10)
- Vidages `--state Battle --frames 300`, `CENT_ANS_BATTLE_WEATHER=` rain / snow / fog / clear (qualité
  `high` = volumétrique `weather`), aucune erreur de shader dans les journaux :
  pluie 9 000 gouttes, neige 7 000 flocons (boîte suivant la caméra) ; volumétrique actif pluie/neige/
  brouillard (caméra + soleil + 1 volume, 2 avec la nappe au brouillard), coupé par temps clair ;
  fumées : 2 colonnes à 3,6 et 3,9 km (clair, brouillard), 0 sous pluie/neige ; passe d'étalonnage sur
  la caméra (3 étalonnages, lift du brouillard [0,006 0,006 0,008]) ; disque solaire 6 (clear_midday),
  0 sur ciels couverts.
- Tests client 1136 + annexes, clippy `-D warnings`, schémas (1216) verts.

## Prochaine étape
Lot prêt à fusionner. Contrôle visuel par le joueur ou la session principale (pas de capture ici) :
densité perçue de la pluie (traits de 12 mm, comme Godot), intensité du volumétrique, disque solaire.
