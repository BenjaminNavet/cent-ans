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

## État
- [ ] squelette (note, enveloppe, script) commité
- [ ] appels fal
- [ ] planches T1 / T1+retex / Meshy gen
- [ ] verdict + recommandation abonnement (ADR si adopté)

## Prochaine étape
Lancer `meshy_bench.py run`, puis `sheets`.
