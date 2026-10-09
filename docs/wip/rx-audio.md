# RX audio — état

Branche `rx/audio`, ADR 0247. Fait : gains par piste et par variante (outil `audio_mastering.py`), fondu
enchaîné, fallback en rotation (`fallback_every`), bataille enchaînée sans boucle, pièces exclusives
menu/guerre/bataille, limiteur + fondus de boucle, clic unique, langues de répliques, curseur
« Musique de bataille ». Test : `game/tests/rx_audio_test.gd`.
Reste : lancer les tests Godot (import en cours), pytest `tools/tests/test_rx_audio.py`, rapport.
Hors lot : nouvelles voix (es/it/de/grec/arabe), conversion mp3 -> ogg, écoute.
