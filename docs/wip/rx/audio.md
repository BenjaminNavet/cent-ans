# RX audio — revue sound design / direction musicale

Méthode : lecture de `data/audio/*.json`, `game/scripts/audio/*.gd`, `battle_music.gd`, ADR 0145/0166 ; mesures `ffprobe`/`ffmpeg volumedetect` sur les 141 sons hors voix, les 72 morceaux de `third_party/music` et les 294 voix (niveau moyen/crête en dBFS, durée, taille) ; contrôle de présence de tous les chemins des JSON et du manifeste de voix. Aucune capture, aucun fichier du dépôt modifié.

## 1. Verdict
- Forces : zéro fichier manquant (81 chemins JSON, 294 voix du manifeste, toutes les `files` de `sound_bank.json`) ; chaîne de bus propre (limiteur maître -0,5 dB, compresseurs par bus, sidechain Voix -> Musique, filtre/réverb « lointain ») ; ambiances et nappes normalisées (~ -20 dBFS moyen) ; voix criées ElevenLabs et chœur de cri de guerre mesurés et cohérents (-13 LUFS).
- Faiblesse majeure 1 : aucune normalisation de sonie des morceaux (moyenne de -35,6 à -11,7 dBFS selon la piste) et enchaînement dur entre morceaux.
- Faiblesse majeure 2 : la musique de bataille peut boucler un chant de 9 s pendant toute la bataille ; les listes `fallback` sont du poids mort (83 Mo sur 182) tant qu'une `primary` existe.
- Faiblesse majeure 3 : le casting vocal ne couvre que 8 factions sur 177 ; les autres parlent français quelle que soit leur culture.
- Mixage de base sain, quelques crêtes à 0 dB sur des échantillons de bataille et dispersion de niveau entre variantes (jusqu'à 13 dB).

## 2. Constats

### [majeur] [équilibrage] Pas de normalisation de sonie des morceaux : écarts jusqu'à 24 dB
**Constat** : le joueur entend des morceaux nettement plus forts ou plus faibles à chaque rotation ; `volume_db = 0` est appliqué à tous les morceaux (`_on_music_finished`, `play_music`).
**Preuve** : `volumedetect` sur 72 morceaux : moyenne de -35,6 (`suonatore_di_liuto`) à -11,7 dBFS (`village_consort`). Dans les listes `primary` seules : de -21,3 (`bach_bwv996_sarabande`) à -16,7 (`kapsberger_capona`), soit ~5 dB, déjà audible ; les `fallback` Kevin MacLeod (`minstrel_guild` -12,5, `the_britons` -14,8, `crusade` -15,0) sont 6 à 9 dB au-dessus des luths. Aucun gain par piste dans `audio_director.gd`.
**Correction proposée** : ajouter une clé optionnelle `gain_db` par piste dans `music.json` (ou un dictionnaire `track_gain_db`) calculée par un script d'outil (`tools/`, cible -19 dBFS moyen ou -18 LUFS), appliquée à `player.volume_db` à la place du `0.0` codé en dur (audio_director.gd:334 et le fondu cible ligne 374). Alternative : renormaliser les fichiers à l'import.
**Coût** : S

### [majeur] [bug] Musique de bataille : un chant de 9 s peut boucler toute la bataille
**Constat** : `BattleMusic._pick_base_track` tire au hasard un morceau parmi `playlist("battle")` (primary + fallback, 8 pistes) et le boucle (`_load(path, true)`). `agincourt_carol_deo_gracias.mp3` fait 8,9 s : 1 chance sur 8 (et c'est une des deux pistes « primary » voulues par l'ADR 0166) de réentendre ce chant en boucle des dizaines de minutes ; mp3 : trou de remplissage de l'encodeur à chaque boucle.
**Preuve** : `battle_music.gd:242-251` (`pick_random`, `loop=true`) ; durée 8,9 s mesurée ; `music.json` contexte `battle`. Le même chant est en fallback de `war`.
**Correction proposée** : exclure les pistes < 60 s du choix de la base de bataille (ou les déclarer avec `loop_min_s`), ou enchaîner les pistes comme la campagne (`finished` -> piste suivante) au lieu de boucler. Retirer `agincourt_carol` des listes de fond, le garder pour un stinger de victoire/ouverture.
**Coût** : S

### [majeur] [conception] Les listes `fallback` ne servent jamais : 83 Mo de poids mort
**Constat** : `next_track` ne passe au tier `fallback` que si aucun fichier `primary` ne charge. Les 12 à 15 pistes `fallback` par contexte (MacLeod, ars nova) ne sont donc jouées nulle part, hors `battle` qui concatène. Ce n'est pas ce que suggère le nom, et la variété annoncée en campagne n'existe pas (campagne France : 8 pistes en rotation, ~17 min, soit un cycle de ~17 min sans nouveauté).
**Preuve** : `audio_director.gd` `next_track` (boucle sur `TIERS`, retour au premier tier non vide) ; calcul : 34 fichiers atteignables seulement en fallback = 82,8 Mo sur 182 Mo de `third_party/music` (58 Mo MacLeod, 98 Mo Wikimedia, 17 Mo ars nova).
**Correction proposée** : (a) assumer le fallback comme « secours d'absence de fichier » et sortir les 34 pistes du paquet de jeu (téléchargement à la demande ou dépôt séparé), ou (b) mélanger le fallback à faible poids (ex. 1 piste sur 4) pour rallonger la rotation. Décision à écrire en note dans l'ADR 0166.
**Coût** : S (a) / M (b)

### [majeur] [conception] Voix de bataille : 169 factions sur 177 parlent français
**Constat** : `barks.json` ne mappe que 9 factions ; `fallback_language` est le français. Castille, Italie, Empire, mondes islamique et orthodoxe (déjà couverts par des playlists régionales) crient en français. Les répliques sont en 7 « langues » dont seules fr/en/an/oc/nl/cy/sco existent. Le chœur de cri accepte `fac_castile: "es"` mais `barks` n'a pas de langue es.
**Preuve** : `data/voice/barks.json` : `faction_language` 9 entrées, `languages` = 7, 177 factions dans `data/factions/` ; casting `speech_voices.json` : 3 factions + `default`. Voix : 294 fichiers, 120 barks.
**Correction proposée** : ajouter par culture (`culture` de la faction, comme `culture_regions` pour la musique) une table `culture_language` dans `barks.json` ; produire d'abord es, it (toscan), de, puis grec/arabe en réutilisant `voice_tts.py --shouts` (budget < 5 $, voir `docs/budget.md`). En attendant, ne pas faire crier en français des armées arabes : préférer des cris sans paroles (chœur « Ah ! » ElevenLabs) pour les cultures sans langue.
**Coût** : M

### [majeur] [finition] Enchaînement dur entre morceaux et nouvelle piste à plein volume
**Constat** : en fin de morceau, `_on_music_finished` remplace le flux sur le même lecteur et relance à `volume_db = 0` : coupure sèche, sans respiration, y compris pendant une pause de silence qui n'existe pas. Les fondus (1,5 s) n'existent que pour les changements de contexte. Les morceaux mp3 se terminent parfois sur une queue de réverb coupée.
**Preuve** : `audio_director.gd` `_on_music_finished` (~l. 325-336) ; `FADE_SECONDS = 1.5`.
**Correction proposée** : avant la fin (durée - 2 s, via un `Timer` armé au `play`), lancer le crossfade déjà codé dans `play_music` vers `next_track`, ou insérer 3 à 6 s de silence aléatoire entre morceaux (musique de fond de strategy game : respiration attendue).
**Coût** : S

### [majeur] [bug] Une même pièce répétée dans menu, guerre et bataille
**Constat** : `estampie_retrove_robertsbridge` est primary du menu, fallback de `war`, primary de `battle` ; `chominciamento_di_gioia` ouvre le menu et reste dans les fallbacks de campagne ; avec 2 pistes primary en bataille (dont une de 9 s), toute bataille entend l'estampie de 2:29 ou le chant. Le tirage `pick_random` n'a pas de mémoire entre batailles (contrairement à la rotation sauvegardée de la campagne).
**Preuve** : `music.json` (`battle`, `menu`, `war`) ; `_pick_base_track` sans état.
**Correction proposée** : alimenter `battle` avec 4 à 6 pistes martiales ≥ 90 s déjà présentes (`crusade`, `celtic_impulse`, `angevin_b`, `procession_of_the_king` de MacLeod, aujourd'hui condamnées au rôle de fallback) et utiliser `AudioDirector.next_track("battle")` pour bénéficier du sac mélangé et sauvegardé.
**Coût** : S

### [mineur] [bug] Crêtes à 0 dBFS sur des échantillons de bataille et d'interface
**Constat** : 16 fichiers touchent 0 dBFS (écrêtage possible avant le limiteur maître et les compresseurs) : `sword_clash_1/5/10`, `armor_hit_5/6`, `crossbow_release_1`, `bombard_1`, `fire_bed`, `cavalry_bed`, `ui/order_refused.wav`, etc.
**Preuve** : `volumedetect` max_volume = 0,0 ou -0,0 dB ; liste complète via le script de mesure de ce rapport.
**Correction proposée** : renormaliser à -1 dBFS de crête (et -20 dBFS moyen) dans le script de génération, sans changer les `volume_db` du JSON.
**Coût** : S

### [mineur] [équilibrage] Grande dispersion de niveau entre variantes d'un même événement
**Constat** : les variantes d'un événement diffèrent de 10 à 14 dB, donc un « choc d'épée » ou « cri d'effort » tiré au hasard est tantôt inaudible tantôt dominant.
**Preuve** : étendue des niveaux moyens : `arrow_impact` 13,7 dB (-29,0 à -15,3), `effort_cry` 12,4, `body_fall` 12,6, `wall_collapse` 12,1, `sword_clash` 11,8, `death_groan` 11,7, `horse_neigh` 10,6, `bow_release` 10,4.
**Correction proposée** : normaliser les variantes d'un même groupe à ±3 dB (script d'outil, une passe `ffmpeg loudnorm` par groupe) ; garder l'écart de portée dans `volume_db`/`pitch` du JSON, pas dans les fichiers.
**Coût** : S

### [mineur] [finition] Raccords de boucle d'ambiance : sauts de niveau et de signal
**Constat** : quelques bruits de fond bouclent avec un pas de niveau (countryside 0,068 -> 0,042 RMS soit ~4 dB entre la 1re et la dernière seconde ; forest -5 dB) et un saut d'échantillon important (wind_strong 0,16, clamor_bed 0,18, melee_bed_3 0,09). Au bout de 20 à 45 s, un « tic » ou un souffle audible est possible.
**Preuve** : mesure RMS de la première/dernière seconde et saut sample[0] vs sample[-1] sur `ambience/*.ogg` et `battle/*_bed*.ogg` (8 kHz mono) ; le bouclage est un `loop` Ogg sans fondu enchaîné (`audio_director._load_stream`).
**Correction proposée** : régénérer ces 6 fichiers avec un crossfade interne de 1 à 2 s (script d'outil) ; à l'oreille d'abord (ces mesures sont indicatives, pas un jugement d'écoute).
**Coût** : S

### [mineur] [finition] Sons d'interface très courts, doublon de ressources
**Constat** : `ui/click.wav` (70 ms) et `sfx/ui_click.ogg` (61 ms) coexistent (deux chemins : bouton automatique via `play_sfx("ui_click")` et événement `ui_click` de la banque). Risque de clic doublé selon le chemin ; `ui/card.wav` 140 ms. Aucun bruitage d'interface propre aux infobulles, onglets, refus, fin de tour hors cloche.
**Preuve** : `audio_director.gd:_on_button_pressed` -> `play_sfx("ui_click")` ; `sound_bank.json` `ui_click` ; durées mesurées.
**Correction proposée** : un seul chemin (la banque) ; vérifier à l'oreille l'absence de doublon.
**Coût** : S

### [mineur] [conception] Aucune option « musique de bataille/ambiance » séparée dans les réglages
**Constat** : `PLAYER_BUSES` expose Général, Musique, Ambiance, Bataille, Interface, Voix (bon) mais `Settings` règle Musique à 0,6 par défaut alors que les voix sont à -13 LUFS à 0,9 : la musique (-19) reste ~8 dB sous les cris, ce qui convient en bataille mais efface la musique de campagne sous l'ambiance (0,8) si l'ambiance est forte (-8 dB sur countryside).
**Preuve** : `settings.gd:44-49` (défauts) ; niveaux ambiance -20,3 dBFS vs musique -19 dBFS moyen.
**Correction proposée** : à valider à l'oreille : ambiance de campagne -3 dB, ou défaut Musique 0,7. Ne rien changer sans écoute.
**Coût** : S

### [mineur] [finition] Poids : 98 Mo de mp3 Wikimedia + 58 Mo mp3 MacLeod
**Constat** : formats mixtes (ogg pour le contenu maison, mp3 pour les pièces tierces) ; mp3 = trou de remplissage à la boucle, plus lourd à qualité égale. 182 Mo de musique + 25 Mo d'audio interne dans un dépôt public.
**Preuve** : `du -sh game/assets/third_party/music/*` ; 72 morceaux = 180 Mo, 185 min.
**Correction proposée** : après le tri des fallbacks (constat 3), convertir les morceaux restants en Ogg Vorbis q4 (~ -50 % de poids), ce qui règle aussi la boucle sans trou.
**Coût** : M

### [mineur] [finition] Qualité TTS de l'accent et de la voix du conseiller
**Constat** : les 21 répliques du conseiller et 143 répliques parlées sont en gpt-audio-mini (voix « cedar » seule), durées de 6 à 12 s ; l'ADR 0145 reconnaît des répliques (gascon, gallois, flamand, « À eux ! ») gardées en VO1 plus molles. Jugement de goût, non mesurable ici.
**Preuve** : manifeste `voice/manifest.json` (215 gpt-audio-mini, 79 ElevenLabs).
**Correction proposée** : aucune avant retour d'oreille du joueur ; si repris, ne refaire que les cris courts (ADR 0145 règle déjà la méthode).
**Coût** : M

## 3. À ne surtout pas changer
- Le chaînage des bus (limiteur maître, compresseurs Ambiance/Bataille/Voix, sidechain Voix -> Musique, `BatailleLointain`) et les réglages `AudioBuses`.
- Le traitement des voix criées et le chœur de cri de guerre (ADR 0145) : niveaux homogènes (-19 à -13 dBFS en moyenne, crête max -0,4 dB), 0 fichier manquant.
- La rotation en sac mélangé sauvegardée de la campagne (`next_track`, `rotation_path`) et la playlist régionale par culture (ADR 0166).
- Les couches de bataille d'`battle_layers.json` (états approach/engagement/critical/victory avec filtre passe-bas et fondu 2 s).
- Les écarts de volume par événement dans `sound_bank.json` (priorités, `max_instances`, cooldown) : à garder, on normalise les fichiers, pas les paramètres.
- Absence de fichier = silence sans erreur (`SoundBank`) : conserver ce comportement.
