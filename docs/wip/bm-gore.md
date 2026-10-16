# bm-gore (Bevy) : sang et membres tranchés

Fait : `client/src/battle/decor/gore.rs` (port de `battle_gore.gd` + appels spray/sever de `battle_soldiers.gd`),
niveaux `levels` de `data/fx/battle_gore.json` (réglage `blood` 0/1/2), `blood.rs` : taille des mares = `levels.stains`
(0,55 en modéré au lieu de 0,7 codé en dur). Section de dump `battle_gore`.

Écarts : gouttes simulées sur CPU (un seul maillage reconstruit par image), pas de shader ; pas de gouttes de
renversés/blessés (pas d'événement côté client) ; `heavy_charge_mass` absent du sim (charge mortelle de cavalerie =
chance `heavy_charge`) ; poids `pieces` en ordre alphabétique ; pas de gouttes pour la cause `fire`.
Restes : brancher les renversements de charge (`knockdown`) quand le sim les exposera.
