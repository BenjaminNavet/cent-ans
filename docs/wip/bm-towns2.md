# bm-towns2 : villes de campagne au zoom proche (client Bevy)

Branche `bm/towns2` (fusion de bm/mats et bm/models). Dossiers : `client/src/campaign/{towns,markers}/`.

## Fait
- Diagnostic : sous Godot, le style par défaut (`map.town_style = maquette`, ADR 0158) ne crée AUCUNE ville 1:1, ni
  ville emblématique 1:1, ni lointain (`SettlementLayer._setup_towns`) : seuls maquettes + glb DN (fondu croisé à
  `near_distance` 1300, au-delà de toute portée, donc glb DN partout où la table le sert). Il n'y a pas de passage
  maquette → 1:1 à porter : c'était l'écart « faire mieux » du lot townfar qui enfonçait 391/392 maquettes à d < 45.
- `towns/style.rs` : `TownStyle` (données + `CENT_ANS_TOWN_STYLE`). Maquette : couche 1:1 jamais active, lointain coupé.
  Real : comportement VT conservé.
- Portée des maquettes sur la distance du rig (Godot `_update_fades`) au lieu de la distance œil-lieu.
- Hachage `String.hash` de Godot (djb2) pour variante, lacet, choix du glb DN ; vecteurs de test pris dans Godot 4.
- Onde de vent des bannières DN (`markers/banner.rs`, `banner.wesl`).
- Bâtiments blancs : déjà corrigés par la fusion de bm/mats (remappage `Building` → atlas, 1005 maillages) ; kit de
  repli (matériau `Kit`, couleurs de sommet) coloré, vérifié par `CENT_ANS_DN_MODELS=0`.
- Preuve (dumps, Paris xz 1651,2731) : d = 60 / 45 / 25 / 22 (plancher) : 392/392 lieux visibles, 0 enfoncé, couche 1:1
  inactive ; Paris = maquette emblématique (base 0,12 = sol 0,12, sommet 11,1), Saint-Denis = glb DN + bannière
  (base 0,14 = sol, sommet 3,9). Captures 25/60 : Saint-Denis visible derrière Paris, hors-les-murs texturés.

## Écarts
- Fondu de portée et fondu croisé maquette/DN : coupure franche (le fondu demande des matériaux transparents).

## Restes
- Faces de bannière des maquettes `Kit` (alpha 0) à la couleur du contrôleur (`maquette_kit.gdshader`) : repli seulement,
  jamais vu tant que la table DN sert 392/392 lieux.
- Ombres par type (`shadow_range`), découpage en tuiles/MultiMesh (perf).
- Trous du terrain et brume visibles autour de Paris : lots campvis2 / météo.
