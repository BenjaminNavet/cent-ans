# bm-audio : directeur audio Bevy (client/src/audio)

Etat : directeur (playlists, rotation mélangée, fondus, banque, pools, voix, nappes campagne, ducking) + lot baudio (vague 4) : audio de bataille spatialisé (`battle.rs`, `cues.rs`, `speech.rs`), voir README client.
Reste : persistance de la rotation (user://music_rotation.cfg), discours du général (speech sha1, `speech.rs` fournit le chemin), musique de bataille par couches (battle_layers.json), advisor.gd, passe-bas/réverbération du bus lointain, limiteur/compresseurs, simulation des instances max en mode muet, hauteur du sol des régiments (y = 1,5 m fixe), `missile_kind` par id d'unité (le cœur le garde privé), impacts de pierre joués par le rendu du siège (`stones` = true ici).

## audio3 (vague 5, fait)
Fait : rotation persistée, couches de musique de bataille, volées via `ShotFeed`, bus lointain (paramètres + gain `dry`), conseiller, discours du général (message `PlaySpeech`), glue campagne (hints, ambiance, tour, ui_open). Voir README client.
Reste : vrais filtres passe-bas / réverbération (bevy_audio n'en a pas : décodeur enveloppé rodio), limiteur / compresseurs de bus, simulation des instances max en mode muet, hauteur du sol des régiments (y = 1,5 m), impacts de pierre par le rendu du siège, bulle du conseiller avec bouton de fermeture (toast aujourd'hui), l'écran de discours de bataille doit envoyer `PlaySpeech`, ambiance : parts forêt / champs approchées par le terrain des provinces (pas de splat), court_open perdu si le panneau se ferme par Échap, durée des voix estimée (pas de sonde du clip).

## uisound (vague 5, fait)
`audio/uisound.rs` : ui_order(_refused), ui_army_select, ui_recruit, ui_letter, ui_card, ui_build câblés (observateurs d'état + un crochet dans le panneau de province) ; bus lointain approché par gain (`muffle_gain`, `distant_bed_gain`). Reste : autres panneaux (diplomatie, tech) sans `ui_order`, vrai passe-bas/réverbération.

## audio4 (vague 8, fait)
- `audio/dynamics.rs` : limiteur dur Master (plafond -0,5 dB) + compresseurs Ambiance / Bataille / Voix + compresseur side-chain Musique (clé Voix), mêmes seuils, ratios, attaques, relâchements que `audio_buses.gd`. Pas de graphe d'effets dans bevy_audio : la loi de gain tourne au débit de contrôle sur un niveau estimé (somme des volumes des sinks vivants × facteur de crête), le gain multiplie le volume des sinks du bus (`channel_volume`, `music_volume`, ambiances). Écart assumé : pas de vrai traitement d'échantillons. Non couvert : musique de bataille (couches) et `battle_distant` ne passent pas par les gains.
- `audio/clip.rs` : durée réelle d'un clip (en-tête Ogg Vorbis / WAV), utilisée pour la tenue du ducking des discours (repli : estimation par caractères) et pour le pool virtuel.
- Instances max en muet : `claim_virtual` rejoue la politique `bank::claim` sur des voix virtuelles (fin = durée réelle, 1 s si inconnue) ; dump `audio.virtual_pools` / `audio.dynamics`. Pool des voix et des SFX du directeur seulement (le pool de bataille garde son `continue` muet).
- `court_open` : lu sur `CourtUi.screen == Council` (plus sur les messages `OpenPanel`) : Échap ou un autre panneau termine la musique « court ».
- Bus lointain (farbus) : seuls les événements `Bataille` démarrés au-delà de `near_distance_m` passent par `FarSource` ; gain `dry` limité à ceux-là ; les lits (beds) et sons proches sont secs comme Godot. `battle_distant` reste sur le bus lointain (choix du lot précédent, pas Godot).
- Reste : `battle/orders/speech.rs` utilise encore `spoken_seconds` pour les sous-titres ; pool de bataille muet.
