# Critique audio (sound design / mixage / intégration) — nuit du 2026-10-22

Méthode : lecture de `client/src/audio/*`, `client/src/settings/*`, `data/audio/music.json`, `ls game/assets/audio` et `third_party/music`. Aucun son joué, aucun client lancé. Constats de lecture, non écoutés : la qualité sonore elle-même n'est pas jugée.

Base solide : bus Général/Musique/Musique de bataille/Ambiance/Interface/Effets de bataille/Voix, ducking, compresseur et limiteur (`dynamics.rs`), spatialisation Bevy en bataille avec bus lointain (passe-bas + réverbération, `farbus.rs`), ambiances saisonnières, barks multilingues (fr, en, an, oc, sco, nl, cy), 105 sons de bataille CC0 documentés (`game/assets/audio/SOURCE.md`).

## Constats

### 1. Attribution CC BY (Kevin MacLeod) absente du jeu — majeur (légal)
- Problème : 17 pistes MacLeod (57 Mo) sont utilisées (`data/audio/music.json`, 46 références). Sa licence CC BY 4.0 impose l'attribution (`game/assets/third_party/music/kevin_macleod/SOURCE.md` l.9 : « attribution obligatoire »). Aucune mention « macleod / incompetech » dans `client/src/menu/`. Le jeu est public.
- Correctif : écran Crédits avec titre/auteur/licence pour chaque piste CC BY, généré depuis les `SOURCE.md`. Même traitement pour les pistes Wikimedia CC BY / BY-SA (obligation de partage à l'identique à vérifier).
- Effort : S

### 2. Anachronismes dans les playlists — majeur (cohérence d'époque)
- Problème : `data/audio/music.json` référence du Bach (`bach_bwv996…998`, XVIIIe), Hume (`hume_pavan`, `hume_loves_farewell`, début XVIIe), Le Roy (`le_roy_bransles`, XVIe), Mudarra (`mudarra_*`, 1546), des maqams enregistrés en 1931 et du chant znamenny russe (`znamenny_*`). Le jeu se passe en 1337-1453. Les fichiers eux-mêmes sont corrects pour un public non spécialiste, mais cela contredit l'objectif de réalisme du joueur.
- Correctif : retirer Bach/Hume/Le Roy/Mudarra des listes campagne et cour. Garder Machaut, Landini, Binchois, Dufay, Agincourt Carol, Sumer is icumen in. Pour compléter : Cantigas de Santa Maria (Wikimedia, XIIIe), Llibre Vermell de Montserrat (XIVe), Robertsbridge Codex, Estampies du manuscrit du Roi, Codex Squarcialupi, enregistrements Musopen/IMSLP sous domaine public. Ces maqams ne sont pas rattachables à la France, l'Angleterre ou la Bourgogne : à enlever sauf campagne orientale.
- Effort : S

### 3. Musique : MP3 décodés par Bevy, 172 Mo de pistes — majeur (mémoire / chargement)
- Problème : `audio/mod.rs` l.725-745 charge chaque piste via `asset_server.load` + `AudioPlayer`. Bevy décode un `AudioSource` entier en mémoire. Une piste de 4-6 min en MP3 (`third_party/music/wikimedia` : 98 Mo, MacLeod 57 Mo, ars_nova 17 Mo) représente des dizaines de Mo de PCM par piste, plus un second exemplaire pendant le fondu enchaîné (`crossfade`). `clip.rs` sonde la durée à la main parce que Bevy ne l'expose pas. Les MP3 non bouclés avec gaps peuvent aussi présenter des silences d'encodeur.
- Correctif : transcoder en Ogg Vorbis ~96 kbit/s (comme le reste de `audio/`) et mesurer le pic mémoire en campagne + bataille. Si besoin, source streamée (`symphonia` en lecture incrémentale) pour les pistes longues. Décharger les handles des pistes écartées par la rotation.
- Effort : M

### 4. Aucune réverbération/acoustique ni ambiance intérieure — mineur
- Problème : le bus lointain (`farbus.rs`) ne sert qu'à la bataille. Les vues intérieures (conseil/cour, scènes de ville, siège) n'ont ni salle ni changement d'ambiance (`ambience.rs` ne traite que saison / météo / zoom, pas de lieu). Le conseil joue la même ambiance qu'en campagne.
- Correctif : contexte d'ambiance « salle » (réverbération courte, tapisserie, feu de cheminée, pas lointains), réutiliser le Schroeder de `farbus.rs`. Sources libres : Freesound CC0 « fireplace », « castle interior room tone ».
- Effort : M

### 5. Voix des conseillers et barks : synthèse cloud, pas de voix par personnage ni de langue d'époque — mineur/majeur
- Problème : `voice/speech/{ash,ballad,echo,onyx,verse}` (125 fichiers au hash, voix nommées d'un fournisseur TTS) + `voice/barks` (120 : 35 fr, 35 en, 12 an, 10 nl, 10 oc, 10 sco, 8 cy). Pour une bataille de dizaines de régiments, 5-6 variantes par ordre et par camp risquent de se répéter vite (`fr_select` : 6, `an_victory` : 1, `cy_attack` : 1). Les factions mineures ont 1 à 2 clips. Pas d'« effort » vocal (cri, souffle) mêlé aux réponses.
- Correctif : multiplier les variantes par transposition légère (±1,5 demi-ton) et filtres (déjà un `pitch_span` dans le bank, l'appliquer aussi aux barks), limiter la fréquence des barks (cooldown par faction), compléter les factions à 1-2 clips en réutilisant la banque `fr`/`en` avec voix décalée. Garder la synthèse locale libre en priorité (voir contrainte budget).
- Effort : M

### 6. Pas de mise en sourdine à la perte de focus, pas de choix du périphérique — mineur
- Problème : aucun `WindowFocused` / `Focused` dans `client/src/audio/mod.rs`. Le jeu continue de jouer en arrière-plan (musique à fort volume pendant qu'on travaille sur autre chose). Réglages (`settings/rows.rs`) : 7 curseurs et 2 cases (conseiller, barks), mais ni « couper au changement de fenêtre », ni mono/stéréo, ni plage dynamique (nuit), ni sous-titres ou texte des barks (accessibilité).
- Correctif : option « Couper le son en arrière-plan » (défaut oui), option « Mode nuit » (compresseur plus fort, déjà dans `dynamics.rs`), indicateur visuel des barks pour malentendants.
- Effort : S

### 7. Sons d'interface limités et sans retour par catégorie — mineur
- Problème : `audio/ui` ne compte que 10 WAV (alert, army_select, build, card, click, letter, open, order_refused, order, recruit). Pas de son pour : fin de tour distincte de la cloche, revenu / pénurie d'or, traité signé / guerre déclarée, survol de bouton, onglets, curseurs de réglages, mort d'un chef, victoire/défaite d'un siège. `sfx/` contient en doublon `ui_click.ogg`. Les `uisound_script*.txt` indiquent un script de génération procédurale.
- Correctif : étendre avec packs CC0 (Kenney « Interface Sounds » / « RPG Audio », OpenGameArt « UI Sounds », tous CC0), filtrés pour éviter l'aspect arcade ; sceau et parchemin via Freesound CC0.
- Effort : S/M

### 8. Musique : seulement 3 fichiers de repli et 4 couches de bataille — mineur
- Problème : `game/assets/audio/music/` = `campaign.ogg`, `court.ogg`, `war.ogg` (procéduraux, 2,6 Mo) + `battle_layers` (4). La musique de bataille adaptative (`battle_music.rs`, 520 lignes) repose sur 4 couches seulement : risque de monotonie sur une bataille longue ou après 10 batailles. Pas de thème par faction ni par année.
- Correctif : ajouter des variantes d'intensité (2-3 boucles par couche, rotation aléatoire), thèmes percussifs/cornes d'époque (Kevin MacLeod « Crusade » est déjà utilisé ; compléter avec OpenGameArt CC0 « medieval battle drums », Freesound CC0 tambours et cornes). Menu principal : un thème dédié.
- Effort : M

### 9. Variété et plage dynamique des sons de bataille — mineur
- Problème : certains sons n'ont qu'un exemplaire (`crossbow_release_1`, `cavalry_charge_impact`, `cavalry_bed`, `clamor_bed`) pendant que d'autres en ont 6-10 (`armor_hit`, `death_groan`). `SOURCE.md` signale que plusieurs sont des extraits de MP3 128 kbit/s, ré-encodés (perte cumulée) et mono. Sources dupliquées : `shield_bash_2` et `shield_bash_3` viennent du même fichier 182112. Crossbow : un seul son pour arbalète + cris variés dans les rangs.
- Correctif : 3-4 variantes pour arbalète, impact de cavalerie, ordres de cor ; télécharger les WAV d'origine (Freesound CC0 en qualité originale, pas l'aperçu MP3) quand la licence le permet. Ajouter pas de chevaux sur sol mou/boue, cottes de mailles, cordes d'arbalète, machines de siège (contrepoids, trébuchet, boulet sur pierre).
- Effort : M

### 10. Documentation / test du mixage par scène absente — mineur
- Problème : `audio/dynamics.rs` et `audio/farbus.rs` définissent compresseur, limiteur et DSP maison, avec tests unitaires de gains, mais pas de test de niveau de référence (LUFS cible musique/ambiance/voix, par scène campagne vs bataille). Les gains en dB par piste (`music.gain_db`) sont réglés à la main.
- Correctif : script `tools/` qui calcule le LUFS intégré de chaque fichier de `audio/` et `third_party/music` hors ligne (pyloudnorm, sans lecture) et échoue au-dessus de ±3 LU de l'objectif ; l'intégrer à `uv run --project tools pytest`.
- Effort : S

## Top 3

1. Crédits / attribution CC BY (constat 1) : risque légal, effort S.
2. Retrait des morceaux hors période (constat 2) : cohérence d'époque immédiate, effort S.
3. Transcodage des 172 Mo de MP3 en Ogg et mesure mémoire du crossfade (constat 3) : stabilité et temps de chargement, effort M.
