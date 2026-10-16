# bm-uisound2 (Bevy) — termine
- `client/src/audio/farbus.rs` : `FarSource` (asset + decodeur enveloppe) = biquad passe-bas + reverb Schroeder (4 combs + 2 all-pass par canal), parametres partages `FarBusParams` (cutoff/wet/room_size) mis a jour chaque image par `battle::drive_far_bus` selon la hauteur de camera. Utilise par les sons spatiaux, les lits (`Bed`) et `battle_distant`. Les approximations `muffle_gain` / `distant_bed_gain` sont supprimees ; `dry` reste un gain.
- `ui_order` / `ui_order_refused` : `uisound::panel_sound` sur diplomatie, feudal, court ; tech emet directement.
- Ecart Godot : la bande lointaine s'applique a tous les sons de bataille spatiaux (Godot : seulement au-dela de `near_distance`) ; le filtre est global, pilote par la camera.
- Restes : refus d'ordre de bataille seulement teste unitairement ; pas d'ecoute reelle (muet impose) ; reverb non reglee a l'oreille.
- Tests : 1034 lib + audio_playback OK sans CENT_ANS_MUTE ; clippy propre.
