# bm-bunits — unités de bataille (Bevy)

État : terminé pour le périmètre du lot (branche bm/bunits).
- Soldats skinnés animés sur GPU (CAM1/CAB1 + manifeste fusionné), clips par style et par état, allures de cavalerie, apparences par type d'unité (looks), cadence suivant la vitesse, LOD par régiment (plein / moyen / lointain) et imposteurs au-delà de 300 m.
- Bannières (mât + étoffe ondulée) et repères (plaques UI, regroupement, dépilage, clic = `SelectBattleUnits`).
- Dump : `battle_units`, `battle_banners`, `battle_markers`. Crochet de dev : `CENT_ANS_UNITS_VIEW=<id>,<distance>[,<lacet>]`.

Reste (fichiers Godot) : cadavres / blessés / sang / membres (battle_soldiers.gd, battle_gore.gd, battle_dropped_arms.gd), armoiries de poitrine et cartes FG3 (battle_soldier_skinned.gdshader), fondu de cycle de mêlée et réactions aux pertes (`cycle_blend`, `hit`), étendards portés (battle_standards.gd), bannières royales et de maison (battle_banner_layer.gd `cloth_for`), vent selon la météo seulement par défaut du fichier, atlas d'imposteurs cuit (battle_impostors.gd : ici silhouette procédurale), ombres portées des figurines (pipeline instancié sans ombre), infobulle au survol et clic droit des repères.
Non vérifié : rendu en mêlée / tir / déroute (aucun ordre donné dans les runs de dump), LOD plein à 14 m vu une fois.
