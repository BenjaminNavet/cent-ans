# bm-bwater : eau de bataille

Branche `bm/bwater`. Voir `client/src/battle/scene/water.rs` + `water.wesl`.

## Etat
- Rivière (ruban de 9 sommets en travers, profondeur cuite dans la couleur de sommet), affluents et ruisseaux, mer côtière (houle trois échelles, Fresnel, écume de rivage), écume aux piles de pont (quad par pile, `VisibilityRange` 900 m).
- Hook dev : `CENT_ANS_BATTLE_FIELD=river,coast` (setup.rs) force rivière / côte sur la bataille personnalisée.
- Preuve : `client/src/battle/scene/proof_water.txt`, section de dump `battle_water`.

## Ecarts
- Profondeur par sommet au lieu du tampon de profondeur : pas de réfraction d'écran (fond de galets fixe).
- Eau éclairée par le PBR (pas émissive) ; normales d'ondes procédurales (pas de NoiseTexture).
- La « mer lointaine » de bdecor5 n'existait pas dans la base : `sea_rect` couvre déjà 6 km, un seul plan de mer.
- Ondulation de la mer en normales seulement (comme Godot), pas de déplacement de sommets.

## Reste
- Contrôle visuel (aucune capture dans ce lot), calage des couleurs éclairées.
- Douves (`features.rs`) gardent un StandardMaterial simple.
