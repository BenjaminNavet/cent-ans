# bm-audio : directeur audio Bevy (client/src/audio)

Etat : directeur (playlists, rotation mélangée, fondus, banque, pools, voix, nappes campagne, ducking) + lot baudio (vague 4) : audio de bataille spatialisé (`battle.rs`, `cues.rs`, `speech.rs`), voir README client.
Reste : persistance de la rotation (user://music_rotation.cfg), discours du général (speech sha1, `speech.rs` fournit le chemin), musique de bataille par couches (battle_layers.json), advisor.gd, passe-bas/réverbération du bus lointain, limiteur/compresseurs, simulation des instances max en mode muet, hauteur du sol des régiments (y = 1,5 m fixe), `missile_kind` par id d'unité (le cœur le garde privé), impacts de pierre joués par le rendu du siège (`stones` = true ici).

## audio3 (vague 5, en cours)
Fait : rotation persistée (`music_rotation.json` à côté de settings.json), musique de bataille par couches (`battle_music.rs`), volées lues dans `ShotFeed` (`cues::ShotView`, plus de détection par munitions), paramètres du bus lointain (`cues::far_bus`, seul `dry` appliqué).
Prochaine étape : campagne (`campaign.rs` : MusicHints, AmbienceInput, PlayTurnSfx, ui_open), conseiller, discours du général.
