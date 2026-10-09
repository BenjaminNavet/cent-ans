# Budget cloud (v1)

**Synthèse (2026-10-09)**

Générée par `cent-ans budget summary` (`BudgetLedger.render_summary()`), qui lit **toutes** les tables ci-dessous
et somme la colonne « Coût réel » (la colonne « Cumul » est recalculée, plus jamais lue). À régénérer quand une
enveloppe bouge.

**Plafond v1 de 50 $ dépassé : ~117 $ (116,78 $) dépensés, sur 16 enveloppes — décision du joueur à prendre.**
Plafond d'origine : 50,00 $ pour la v1, non modifié ici ; chaque enveloppe garde son plafond propre (voir ses titres), et la règle
« en dessous du plafond, aucune confirmation » ne vaut que pour le plafond de 50 $ de l'enveloppe en cours.

Par fournisseur :

- OpenRouter : 59,82 $
- fal.ai : 56,96 $
- OpenAI, local, Poly Haven, Freesound (CC0), données ouvertes : 0,00 $
- **Total général : 116,78 $**

Par enveloppe (plafond propre) :

- Principal (sessions 1-5, plafond v1 50 $) : 19,20 $
- Session 7 (50 $) : 1,07 $
- Batailles épiques (20 $) : 0,57 $
- Direction artistique (50 $) : 21,34 $
- Polish PO (≤ 3 $) : 0,00 $
- Assets générés GA (15 $) : 9,50 $
- Féodalité FE (15 $) : 5,13 $
- Oural–Méditerranée OM (10 $) : 8,40 $
- Nano Banana NB (10 $) : 3,10 $
- Figurines SR (3 $) : 0,42 $
- Voix criées VX (2 $) : 0,76 $
- Rivières RC (5 $) : 0,00 $
- Biomes HB (8 $) : 3,82 $
- Nuit visuelle VN (5 $) : 1,04 $
- Campagne TB (25 $, annulée ADR 0152) : 0,00 $
- **Nuit DN (≤ 10 $, ADR 0210)** : **42,43 $** (**dépassement de ~32,4 $ sur l'enveloppe**)


Dépassements à signaler :

- **Plafond global v1** : 50 $ annoncés, ~117 $ réels (OpenRouter 59,82 $ + fal.ai 56,96 $). Les enveloppes
  propres (50 + 20 + 50 + 15 + 15 + 10 + 10 + …) ont été ouvertes au fil des sessions sans relever le plafond de
  l'en-tête.
- **Nuit DN** : enveloppe ≤ 10 $ (ADR 0210), 42,43 $ consignés, dont 9,00 $ de TRELLIS 2 (30 appels,
  `fal-ai/trellis-2`) alors que le joueur interdit TRELLIS 2 (lot B du plan QW le retire du code).
- **Plafonds contradictoires dans le code** : `tools/experiments/dn_batch.py` lit `DN_FAL_CAP_USD` avec une valeur par
  défaut de **43,5 $** ; la valeur de 29,50 $ citée ailleurs n'existe plus dans l'arbre (non retrouvée) ; l'ADR
  fixe ≤ 10 $. Aucune des trois valeurs n'est cohérente avec les autres : à trancher avec le plafond ci-dessus.
- **Colonne « Cumul » de la table DN** : mélangeait cumul du journal, « cumul chantier » par lot et texte ; elle
  était lue comme 0,02 $ par l'ancien `budget.py`. Recalculée ici (dernière ligne : 42,43 $).

Rapprochement avec le journal fal (`~/dev/cent-ans-raw/dn/fal_spend.jsonl`, 2 667 appels, 8-9/10) : **43,14 $**
au journal contre **42,43 $** au registre, soit **0,71 $ d'écart** (le registre est plus bas). Journal par
endpoint : `flux-2/edit` 11,52 $, `trellis/multi` 8,06 $, `trellis` 7,72 $, `z-image/turbo` 6,84 $, `trellis-2`
9,00 $. Cause non établie (appels de comparaison hors journal ou arrondis par lot) ; le journal, plus fin, est la
référence pour DN. Le solde fal du compte n'a pas été relu (aucun accès réseau pris).

Harmonisation (2026-10-09) : décimales en virgule et à deux chiffres partout (la table DN mêlait « 1.43 $ » et
« 1,43 $ ») ; une ligne datée `2026-10-10` (reprise locale Z-Image mflux, 0 $, rangée entre des lignes du
09/10) corrigée en `2026-10-09`, faute de frappe évidente puisque nous sommes le 09/10.

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
| 2026-09-30 | fal.ai | GA3-L4 : variantes de visage des 6 figurines L3 (6 × fal-ai/nano-banana-2/edit 1K 0,08 $ planche éditée, tête seule + 6 × fal-ai/bria/background/remove 0,018 $ + 6 × fal-ai/trellis/multi 0,02 $ face+dos) — prix catalogue | 0,71 $ | 0,71 $ | 7,26 $ |
| 2026-09-30 | fal.ai | GA3-L4 : 10 recettes restantes, un essai chacune (10 × fal-ai/nano-banana-2/edit 1K 0,08 $ planche A-pose éditée depuis une planche L3 + 10 × fal-ai/bria/background/remove 0,018 $ + 10 × fal-ai/trellis/multi 0,02 $) — prix catalogue | 1,18 $ | 1,18 $ | 8,44 $ |
| 2026-09-30 | fal.ai | GA3-L4 : variantes de visage de 9 recettes neuves (9 × nano-banana-2/edit 1K 0,08 $ + 9 × bria 0,018 $ + 9 × trellis/multi 0,02 $ ; standard_1 sans variante, enveloppe L4 ≤ 3 $) — prix catalogue | 1,06 $ | 1,06 $ | 9,50 $ |

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

## Voix criées VX (30/09) — plafond propre de 2 $

Répliques de combat et cris de guerre en chœur, ElevenLabs v3 via fal.ai (0,10 $ / 1000 caractères,
balises comprises) ; `tools/cent_ans_tools/voice_tts.py --shouts`. Section placée avant RC pour ne
pas détourner les écritures « dernière table » des autres lots.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul VX |
|---|---|---|---|---|---|
| 2026-09-30 | fal.ai | VX : cris de bataille ElevenLabs v3 (répliques criées, chœurs des cris de guerre), contrôle whisper, langue et hauteur | 0,04 $ | 0,02 $ | 0,02 $ |
| 2026-09-30 | fal.ai | VX : passe interrompue (machine chargée), 14 répliques gardées + prises rejetées ; coût réel non relevé par l'outil, estimé (caractères × prises) | 0,06 $ | 0,06 $ | 0,08 $ |
| 2026-09-30 | fal.ai | VX : passe complète (74 clips : répliques criées et chœurs), prises rejetées comprises | 0,36 $ | 0,46 $ | 0,54 $ |
| 2026-09-30 | fal.ai | VX : rattrapage (18 clips : langues régionales, chœurs), prises rejetées comprises | 0,07 $ | 0,12 $ | 0,66 $ |
| 2026-09-30 | fal.ai | VX : cris de bataille ElevenLabs v3 (répliques criées, chœurs des cris de guerre), contrôle whisper, langue et hauteur | 0,21 $ | 0,05 $ | 0,71 $ |
| 2026-09-30 | fal.ai | VX : passe coupée à 55/90 (délai de tâche, machine surchargée), coût non relevé par l outil : estimé (prises refaites) | 0,05 $ | 0,05 $ | 0,76 $ |

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
| 2026-09-30 | fal.ai | HB4 sonde essences : olivier + chêne kermès (flux-2 0,0126 + nano-banana-2/edit 0,08 + bria 0,018 par essence ; tarifs API fal vérifiés le 30/09) | 0,22 $ | 0,22 $ | 1,62 $ |
| 2026-09-30 | fal.ai | HB4 essences : 18 nouvelles + reprise chêne kermès (arbuste en boule, graine 7), 19 × 0,11 $ | 2,09 $ | 2,09 $ | 3,71 $ |
| 2026-09-30 | fal.ai | HB4 reprise bouleau (houppier trop clairsemé, graine 7) | 0,11 $ | 0,11 $ | 3,82 $ |

## Nuit visuelle VN (30/09) — fal.ai (OpenRouter indisponible), enveloppe propre 5 $

| Date | Service | Objet | Coût estimé | Coût réel | Cumul VN |
|---|---|---|---|---|---|
| 2026-09-30 | fal.ai | VN9 : miniatures des 12 types d'unités sans illustration (12 × fal-ai/nano-banana-2/edit 1K, référence de style unit_knights / unit_longbowmen) — prix catalogue | 0,96 $ | 0,96 $ | 0,96 $ |
| 2026-09-30 | fal.ai | VN9b : miniature de bld_collegiate_church, seul bâtiment sans illustration (1 × fal-ai/nano-banana-2/edit 1K) — prix catalogue | 0,08 $ | 0,08 $ | 1,04 $ |

## Campagne façon Thrones of Britannia TB (02/10) — fal.ai seul, hors plafond v1, enveloppe propre 25 $ (ADR 0149)

**Annulée le 02/10 (ADR 0152)** : compte fal.ai sans crédit, le joueur renonce ; chantier TB à 0 $, aucun appel facturé.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul TB |
|---|---|---|---|---|---|

## Nuit DN (08→09/10) — fal.ai TRELLIS, enveloppe propre ≤ 10 $ (ADR 0210)

Journal automatique : `~/dev/cent-ans-raw/dn/fal_spend.jsonl` (une ligne par appel `fal-ai/trellis`, 0,02 $).
Compte fal : crédit OK le 08/10 (la mention « fal vide » de l'ADR 0152 est périmée).

| Date | Service | Objet | Coût estimé | Coût réel | Cumul DN |
|---|---|---|---|---|---|
| 2026-10-08 | fal.ai | banc `dn_batch.py` : test de solde (cavalier I3D) + moulin + chariot, 3 × `fal-ai/trellis` | 0,06 $ | 0,06 $ | 0,06 $ |
| 2026-10-08 | fal.ai | comparatif cavalier `fal-ai/trellis` (1024) vs `fal-ai/trellis-2` (1024, 100 k faces), `cent-ans-raw/cmp-trellis/cav/` | 0,32 $ | 0,32 $ | 0,38 $ |
| 2026-10-08 | fal.ai | paquet campagne + nature + animaux (cumul log) : images Z-Image fal (`fal-ai/z-image/turbo`), 285 appels | 1,43 $ | 1,43 $ | 1,81 $ |
| 2026-10-08 | fal.ai | paquet campagne + nature + animaux (cumul log) : vues dos/côté `fal-ai/flux-2/edit`, 68 appels | 1,63 $ | 1,63 $ | 3,44 $ |
| 2026-10-08 | fal.ai | paquet campagne + nature + animaux (cumul log) : 3D `fal-ai/trellis` / `trellis/multi`, 190 appels | 3,80 $ | 3,80 $ | 7,24 $ |
| 2026-10-08 | fal.ai | paquet battle + env_* + nature + animaux : images Z-Image fal (`fal-ai/z-image/turbo`), 257 appels | 1,28 $ | 1,28 $ | 8,52 $ |
| 2026-10-08 | fal.ai | paquet battle + env_* + nature + animaux : vues dos/côté `fal-ai/flux-2/edit`, 175 appels | 4,20 $ | 4,20 $ | 12,72 $ |
| 2026-10-08 | fal.ai | paquet battle + env_* + nature + animaux : 3D `fal-ai/trellis` / `trellis/multi`, 205 appels | 4,10 $ | 4,10 $ | 16,82 $ |
| 2026-10-08 | fal.ai | paquet architecture + mobile + economy : images Z-Image fal (`fal-ai/z-image/turbo`), 109 appels | 0,55 $ | 0,55 $ | 17,37 $ |
| 2026-10-08 | fal.ai | paquet architecture + mobile + economy : vues dos/côté `fal-ai/flux-2/edit`, 69 appels | 1,66 $ | 1,66 $ | 19,03 $ |
| 2026-10-08 | fal.ai | paquet architecture + mobile + economy : 3D `fal-ai/trellis` / `trellis/multi`, 171 appels | 3,42 $ | 3,42 $ | 22,45 $ |
| 2026-10-08 | fal.ai | illustrations (308 images, 1 graine) : images Z-Image fal (`fal-ai/z-image/turbo`), 403 appels | 2,02 $ | 2,02 $ | 24,47 $ |
| 2026-10-08 | fal.ai | illustrations (308 images, 1 graine) : vues dos/côté `fal-ai/flux-2/edit`, 10 appels | 0,24 $ | 0,24 $ | 24,71 $ |
| 2026-10-08 | fal.ai | illustrations (308 images, 1 graine) : 3D `fal-ai/trellis` / `trellis/multi`, 23 appels | 0,46 $ | 0,46 $ | 25,17 $ |
| 2026-10-09 | fal.ai | paquet nature_extra + battle_extra + reprises : images Z-Image fal (`fal-ai/z-image/turbo`), 22 appels | 0,11 $ | 0,11 $ | 25,28 $ |
| 2026-10-09 | fal.ai | paquet nature_extra + battle_extra + reprises : vues dos/côté `fal-ai/flux-2/edit`, 67 appels | 1,61 $ | 1,61 $ | 26,89 $ |
| 2026-10-09 | fal.ai | paquet nature_extra + battle_extra + reprises : 3D `fal-ai/trellis` / `trellis/multi`, 104 appels | 2,08 $ | 2,08 $ | 28,97 $ |
| 2026-10-09 | local | reprise locale des assets refusés (Z-Image mflux, 5 ids sur 27 faits, pause) | 0,00 $ | 0,00 $ | 28,97 $ |
| 2026-10-09 | fal.ai | DN-RESTE (67 restants : 22 refusés D5 + 5 sans 3D + 21 figures + 14 cartes `card_*`) : images Z-Image fal (`fal-ai/z-image/turbo`), 202 appels | 1,01 $ | 1,01 $ | 29,98 $ |
| 2026-10-09 | fal.ai | DN-RESTE (67 restants : 22 refusés D5 + 5 sans 3D + 21 figures + 14 cartes `card_*`) : vues dos/côté `fal-ai/flux-2/edit`, 29 appels | 0,70 $ | 0,70 $ | 30,68 $ |
| 2026-10-09 | fal.ai | DN-RESTE (67 restants : 22 refusés D5 + 5 sans 3D + 21 figures + 14 cartes `card_*`) : 3D `fal-ai/trellis`, 20 appels | 0,40 $ | 0,40 $ | 31,08 $ |
| 2026-10-09 | fal.ai | DN-RESTE (67 restants : 22 refusés D5 + 5 sans 3D + 21 figures + 14 cartes `card_*`) : 3D `fal-ai/trellis/multi`, 28 appels | 0,56 $ | 0,56 $ | 31,64 $ |
| 2026-10-09 | fal.ai | DN-FIX3 : 2 canons + 6 figures montées refaits (images Z-Image 30 appels, vues `flux-2/edit` 10, 3D `trellis/multi` 13) | 0,65 $ | 0,65 $ | 32,29 $ |
| 2026-10-09 | fal.ai | DN-TROUS : images Z-Image fal (`fal-ai/z-image/turbo`, port 3 graines, 13 flagrants refaits, 2 figures), 68 appels | 0,34 $ | 0,34 $ | 32,63 $ |
| 2026-10-09 | fal.ai | DN-TROUS : 3D TRELLIS 2 (`fal-ai/trellis-2`, 1024), 30 appels : 13 flagrants de la revue, port x2, 15 maquettes de ville/abbaye/village essayées (puis repassées en TRELLIS 1 : à 4000 triangles le LOD1 d'un maillage TRELLIS 2 de 100 000 faces s'effondre) | 9,00 $ | 9,00 $ | 41,63 $ |
| 2026-10-09 | fal.ai | DN-TROUS : vues dos/côté `fal-ai/flux-2/edit`, 24 appels (8 maquettes multi-vues, contrôle de luminance) + 2 figures | 0,58 $ | 0,58 $ | 42,21 $ |
| 2026-10-09 | fal.ai | DN-TROUS : 3D `fal-ai/trellis/multi`, 10 appels (8 maquettes + 2 figures). Solde fal épuisé en cours de lot (« Exhausted balance ») : le reste des maquettes en liste d'attente | 0,20 $ | 0,20 $ | 42,41 $ |
| 2026-10-09 | fal.ai | env_harvest_sheaves : comparaison 3D `fal-ai/trellis`, 1 appel (image Z-Image locale ; SF3D local retenu par le joueur) | 0,02 $ | 0,02 $ | 42,43 $ |
| 2026-10-09 | fal.ai | DN-CHAMPS (abandonné, branche supprimée) : 3D `fal-ai/trellis`, 12 appels (11 parcelles entières + terrasses refaites ; images Z-Image locales, 0 $) | 0,24 $ | 0,24 $ | 42,67 $ |

## Textures régionales TX (09/10) — fal.ai Z-Image Turbo, enveloppe propre 10 $ (validée par le joueur, ADR 0236)

`fal-ai/z-image/turbo` à 0,005 $/Mpx : 2048² ≈ 0,021 $, 1024² ≈ 0,005 $. `cent-ans textures generate` affiche le coût
estimé de chaque lot et refuse au-delà de `--max-cost` (2 $ par défaut). Solde fal rechargé par le joueur le 09/10.

| Date | Service | Objet | Coût estimé | Coût réel | Cumul TX |
|---|---|---|---|---|---|
| 2026-10-09 | fal.ai | essai de solde et de détail : prairie 2048² natif (`fal-ai/z-image/turbo`), 1 appel | 0,02 $ | 0,02 $ | 0,02 $ |
| 2026-10-09 | fal.ai | DN-TROUS 2 (21 maquettes de ville/cité/château en multi-vues, après recharge) : vues dos/côté `fal-ai/flux-2/edit`, 35 appels | 0,84 $ | 0,84 $ | cumul chantier 1,58 $ |
| 2026-10-09 | fal.ai | DN-TROUS 2 : 3D `fal-ai/trellis/multi`, 21 appels (3 refaits : glb tronqués par une coupure réseau) + 1 `fal-ai/trellis` | 0,44 $ | 0,44 $ | cumul chantier 1,58 $ |
| 2026-10-09 | fal.ai | DN-CHAMPS 2 (11 parcelles entières, + 2 objets associés) : images Z-Image fal (`fal-ai/z-image/turbo`), 8 appels | 0,04 $ | 0,04 $ | cumul chantier 0,28 $ |
| 2026-10-09 | fal.ai | DN-CHAMPS 2 : 3D `fal-ai/trellis`, 12 appels | 0,24 $ | 0,24 $ | cumul chantier 0,28 $ |
| 2026-10-09 | fal.ai | sols de campagne `ground_campaign` : pilote 6 images 2048² puis lot complet 83 images 2048² (`fal-ai/z-image/turbo`), 89 appels | 1,87 $ | 1,87 $ | 1,89 $ |
