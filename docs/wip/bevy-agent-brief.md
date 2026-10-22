# Consignes communes des agents (chantier Bevy 0.20)

À lire avant tout lot. Chemin durable : `docs/wip/bevy-agent-brief.md` (le dossier temporaire de session est nettoyé).

## Git
- Travailler dans SON worktree isolé, sur sa branche. Commits explicites `git commit -m ... -- <chemins>`, jamais `-a`, jamais `git add -A`, jamais `git stash`. Aucune ligne Co-Authored-By.
- Premier commit (squelette) sous 5 min, puis commits `wip:` réguliers ; note `docs/wip/<lot>.md` à jour (état, prochaine étape).
- Ne pas fusionner dans main (la session principale le fait). Commencer par `git merge --no-edit main` (merge normal, pas `--ff-only`).

## Zones
- Ne pas toucher : `client/src/asset_processing.rs`, la feature `processed-assets`, les fichiers d'un autre lot en cours (la consigne du lot les nomme).
- Règles de jeu dans `core/`, client = rendu/UI/entrées. Réglages dans `data/` ou `game/resources`, jamais en dur. Code en anglais, docs en français.

## Environnement (pièges appris)
- Un worktree n'a que les fichiers versionnés. Avant tout lancement ou capture : `ln -s /Users/jean_hubert/dev/cent-ans-raw/compose/pyramid data/map/pyramid` et `rsync -a --ignore-existing /Users/jean_hubert/dev/game_project/game/assets/ game/assets/` (sinon rendu et captures faux).
- Disque : compiler avec `CARGO_INCREMENTAL=0`. Un seul `cargo` à la fois par worktree. Supprimer `client/target` de son worktree en fin de lot si le disque est bas (`df -h /`).
- Lancer le client UNIQUEMENT via `client/run_bg.sh` avec `CENT_ANS_MUTE=1` et `--offscreen` (jamais de fenêtre, jamais `osascript`). Tuer un client par pid (`ps -eo pid,command | grep cent-ans`) ; `pkill -f` ne marche pas. Ne laisser aucun client en vie.
- Machine partagée : un seul banc release à la fois ; ne lancer aucun banc sans l'autorisation de la session principale. Mesures relatives si la charge machine est élevée (`uptime`).
- Mac / Metal d'abord (M4 Pro). Windows n'est jamais un critère de rejet.

## Metal : limites qui nous ont mordus
- 16 samplers par étage fragment. Vue forward = 8 (4 ombres, LTC, DFG, environnement, décals) ; `StandardMaterial` = 6 même avec les features `pbr_*_textures` coupées. Ne pas ajouter de sampler ; fusionner les descripteurs identiques, `textureLoad` pour les lectures nearest, `bilinear_clamped` pour le linéaire clampé.
- Solari impose le rendu différé (G-buffer) : un matériau forward maison est écarté de la passe principale s'il est résolu en différé ; voir `docs/wip/bevy-solari.md`.

## Vérification
- Tests/clippy : `cd client && source sccache-env.sh && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo clippy --features dev --all-targets -- -D warnings && cargo test --features dev`. Échecs connus sans rapport s'ils réapparaissent : `campaign::markers::dn::tests::real_table_and_glbs` (glb absent d'un worktree).
- Démarrage : bataille ET campagne doivent rendre 300 images sans erreur : `CARGO_INCREMENTAL=0 CENT_ANS_MUTE=1 CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug client/run_bg.sh --state Battle|Campaign --frames 300 --offscreen --dump-state out/<nom>.json`.
- Captures : `--shot out/shots/<nom>.png --frames 500` ; budget 5 par tâche, jamais en boucle ; décrire en une phrase ce qu'on voit (monde noir = échec). Texte/dump d'abord.

## Rapport
Concis : ce qui est fait (commits), ce qui est vérifié, ce qui ne l'est pas, blocages, fichiers touchés.
