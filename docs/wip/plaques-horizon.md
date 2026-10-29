# PLAQUES-HORIZON — fondu des plaques lointaines

État : règle `horizon` de `data/map/settlement_markers.json` (schéma mis à jour), fonction pure
`HorizonRule::opacity` (`client/src/campaign/markers/catalog.rs`, test pur), appliquée aux noms de
lieux (`markers/mod.rs::update_labels`) et aux plaques d'armées (`armies/markers.rs::update_plates`).
Fondu = réduction d'échelle (`UiTransform`), car Bevy UI 0.20 n'a pas d'opacité de groupe et les
couleurs des plaques sont réécrites par `plate::update_plates`.

Prochaine étape : mesurer `labels_visible` / `plates_visible` à camp_0060 avant/après, régler
`range_ratio`. Bataille : non traité (cause différente : regroupement en pastilles au-delà de 480 m).
