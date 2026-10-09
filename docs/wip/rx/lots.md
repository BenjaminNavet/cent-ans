# RX — lots de correction

| lot | source | agent | ADR | état |
|---|---|---|---|---|
| victory | mecaniques #1 #2 #11 | dev | 0245 | PARTIEL (ec50ccc7d) : #1 #2 faits ; #11 victoire non signalée sans bloc `victory` → lot équilibrage |
| probe | mecaniques #3 (sonde de campagne) | mech | 0246 | FAIT (campaign_probe, ADR 0246) |
| audio | audio (tous) | mech | 0247 | FAIT sauf : nouvelles voix (65 factions slaves/turques/arabes/grecques…), mp3→ogg (proposition), écoute à faire |
| histoire | historien (tous) | mech | 0248 | FAIT (blasons Montferrat/Siena non modifiés, voir historien.md) |
| uifin | ui (tous) | mech | 0249 | à faire |
| anim | animation (tous) | dev | 0250-0251 | FAIT sauf pas du cheval Muybridge (pas de source) et sync du coup porté ; tests verts (rx_anim, an1b, as3, as8b, nt7) |
| (vague 2) | bataille-v2, campagne-v2, ia, perf, équilibrage campagne avec la sonde | — | 0252+ | en attente |
| mapa | campagne + campagne-v2 : dalles blanches (champs/parcelles), maquettes blanches surexposées, sol de Paris | dev | 0252 | FAIT (gain d’albédo lu en sRGB, test rx_mapa_textures_test ; sol olive de Paris non traité) |
| mapb | campagne + campagne-v2 : nuages opaques, rouge anglais en aplat, étiquettes, fleuves du parchemin, camouflage au dézoom, Oural, colormap/zoom | dev | 0253 | PARTIEL : nuages, lavis de faction, halo des noms, fleuves du parchemin, détail/teinte au dézoom, fondu du brouillard (rx/mapb) ; reste : brun terne de l'Oural et ocre de la colormap (re-cuisson `geo colormap`) |
| batvis | bataille + bataille-v2 + assets3d : caméra closeup dans un toit, motif de feuilles du sol à mi-distance, neige en lattes, maisons de siège orange trouées, échelles orange, caméra de déploiement, mêlée lisible (contours), aperçus sur l'eau | dev | 0254 | PARTIEL (rx/batvis) : fait closeup hors bâtiments, échelles bois fines, lissage lointain du sol, contour de mêlée, caméra de déploiement + toast, aperçus de chemin sur l'eau. Reste : lattes de neige (cause non isolée), maisons de siège orange, toits étirés, décales d'aperçu sur l'eau |
| batsim | bataille : pont unique en colonne, déroute par contagion (arbalétriers 0 perte), régiment anéanti 0 tué, sonde d'issue `--autoplay` avec graine, smoke_battle court + test long, message « Chevaliers à placer » permanent, avertissement CampaignSim | dev | 0255 | en cours |
| robust | perf (tous sauf tests UI de mise en page → après uifin) : tests bloqués/cassés, erreurs moteur, migration de sauvegarde, smoke bruyant | dev | 0256 | en cours |
| equil | mecaniques #4-#11 + ia #4 #7 : trésors, éliminations, guerre permanente, bande FR-EN, guerres rejouées après trêve, croisés/mamelouks, tech, ordre public, journal, victoire non signalée ; mesures campaign_probe | dev | 0257 | en cours |
| iaplay | ia #1 #3 #5 #6 : difficulté qui agit sur l'IA, mémoire de cible de siège, hystérésis d'objectifs, tour 1 Hongrie/Holstein (>250 ms) ; ia #2 #8 déjà couverts par ADR 0246 | dev | 0258 | en cours |
| restes | batvis + mapb + mapa + assets3d : maisons de siège orange (atlas/BuildingMaterials), stries de neige, décales d’aperçu sur l’eau, toits ardoise étirés, rectangle vert de Grand Perm + ocre colormap Oural (geo colormap), sol olive de Paris, réf. cassée map_freshwater.json:23 | dev | 0259 | en attente (charge machine) |
