# bm-perf3 : envoi des instances des soldats skinnés (Bevy)

Mission : `battle/units/skinned.rs::prepare_buffers` créait un tampon GPU neuf
(`create_buffer_with_data`) par lot et par image dès que `version` changeait (chaque image : animation),
2-4 ms d'envoi réel.

## État : TERMINÉ (prêt à fusionner)
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
| final : une `write_buffer_with` (staging direct) | 253-263 / 458-526 | 13,5-27,5 / 52-66 | 1 (880 Kio avec les trous) |

Médianes : `prepare_buffers` p50 1,39 → 0,26 ms (÷5), p95 11,5 → 0,46 ms (÷25). Le temps CPU du rendu
complet n'est pas comparable : charge machine 40-120 pendant les essais (le dernier lot à ~105), ses
écarts d'un essai à l'autre dépassent le gain attendu (~1 ms).

Campagne (vue de départ, 400 images) : 35 Kio et ~1 écriture par image, `prepare_buffers` p50 61 µs,
p95 0,8 ms, aucune panique.

Tests : 1030 réussis (`cargo test --features dev`, +1 : `instances_write_in_place_and_relayout_only_on_overflow`).

## Restes / pistes
- Envoi des seules instances visibles : non fait (aucun lot n'a de boîte englobante ; la
  file ne trie ni ne cull). Il faudrait une sphère par lot calculée par les producteurs
  (regiments, corpses, crew, armies, life/skin) et un test de frustum dans `queue_batches`.
- Lots inchangés (bataille en pause) : `regiments.rs` incrémente `version` à chaque image même si les
  poses et l'horloge sont figées ; comparer les octets comme `campaign/armies/skinned.rs` éviterait
  l'envoi en pause.
- Trous des emplacements (puissance de deux) : 880 Kio envoyés pour 632 Kio utiles ; un arrondi plus
  serré (×1,25 au multiple de 16) réduirait l'envoi sans réagencements fréquents.
- Blocage à la sortie (indépendant de ce lot) : 1 essai sur 9, le fil de rendu attend le fil principal
  en libérant la fenêtre winit (`Window` drop → `run_on_main`) pendant que le fil principal attend le
  fil de rendu (`RenderAppChannels::drop`) : interblocage à `AppExit`, processus à tuer.
