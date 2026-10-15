# bm-audio : directeur audio Bevy (client/src/audio)

Etat : directeur (playlists, rotation mélangée, fondus, banque, pools, voix, nappes campagne, ducking) + lot baudio (vague 4) : audio de bataille spatialisé (`battle.rs`, `cues.rs`, `speech.rs`), voir README client.
Reste : persistance de la rotation (user://music_rotation.cfg), discours du général (speech sha1, `speech.rs` fournit le chemin), musique de bataille par couches (battle_layers.json), advisor.gd, passe-bas/réverbération du bus lointain, limiteur/compresseurs, simulation des instances max en mode muet, hauteur du sol des régiments (y = 1,5 m fixe), `missile_kind` par id d'unité (le cœur le garde privé), impacts de pierre joués par le rendu du siège (`stones` = true ici).

## audio3 (vague 5, fait)
Fait : rotation persistée, couches de musique de bataille, volées via `ShotFeed`, bus lointain (paramètres + gain `dry`), conseiller, discours du général (message `PlaySpeech`), glue campagne (hints, ambiance, tour, ui_open). Voir README client.
Reste : vrais filtres passe-bas / réverbération (bevy_audio n'en a pas : décodeur enveloppé rodio), limiteur / compresseurs de bus, simulation des instances max en mode muet, hauteur du sol des régiments (y = 1,5 m), impacts de pierre par le rendu du siège, bulle du conseiller avec bouton de fermeture (toast aujourd'hui), l'écran de discours de bataille doit envoyer `PlaySpeech`, ambiance : parts forêt / champs approchées par le terrain des provinces (pas de splat), court_open perdu si le panneau se ferme par Échap, durée des voix estimée (pas de sonde du clip).
