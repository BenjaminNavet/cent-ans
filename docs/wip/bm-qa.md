# bm-qa : fumée et banc (Bevy)

Fait : `client/tests/smoke.sh` + 10 scénarios, `--bench-map` (`agent/bench.rs`) + `tools/bevy_bench/bench.sh`, marqueurs `AGENT_EXIT`, `CENT_ANS_BACKTRACE`.

Panique trouvée (hors dossier, non corrigée, source probable) : `campaign/countryside/decor.rs:535`, changement de niveau de détail (`commands.entity(scene).insert(WorldAssetRoot(..))`) sur l'enfant d'une instance déjà supprimée dans la même image (le même système retire les instances périmées plus haut) : utiliser `try_insert`. Reproduit par la descente du banc (150 -> 5) en release et en debug (type de la commande lu dans la pile : `insert<WorldAssetRoot>`) ; le scénario `campaign_descent` de la fumée l'attrape. Ligne exacte non confirmée par un correctif (hors dossier) ; l'autre candidat est `campaign/markers/mod.rs:810`.
Restes : scénario de fumée avec zoom de la caméra (LOD) ; compteur d'appels de dessin (Bevy n'en expose pas) ; chiffres du banc sur machine calme (charge 80-100 pendant la mesure).
