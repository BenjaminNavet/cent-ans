# bm-bdecor3 : décor de bataille Bevy, vague 5

Branche `bm/bdecor3`. Suite de `bm-bdecor2.md`.

## État
- `decor/flatten.rs` : carte RG8 (herbe couchée / sang sur l'herbe), alimentée par la sim (empreinte des
  régiments, front de mêlée, chutes détectées par baisse d'effectif) ; lue par `wind.wesl` (herbe).
- `decor/blood.rs` : décalques de sang (flaques, éclaboussures, traînées de fuyards), maillage fusionné, réglage `blood`.
- `decor/features.rs` : feux de camp (flammes + fumée via `SmokeSources`), douves, mer des champs côtiers.

## Reste
- BuildingKit (maisons du site en modèles), chevaux des lignes de chevaux, cimetières (murs), panorama peint,
  mer lointaine de l'horizon réel, imposteurs d'arbres, atlas FA7, groupes d'herbe (`battle_grass_groups.gd`).
- Shaders animés de flamme / eau / mer ; lueur nocturne des feux.
