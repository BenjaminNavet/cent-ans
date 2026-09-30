# Budget cloud (v1)

Plafond : **50,00 $** pour la v1. En dessous, aucune confirmation demandée.

Note (2026-09-23, M10) : la clé OpenRouter a en plus sa propre limite mensuelle (100 $, consommée
hors projet) ; elle bloquait les portraits après le premier (402). Les lots M10 sont consignés
ligne par ligne ci-dessous (estimation au jeton, coût réel `usage.cost`).

Rapprochement (2026-09-24, session 5) : compteur OpenRouter de la nouvelle clé, `total_usage`
99,4390 $ − 80,3766 $ au départ = 19,06 $ dépensés, contre 19,15 $ consignés pour la même période
(lignes du 2026-09-24) : le registre est légèrement prudent. Crédit restant ≈ 0,56 $.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul |
|---|---|---|---|---|---|
| 2026-09-23 | OpenRouter | Sonde gpt-5-image-mini (16 jetons, test de la limite de clé) | 0,01 $ | 0,00 $ | 0,00 $ |
| 2026-09-23 | OpenRouter | Portraits M10 (1 × openai/gpt-5-image-mini) | 0,02 $ | 0,05 $ | 0,05 $ |
| 2026-09-24 | OpenRouter | Portraits M10 (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,15 $ | 0,20 $ |
| 2026-09-24 | OpenRouter | Miniatures d'événements (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,15 $ | 0,35 $ |
| 2026-09-24 | OpenRouter | Miniatures d'événements (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,15 $ | 0,50 $ |
| 2026-09-24 | OpenRouter | Illustrations de l'encyclopédie (2 × openai/gpt-5-image-mini) | 0,10 $ | 0,09 $ | 0,59 $ |
| 2026-09-24 | OpenRouter | Illustrations de l'encyclopédie (29 × openai/gpt-5-image-mini) | 1,32 $ | 1,36 $ | 1,95 $ |
| 2026-09-24 | OpenRouter | Portraits M10 (84 × openai/gpt-5-image-mini) | 3,83 $ | 3,85 $ | 5,80 $ |
| 2026-09-24 | OpenRouter | Illustrations de l'encyclopédie (85 × openai/gpt-5-image-mini) | 3,87 $ | 3,92 $ | 9,72 $ |
| 2026-09-24 | OpenRouter | Miniatures d'événements (114 × openai/gpt-5-image-mini) | 5,19 $ | 5,41 $ | 15,13 $ |
| 2026-09-24 | OpenRouter | Sonde `image_config` 16:9 (1 × openai/gpt-5-image-mini, ignorée par le modèle) | 0,04 $ | 0,04 $ | 15,17 $ |
| 2026-09-24 | OpenRouter | Illustrations du Codex (84 × openai/gpt-5-image-mini) | 3,83 $ | 4,03 $ | 19,20 $ |

## Session 7 (nuit du 24/09) — enveloppe propre de 50 $

| Date | Service | Objet | Coût estimé | Coût réel | Cumul session 7 |
|---|---|---|---|---|---|
| 2026-09-25 | OpenRouter | UR1 : illustrations des 14 nouveaux types d'unités (14 × openai/gpt-5-image-mini, script hors registre automatique car le parseur de `budget.py` ne lit pas la table de session 7) | 0,64 $ | 0,64 $ | 0,64 $ |
| 2026-09-25 | OpenAI | VO1 : voix (294 clips gpt-4o-mini-tts : 120 répliques, 21 interventions du conseiller, 153 phrases et cris de discours ; `--dry-run` de `tools/cent_ans_tools/voice_tts.py`, plafond du lot 3 $). Sonde d'un clip refusée : HTTP 401, clé `OPENAI_API_KEY` invalide ; rien généré, rien facturé | 0,37 $ | 0,00 $ | 0,64 $ |
| 2026-09-25 | OpenRouter | VO1 : voix synthétiques, 294 clips openai/gpt-audio-mini (120 répliques, 21 interventions du conseiller, 153 phrases et cris de discours), contrôle mot pour mot (transcription et durée) avec reprises ; coût réel = somme de usage.cost des passes (0,194 + 0,027 + 0,006 + 0,007 + sondes 0,004) | 0,36 $ | 0,24 $ | 0,88 $ |
| 2026-09-25 | OpenRouter | VO1 : reprise de 2 interventions du conseiller (adv_start_france, adv_first_siege) à transcription strictement conforme, 1 essai chacune (0,0016 $) | 0,01 $ | 0,00 $ | 0,88 $ |
| 2026-09-25 | OpenRouter | AR1 : planches illustrées (loading) (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 0,93 $ |
| 2026-09-25 | OpenRouter | AR1 : planches illustrées (vignette) (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,14 $ | 1,07 $ |

## Batailles épiques (25/09) — plafond propre de 20 $

| Date | Service | Objet | Coût estimé | Coût réel | Cumul batailles épiques |
|---|---|---|---|---|---|
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « picardy_plains » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,04 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « channel_coast » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,08 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « pyrenees » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,12 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « alps » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,16 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « massif_central » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 0,21 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « wooded_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,25 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « norman_bocage » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,29 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « loire_valley » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,33 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « gascony_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,37 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « flanders_flat » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,41 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « moorland_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,45 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « mediterranean_hills » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,49 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « winter_lowlands » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,53 $ |
| 2026-09-25 | OpenRouter | EP2 : panorama d'horizon « flanders_flat » (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,04 $ | 0,57 $ |
| 2026-09-25 | Freesound (CC0) | EP4 : 27 sons libres supplémentaires (chocs acier/acier et acier/bois, impacts d'armure, cris d'effort, râles, chutes, chevaux, 3e nappe de mêlée), licence vérifiée page par page, aucun appel payant | 0,00 $ | 0,00 $ | 0,57 $ |
| 2026-09-26 | — (données ouvertes) | EP7 : cartes historiques Crécy, Poitiers, Azincourt — relief Copernicus GLO-30 et couvert ESA WorldCover déjà en cache, tuiles d'horizon et aperçus calculés localement, aucune image générée | 0,00 $ | 0,00 $ | 0,57 $ |

## Direction artistique (25/09) — plafond propre de 50 $ (clé OpenRouter personnelle du joueur, depuis le 25/09 ~23 h)

| Date | Service | Objet | Coût estimé | Coût réel | Cumul DA |
|---|---|---|---|---|---|
| 2026-09-25 | OpenRouter | DA2 : portraits vivants (archétypes et variantes âgées) (6 × openai/gpt-5-image-mini) | 0,28 $ | 0,27 $ | 0,27 $ |
| 2026-09-25 | OpenRouter | DA : planche de style, bouton de fin de tour (cloche) et planche d'icônes à l'encre (2 × openai/gpt-5-image-mini) | 0,09 $ | 0,09 $ | 0,36 $ |
| 2026-09-26 | OpenRouter | DA5 : icônes d'action à l'encre et boutons-médaillons (2 × openai/gpt-5-image-mini) | 0,10 $ | 0,09 $ | 0,45 $ |
| 2026-09-26 | OpenRouter | DA5 : icônes d'action à l'encre et boutons-médaillons (2 × openai/gpt-5-image-mini) | 0,10 $ | 0,09 $ | 0,54 $ |
| 2026-09-26 | OpenRouter | DA5 : icônes d'action à l'encre et boutons-médaillons (76 × openai/gpt-5-image-mini) | 3,46 $ | 3,43 $ | 3,97 $ |
| 2026-09-26 | OpenRouter | DA5 : icônes d'action à l'encre et boutons-médaillons (12 × openai/gpt-5-image-mini) | 0,55 $ | 0,54 $ | 4,51 $ |
| 2026-09-26 | OpenRouter | DA5 : icônes d'action à l'encre et boutons-médaillons (6 × openai/gpt-5-image-mini) | 0,28 $ | 0,28 $ | 4,79 $ |
| 2026-09-26 | OpenRouter | DA5 : icônes d'action à l'encre et boutons-médaillons (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 4,84 $ |
| 2026-09-26 | OpenRouter | DA3 : marqueurs de carte peints (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,14 $ | 4,98 $ |
| 2026-09-26 | OpenRouter | DA3 : marqueurs de carte peints (10 × openai/gpt-5-image-mini) | 0,46 $ | 0,45 $ | 5,43 $ |
| 2026-09-26 | OpenRouter | DA5b : icônes d'entité en miniatures peintes (3 × openai/gpt-5-image-mini) | 0,14 $ | 0,14 $ | 5,57 $ |
| 2026-09-26 | OpenRouter | DA5b : icônes d'entité en miniatures peintes (28 × openai/gpt-5-image-mini, lot de 52 interrompu par un délai réseau) | 1,28 $ | 1,29 $ | 6,86 $ |
| 2026-09-26 | OpenRouter | DA5b : icônes d'entité en miniatures peintes (24 × openai/gpt-5-image-mini) | 1,10 $ | 1,10 $ | 7,96 $ |
| 2026-09-26 | OpenRouter | DA2 : portraits vivants (archétypes et variantes âgées) (141 × openai/gpt-5-image-mini) | 6,47 $ | 6,33 $ | 14,29 $ |
| 2026-09-26 | OpenRouter | DA2 : portraits vivants (archétypes et variantes âgées) (80 × openai/gpt-5-image-mini) | 3,69 $ | 3,59 $ | 17,88 $ |
| 2026-09-26 | OpenRouter | DA7c : icônes de trait à l'encre (59 × openai/gpt-5-image-mini) | 2,69 $ | 2,68 $ | 20,56 $ |
| 2026-09-28 | OpenRouter | CB : icônes des contrôles de bataille à l'encre, sonde (2 × openai/gpt-5-image-mini) | 0,10 $ | 0,09 $ | 20,65 $ |
| 2026-09-28 | OpenRouter | CB : icônes des contrôles de bataille à l'encre (14 × openai/gpt-5-image-mini) | 0,64 $ | 0,64 $ | 21,29 $ |
| 2026-09-28 | OpenRouter | RS : icône « Raser » à l'encre (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 21,34 $ |

## Polish PO (27/09) — 0 $ prévu, enveloppe ≤ 3 $ (ADR 0097)

LUT procédurales existantes, textures et sons CC0. L'enveloppe ne sert que si des cadres
d'interface doivent être régénérés dans le registre enluminure.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul PO |
|---|---|---|---|---|---|
| 2026-09-27 | — | PO0 : planche « avant », gabarit, squelette (aucun appel payant) | 0,00 $ | 0,00 $ | 0,00 $ |

## Assets générés GA (28/09) — plafond propre de 15 $ (clé OpenRouter personnelle du joueur)

Spec `docs/superpowers/specs/2026-09-28-ga-assets-generes-design.md`. GA1 ≤ 4 $, GA3 ≤ 8 $,
GA4 ≤ 2 $, GA5 ≤ 1 $, GA2 0 $ (CC0).

| Date | Service | Objet | Coût estimé | Coût réel | Cumul GA |
|---|---|---|---|---|---|
| 2026-09-28 | OpenRouter | GA1 : matière wool (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,04 $ |
| 2026-09-28 | OpenRouter | GA1 : matière mail (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,08 $ |
| 2026-09-28 | OpenRouter | GA1 : matière wool (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,12 $ |
| 2026-09-28 | OpenRouter | GA1 : matière linen (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,16 $ |
| 2026-09-28 | OpenRouter | GA1 : matière fustian (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,20 $ |
| 2026-09-28 | OpenRouter | GA1 : matière gambeson (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,24 $ |
| 2026-09-28 | OpenRouter | GA1 : matière leather (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,28 $ |
| 2026-09-28 | OpenRouter | GA1 : matière plate (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,32 $ |
| 2026-09-28 | OpenRouter | GA1 : matière wood (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,36 $ |
| 2026-09-28 | OpenRouter | GA1 : matière skin (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,40 $ |
| 2026-09-28 | OpenRouter | GA1 : matière hair (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,44 $ |
| 2026-09-28 | OpenRouter | GA1 : matière coat_light (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,48 $ |
| 2026-09-28 | OpenRouter | GA1 : matière coat_dark (openai/gpt-5-image-mini) | 0,02 $ | 0,04 $ | 0,52 $ |
| 2026-09-28 | OpenRouter | GA1 : rapprochement du solde (appel coat_light coupé en cours de réponse, arrondis) | 0,00 $ | 0,07 $ | 0,59 $ |
| 2026-09-28 | Poly Haven | GA5 : bâtiments en 2k (10 identifiants existants, téléchargement direct, CC0) + torchis/colombage procédural (composite local, pas d'appel IA) | 0,00 $ | 0,00 $ | 0,59 $ |
| 2026-09-30 | fal.ai | GA3-S1 : maison à colombages (1 × fal-ai/flux/dev 1024², 0,025 $/MP + 1 × fal-ai/trellis 0,02 $) | 0,05 $ | 0,05 $ | 0,64 $ |
| 2026-09-30 | fal.ai | GA3-S2 : longbowman image→3D (1 × fal-ai/trellis/multi, 3 vues découpées de `sr3/longbowman.png`, 0,02 $) | 0,02 $ | 0,02 $ | 0,66 $ |
| 2026-09-30 | fal.ai | GA3-S4 : maison v2 (1 × fal-ai/flux-2 1024² 0,012 $ + 1 × fal-ai/bria/background/remove 0,018 $ + 1 × fal-ai/trellis 0,02 $ + 1 × fal-ai/trellis-2 1024 0,30 $) | 0,35 $ | 0,35 $ | 1,01 $ |
| 2026-09-30 | fal.ai | GA3-S5 : végétation campagne (5 × fal-ai/flux-2 1024² 0,012 $/MP + 5 × fal-ai/bria/background/remove 0,018 $ + 3 × fal-ai/trellis 0,02 $ : chêne, buisson, rocher) | 0,21 $ | 0,21 $ | 1,22 $ |
| 2026-09-30 | fal.ai | GA3-S3 : comparatif figurine longbowman (1 × tripo3d/h3.1/multiview-to-3d texturé standard + géométrie détaillée 0,50 $ ; 1 × meshy/v7.1/multi-image-to-3d texturé + rig + anim 1,52 $ ; 1 × fal-ai/trellis-2 1024 0,30 $ ; 1 appel Tripo refusé en validation 422, non facturé) — prix catalogue, l'API d'usage refuse la clé | 2,32 $ | 2,32 $ | 3,54 $ |
| 2026-09-30 | fal.ai | GA3-L1 : décor de bataille, 8 objets neufs (8 × fal-ai/flux-2 1024² 0,012 $ + 16 × fal-ai/bria/background/remove 0,018 $ + 7 × fal-ai/trellis 0,02 $ + 1 × fal-ai/trellis-2 1024 0,30 $ église + 8 × fal-ai/flux-2/edit vues ≈ 0,024 $ + 4 × fal-ai/trellis/multi 0,02 $ ; maison reprise de S4) — prix catalogue | 1,10 $ | 1,10 $ | 4,64 $ |
| 2026-09-30 | fal.ai | GA3-L2 : végétation campagne (5 × fal-ai/flux-2 1024² 0,012 $ + 1 × flux-2 1024×512 + 3 × fal-ai/nano-banana-2/edit 0,08 $ planches 8 azimuts + 6 × fal-ai/bria/background/remove 0,018 $ + 3 × fal-ai/trellis 0,02 $ rochers, dont 1 reprise) — prix catalogue | 0,47 $ | 0,47 $ | 5,11 $ |
| 2026-09-30 | fal.ai | GA3-L1b : reprise maison, puits, bélier, un essai chacun (3 × fal-ai/flux-2 1024² 0,012 $ + 3 × fal-ai/bria/background/remove 0,018 $ + 3 × fal-ai/trellis 0,02 $ ; 1 envoi bria échoué au téléchargement, compté) — prix catalogue | 0,17 $ | 0,17 $ | 5,28 $ |
| 2026-09-30 | fal.ai | GA3-L3a : figurine longbowman (1 × fal-ai/nano-banana-2/edit 2K 0,12 $ planche A-pose 3 vues + 1 × fal-ai/bria/background/remove 0,018 $ + 1 × fal-ai/trellis/multi 0,02 $ face+dos, livrée + 1 × fal-ai/trellis 0,02 $ comparaison + 1 × fal-ai/trellis-2 1024 0,30 $ parti avant la consigne « moins cher », comparaison seulement) — prix catalogue | 0,48 $ | 0,48 $ | 5,76 $ |
| 2026-09-30 | fal.ai | GA3-L3b : figurines homme d'armes, arbalétrier, sergent, milicien, un essai chacune (4 × fal-ai/nano-banana-2/edit 2K 0,12 $ planche A-pose 3 vues + 4 × fal-ai/bria/background/remove 0,018 $ + 4 × fal-ai/trellis/multi 0,02 $ face+dos) — prix catalogue | 0,63 $ | 0,63 $ | 6,39 $ |
| 2026-09-30 | fal.ai | GA3-L3c : cavalier du chevalier, un essai (1 × fal-ai/nano-banana-2/edit 2K 0,12 $ planche A-pose 3 vues à pied depuis sr3/knight_mounted + 1 × fal-ai/bria/background/remove 0,018 $ + 1 × fal-ai/trellis/multi 0,02 $ face+dos) — prix catalogue | 0,16 $ | 0,16 $ | 6,55 $ |

## Féodalité FE (28/09) — plafond propre de 15 $ (portraits F7 seulement, ADR 0098)

Format du grand livre (6 colonnes, lu par `tools/cent_ans_tools/budget.py`).

| Date | Service | Objet | Coût estimé | Coût réel | Cumul FE |
|---|---|---|---|---|---|
| 2026-09-28 | — | F0 (titres, migration) | 0,00 $ | 0,00 $ | 0,00 $ |
| 2026-09-28 | OpenRouter | F7 : portraits des souverains et héritiers FE (62 × openai/gpt-5-image-mini) | 2,83 $ | 2,86 $ | 2,86 $ |
| 2026-09-28 | OpenRouter | F7 : variantes âgées des personnages FE (49 × openai/gpt-5-image-mini) | 2,23 $ | 2,27 $ | 5,13 $ |

## Carte Oural–Méditerranée OM (28/09) — plafond propre de 10 $ (portraits)

Format du grand livre (6 colonnes, lu par `tools/cent_ans_tools/budget.py`). Lot P1 : plafond 5 $ (portraits des nouvelles factions D1-D3 et variantes âgées).

| Date | Service | Objet | Coût estimé | Coût réel | Cumul OM |
|---|---|---|---|---|---|
| 2026-09-28 | — | P1 : écus et bannières (génération locale) | 0,00 $ | 0,00 $ | 0,00 $ |
| 2026-09-29 | OpenRouter | P1 : portraits des souverains et héritiers D1-D3 (46 × openai/gpt-5-image-mini) | 2,10 $ | 2,16 $ | 2,16 $ |
| 2026-09-29 | OpenRouter | P1 : variantes âgées D1-D3 (37 × openai/gpt-5-image-mini) | 1,69 $ | 1,69 $ | 3,85 $ |
| 2026-09-29 | OpenRouter | P2 : portraits des personnages D4-D6 (49 × openai/gpt-5-image-mini) | 2,23 $ | 2,26 $ | 6,11 $ |
| 2026-09-29 | OpenRouter | P2 : variantes âgées D4-D6 (42 × openai/gpt-5-image-mini) | 1,92 $ | 1,94 $ | 8,05 $ |
| 2026-09-29 | OpenRouter | R6 : portrait d'Andronic III, couronne byzantine (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 8,10 $ |
| 2026-09-29 | OpenRouter | R6 : variante âgée d'Andronic III, couronne byzantine (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 8,15 $ |
| 2026-09-29 | OpenRouter | R6 : portrait d'Abu l-Hasan sans auréole (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 8,20 $ |
| 2026-09-29 | OpenRouter | R6 : variante âgée d'Abu l-Hasan sans auréole (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 8,25 $ |
| 2026-09-29 | OpenRouter | OMR : portrait de Hızır Bey, 1re tentative avec auréole, écartée (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 8,30 $ |
| 2026-09-29 | OpenRouter | OMR : portrait de Hızır Bey, bey de Hamid (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 8,35 $ |
| 2026-09-29 | OpenRouter | OMR : variante âgée de Hızır Bey (1 × openai/gpt-5-image-mini) | 0,05 $ | 0,05 $ | 8,40 $ |

## Nano Banana NB (30/09) — plafond propre de 10 $ (clé OpenRouter personnelle du joueur)

Spec `docs/superpowers/specs/2026-09-30-nb-nano-banana-interface-design.md`. NB-DA ≤ 1 $,
NB0 ≤ 1,50 $, NB1 ≤ 5 $, réserve 2,50 $.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul NB |
|---|---|---|---|---|---|
| 2026-09-30 | — | NB-S (squelette) | 0,00 $ | 0,00 $ | 0,00 $ |
| 2026-09-30 | OpenRouter | NB-DA : planche maîtresse v0 (google/gemini-3.1-flash-image, 2K) | 0,11 $ | 0,10 $ | 0,10 $ |
| 2026-09-30 | OpenRouter | NB-DA : planche maîtresse v1 (google/gemini-3.1-flash-image, 2K) | 0,11 $ | 0,10 $ | 0,20 $ |
| 2026-09-30 | OpenRouter | NB-DA : planche maîtresse v2 (google/gemini-3.1-flash-image, 2K) | 0,11 $ | 0,10 $ | 0,30 $ |
| 2026-09-30 | OpenRouter | NB-DA : planche maîtresse v3 (google/gemini-3.1-flash-image, 2K) | 0,11 $ | 0,10 $ | 0,40 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde portrait sans ancre (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,47 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde portrait avec ancre (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,54 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde portrait sans ancre (google/gemini-3.1-flash-lite-image, 1K) | 0,04 $ | 0,03 $ | 0,57 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde portrait avec ancre (google/gemini-3.1-flash-lite-image, 1K) | 0,04 $ | 0,03 $ | 0,60 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde portrait sans ancre (google/gemini-3-pro-image, 1K) | 0,14 $ | 0,14 $ | 0,74 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde portrait avec ancre (google/gemini-3-pro-image, 1K) | 0,14 $ | 0,14 $ | 0,88 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde event sans ancre (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,95 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde event avec ancre (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 1,02 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde event sans ancre (google/gemini-3.1-flash-lite-image, 1K) | 0,04 $ | 0,03 $ | 1,05 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde event avec ancre (google/gemini-3.1-flash-lite-image, 1K) | 0,04 $ | 0,03 $ | 1,08 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde event sans ancre (google/gemini-3-pro-image, 1K) | 0,14 $ | 0,14 $ | 1,22 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde event avec ancre (google/gemini-3-pro-image, 1K) | 0,14 $ | 0,14 $ | 1,36 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde icon sans ancre (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 1,43 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde icon avec ancre (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 1,50 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde icon sans ancre (google/gemini-3.1-flash-lite-image, 1K) | 0,04 $ | 0,03 $ | 1,53 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde icon avec ancre (google/gemini-3.1-flash-lite-image, 1K) | 0,04 $ | 0,03 $ | 1,56 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde icon sans ancre (google/gemini-3-pro-image, 1K) | 0,14 $ | 0,14 $ | 1,70 $ |
| 2026-09-30 | OpenRouter | NB0 : sonde icon avec ancre (google/gemini-3-pro-image, 1K) | 0,14 $ | 0,14 $ | 1,84 $ |
| 2026-09-30 | OpenRouter | NB1 : panel_illuminated v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 1,91 $ |
| 2026-09-30 | OpenRouter | NB1 : panel_illuminated v1 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 1,98 $ |
| 2026-09-30 | OpenRouter | NB1 : panel_illuminated v2 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,05 $ |
| 2026-09-30 | OpenRouter | NB1 : panel v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,12 $ |
| 2026-09-30 | OpenRouter | NB1 : panel v1 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,19 $ |
| 2026-09-30 | OpenRouter | NB1 : panel v2 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,26 $ |
| 2026-09-30 | OpenRouter | NB1 : top_bar v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,33 $ |
| 2026-09-30 | OpenRouter | NB1 : top_bar v1 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,40 $ |
| 2026-09-30 | OpenRouter | NB1 : top_bar v2 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,47 $ |
| 2026-09-30 | OpenRouter | NB1 : tooltip v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,54 $ |
| 2026-09-30 | OpenRouter | NB1 : tooltip v1 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,61 $ |
| 2026-09-30 | OpenRouter | NB1 : tooltip v2 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,68 $ |
| 2026-09-30 | OpenRouter | NB1 : initial_dragon v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,75 $ |
| 2026-09-30 | OpenRouter | NB1 : fleuron_divider v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,82 $ |
| 2026-09-30 | OpenRouter | NB1 : drollery_hare v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,89 $ |
| 2026-09-30 | OpenRouter | NB1 : drollery_musician v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 2,96 $ |
| 2026-09-30 | OpenRouter | NB1 : corner_rinceau v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 3,03 $ |
| 2026-09-30 | OpenRouter | NB1 : cartouche_title v0 (google/gemini-3.1-flash-image) | 0,07 $ | 0,07 $ | 3,10 $ |

## Figurines semi-réalistes SR (30/09) — plafond propre de 3 $ (NB2, enveloppe NB2 globale ≈ 20 $)

| Date | Service | Objet | Coût estimé | Coût réel | Cumul SR |
|---|---|---|---|---|---|
| 2026-09-30 | — | SR (squelette) | 0,00 $ | 0,00 $ | 0,00 $ |
| 2026-09-30 | OpenRouter | SR3 : planche de référence man_at_arms (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,07 $ |
| 2026-09-30 | OpenRouter | SR3 : planche de référence longbowman (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,14 $ |
| 2026-09-30 | OpenRouter | SR3 : planche de référence crossbowman (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,21 $ |
| 2026-09-30 | OpenRouter | SR3 : planche de référence sergeant (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,28 $ |
| 2026-09-30 | OpenRouter | SR3 : planche de référence militia (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,35 $ |
| 2026-09-30 | OpenRouter | SR3 : planche de référence knight_mounted (google/gemini-3.1-flash-image, 1K) | 0,07 $ | 0,07 $ | 0,42 $ |

## Fleuves et rivières RC (30/09) — plafond propre de 5 $ (matières d'eau Nano Banana 2, ADR 0141)

Format du grand livre (6 colonnes, lu par `tools/cent_ans_tools/budget.py`). Lot RC5 : 4 matières
(`sea`, `ocean`, `river_large`, `river_small`) × `google/gemini-3.1-flash-image` (~0,078 $ l'image),
reprises éventuelles comprises : ~0,62 $ prévu. Plafond vérifié par `data/art/water_materials.yaml`
(bloc `budget`) avant chaque appel payant.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul RC |
|---|---|---|---|---|---|
| 2026-09-30 | — | RC5 : squelette, prompts et essai à blanc (aucun appel payant) | 0,00 $ | 0,00 $ | 0,00 $ |

## Habillage par biomes HB (30/09) — plafond propre de 8 $ (fal.ai, ADR 0143)

Format du grand livre (6 colonnes, lu par `tools/cent_ans_tools/budget.py`). Matières de sol (HB2), essences (HB4), rochers (HB5).

| Date | Service | Objet | Coût estimé | Coût réel | Cumul HB |
|---|---|---|---|---|---|
| 2026-09-30 | — | HB : plan et ADR 0143 (aucun appel payant) | 0,00 $ | 0,00 $ | 0,00 $ |
| 2026-09-30 | fal.ai | HB5 rochers : 6 affleurements (flux-2 + bria + trellis, 0,05 $ chacun, aucune reprise) | 0,30 $ | 0,30 $ | 0,30 $ |
| 2026-09-30 | fal.ai | HB2 sonde : 3 matières (vigne, garrigue, steppe), `fal-ai/flux-2-pro` 1024² (0,03 $/Mpx) | 0,09 $ | 0,09 $ | 0,39 $ |
| 2026-09-30 | fal.ai | HB2 : 24 autres matières de sol, flux-2-pro 1024² | 0,76 $ | 0,76 $ | 1,15 $ |
| 2026-09-30 | fal.ai | HB2 reprises : 6 (blé, orge, pré de fauche, seigle, boréale, maquis) + 2 (orge, boréale) | 0,25 $ | 0,25 $ | 1,40 $ |
