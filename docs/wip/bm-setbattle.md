# bm-setbattle (réglages de bataille / audio)
Constat : à la base, `unit_size`, `max_figures` (units/mod.rs `figure_scale`), `cinematic*` (orders/cinematic.rs), `audio_battle_music/sfx` (AudioSettings, Channel::Battle), `voice_advisor` (audio/advisor.rs), `voice_barks` (audio/battle.rs), `blood` (decor/blood.rs, flatten.rs), `auto_pause_on_alert` étaient déjà lus.
Fait : helpers purs testés `cinematic_on`, `slowmo_on`, `advisor_voice_on`, `unit_barks_on`, `blood::pool_scale` (+ tests ; `figure_scale` et `battle_buses_follow_their_own_sliders` existaient).
Reste : gerbes de sang (sprays) et membres tranchés (`battle_gore.gd`, niveau « complet »), `levels` de `data/fx/battle_gore.json` non lus (facteur 0,7 codé en dur), `show_zoc` (lot setcamp).
