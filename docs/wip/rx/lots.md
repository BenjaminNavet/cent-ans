# RX — lots de correction

| lot | source | agent | ADR | état |
|---|---|---|---|---|
| victory | mecaniques #1 #2 #11 | dev | 0245 | PARTIEL (ec50ccc7d) : #1 #2 faits ; #11 victoire non signalée sans bloc `victory` → lot équilibrage |
| probe | mecaniques #3 (sonde de campagne) | mech | 0246 | FAIT (campaign_probe, ADR 0246) |
| audio | audio (tous) | mech | 0247 | à faire |
| histoire | historien (tous) | mech | 0248 | à faire |
| uifin | ui (tous) | mech | 0249 | à faire |
| anim | animation (tous) | dev | 0250-0251 | à faire |
| (vague 2) | bataille-v2, campagne-v2, ia, perf, équilibrage campagne avec la sonde | — | 0252+ | en attente |
| mapa | campagne + campagne-v2 : dalles blanches (champs/parcelles), maquettes blanches surexposées, sol de Paris | dev | 0252 | FAIT (gain d’albédo lu en sRGB, test rx_mapa_textures_test ; sol olive de Paris non traité) |
| mapb | campagne + campagne-v2 : nuages opaques, rouge anglais en aplat, étiquettes, fleuves du parchemin, camouflage au dézoom, Oural, colormap/zoom | dev | 0253 | en attente (place libre) |
| batvis | bataille + bataille-v2 + assets3d : caméra closeup dans un toit, motif de feuilles du sol à mi-distance, neige en lattes, maisons de siège orange trouées, échelles orange, caméra de déploiement, mêlée lisible (contours), aperçus sur l'eau | dev | 0254 | en cours |
| batsim | bataille : pont unique en colonne, déroute par contagion (arbalétriers 0 perte), régiment anéanti 0 tué, sonde d'issue `--autoplay` avec graine, smoke_battle court + test long, message « Chevaliers à placer » permanent, avertissement CampaignSim | dev | 0255 | en cours |
| robust | perf (tous sauf tests UI de mise en page → après uifin) : tests bloqués/cassés, erreurs moteur, migration de sauvegarde, smoke bruyant | dev | 0256 | en cours |
| equil | mecaniques + ia + victory #11 : équilibrage campagne avec campaign_probe | dev | 0257-0258 | en attente (rapport IA) |
