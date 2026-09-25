# Sons d'interface (UB1)

Découpés hors ligne par `tools/cent_ans_tools/ui_sounds.py` dans des fichiers déjà présents :
la banque AU1 (`battle/`, enregistrements Freesound CC0, voir `../SOURCE.md`) et les effets
procéduraux du projet (`sfx/`). Licence : CC0 pour les dérivés des sons Freesound, celle du
projet pour les dérivés de `sfx/`. WAV mono 16 bits, 44,1 kHz.

| Fichier | Rôle | Sources | Traitement |
|---|---|---|---|
| `ui/order.wav` | Ordre donné : un coup de tambour sec | `battle/drum_1.ogg` | first drum hit, 0.45 s, fade-out |
| `ui/order_refused.wav` | Ordre refusé : choc sourd et grave | `battle/shield_bash_1.ogg` | shield bash pitched down (×0.7), low-passed at 900 Hz, 0.4 s |
| `ui/alert.wav` | Alerte : un coup de cloche | `battle/bell_toll_1.ogg` | first bell stroke, 1.8 s, long fade-out |
| `ui/letter.wav` | Lettre reçue : parchemin déplié puis sceau de cire | `sfx/page_turn.ogg`<br>`battle/shield_bash_1.ogg` | page turn slowed (×0.92) + muffled shield tap as the wax seal, 0.8 s |
| `ui/recruit.wav` | Recrutement : roulement de tambour de levée | `sfx/march_drum.ogg` | march drum, 1.1 s, fade-out |
| `ui/build.wav` | Construction : deux coups de maillet sur le bois | `battle/ram_hit_1.ogg` | battering-ram hit pitched up (×1.9) to a mallet, twice, 0.6 s |
| `ui/army_select.wav` | Sélection d'une armée : piétinement de la troupe | `battle/march_bed.ogg` | one second of marching feet, fade-in and fade-out |
| `ui/card.wav` | Clic sur une carte d'unité : tintement de métal | `battle/sword_clash_1.ogg` | attack of a sword clash, high-passed at 900 Hz, 0.14 s |
