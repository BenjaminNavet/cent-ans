# MX — banc Meshy (13/10)

Demande du joueur : essayer Meshy (meshy.ai) ; abonnement Pro 20 $/mois (1 000 crédits) si le gain est net,
puis génération massive. Enveloppe du banc : **8 $ via fal** (validée par le joueur le 13/10).

## Banc (`tools/experiments/meshy_bench.py`)
Deux usages comparés au T1 livré, mêmes objets :
- **gen** : `meshy/v7.1/multi-image-to-3d` texturé (1,20 $), à partir de nos vues découpées (face, dos, côté) :
  city_west, castle_west, siege_tower, siege_trebuchet_counterweight.
- **retex** : `meshy/v6-lite/retexture` (0,30 $), UV d'origine gardées, sur le lod0 livré, style = image
  découpée + prompt : les 4 ci-dessus + bombard, abbey_west, animal_horse_draught.
Total prévu 6,90 $. Le script refuse tout appel qui ferait dépasser 8 $ (`costs.json`).
Sorties : `~/dev/cent-ans-raw/meshy/<id>/` ; planches `~/dev/cent-ans-raw/meshy/sheets/`.

## Si adopté : passer par le MCP Meshy (consigne du joueur, 13/10)
- Serveur officiel `@meshy-ai/meshy-mcp-server` (source meshy-dev/meshy-mcp-server), clé `MESHY_API_KEY`
  (Developer Platform → API Keys ; abonnement payant requis pour l'API).
- Installation **portée utilisateur, jamais dans `.mcp.json`** (dépôt public, clé secrète) :
  `claude mcp add meshy -s user -e MESHY_API_KEY=msy_… -- npx -y @meshy-ai/meshy-mcp-server`.
- Outils utiles : `meshy_multi_image_to_3d`, `meshy_image_to_3d`, `meshy_retexture`, `meshy_remesh`,
  `meshy_get_task_status`, `meshy_download_model` (vers `~/dev/cent-ans-raw/meshy/`), `meshy_check_balance`
  (avant chaque vague). Puis ingestion habituelle (`cent-ans dn-ingest`).
- Coût en contexte : schémas des ~24 outils à mesurer à l'activation ; ne l'activer que pour les sessions
  de génération (même règle que le MCP d'éditeur).

## Essai du joueur (13/10, crédits gratuits, site web)
Coutilier depuis la planche nb2-l4 (face/profil/dos, format recommandé par Meshy) : résultat très bon
(visage, rivets, maille, ceinture, bourse, dague nets), nettement au-dessus du T1. Téléchargement réservé
aux abonnés. Abonnement relevé par le joueur : **20 €/mois pour 100 assets** (≈ 0,20 €/asset, ×6 moins
cher que fal à 1,20 $).

Ordre proposé à 100/mois : M1 = 16 figurines + 17 châteaux + 17 villes + ~30 engins de siège + 12 abbayes
+ ~8 réserve ; ensuite les objets proches de la caméra en bataille ; le petit décor lointain reste en T1.

## Abonnement pris (13/10)
- MCP `meshy` installé portée utilisateur (`~/.claude.json`, hors dépôt), connecté ; actif à la prochaine
  session. Clé jamais dans le dépôt : crochet local `pre-commit` (dépôt commun) refuse tout `msy_…`.
- Solde API : 1 060 crédits/mois. Barème API : image→3D texturé meshy-6/7.1 = 30 crédits (≈ 35 modèles),
  meshy-6-lite texturé = 15 (≈ 70), retexture = 10. Les « 100 assets » du site ≠ barème API : à mesurer.
- Décor : **vue de face seule** (essai du joueur sur l'abbaye byzantine : dos fidèle). Multi-vues gardé
  pour les figurines (planches 3 vues).
- Plugin Blender Meshy : pont interactif (import dans l'interface), inutile pour nos scripts headless.

## État
- [ ] squelette (note, enveloppe, script) commité
- [ ] appels fal
- [ ] planches T1 / T1+retex / Meshy gen
- [ ] verdict + recommandation abonnement (ADR si adopté)

## Prochaine étape
Lancer `meshy_bench.py run`, puis `sheets`.
