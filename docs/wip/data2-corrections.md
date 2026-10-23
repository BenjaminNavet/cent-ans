# DATA2 — corrections de données et de textes (itération 3 du juge)

## Faites
- Journal des causes : `data/ui/causes.fr.json` utilise désormais `{de_lieu}`, `{de_sujet}`, `{de_acteur}` (élision via `sim_campaign::events::de`, remplis dans `game-api::causes`), plus de « perdue » accordé au féminin, tournures neutres (« Perte d’Agenais, au profit d’Angleterre », « Défaite face à Albret, près d’Agen »). Apostrophes typographiques. Test : `game-api::causes::tests::summaries_elide_and_never_agree_with_a_place_name`. Schéma : marqueurs documentés.
- Humbert II : André mort en 1333 (codex + description du personnage), « évêque de Paris » remplacé par patriarche latin d’Alexandrie et administrateur de l’archevêché de Reims. `codex_bundle.json` régénéré.
- Codex Casimir III : « de Iouri » devient « d’Iouri ».
- Apostrophes droites restantes converties par `tools/typo_apostrophes.py` (82, clés d’affichage seulement).
- Sondage de 30 chaînes `data/ui` et `data/events` : aucune autre élision fautive (« de Ayala » est correct, nom castillan).

## Refusées / à faire ailleurs
- Prix de l’achat du Dauphiné (12 000 ₶) : conservé. Le texte de l’événement n’affiche aucun chiffre historique ; l’échelle du jeu (ADR 0183 : trésor de départ de la France 28 000 ₶) rend 12 000 proportionné aux 120 000 florins de Romans.
- Maître de Calatrava marié : aucun conjoint dans `chr_juan_nunez_de_prado.json` ; le mariage vient de l’IA (événement `mariage_de_notables` et négociation, Rust). Correction à faire en code : exclure `government: order`/théocratie militaire.
- `prov_murcia` : capitale Séville, aucune colonie « Murcie » (colonies : Séville, Cordoue, Cadix, Carthagène…). Capitale non modifiée ; correction recommandée : renommer la province (« Andalousie et Murcie ») ou créer une colonie Murcie, choix de conception.
- Doublon de messages (cause_id null + cause) pour Agen/Lille : vient du fil d’événements (siège + `capture::on_captured`), logique Rust, non traité. Boulonnais perdu deux fois : événements réels du jeu.
