# bm-uiinfra
Fait : `ui/fader.rs` (fondu entre états : message `FadeToState`, fondu d'entrée automatique après toute transition, `ReduceMotion`, `CENT_ANS_NO_FADE=1`), `ui/art_plates.rs` (ressource `ArtPlates` : chargements, vignettes, fins), `ui/ink_glyph.rs`, `ui/zones.rs` (rectangles de `ui_layout.gd`), `codex/bubble_layout.rs` (`place_beside`/`free_y`, pur).
Écarts : fondu d'entrée automatique (Godot : appel explicite) ; les flux (menu, campagne, bataille) peuvent envoyer `FadeToState` au lieu de `NextState` pour le fondu de sortie.
Restes : brancher `FadeToState` dans flow/menu (dossiers d'autres lots) ; `place_beside` pas encore appelé par codex/bubbles.rs (bulles filles alignées sur la ligne du lien, `keyword_y`) ; `window_resize_grip`, `build_companion`, `build_hammer` (lot Colonies) ; migrer `flow/loading.rs` et `aftermath.rs` vers `ArtPlates`.
Ignorés : ui_build.gd, game_catalog.gd, audio_buses.gd (l'audio a déjà ses volumes), échelle d'interface (déjà dans settings).
