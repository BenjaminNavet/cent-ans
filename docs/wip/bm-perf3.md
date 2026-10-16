# bm-perf3 : envoi des instances des soldats skinnés (Bevy)

Mission : `battle/units/skinned.rs::prepare_buffers` créait un tampon GPU neuf
(`create_buffer_with_data`) par lot et par image dès que `version` changeait (chaque image : animation),
2-4 ms d'envoi réel.

## État
- Sonde `skinned_upload` (section de vidage) : temps CPU de `prepare_buffers` et du programme de
  rendu complet par image, octets / écritures par image, sur les 600 dernières images avec lots.
- `SkinnedInstances` (ressource du monde de rendu) : un tampon de sommets persistant partagé par tous
  les lots, un emplacement par lot (capacité = puissance de deux d'enregistrements, ≥ 16), réécrit en
  place ; réagencement compacté (et agrandi ×1,5 en puissance de deux, ≥ 64 Kio) seulement quand un lot
  déborde en fin de tampon. Quand la plupart des lots changent (≥ 8 et ≥ la moitié), une seule
  `write_buffer` de toute la plage utilisée au lieu d'une par lot. Lots vides : plus mis en file
  (`queue_batches`), plus de `remove::<InstanceBuffer>` par image.
- Mesure : `perf3_measure.sh <étiquette> 3` (bloc-notes de session) = Crécy (`Battle_crecy_defender`,
  `CENT_ANS_NO_DEPLOY=1`), 900 images, release, hors écran, muet ; section `skinned_upload`.

## Chiffres (release, Crécy, 76 régiments / 13 180 soldats, 632 Kio par image ; charge 40-120)
| | prepare_buffers p50 / p95 (µs) | rendu CPU p50 / p95 (ms) | écritures / image |
|---|---|---|---|
| avant (3 essais) | 1356-1539 / 7900-11600 | 17,7-20,8 / 35-58 | 76 `create_buffer_with_data` |
| tampon persistant, write par lot | 641-945 / 10700-11100 | 11,9-20,6 / 40-54 | 76 `write_buffer` |
