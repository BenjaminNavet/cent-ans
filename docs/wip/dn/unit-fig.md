# DN unit-fig : champ `figure` des types d'unité

Branche `dn/unit-fig`. Décision du DA appliquée dans `data/unit_types/` (format `<famille>_<variante>`,
cf. `data/schemas/unit_type.schema.json`, motif `^(infantry|archer|cavalry)_[0-9]+$`).

| Type | figure |
|---|---|
| longbowmen | archer_0 |
| crossbowmen | archer_2 |
| genoese_crossbowmen | archer_1 |
| men_at_arms_foot | infantry_0 |
| flemish_pikemen | infantry_4 |
| urban_militia | infantry_2 |
| knights | cavalry_0 |
| mounted_sergeants | cavalry_3 |
| mounted_archers | cavalry_3 |

Engins (bombard, trebuchet, mangonel, siege_tower) : inchangés, pas de figurine.
Le champ est rendu seulement (core ne le lit pas). Tests : pytest schémas verts (2354), smoke Godot : voir rapport.
