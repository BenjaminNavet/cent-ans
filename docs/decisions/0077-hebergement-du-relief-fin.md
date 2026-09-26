# ADR 0077 — Hébergement du relief fin « Cent Ans relief »

Date : 2026-09-26. Statut : accepté (publication effective soumise à l'accord du joueur, voir
« Conséquences »). Chantier SZ (`docs/wip/sz-suites-zoom.md`), suite de l'ADR 0036 (addendum ZG7b).

## Contexte

Le relief fin (pyramide E1-E7, fleuves et routes fins) pèse ≈ 2,8 Go hors git
(`data/map/pyramid/`). ZG7b a prévu trois modes d'export (`CENT_ANS_EXPORT_RELIEF=bundle|external|none`)
et un dossier « Cent Ans relief » cherché à côté de l'application, mais ce dossier n'est hébergé
nulle part : un joueur qui reçoit le jeu sans relief ne peut que le recalculer (plusieurs heures,
≈ 20 Go de données brutes à télécharger).

Sources : Copernicus DEM GLO-30/GLO-90 (licence Copernicus, redistribution libre avec mention),
IGN RGE ALTI (Licence Ouverte 2.0), Environment Agency LIDAR (OGL v3), DHM Vlaanderen (licence
flamande de réutilisation gratuite), AHN (CC0). Toutes permettent la redistribution de produits
dérivés avec attribution ; les crédits existent déjà (`docs/credits.md`, ZG7b).

## Options

- **A. Tout embarquer dans l'application** (`bundle`) : un seul fichier, mais ≈ 3 Go à chaque
  mise à jour du jeu, même quand le relief ne change pas ; au-delà des limites par défaut de
  certaines boutiques (itch.io : 1 Go sans demande).
- **B. Git LFS du dépôt du jeu** : quota gratuit de 1 Go, payant au-delà ; dépôt privé.
- **C. Versions (Releases) GitHub d'un dépôt public dédié aux données** (`cent-ans-relief`) :
  gratuit, sans limite de bande passante, fichiers ≤ 2 Gio (archive découpée en parts), versionné
  indépendamment du jeu ; le code du jeu reste privé.
- **D. Canal séparé sur itch.io (butler)** : gratuit, correctifs différentiels, mais lie le relief à
  une boutique qui n'est pas encore choisie.

## Décision

**Option C**, avec **A** conservée pour une livraison hors ligne. Le relief est publié comme
« Cent Ans relief » vN (N = version de cuisson de la pyramide) : archive `.tar` découpée en parts
< 1,9 Gio, manifeste JSON (version, parts, tailles, SHA-256, crédits). Le jeu ne télécharge rien
seul : l'avis « relief incomplet » (ZG7b) propose la commande de récupération
(`cent-ans geo relief-fetch`), qui télécharge, vérifie et installe dans le dossier « Cent Ans
relief » (ou `CENT_ANS_RELIEF_DIR`). L'URL de base est une donnée (`data/map/relief_hosting.json`),
pas une constante. Si une boutique est choisie plus tard, D pourra doubler C sans changer le format.

## Conséquences

- Outillage (lot SZ7) : `cent-ans geo relief-pack` (archive + manifeste), `relief-fetch`
  (téléchargement repris, vérification, installation atomique), tests hors réseau.
- La **création du dépôt public et l'envoi des ≈ 3 Go** sont une publication sous le compte du
  joueur : ils restent à faire par lui ou avec son accord explicite (`gh release create`,
  commande consignée dans `docs/geo.md`). Coût : 0 $.
- Toute recuisson de la pyramide (par exemple SZ2) incrémente la version du paquet.
