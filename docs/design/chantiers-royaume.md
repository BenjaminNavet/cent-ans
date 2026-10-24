# Vue d'ensemble des chantiers du royaume

Statut : validé par le joueur (2026-10-24). Spec avant implémentation.

## Problème
Avec 35 emplacements libres, il faut ouvrir chaque province puis son arbre de bâtiments (`buildings_view`, `client/src/campaign/panels/province/view.rs`). Trop fastidieux.

## Décision
Un onglet « Chantiers » dans le panneau de faction (`client/src/campaign/panels/faction`), joueur uniquement. L'onglet Bâtiments des provinces reste inchangé (détail).

## Contenu de l'onglet
- En-tête : emplacements libres (total), chantiers en cours, trésorerie, coût de la sélection.
- Une ligne par colonie : nom (clic = ouvre la province), palier, emplacements libres/total, chantiers x/y. File pleine : ligne grisée. Menu des bâtiments constructibles (coût et durée en infobulle).
- Filtres : colonies avec emplacement libre (défaut), région, type de colonie.
- « Appliquer à tout » : un bâtiment choisi dans l'en-tête est posé dans chaque colonie où c'est possible. Récapitulatif : colonies concernées, coût total, colonies ignorées avec raison (fonds, file pleine, prérequis).
- Bouton « Lancer (N chantiers, X écus) ». Fonds insuffisants : le lot suit l'ordre de la liste et une entrée refusée n'arrête pas les suivantes (un chantier moins cher peut passer) ; le récapitulatif montre chaque ligne lancée ou ignorée avec sa raison, le total et un avertissement avant validation.

## Règles (core/)
- Aucune règle dans le client. Réutiliser `order_build` (`core/crates/sim-campaign/src/orders/build.rs`) pour chaque chantier : mêmes contrôles, même paiement.
- Nouvelle requête : chantiers possibles par colonie (`buildable` existant, agrégé sur les colonies du joueur) avec emplacements libres et file.
- Nouvelle commande de lot : liste de (colonie, bâtiment), appliquée dans l'ordre via `order_build`, retourne par entrée succès ou `OrderError`. Un échec n'annule pas les autres.
- Même chemin que les ordres du joueur : cohérent pour les seed bots.

## Tests
- Rust : lot avec fonds insuffisants (arrêt dans l'ordre), file pleine, prérequis non remplis, bâtiment interdit dans le type de colonie, échec partiel.
- Client : dump d'état filtré de l'onglet, script `click faction:tab:works` puis `click` par nom ; au plus une capture de contrôle. Lancement via `client/run_bg.sh` (muet, hors écran).

## Hors périmètre
Construction automatique par politique de colonie (écartée pour cette itération). Modification de l'onglet Bâtiments des provinces.

## Réalisation (2026-10-24)
- Noyau : `core/crates/sim-campaign/src/realm_works.rs` (`realm_works`, `plan_works_batch`, `apply_works_batch` sur `Order::Build`, pas de nouvel ordre).
- Client : onglet `faction:tab:works` ; le « menu » par colonie est un bouton qui cycle entre les bâtiments disponibles.
