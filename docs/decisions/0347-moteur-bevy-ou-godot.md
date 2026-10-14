# 0347 — Moteur : migration vers Bevy (verdict du prototype P0)

Statut : accepté (15/10).

## Contexte
L'ADR 0093 avait gardé Godot face à Unreal. Le 13/10, le joueur a placé la contrôlabilité par les agents
au-dessus de la vitesse et de la beauté, et accepté une réécriture complète en Rust. Le prototype P0
(`tools/proto_bevy/`, spec `docs/superpowers/specs/2026-10-13-proto-bevy-design.md`) rend la même scène
dans les deux moteurs (`scene.json`) : terrain texturé, 5 000 fantassins instanciés qui marchent,
200 000 brins d'herbe, 2 000 sapins, une grange, un panneau parchemin et un trajet de caméra scripté.
Seuil fixé avant la mesure : Bevy au moins aussi rapide (p50 et p95), pas visiblement moins beau, et
nettement meilleur sur le critère 3 (contrôle par agents).

## Mesures (banc n° 4, 15/10 vers 1 h 25, machine calme, charge 0,9-4,8)
Trois passes par moteur, en alternance, qualité `comparable`, ombres de l'herbe et des sapins coupées,
fenêtres visibles au premier plan (`PROTO_FOREGROUND=1`, aucune image forcée côté Godot, Bevy
présente dans sa fenêtre). 1 600 × 900, vsync coupée.

| Moteur | Images | p50 (ms) | p95 (ms) | Draw calls | Passes échouées |
|---|---|---|---|---|---|
| Bevy 0.20.0 | 998 | **61,8** | **67,2** | 31 | 0 |
| Godot 4 | 588 | 102,3 | 103,0 | 94 | 0 |

Passes très reproductibles (Bevy 62,3 / 61,7 / 61,4 ; Godot 102,3 ×3). Les bancs précédents
(n° 1 à 3) étaient invalides : fenêtre recouverte, donc scène non dessinée ; fenêtre Godot masquée,
avec `force_draw`, entre 170 et 265 ms. Seule une fenêtre visible donne une mesure honnête sur macOS.

1. **Vitesse** : Bevy est 1,65 fois plus rapide en p50 et 1,53 fois en p95. ✔
2. **Rendu** : moyennes RVB des 300 lignes du haut et du bas à moins de 5 % de Godot en `comparable`
   (Godot retombe à ±1 sur ses références) ; montages côte à côte semblables (même grange, mêmes
   murets, même lumière). En `best`, Bevy est plus chaud et plus clair. La capture Godot `best` à
   t = 20 s sort noire (défaut du client Godot, sans incidence sur le verdict). Jugement à l'œil du
   joueur à faire (`tools/proto_bevy/out/compare_*.png`, hors git). ✔ (provisoire)
3. **Contrôle par agents** : même consigne pour un agent Sonnet par moteur (« herbe à 50 % dans le
   code, vérifier sans capture »). Les deux réussissent, vérification par `--dump-state`
   (200 000 → 100 000).
   - Bevy : 95 s, 56 k tokens, 6 appels d'outil, lancement avec build ≈ 15 s.
   - Godot : 135 s, 59 k tokens, 8 appels, lancement ≈ 24 à 37 s. Le mode headless ne produit pas
     de vidage (0 draw call).
   - Recompilation Bevy après une retouche de rendu : 0,8 s avec `dynamic_linking`, 3,5 s en
     release.
   - Gains structurels : en Bevy, tout est en Rust, le client appelle la simulation directement
     (plus de frontière FFI de 366 `#[func]` ni de dictionnaires), l'état est interrogeable en
     texte (ECS sérialisable, Bevy Remote Protocol) et les tests sont `cargo test`.

   Avantage net, plus structurel que chronométré. ✔
4. **Interface** : le panneau parchemin demande 116 lignes en Bevy contre environ 45 en Godot, plus
   une police TTF chargée à la main (sans elle, Bevy n'affiche que de l'ASCII ; avec EB Garamond,
   les accents passent). Bevy n'a ni thème, ni infobulle, ni texte enrichi intégrés. C'est le vrai
   coût de la migration (≈ 30 000 lignes d'UI GDScript) ; il se paie une fois, avec une boîte à
   outils d'UI maison.

## Décision
Le seuil est atteint : **migration complète vers Bevy**, en Rust, avec Bevy épinglé à une version
exacte. Le fork vendu dans le dépôt est reporté au premier correctif moteur nécessaire
(`[patch.crates-io]`). Godot reste le client jouable sur `main` jusqu'à la parité ; la migration se
fait sur la branche locale `bevy-migration` (architecture : ADR 0357).

## Conséquences
- Le prototype reste dans `tools/proto_bevy/` comme banc de référence (`bench.py`, mode
  `PROTO_FOREGROUND` pour toute mesure future).
- Le coût dominant de la migration est l'interface : priorité à une boîte à outils d'UI
  (parchemin, infobulles, texte enrichi, polices UTF-8) avant de porter les écrans.
- Les mesures sous macOS exigent une fenêtre visible. Le rendu hors écran de Bevy (sans
  présentation) reste disponible pour les captures d'agents.
