# fix-render (lot 1) — état

## Fait
- `--visible-window` (client/src/agent/cli.rs, main.rs, run_bg.sh) : vraie fenêtre visible, rendu vers la swapchain même avec `--script/--shot` (`shot` = `Screenshot::primary_window`), bundle sans `LSBackgroundOnly`. Reste muet. Test `visible_window_keeps_the_real_window_and_swapchain`.
- Durcissement MSAA (cause la plus probable des bloquants 1 et 2) : `settings/quality.rs apply_msaa` synchronise chaque frame la caméra UI sur le MSAA de la caméra 3D active (Campagne/Bataille) ; la caméra de campagne reçoit `msaa_campaign` dès son spawn. Test `ui_camera_follows_the_world_camera_msaa`.
- UiScale (3) : campagne déjà couverte par a90580726 (toutes les sources utilisent `viewport_to_ui`, le parchemin divise une seule fois via `ProjectionCtx`, pas de double division). Trouvé et corrigé en bataille : plaques d'unités (`battle/units/markers.rs`) et numéros de waypoints (`battle/orders/marks.rs`) ne divisaient pas.

## Analyse de la cause (hypothèse, non prouvée en fenêtre réelle)
Bevy clé la texture principale d'une vue par (cible, format, échantillons). Une caméra UI (`ClearColorConfig::None`) dont le MSAA diffère de la caméra 3D reçoit sa propre texture, jamais effacée, et le blit final remplace le monde par elle : fond noir (bloquant 1), étiquettes des images précédentes qui s'accumulent, y compris sous/sur le menu pause et en doublons décalés (bloquant 2, bien que tous les nœuds monde soient à `GlobalZIndex(-1)` : l'ordre de pile est correct dans le code, vérifié dans `bevy_ui::stack`). Le MSAA de la caméra UI n'était posé qu'une fois à l'entrée de la scène, alors que `apply_msaa` le faisait évoluer sur la caméra 3D (qualité résolue après coup : adaptateur, réglages).

## Preuves
- Hors écran, ui_size 2.0, ultra : étiquettes/écus/plaque « 620 » alignés sur Paris/Coucy/Orléans, étiquettes sous les panneaux (capture ui2.png du bac à sable).
- Fenêtre réelle : IMPOSSIBLE dans cette session. `open -g` donne une fenêtre `onscreen=0` jamais présentée (screencapture : blanc ; `shot` swapchain : noir même pour le menu principal, donc l'outil ne distingue pas le bug). Pas de preuve avant/après du noir.

## Procédure pour le joueur (si le noir persiste après rebuild depuis cette branche)
1. Reconstruire (`tools/launch.sh`), plein écran, qualité ultra : le noir a-t-il disparu ?
2. Sinon un facteur à la fois dans Réglages : MSAA = Désactivé ; puis Plein écran off ; puis Taille de l'interface 1.0. Noter lequel supprime le noir et le dire.
3. Lancer avec `RUST_LOG=wgpu=warn,bevy_render=debug tools/launch.sh` et joindre le log.

## Restants / hors lot
- Impossibilité de vérifier le z-order au-dessus du menu pause en fenêtre réelle.
- Pistes non traitées : `terrain/lod.rs:242` (`physical_target_size`), `scale_factor_override(1.0)` (`main.rs:48`, fenêtre logique 3456x2234 en plein écran Retina : toutes les textures d'UI sont rendues 2x plus grandes que nécessaire).
