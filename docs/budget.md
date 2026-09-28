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
