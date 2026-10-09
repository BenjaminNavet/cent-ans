# 0212 — Les modèles 3D générés voyagent en paquet de release, pas par git

Date : 2026-10-09 (`docs/wip/dn-paquet-modeles.md`). Calqué sur l'ADR 0077 (hébergement du relief) et
l'ADR 0149 (mise à jour automatique).

## Contexte

La nuit DN a commité dans `main` environ 890 Mo de glb générés (`game/assets/models/dn/**`, 2 001
glb `_lod0/1/2` plus 378 textures `*_Image_0.jpg` extraites par Godot à l'import). Ces commits ne sont
pas poussés et le dépôt est public : les pousser alourdirait le dépôt de façon définitive (clone de
plus de 500 Mo compressés pour des fichiers régénérables). Le relief fin pose le même problème depuis
l'ADR 0077 et l'a résolu par un paquet publié dans les Releases de `BenjaminNavet/cent-ans-relief`.

Mesure en lecture seule (09/10, `git rev-list --objects origin/main..main` + `git cat-file
--batch-check`), 299 commits non poussés :

| | blobs | taille brute | sur disque (delta + zlib) |
|---|---|---|---|
| `game/assets/models/dn/**` | 2 127 | 905,8 Mo | 505,3 Mo |
| tout le reste | 2 908 | 138,1 Mo | 101,8 Mo |

Sans ces glb, l'historique non poussé pèse donc 138 Mo bruts (102 Mo sur disque), contre 1 044 Mo.

## Décision

1. **Un paquet « Cent Ans modèles »**, publié dans les Releases `models-v<N>` de
   `BenjaminNavet/cent-ans-relief` (même dépôt de données que le relief, étiquettes distinctes).
   Archive tar non compressée de `game/assets/models/dn/` (les glb sont déjà compressés), coupée en
   parts de moins de 1,9 Gio, manifeste JSON avec SHA-256 par part. Les fichiers que Godot écrit à
   l'import (`*.import`, `*.uid`, `*_Image_<n>.jpg|png`) n'en font pas partie.
2. **Commandes** (`tools/cent_ans_tools/models_package.py`, qui réutilise `relief_pack.archive_tree`,
   `relief_fetch.download_part/verify_parts/extract_atomic` et `relief_update.publish` sans les
   dupliquer) :
   - `cent-ans art models-pack` : empaquette ; incrémente la version si l'empreinte a changé ;
   - `cent-ans art models-fetch [--if-needed]` : télécharge (reprise HTTP Range), vérifie, installe
     atomiquement ; `--from-dir` pour des parts locales ;
   - `cent-ans art models-update [--no-publish]` : empaquette si l'arbre diffère du paquet publié, puis
     crée la Release `models-v<N>` avec `gh` (manifeste envoyé en dernier).
   La publication reste soumise à l'accord du joueur : l'accord durable de l'ADR 0149 ne couvre que
   le paquet de relief.
3. **Fichier suivi par git : `data/art/dn_models_hosting.json`** (version, empreinte `content.signature`
   = SHA-256 sur les couples chemin / SHA-256 de fichier, URL de base), validé par
   `data/schemas/art_dn_models_hosting.schema.json`. `data/art/dn_manifest.json` reste suivi.
4. **Marque d'installation : `game/assets/models/dn/package.json`** (une clé par ligne, lue par
   `tools/launch.sh`). Un arbre sans marque mais contenant des modèles (ingérés ici, ou récupérés par
   git avant cet ADR) est adopté, jamais écrasé.
5. **`tools/launch.sh`** (étape 1b, avant l'import Godot pour qu'il voie les glb) compare la version de
   `dn_models_hosting.json` à celle de la marque et, si elles diffèrent, lance
   `models-fetch --if-needed`. Un échec n'empêche jamais de jouer ; `--no-models` saute l'étape. Le
   lanceur Windows (ADR 0153) délègue tout à `launch.sh` : rien à changer. Dossier d'installation :
   `CENT_ANS_MODELS_DIR` (défaut `game/assets/models`).
6. **Le jeu fonctionne sans le paquet** (vérifié le 09/10, tests passés avec `dn/` vide) :
   `ModelLibrary.get_scene` rend `null` pour un glb absent ; `TownMaquetteLayer` garde alors la
   maquette stylisée (`dn_index` = -1) ; `FaunaLayer` retombe sur le gabarit `placeholder` puis sur
   une boîte ; `MapBirdFlocks` est procédural (aucun glb) ; `FreshwaterLayer` et `AgriSeasons` testent
   l'existence du fichier ou n'ont pas de consommateur. `dn_campaign_models_test.gd` saute, avec un
   message, la vérification « chaque entrée livrée pointe un glb » quand aucun glb livré n'existe et
   que `dn/package.json` est absent : la CI (sans paquet) reste verte.
7. **Ingestion** : `cent-ans dn-ingest` écrit toujours dans `game/assets/models/dn/` et dans
   `dn_manifest.json`, mais les glb ne sont plus commités (voir `docs/pipeline-assets-3d.md`) ; on
   lance ensuite `cent-ans art models-update`.

## Conséquences

- Un clone neuf n'a pas les modèles ; le premier `tools/launch.sh` télécharge ≈ 0,9 Go (ou le jeu
  tourne avec les maquettes de repli).
- Tant que les glb sont dans l'historique non poussé, `models-update --no-publish` et le fetch
  fonctionnent déjà, mais ignorer le dossier ne servirait à rien (git suit déjà les fichiers) et
  masquerait l'état : **le `.gitignore` n'est pas modifié à ce stade.**
- Le fichier suivi `dn_models_hosting.json` livré ne porte pas encore d'empreinte : la première
  `models-update` fixera la version 1 et l'empreinte, à commiter après la publication.

## Étape restante (actions à faire avec l'accord du joueur, pas par un agent)

Retirer les glb de l'historique non poussé, publier le paquet, puis pousser :

1. **Sauvegarde mirror d'abord** : `git clone --mirror /Users/jean_hubert/dev/game_project
   ~/dev/cent-ans-backup-pre-dn-glb.git` (ou `git bundle create … --all`), et noter le hash de `main`.
2. **Réécrire les seuls commits `origin/main..main`** (aucun commit déjà poussé ne change) dans un
   worktree dédié partant de `main` : `git filter-repo --path game/assets/models/dn --invert-paths
   --refs origin/main..main` n'accepte pas de plage, on utilise donc `git rebase --onto` ou
   `git filter-branch --index-filter 'git rm -r --cached --ignore-unmatch -q game/assets/models/dn'
   origin/main..main` dans ce worktree. Vérifier : `git rev-list --objects origin/main..<nouveau>`
   ne contient plus `game/assets/models/dn/` et `git diff main <nouveau> --stat -- . ':!game/assets/models/dn'`
   est vide.
3. **`git update-ref refs/heads/main <nouveau> <ancien hash de main>`** (compare-and-swap : échoue si
   une autre session a avancé `main` entre-temps ; recommencer alors à l'étape 2).
4. **Aligner l'index partagé** du checkout principal sur le nouveau `main` (`git read-tree` sur
   l'arbre du nouveau `main`, sans toucher aux modifications non commitées des autres sessions) :
   les glb restent sur le disque, mais deviennent non suivis, puis ignorés (étape suivante).
5. **Ajouter au `.gitignore`**, après la ligne `!game/**/*.import` :
   ```
   # Modèles 3D générés : paquet de release (ADR 0212), installés par tools/launch.sh
   game/assets/models/dn/
   ```
6. **Publier la Release avant le push** : `uv run --project tools cent-ans art models-update`, puis
   commiter `data/art/dn_models_hosting.json`. Pousser avant la fin de la publication donnerait un 404
   aux lanceurs.
7. Pousser `main`, et seulement alors supprimer la sauvegarde si le joueur le décide.
