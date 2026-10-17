# VQ — qualité visuelle sans passer sous 40 i/s (17/10)

Mandat joueur (17/10 ~0 h 40) : « augmenter la qualité visuelle sans passer sous 40 fps » ; plan accepté puis « tu es en autonomie, feu vert pour tout ». Pas de push (règle Bevy), pas de dépense cloud prévue. Seuil : ADR 0404 (p95 ≤ 25 ms, A/B alterné par lot).

## Reprise
- Intégration : branche `vq/integ`, worktree `../gp-vq-integ` (depuis main 9395b327a). Lots : `vq/<lot>`, worktrees `../gp-vq-<lot>` depuis `vq/integ`.
- Fusion : agent Opus de fusion par fournée (`cargo test --features dev` dans `client/`, smoke), puis `vq/integ` → main en `--ff-only` depuis un worktree, jamais de conflit dans le checkout principal.
- Session voisine : chantier BL (`game-project-eb`, `docs/wip/bl-backlog.md` sur `bl/integ`) ; ne pas toucher à ses items : I009 (ombres imposteurs, bunits), I188/I006/I013 (sol/herbe de bataille, bdecor), forêts de campagne I096/I163/I088 (campveg), villes I168/I170 (towns). Une autre session fait `bm/degodot` : ne pas toucher `game/`.
- ADR réservés : 0404 (seuil), 0405-0409 (lots).

## État de départ (captures after5 du 16/10 22 h, Bevy seul)
- Bataille : soldats ombrés (ombres présentes), mais lumière plate, voile laiteux, contraste faible, herbe monotone, nuée de plaques.
- Campagne d=25-60 : villes et maquettes bonnes ; sol vert foncé uniforme sans parcelles ni matières ; rivières cyan plates sans berges ; forêts clairsemées (BL campveg).

## Lots

| Lot | Modèle | Vague | Objet | État |
|---|---|---|---|---|
| bench | sonnet | 1 | banc bataille `--bench-battle` (calme + mêlée), script A/B alterné | fusionné (eb3f8e4fb) ; tests non lancés (machine chargée) → fournée de fusion |
| cperf | opus | 1 | campagne : `fade_maquettes`/VisibilityRange 3,8 ms, rafale de spawns en descente, marge p95 | fusionné (ADR 0405) ; −3,3 ms CPU/image, pire image de descente 250 → 140 ms ; reste : rafale de départ (pluie/neige/bannières/nuages/villes, plancher 130 ms), énigme GPU à vérifier au calme |
| blight | opus | 1 | bataille : lumière, contraste, voile, perspective aérienne, mise à l'échelle 3D si marge | fusionné ; voile remplacé par perspective aérienne (`fog.clear.aerial`), soleil 30°/2,2, ambiante réduite, `ssao_quality` lu ; reste : banc calme, SSAO ultra, tonemapping, aerial pluie/neige |
| rivers | opus | 1 | campagne : rivières réalistes (couleur, berges, reflets, lisibles à toute distance) | fusionné (0366d89f4) ; reste : liseré clair des bords trop « trait de carte », bande de sol mouillé hors ruban, river_fine.wesl, brume sur l'eau |
| cground | opus | 1 | campagne : sol de près (parcelles, matières, colormap, variation) | fusionné ; `ground_detail.wesl` + `data/fx/ground_detail.json`, réglage `ground_detail` 0/1/2 ; écart-type +16 % (Godot ×1,6 non atteint) ; reste : relief/haies, contraste après regard du joueur |
| rivers2 | opus | 2 | rivières : liseré clair, bande de sol mouillé, brume sur l'eau, river_fine | fusionné ; reste : teinte aérienne par pixel sur l'eau, bande mouillée des lacs, palier fin sans brume |
| cgrade | opus | 2 | campagne : étalonnage commun à toute la 3D, perspective aérienne par distance/inclinaison, équilibre soleil/ambiance | fusionné ; grade de saison dans la passe post (une seule fois pour toute l'image) ; brume déjà correcte ; reste : caméra non HDR, ombres de nuages très sombres |
| cground2 | opus | 3 | sol : parcelles en polygones anguleux trop contrastés (effet cristal) à d=60 → bords doux, haies/talus, contraste par parcelle réduit | fusionné ; reste : d=25/200 sur labours non vus, `tints.brown`, masque bocage |
| bsoldiers | opus | 2 | soldats : livrées ternies, acier, peau, variation, boucliers usés ; `battle_lod` enfin lu (high 1,5, ultra 2,0) | fusionné ; coût non vérifié (machine chargée) → banc calme ; regard joueur sur l'acier |

## Journal
- 17/10 0 h 45 : état de départ, ADR 0404, vague 1 (5 lots).
- 17/10 : vq-bench fusionné dans vq/integ (`tools/bevy_bench/ab.sh`, `--bench-battle`). Disque à 22 Go libres : garde-disque VQ (scratchpad vq_disk_guard.sh), tests reportés à la fusion.
- 17/10 : vq-rivers fusionné (bleu-gris sombre, hauts-fonds boueux, reflet rasant, éclat ; coût nul au bruit près ; `river_display.json > look`). Disque 13 Go libres.
- 17/10 : vq-cground fusionné (parcelles contrastées par saison, dérive de sol, bandes au plancher caméra ; coût ≤ bruit, à remesurer au calme).
- 17/10 : disque revenu à 122 Go libres ; vague 2 lancée (rivers2, cgrade).
- 17/10 : vq-blight fusionné (pas de MetalFX côté Bevy : bataille déjà en natif).
- 17/10 : vq-rivers2 fusionné (berges sombres, bande mouillée 3 px, DistanceFog sur l'eau ; coût nul).
- 17/10 : vq-cgrade fusionné ; capture d=60 : parcelles cground en facettes polygonales → lot cground2.
- 17/10 : vq-cperf fusionné (fondu par bande + plages quantifiées : la table RenderVisibilityRanges de Bevy saturait ; budget de spawn partagé).
- 17/10 : main fusionné dans vq/integ (bb832458b VERT : 1306 tests, 10 smoke) ; puis main (+5), cground2, bsoldiers fusionnés → revérification.
