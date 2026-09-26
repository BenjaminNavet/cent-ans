# ADR 0060 — Musique d'époque libre de droits et bataille en couches

Date : 2026-09-25. Statut : accepté. Lot DA4 (bible `docs/design/2026-09-25-bible-da.md` § 9,
suivi `docs/wip/da4-musique.md`). S'appuie sur AU1 (`docs/wip/au1-audio.md`, `AudioDirector`,
`BattleAudio`) et B3 (`BattleMusicDirector`).

## Contexte

La bible DA (§ 9) demande une identité sonore d'époque : instruments et répertoire médiévaux
(Ars nova, Machaut, Landini, estampies, Cantigas de Santa María, Agincourt Carol…), au moins deux
pistes de campagne par culture jouable (France, Angleterre, Bourgogne/Flandre, Ibérie, Italie), et
une musique de bataille en couches (approche → engagement → mêlée → déroute/victoire) au lieu
d'une seule piste. La musique de repli (Kevin MacLeod, CC BY 4.0, ajoutée en D0/session 7) reste
trop reconnaissable et anachronique pour rester en avant. Contrainte : aucune dépense (sources
libres uniquement), moins de 80 Mo ajoutés, aucune ligne de jeu codée en dur.

## Décision

- **Sources et vérification.** Enregistrements domaine public / CC0 / CC BY / CC BY-SA (jamais NC
  ni ND) de Wikimedia Commons, licence relue page à page via l'API (`extmetadata.LicenseShortName`)
  avant tout téléchargement ; couches instrumentales de bataille (tambour, trompette droite,
  bourdon de cornemuse) tirées de Freesound, licence CC0 revérifiée sur la page de chaque son
  (même méthode que `audio_bank.verify_licence`, lot AU1). Script reproductible
  `tools/cent_ans_tools/era_music.py` (conversion ffmpeg : MP3 128 kbit/s pour la musique, OGG
  Vorbis pour les couches de bataille, normalisation `loudnorm` à ~-16 LUFS). Beaucoup de pièces
  d'Ars nova italien/français ne survivent que sous forme de réalisations MIDI publiées comme
  domaine public sur Commons (aucun enregistrement d'ensemble libre de droits trouvé) : elles sont
  retenues et clairement indiquées comme telles dans `SOURCE.md`, plutôt que d'exclure des pans
  entiers du répertoire demandé (Machaut, Landini, Binchois).
- **Playlists par culture et par contexte** (`data/audio/music.json`, schéma révisé
  `music.schema.json`) : chaque contexte (`campaign`, `campaign_france`, `campaign_england`,
  `campaign_burgundy`, `campaign_iberia`, `campaign_italy`, `war`, `court`, `menu`) porte deux
  listes, `primary` (musique d'époque, tirée en priorité) et `fallback` (repli, Kevin MacLeod
  inclus — **jamais supprimé**, seulement rétrogradé). `AudioDirector._pick_track` épuise
  `primary` avant de retomber sur `fallback`. `culture_regions` associe la culture d'une faction
  (`data/factions/<id>.json`, champ `culture`) à une région musicale ;
  `AudioDirector.campaign_context(faction_id)` choisit `campaign_<région>` si la playlist existe,
  sinon `campaign` générique. Écran-titre : nouveau contexte `menu`.
- **Bataille en couches** (`data/audio/battle_layers.json`, schéma `battle_layers.schema.json`) :
  `BattleMusicDirector` (déjà responsable de l'hystérésis et du choix de piste de base, lot B3)
  lit désormais ses couches et ses volumes par état (`approach`/`engagement`/`critical`/
  `victory`/`defeat`) depuis ce fichier de données au lieu de constantes codées en dur. Couches
  déclarées : `melee_din` (clameur de mêlée héritée de B3, coupée quand `BattleAudio` — AU1 — est
  actif pour éviter le doublon), `drums`, `straight_trumpet`, `bagpipe_drone`, `shawm`, montées et
  descendues en fondu (`fade_seconds`, `hysteresis_seconds` — également dans le fichier). Les
  enregistrements libres ne se prêtant pas à un vrai mixage à la partition (boucles courtes issues
  d'un seul son), le mélange reste un jeu de volumes par palier d'intensité plutôt qu'une
  superposition musicalement écrite — répond à la clause de repli explicitement prévue par le
  mandat.
- **Kevin MacLeod en repli.** Ses 17 pistes restent sur le disque et dans `CREDITS.md`, mais ne
  figurent plus jamais en `primary` (testé, voir plus bas) : elles ne jouent que si toutes les
  pistes d'époque d'un contexte sont absentes.

## Alternatives écartées

- **Ne garder que des enregistrements d'ensemble** (rejeter tout MIDI) : aurait réduit la
  couverture Ars nova à presque rien (peu d'enregistrements CC sur Commons pour Machaut/Landini/
  Binchois) ; la bible cite nommément ces compositeurs. Retenu avec transparence sur la nature de
  la réalisation dans `SOURCE.md` et le tableau de `CREDITS.md`.
- **Un flux « stem » multipiste par bataille** (couches vraiment composées ensemble) : demanderait
  soit des enregistrements commandés (hors budget, payant), soit une composition procédurale
  (hors mandat DA4, qui demande des enregistrements). Le mélange par volumes/paliers reproduit
  l'intention (montée en intensité perceptible) avec des sources 100 % libres.
- **Étendre `war` par culture** (`war_france`, etc.) : la bible ne le demande que pour la
  campagne ; ajouté seulement si un futur lot le justifie, pour ne pas complexifier `AudioDirector`
  sans bénéfice observé.

## Conséquences

- `data/audio/music.json` et `data/audio/battle_layers.json` valident leurs schémas
  (`tools/tests/test_era_music.py`, 10 tests : schémas, existence de chaque fichier référencé,
  crédit `SOURCE.md`, couverture des 5 régions avec ≥ 2 pistes, Kevin MacLeod jamais en primary,
  couches de bataille déclarées sur les 5 états).
- ~18 Mo ajoutés (musique + couches de bataille), très en dessous du plafond de 80 Mo.
- `AudioDirector.load_playlists` reste rétro-compatible avec l'ancien format liste simple
  (`entry is Array`) si un autre lot en dépendait encore au moment du merge.
- Limite connue : pas d'écoute humaine des niveaux (session sans sortie audio) ; à affiner à
  l'oreille comme le signalait déjà AU1.

## DA7a — vrais enregistrements libres d'Ars nova (2026-09-26)

Complément sans nouvel ADR (suivi `docs/wip/da7a-ars-nova.md`). But : remplacer en tête de
playlist les rendus MIDI d'Ars nova (Machaut, Solage, Landini, Binchois) par des interprétations
réelles sous licence libre, le MIDI restant en `fallback`.

**Trouvé et retenu** (licence relue sur la page Commons de chaque fichier, via l'API) :

- Bande de concert du **Studio der frühen Musik** (Thomas Binkley, Andrea von Ramm, Nigel Rogers,
  Sterling Jones), Stockholm, 23 octobre 1963, numérisée et versée sur Commons par Musikverket
  (Svenskt visarkiv) en **domaine public** (`{{PD-old}}` ; enregistrement non publié, droits
  voisins suédois de 50 ans échus). Copie d'écoute montée, sans annonces : 10 plages séparées par
  ~15 s de silence numérique, découpées par `silencedetect` et attribuées dans l'ordre du
  programme de la page (10 pièces). Retenues : Jacopo da Bologna « Fenice fu » et un Saltarello
  (Italie), « Onques ne fut », « Souvent souspire », Pierrekin de la Coupele, « Hé Robinet »
  (France), Binchois et Dufay « Adieu m'amour » (Bourgogne), « Bryd one brere » (Angleterre).
- « Bel fiore dança » (codex de Faenza), Francesco Ariis au clavier, **CC BY 4.0** (Italie).

10 pistes, ~20 Mo, OGG Vorbis q5, `loudnorm` -16 LUFS (mesuré -14,3 à -16,5 LUFS), pipeline
reproductible `tools/cent_ans_tools/ars_nova.py` (écrit aussi `SOURCE.md` et les `.import`
Godot). Crédits : `game/assets/third_party/music/ars_nova/SOURCE.md`, `CREDITS.md`. Placement
(`data/audio/music.json`, en tête de `primary`) : `campaign_france` (4), `campaign_italy` (3),
`campaign_burgundy` (2), `campaign_england` (1), `court` (3), `menu` (1), `campaign` (2).
Test `tools/tests/test_ars_nova_recordings.py` : France et Italie ont au moins un enregistrement
réel DA7a en `primary`, aucun rendu MIDI en `primary`, et chaque piste tierce déclare une
licence autorisée (domaine public, CC0, CC BY, CC BY-SA ; NC/ND refusés).

**Écarté** :

- Commons : Machaut, Landini, Solage, Vitry, Senleches, Ciconia, Dunstable, Power n'ont que des
  réalisations MIDI (Tetraktys, Future Perfect…) ou des extraits de 15-30 s sans source claire
  (« Je ne cuit pas qu'onques », « Le harpe de melodie »).
- Commons, concert d'orgue de **Gotthard Arnér** (1966, Musikverket, domaine public) : contient
  la ballata de Landini « Angelica biltà » et le *Lamento di Tristano*, mais la bande (15 plages
  pour 11 titres, dont une partie très faible) ne peut pas être associée au programme sans
  écoute ; non utilisée pour ne pas mal attribuer. Piste de suite la plus prometteuse.
- archive.org : « ProyectoMachaut » (CC BY-NC-SA), « Kyrie/Credo » Nielrow et Ensemble Le Remède
  de Fortune (BY-NC-ND), STRANG (BY-NC-SA) — NC/ND interdits. « Messe de Nostre Dame » marquée
  domaine public par l'uploader (interprètes inconnus, LP des années 1950 : statut aux États-Unis
  douteux) et « Srednjeveške ljubezenske pesmi » (netlabel Vanzemlja, CC0, interprète non
  identifié, pas de métadonnées) : licence non vérifiable, écartés. « Missa de Barcelona »
  (Atrium Musicae, disque commercial marqué PD) : écarté.
- Free Music Archive : Gregor Quendel « Douce Dame Jolie » (BY-NC-ND). Musopen : pas de
  répertoire médiéval.

**Manques** : aucun enregistrement réel libre de Machaut, Landini (hors bande Arnér), Solage ni
de la *Messe de Nostre Dame* : ces titres restent en MIDI (`fallback`). Pas d'écoute humaine :
l'attribution titre ↔ plage de la bande de 1963 et les niveaux sont à confirmer à l'oreille.
