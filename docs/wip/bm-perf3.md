# bm-perf3 : envoi des instances des soldats skinnés (Bevy)

Mission : `battle/units/skinned.rs::prepare_buffers` créait un tampon GPU neuf
(`create_buffer_with_data`) par lot et par image dès que `version` changeait (chaque image : animation),
2-4 ms d'envoi réel.

## État
- Sonde `skinned_upload` (section de vidage) : temps CPU de `prepare_buffers` et du programme de
  rendu complet par image, octets / écritures par image, sur les 600 dernières images avec lots.
- Mesure de référence (avant) : à faire, bataille de Crécy (script `perf3_crecy.txt` du bloc-notes).

## Prochaine étape
- Tampon partagé persistant (`SkinnedInstances`) : un emplacement par lot, `write_buffer` en place,
  réagencement seulement quand un lot dépasse son emplacement en fin de tampon.
