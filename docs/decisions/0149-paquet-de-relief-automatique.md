# 0149 — Le paquet de relief se met à jour tout seul

Date : 2026-10-02 (`docs/wip/relief-auto.md`). Complète l'ADR 0077 (hébergement du relief fin).

## Contexte

Le cache du relief fin (`data/map/pyramid/`, ≈ 5 Go, hors git) est publié en paquet « Cent Ans
relief » (Releases GitHub de `BenjaminNavet/cent-ans-relief`). Trois gestes manuels le tenaient à
jour, et tous trois ont été oubliés :

- RS-G (28/09) a monté `detail_dem.BAKE_VERSION` à 6 sans toucher `bake_versions` de
  `relief_pyramid.json` : `geo relief-all --check` comparait le cache au manifeste, pas au code,
  et disait « complet ». Le paquet v2 (29/09) a donc été publié avec un palier 3 périmé ;
- recuire, relancer `geo towns` et `geo landmarks`, empaqueter et créer la Release étaient cinq
  commandes à enchaîner de mémoire, la dernière réservée à l'accord du joueur ;
- côté joueur, rien ne signalait qu'un paquet plus récent existait : `relief-fetch` retéléchargeait
  5 Go sans savoir si le cache installé valait déjà le paquet.

## Décision

1. **Le code fait foi pour les versions de cuisson.** `relief_update.code_bake_versions()` lit
   `pyramid.TIER_VERSIONS` et `detail_dem.BAKE_VERSION` ; un test
   (`test_manifest_bake_versions_follow_the_code`) échoue dès que `relief_pyramid.json` ne suit
   plus, et `geo relief-update` réaligne le manifeste avant toute chose.
2. **Une commande, `cent-ans geo relief-update`**, enchaîne : manifeste ← code ; recuisson des
   étapes périmées ou manquantes (`relief_cache.rebuild`) puis `towns` et `landmarks` ; si
   l'empreinte de cuisson diffère du paquet publié, `relief-pack` (qui incrémente la version) et
   publication de la Release `v<N>` avec `gh`. Chaque étape est sautée quand elle n'a rien à
   faire. Le manifeste du paquet est envoyé en dernier : une Release interrompue reste « non
   publiée » et la relance la complète. Les parts sont supprimées après une publication vérifiée.
3. **La publication est automatique** (accord durable du joueur, 02/10, pour ce seul dépôt de
   données ; `--no-publish` s'arrête après l'empaquetage). Les Releases du jeu et tout autre dépôt
   restent soumis à un accord explicite.
4. **Le cache dit quel paquet il contient** : `pyramid/package.json` (version, empreinte), écrit
   par `relief-pack` avant l'archivage, donc présent dans chaque paquet, et par `relief-fetch`.
   `relief-fetch --if-needed` ne télécharge que si la version installée est plus ancienne que
   `relief_hosting.json`. Un cache sans marque (antérieur à cet ADR) est jugé sur `bake.json` :
   en retard si un palier est plus ancien que celui du paquet, sinon adopté. Un cache recuit
   localement en avance sur le paquet n'est donc jamais écrasé.
5. **`tools/launch.sh` vérifie le relief à chaque lancement** (étape 3) : comparaison en bash de
   deux numéros de version (instantanée), et seulement s'ils diffèrent,
   `relief-fetch --if-needed`. Un échec (pas de réseau, pas d'`uv`) n'empêche jamais de jouer ;
   `--no-relief` saute l'étape.

## Conséquences

- Après un changement de cuisson : `uv run --project tools cent-ans geo relief-update`, puis
  commiter et pousser les fichiers suivis de `data/` (dont `relief_hosting.json`, qui annonce la
  nouvelle version aux autres postes). Pousser avant la fin de la publication donnerait un 404
  aux lanceurs : la commande publie d'abord.
- Premier lancement après un clone : `launch.sh` télécharge ≈ 5 Go (reprise HTTP si interrompu).
  C'est voulu (le zoom rapproché en dépend) et annoncé dans le terminal.
- La cuisson reste locale (14 Go de sources brutes, 30-45 min) : pas d'intégration continue.
- `relief-update` a besoin de `gh` authentifié sur le compte propriétaire du dépôt de données.
- Les fleuves et routes fins n'ont toujours pas de version de cuisson propre (seulement
  `generated_at`) : un changement de `hydro_fine.SNAP_VERSION` seul n'est pas détecté comme
  périmé. À traiter si le cas se présente.
