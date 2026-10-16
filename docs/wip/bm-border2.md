# bm-border2 : ordres de bataille (contour, zone de déploiement, trajets, rejeu +/-, infobulles d'aptitudes)

Fait :
- `battle_formation_outline.gd` -> `battle/orders/outline.rs` : états Sélectionnée (or pâle teinté livrée), Survolée, Ennemie survolée (pointillée), Ennemie ciblée (rouge pulsé), Au corps à corps (trait livrée / rouge pâle), rien pour déroute/absente. Dessiné en gizmos drapés sur le relief (pas de `Decal`), marge de 3 m comme Godot. Dump `battle_orders.overlay` : `outline_dashed`, `outline_pulsing`, `outline_engaged`.
- `deployment_zone.gd` + `.gdshader` -> `battle/orders/zone.rs` + `zone.wesl` : maillage 10 m drapé (+0,6 m), liseré or, halo intérieur, hachures de bordure, voile, fondus avec la distance caméra, anti-moiré. Remplace le simple cadre en gizmos de `deploy.rs`.
- Trajets : `marks::draw_route` en tirets (3 m / trou 2 m, motif continu d'un tronçon à l'autre, `overlay::dashed_line`) comme `_dashes` ; segments par ordre en file déjà routés par le cœur.
- Rejeu +/- : en rejeu, `battle_speed_up/down` parcourent ×1..×8 de la barre (`hud/speed.rs::step_replay_speed`).
- Infobulles d'aptitudes : durées/recharges formatées comme `RuleValues.text` (virgule, décimales) via `codex::rules::number`.

Écarts : contour en gizmos (largeur de trait fixe, pas de trait plus fin pour « corps à corps ») au lieu de décales ; pas de réduction de boîte sur l'eau profonde (inutile avec des gizmos).
