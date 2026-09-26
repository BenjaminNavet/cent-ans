# ADR 0055 — Mise en scène des batailles : heure du jour, nuages, fumées, oiseaux, plan cinématique

Date : 2026-09-25. Statut : accepté. Lot EP8 du chantier « batailles épiques » (suivi :
`docs/wip/ep8-mise-en-scene.md`, plan `docs/wip/epic.md`). Complète les ADR 0017 (atmosphère),
0031 (échelle massive) et 0032 (horizon). Le numéro 0052 prévu au départ a été pris entre-temps
par un autre lot.

## Contexte

Le joueur veut des batailles qui aient l'allure d'un tableau ou d'une scène de film : lumière de
l'aube ou du crépuscule, ombres de nuages, poussière des charges, fumées, oiseaux qui s'envolent,
plan rapproché au premier choc. Une seule de ces idées touche aux règles : la lumière basse de
l'aube et du crépuscule gêne les tireurs. Contraintes : 15 000 soldats à 40 i/s au palier épique,
préréglages de qualité PF1, champ de taille variable (EP1), rien d'imposé au joueur.

## Décision

- **Heure du jour dans le cœur.** `data/rules/battle_time_of_day.json` (schéma
  `battle_time_of_day_rules.schema.json`), module `sim_battle::time_of_day` : phases (aube, matin,
  midi, après-midi, crépuscule, nuit) avec un facteur de **visibilité** sur la portée efficace des
  tireurs (0,7 à l'aube et au crépuscule, 0,45 la nuit), multiplié par celui de la météo
  (`BattleSim::range_factor`, lu par `effective_range`, le tir sur les murailles, les guetteurs du
  tir indirect et l'IA). Le jour avance pendant la bataille (0,2 minute de jour par seconde
  simulée : une bataille de 20 minutes dure 4 heures) ; le journal annonce chaque nouvelle phase.
- **Qui fixe l'heure.** La campagne la tire de la bataille (hachage tour, indice, province :
  aucun flux aléatoire consommé, donc aucune graine de campagne ni de bataille ne bouge) ; le pont
  l'ajoute au dictionnaire de `get_battle_setup` (clé `hour`, lue par `BattleSim.setup`) plutôt que
  d'ajouter un champ à `BattleSetup` (17 constructions littérales dans les tests et les sondes, et
  d'autres lots en parallèle). Les batailles de démonstration la choisissent (menu, `--hour=`).
  Sans heure donnée : **midi**, visibilité pleine, rendu neutre : batailles, digests et tests
  existants inchangés.
- **Rendu seulement pour le reste**, orchestré par `BattleStaging` (`data/fx/battle_staging.json`,
  schéma `fx_battle_staging.schema.json`) :
  - **Lumière** (`BattleTimeOfDay`) : images clés interpolées par heure, appliquées en **facteurs**
    sur le préréglage météo capturé après `BattleAtmosphere.apply` (élévation et lacet du soleil,
    donc longueur et direction des ombres ; énergie, couleur du soleil, du ciel, du brouillard, de
    l'ambiance ; exposition). Midi = facteurs neutres. Hors plein jour, le ciel HDRI laisse la
    place au ciel procédural teinté (disque et halo du soleil rasant) ; le panorama EP2 est
    reteinté par `BattleHorizon.apply_atmosphere`. Recalcul au plus toutes les 1,5 s et quand
    l'heure a bougé de 3 minutes (la carte de radiance du ciel n'est pas refaite à chaque image).
  - **Ombres de nuages** (`BattleCloudShadows`) : un seul `Decal` couvrant le champ et ses abords,
    texture de bruit périodique calculée une fois sur le CPU (`FastNoiseLite.get_seamless_image`,
    une tuile répétée), qui glisse avec le vent des étendards et revient d'une tuile en arrière
    (aucun saut). Un `SubViewport` animé était prévu, mais Godot refuse une `ViewportTexture`
    comme texture de décal, et relire le GPU chaque image aurait bloqué le rendu.
  - **Poussière** (B4 enrichi, `BattleEffects.configure_staging`) : force selon l'effectif du
    régiment, le terrain et la saison ; au-delà de 1, nuages plus gros plutôt que plus de
    particules ; colonnes de poussière des grosses troupes en marche au loin (2 à 8 émetteurs selon
    la qualité). Le nombre total d'émetteurs reste fixe.
  - **Fumées** (`BattleSmoke`) : API `add_smoke_source(position, intensity, kind)` (exposée aussi
    par `BattleScene` pour les camps d'EP6), budget de sources par qualité (la plus faible cède la
    place), feux de camp par défaut derrière les lignes (`auto_campfires`), lueur la nuit, fumée
    de bombarde qui s'attarde, colonnes sombres au-dessus des maisons en feu du système S2.
  - **Oiseaux** (`BattleBirds`) : volées perchées dans les bois et les haies du champ, envolées au
    premier choc ou au passage d'une charge, qui tournent au-dessus de la mêlée puis s'éloignent ;
    corbeaux à la fin de la bataille. Un seul `MultiMesh`, battement d'ailes dans le shader.
  - **Plan cinématique** (`BattleCinematic`) : au premier vrai contact, quelques secondes d'orbite
    lente à hauteur d'homme autour du choc, bandes noires, interface masquée, ralenti facultatif ;
    Espace, Échap ou un clic passent le plan ; la caméra revient exactement où elle était.
    Réglages « Plan cinématique au premier choc » et « Ralenti du plan cinématique » ; jamais en
    banc d'essai, en capture ou en IA contre IA sauf `--cinematic`.
- **Mesures A/B** : `--no-daytime`, `--no-cloud-shadows`, `--no-staging-dust`, `--no-smoke`,
  `--no-birds`, `--no-cinematic`, `--no-ep8`. Niveaux PF1 : ombres de nuages coupées en Basse,
  volées d'oiseaux, sources de fumée, feux de camp et colonnes de poussière comptés par niveau ;
  les particules suivent en plus la réduction globale de PF1.

## Conséquences

- Les batailles de campagne ont désormais une heure : à l'aube et au crépuscule les archers
  tirent de moins loin (−30 %), et une bataille longue peut basculer dans le crépuscule. Les
  batailles de démonstration sans choix d'heure tirent l'heure comme la campagne.
- L'heure n'est pas gardée dans `BattleSetup` : un appelant qui construit le setup lui-même (tests,
  sondes, EP7 pour les cartes historiques) passe par `BattleSim::set_start_hour` ou la clé `hour`
  du dictionnaire.
- Pas de lever ni de coucher du soleil selon la saison (mêmes heures toute l'année) ; pas
  d'étoiles la nuit ; les villages du champ ne brûlent pas (le système S2 ne vit que dans les
  sièges).

## EP8b — correctif du bandeau (2026-09-26)

Signalé sur Azincourt (ADR 0035, départ 10 h 30) : le bandeau affichait « Midi » après 2 min 50 de
bataille jouée, ce qui semblait trop rapide. Vérification : le calcul est correct et unique
(`BattleSim::hour`, `elapsed` en secondes de bataille simulée à pas fixe de 0,1 s, indépendant du
multiplicateur de vitesse x1/x2/x4 qui n'agit qu'en amont sur le nombre de pas par image) — c'est
la compression documentée ci-dessus qui s'applique : 170 s simulées × 0,2 min/s ≈ 34 min de jour,
donc 10 h 30 → 11 h 04, bien dans la phase « midi » (11 h-14 h). Ce n'était pas un bug de calcul
mais un défaut de lisibilité : le bandeau ne montrait que le nom de la phase (« Midi »), jamais
l'heure elle-même, si bien que le changement de phase semblait un saut plutôt qu'une progression
plausible. Correctif : `BattleScene._update_time_label` (`game/scripts/battle/battle_scene.gd`)
utilise désormais `BattleTimeOfDay.clock_label` (déjà écrit pour EP8 mais jamais branché sur le
bandeau) et affiche « Midi, 11 h 00 » plutôt que « Midi » seul ; le libellé se rafraîchit aussi
quand l'heure affichée change (pas seulement au changement de phase). Le facteur de compression
n'a pas changé.
