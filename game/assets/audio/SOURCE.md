# Banque sonore AU1 (`battle/`, `ambience/`)

Générée par `tools/cent_ans_tools/audio_bank.py` (reproductible :
`uv run --project tools --with soundfile python -m cent_ans_tools.audio_bank`).
Sources : Freesound, licence **CC0 1.0** (domaine public) vérifiée page par page au
téléchargement ; fichier téléchargé = aperçu haute qualité public (MP3 128 kbit/s) du son.
Traitements : découpe sur attaques ou fenêtre la plus dense, fondus, bouclage en fondu
enchaîné à puissance constante, filtres, transpositions, mélanges ; normalisation (crête
−1 dBFS pour les sons ponctuels, RMS −20 dBFS pour les boucles) ; encodage Ogg Vorbis.
Les effets `sfx/` et musiques `music/` d'origine restent de la synthèse procédurale
(`tools/cent_ans_tools/audio.py`).

| Fichier | Sources Freesound (auteur — titre) | Traitement |
|---|---|---|
| `battle/sword_clash_1.ogg` | [471095](https://freesound.org/people/spycrah/sounds/471095/) spycrah — Sword clash 1.wav | ponctuel, mono |
| `battle/sword_clash_2.ogg` | [364531](https://freesound.org/people/Christopherderp/sounds/364531/) Christopherderp — Swords Clash - High Quality #3 | ponctuel, mono |
| `battle/sword_clash_3.ogg` | [326868](https://freesound.org/people/JohnBuhr/sounds/326868/) JohnBuhr — Sword_Clash (7).wav | ponctuel, mono |
| `battle/sword_clash_4.ogg` | [440069](https://freesound.org/people/ethanchase7744/sounds/440069/) ethanchase7744 — Sword block combo.wav | ponctuel, mono |
| `battle/shield_bash_1.ogg` | [370203](https://freesound.org/people/nekoninja/sounds/370203/) nekoninja — shield guard | ponctuel, mono |
| `battle/shield_bash_2.ogg` | [182112](https://freesound.org/people/PixelsphereStudios/sounds/182112/) PixelsphereStudios — Shield / sword hits.wav | ponctuel, mono |
| `battle/shield_bash_3.ogg` | [182112](https://freesound.org/people/PixelsphereStudios/sounds/182112/) PixelsphereStudios — Shield / sword hits.wav | ponctuel, mono |
| `battle/arrow_impact_1.ogg` | [205938](https://freesound.org/people/Twisted_Euphoria/sounds/205938/) Twisted_Euphoria — Arrow Impact | ponctuel, mono |
| `battle/arrow_impact_2.ogg` | [521552](https://freesound.org/people/omerbhatti34/sounds/521552/) omerbhatti34 — Arrow Impact | ponctuel, mono |
| `battle/arrow_impact_3.ogg` | [534956](https://freesound.org/people/JoeDinesSound/sounds/534956/) JoeDinesSound — ARROW_WOOD_IMPACT_SINGLE_ARCHERY_01.wav | ponctuel, mono |
| `battle/arrow_impact_4.ogg` | [708223](https://freesound.org/people/Mythmazter/sounds/708223/) Mythmazter — Arrow_Hit_1 | ponctuel, mono |
| `battle/arrow_whistle_1.ogg` | [394004](https://freesound.org/people/DigPro120/sounds/394004/) DigPro120 — Arrows Fly By.mp3 | ponctuel, mono |
| `battle/arrow_whistle_2.ogg` | [675821](https://freesound.org/people/craigsmith/sounds/675821/) craigsmith — S27-05 Five arrows whoosh by.wav | ponctuel, mono |
| `battle/arrow_whistle_3.ogg` | [789389](https://freesound.org/people/modusmogulus/sounds/789389/) modusmogulus — Arrow Flyby<br>[384910](https://freesound.org/people/Ali_6868/sounds/384910/) Ali_6868 — Arrow Flying 2 | ponctuel, mono ; mix of 14 detuned fly-bys |
| `battle/bow_release_1.ogg` | [263675](https://freesound.org/people/PorkMuncher/sounds/263675/) PorkMuncher — Bow_release.wav | ponctuel, mono |
| `battle/bow_release_2.ogg` | [394179](https://freesound.org/people/saturdaysoundguy/sounds/394179/) saturdaysoundguy — Longbow Release 2.wav | ponctuel, mono |
| `battle/bow_release_3.ogg` | [384918](https://freesound.org/people/Ali_6868/sounds/384918/) Ali_6868 — Bow Release (Bow and Arrow) 3 | ponctuel, mono |
| `battle/crossbow_release_1.ogg` | [384919](https://freesound.org/people/Ali_6868/sounds/384919/) Ali_6868 — Crossbow Firing and Hitting Target | ponctuel, mono |
| `battle/charge_cry_1.ogg` | [621352](https://freesound.org/people/WelvynZPorterSamples/sounds/621352/) WelvynZPorterSamples — Male Yelling out a War Cry 3 - WITH reverb.wav | ponctuel, mono |
| `battle/charge_cry_2.ogg` | [866009](https://freesound.org/people/Simonus18/sounds/866009/) Simonus18 — Battle cry scream 04 - male screaming - before the battle, warcry | ponctuel, mono |
| `battle/charge_cry_3.ogg` | [563011](https://freesound.org/people/florianreichelt/sounds/563011/) florianreichelt — People screaming in agony when charging into battle | ponctuel, mono |
| `battle/war_cry_1.ogg` | [563011](https://freesound.org/people/florianreichelt/sounds/563011/) florianreichelt — People screaming in agony when charging into battle<br>[621352](https://freesound.org/people/WelvynZPorterSamples/sounds/621352/) WelvynZPorterSamples — Male Yelling out a War Cry 3 - WITH reverb.wav<br>[866009](https://freesound.org/people/Simonus18/sounds/866009/) Simonus18 — Battle cry scream 04 - male screaming - before the battle, warcry<br>[325548](https://freesound.org/people/Archeos/sounds/325548/) Archeos — Man screaming.wav | ponctuel, mono ; crowd shout mixed from 10 cries |
| `battle/war_cry_2.ogg` | [563011](https://freesound.org/people/florianreichelt/sounds/563011/) florianreichelt — People screaming in agony when charging into battle<br>[621352](https://freesound.org/people/WelvynZPorterSamples/sounds/621352/) WelvynZPorterSamples — Male Yelling out a War Cry 3 - WITH reverb.wav<br>[866009](https://freesound.org/people/Simonus18/sounds/866009/) Simonus18 — Battle cry scream 04 - male screaming - before the battle, warcry<br>[325548](https://freesound.org/people/Archeos/sounds/325548/) Archeos — Man screaming.wav | ponctuel, mono ; crowd shout mixed from 10 cries |
| `battle/death_groan_1.ogg` | [577032](https://freesound.org/people/Blankened/sounds/577032/) Blankened — MaleDeathSound13.wav | ponctuel, mono |
| `battle/death_groan_2.ogg` | [221544](https://freesound.org/people/joseppujol/sounds/221544/) joseppujol — Wounded man scream | ponctuel, mono |
| `battle/death_groan_3.ogg` | [610998](https://freesound.org/people/unfa/sounds/610998/) unfa — Medium Male Pain Grunts | ponctuel, mono |
| `battle/death_groan_4.ogg` | [610998](https://freesound.org/people/unfa/sounds/610998/) unfa — Medium Male Pain Grunts | ponctuel, mono |
| `battle/death_groan_5.ogg` | [610998](https://freesound.org/people/unfa/sounds/610998/) unfa — Medium Male Pain Grunts | ponctuel, mono |
| `battle/death_groan_6.ogg` | [255322](https://freesound.org/people/waxsocks/sounds/255322/) waxsocks — Groans and Screams | ponctuel, mono |
| `battle/death_groan_7.ogg` | [255322](https://freesound.org/people/waxsocks/sounds/255322/) waxsocks — Groans and Screams | ponctuel, mono |
| `battle/rout_cry_1.ogg` | [384401](https://freesound.org/people/FillMat/sounds/384401/) FillMat — Crowd/Mob/Riot Noise (Voices Only) - 14 people, 2 minutes HENRY VI | ponctuel, mono |
| `battle/rout_cry_2.ogg` | [384401](https://freesound.org/people/FillMat/sounds/384401/) FillMat — Crowd/Mob/Riot Noise (Voices Only) - 14 people, 2 minutes HENRY VI<br>[325548](https://freesound.org/people/Archeos/sounds/325548/) Archeos — Man screaming.wav | ponctuel, mono ; mob noise + 4 screams |
| `battle/horse_neigh_1.ogg` | [149024](https://freesound.org/people/foxen10/sounds/149024/) foxen10 — Horse_Whinny.wav | ponctuel, mono |
| `battle/horse_neigh_2.ogg` | [347036](https://freesound.org/people/Kubuzz/sounds/347036/) Kubuzz — horse's whinny | ponctuel, mono |
| `battle/horse_neigh_3.ogg` | [269571](https://freesound.org/people/shadoWisp/sounds/269571/) shadoWisp — horse neigh shortened.wav | ponctuel, mono |
| `battle/horse_neigh_4.ogg` | [437110](https://freesound.org/people/craigsmith/sounds/437110/) craigsmith — G38-15-Perfect Horse Whinny.wav | ponctuel, mono |
| `battle/horn_1.ogg` | [539956](https://freesound.org/people/adharca/sounds/539956/) adharca — war horn.wav | ponctuel, mono |
| `battle/horn_2.ogg` | [175946](https://freesound.org/people/freefire66/sounds/175946/) freefire66 — Horn002.wav | ponctuel, mono |
| `battle/horn_3.ogg` | [512490](https://freesound.org/people/DeVern/sounds/512490/) DeVern — Distant War Horn.wav | ponctuel, mono |
| `battle/drum_1.ogg` | [459876](https://freesound.org/people/Quickmusik/sounds/459876/) Quickmusik — Warrior Tom.wav<br>[459875](https://freesound.org/people/Quickmusik/sounds/459875/) Quickmusik — Warrior bass T.wav | ponctuel, mono ; march pattern of 11 hits |
| `battle/bell_toll_1.ogg` | [454855](https://freesound.org/people/SamuelGremaud/sounds/454855/) SamuelGremaud — TOLLING BELL | ponctuel, mono |
| `battle/bell_toll_2.ogg` | [383192](https://freesound.org/people/Ittaisha/sounds/383192/) Ittaisha — Church Bell | ponctuel, mono |
| `battle/ram_hit_1.ogg` | [675970](https://freesound.org/people/craigsmith/sounds/675970/) craigsmith — S10-24 Battering ram hits castle door; big wooden hit.wav | ponctuel, mono |
| `battle/ram_hit_2.ogg` | [675970](https://freesound.org/people/craigsmith/sounds/675970/) craigsmith — S10-24 Battering ram hits castle door; big wooden hit.wav | ponctuel, mono |
| `battle/ram_hit_3.ogg` | [675970](https://freesound.org/people/craigsmith/sounds/675970/) craigsmith — S10-24 Battering ram hits castle door; big wooden hit.wav | ponctuel, mono |
| `battle/trebuchet_release_1.ogg` | [231438](https://freesound.org/people/6polnic/sounds/231438/) 6polnic — hamp rope creaks<br>[479922](https://freesound.org/people/craigsmith/sounds/479922/) craigsmith — R01-04-Catapult Launch.wav<br>[789389](https://freesound.org/people/modusmogulus/sounds/789389/) modusmogulus — Arrow Flyby | ponctuel, mono ; rope creak + launch + low whoosh |
| `battle/stone_impact_1.ogg` | [513694](https://freesound.org/people/kasparsj/sounds/513694/) kasparsj — impact-stone-heavy.wav | ponctuel, mono |
| `battle/stone_impact_2.ogg` | [703247](https://freesound.org/people/xkeril/sounds/703247/) xkeril — Big falling debris (crash) | ponctuel, mono |
| `battle/stone_impact_3.ogg` | [567249](https://freesound.org/people/iwanPlays/sounds/567249/) iwanPlays — Bricks/Stones/Rocks/Gravel Falling | ponctuel, mono |
| `battle/bombard_1.ogg` | [187767](https://freesound.org/people/qubodup/sounds/187767/) qubodup — Cannon Shot | ponctuel, mono |
| `battle/bombard_2.ogg` | [404166](https://freesound.org/people/DRFX/sounds/404166/) DRFX — Background Cannon Shot | ponctuel, mono |
| `battle/wall_collapse_1.ogg` | [712918](https://freesound.org/people/greyfeather/sounds/712918/) greyfeather — building collapse / demolition | ponctuel, mono |
| `battle/wall_collapse_2.ogg` | [389303](https://freesound.org/people/AlanCat/sounds/389303/) AlanCat — rockfall2a.wav<br>[567249](https://freesound.org/people/iwanPlays/sounds/567249/) iwanPlays — Bricks/Stones/Rocks/Gravel Falling<br>[703248](https://freesound.org/people/xkeril/sounds/703248/) xkeril — Fall debris (crash) | ponctuel, mono ; rockfall + bricks + debris |
| `battle/thunder_1.ogg` | [399656](https://freesound.org/people/bajko/sounds/399656/) bajko — sfx_thunder blast.wav | ponctuel, mono |
| `battle/thunder_2.ogg` | [652690](https://freesound.org/people/AyaDrevis/sounds/652690/) AyaDrevis — Thunder strike | ponctuel, mono |
| `battle/sword_clash_5.ogg` | [442769](https://freesound.org/people/qubodup/sounds/442769/) qubodup — Sword Hit | ponctuel, mono |
| `battle/sword_clash_6.ogg` | [547041](https://freesound.org/people/CogFireStudios/sounds/547041/) CogFireStudios — Hit Swing Sword Small 3 | ponctuel, mono |
| `battle/sword_clash_7.ogg` | [706204](https://freesound.org/people/xkeril/sounds/706204/) xkeril — Nice anime sword hit | ponctuel, mono |
| `battle/sword_clash_8.ogg` | [426322](https://freesound.org/people/MTJohnson/sounds/426322/) MTJohnson — Single Sword Hit.wav | ponctuel, mono |
| `battle/sword_clash_9.ogg` | [518992](https://freesound.org/people/Amaiguri/sounds/518992/) Amaiguri — SwordClash.wav | ponctuel, mono |
| `battle/sword_clash_10.ogg` | [275159](https://freesound.org/people/Bird_man/sounds/275159/) Bird_man — Sword Clash.wav | ponctuel, mono |
| `battle/sword_clash_11.ogg` | [334169](https://freesound.org/people/loudernoises/sounds/334169/) loudernoises — Sword Clash | ponctuel, mono |
| `battle/sword_clash_12.ogg` | [844258](https://freesound.org/people/Foxfire-/sounds/844258/) Foxfire- — Sword Tap | ponctuel, mono |
| `battle/shield_bash_4.ogg` | [114683](https://freesound.org/people/Qat/sounds/114683/) Qat — whack02.mp3 | ponctuel, mono |
| `battle/shield_bash_5.ogg` | [114684](https://freesound.org/people/Qat/sounds/114684/) Qat — whack03.mp3 | ponctuel, mono |
| `battle/shield_bash_6.ogg` | [114685](https://freesound.org/people/Qat/sounds/114685/) Qat — whack04.mp3 | ponctuel, mono |
| `battle/shield_bash_7.ogg` | [330997](https://freesound.org/people/Rudmer_Rotteveel/sounds/330997/) Rudmer_Rotteveel — Stick Hitting a Dreadlock (Small thud) | ponctuel, mono |
| `battle/shield_bash_8.ogg` | [319217](https://freesound.org/people/worthahep88/sounds/319217/) worthahep88 — Sticks Hitting sticks.wav | ponctuel, mono |
| `battle/armor_hit_1.ogg` | [638613](https://freesound.org/people/CaptainYulef/sounds/638613/) CaptainYulef — metal-thud.wav | ponctuel, mono |
| `battle/armor_hit_2.ogg` | [164220](https://freesound.org/people/deleted_user_2104797/sounds/164220/) deleted_user_2104797 — Thud_metallic_1.wav | ponctuel, mono |
| `battle/armor_hit_3.ogg` | [424424](https://freesound.org/people/Lavacoal123/sounds/424424/) Lavacoal123 — metal flick-hit | ponctuel, mono |
| `battle/armor_hit_4.ogg` | [812592](https://freesound.org/people/qubodup/sounds/812592/) qubodup — Clang | ponctuel, mono |
| `battle/armor_hit_5.ogg` | [733887](https://freesound.org/people/velcronator/sounds/733887/) velcronator — Sword Impact | ponctuel, mono |
| `battle/armor_hit_6.ogg` | [616492](https://freesound.org/people/Empiremonkey/sounds/616492/) Empiremonkey — clank.wav | ponctuel, mono |
| `battle/effort_cry_1.ogg` | [511023](https://freesound.org/people/ale-batec/sounds/511023/) ale-batec — Male Fight Grunts | ponctuel, mono |
| `battle/effort_cry_2.ogg` | [511023](https://freesound.org/people/ale-batec/sounds/511023/) ale-batec — Male Fight Grunts | ponctuel, mono |
| `battle/effort_cry_3.ogg` | [511023](https://freesound.org/people/ale-batec/sounds/511023/) ale-batec — Male Fight Grunts | ponctuel, mono |
| `battle/effort_cry_4.ogg` | [718967](https://freesound.org/people/BaggoNotes/sounds/718967/) BaggoNotes — ShoutingPunches_Male | ponctuel, mono |
| `battle/effort_cry_5.ogg` | [718967](https://freesound.org/people/BaggoNotes/sounds/718967/) BaggoNotes — ShoutingPunches_Male | ponctuel, mono |
| `battle/effort_cry_6.ogg` | [718967](https://freesound.org/people/BaggoNotes/sounds/718967/) BaggoNotes — ShoutingPunches_Male | ponctuel, mono |
| `battle/effort_cry_7.ogg` | [623449](https://freesound.org/people/WelvynZPorterSamples/sounds/623449/) WelvynZPorterSamples — Kingly Yell - NO reverb.wav | ponctuel, mono |
| `battle/effort_cry_8.ogg` | [670857](https://freesound.org/people/EvilOldScratch/sounds/670857/) EvilOldScratch — WarriorPain1.wav | ponctuel, mono |
| `battle/effort_cry_9.ogg` | [621370](https://freesound.org/people/WelvynZPorterSamples/sounds/621370/) WelvynZPorterSamples — Male Grunt 3 - WITH reverb.wav | ponctuel, mono |
| `battle/effort_cry_10.ogg` | [661242](https://freesound.org/people/DeqstersLab/sounds/661242/) DeqstersLab — Swing grunt.mp3 | ponctuel, mono |
| `battle/death_groan_8.ogg` | [131710](https://freesound.org/people/oldedgar/sounds/131710/) oldedgar — Male Being Impaled/Beaten AC.wav | ponctuel, mono |
| `battle/death_groan_9.ogg` | [567989](https://freesound.org/people/AncientWarrior/sounds/567989/) AncientWarrior — WoundedMaleShort.wav | ponctuel, mono |
| `battle/death_groan_10.ogg` | [104028](https://freesound.org/people/RutgerMuller/sounds/104028/) RutgerMuller — Scream1_Short.aif | ponctuel, mono |
| `battle/body_fall_1.ogg` | [504626](https://freesound.org/people/leonelmail/sounds/504626/) leonelmail — BODY FALL - V HVY - DIRT | ponctuel, mono |
| `battle/body_fall_2.ogg` | [346695](https://freesound.org/people/deleted_user_2104797/sounds/346695/) deleted_user_2104797 — Body fall_01.wav | ponctuel, mono |
| `battle/body_fall_3.ogg` | [266346](https://freesound.org/people/Ephisus/sounds/266346/) Ephisus — body falls.wav | ponctuel, mono |
| `battle/body_fall_4.ogg` | [266346](https://freesound.org/people/Ephisus/sounds/266346/) Ephisus — body falls.wav | ponctuel, mono |
| `battle/body_fall_5.ogg` | [395567](https://freesound.org/people/SoundsForHim/sounds/395567/) SoundsForHim — Collapsing in grass.MP3 | ponctuel, mono |
| `battle/body_fall_6.ogg` | [815415](https://freesound.org/people/AwenAudio/sounds/815415/) AwenAudio — AAD SWORD FALL ON DIRT | ponctuel, mono |
| `battle/horse_neigh_5.ogg` | [826753](https://freesound.org/people/Salsero_classic/sounds/826753/) Salsero_classic — Renill de cavall / Horse Neigh | ponctuel, mono |
| `battle/horse_neigh_6.ogg` | [636554](https://freesound.org/people/L.Finck/sounds/636554/) L.Finck — Whinnying horse-far away.WAV | ponctuel, mono |
| `battle/cavalry_charge_impact.ogg` | [527430](https://freesound.org/people/bruno.auzet/sounds/527430/) bruno.auzet — 6 horses gallop.wav<br>[182112](https://freesound.org/people/PixelsphereStudios/sounds/182112/) PixelsphereStudios — Shield / sword hits.wav<br>[471095](https://freesound.org/people/spycrah/sounds/471095/) spycrah — Sword clash 1.wav<br>[370203](https://freesound.org/people/nekoninja/sounds/370203/) nekoninja — shield guard | ponctuel, mono ; gallop swell (existing cavalry bed source) + shield/sword impact layer |
| `battle/melee_bed_3.ogg` | [675093](https://freesound.org/people/craigsmith/sounds/675093/) craigsmith — S30-20 Large crowd fighting indoors with swords in BG.wav | boucle, mono |
| `battle/melee_bed_1.ogg` | [376646](https://freesound.org/people/DeadVDI/sounds/376646/) DeadVDI — Vikings in battle (swords crossing, shields bashing, men yelling) | boucle, mono |
| `battle/melee_bed_2.ogg` | [175950](https://freesound.org/people/freefire66/sounds/175950/) freefire66 — SwordBattle1.wav | boucle, mono |
| `battle/clamor_bed.ogg` | [384401](https://freesound.org/people/FillMat/sounds/384401/) FillMat — Crowd/Mob/Riot Noise (Voices Only) - 14 people, 2 minutes HENRY VI | boucle, mono |
| `battle/march_bed.ogg` | [480675](https://freesound.org/people/craigsmith/sounds/480675/) craigsmith — R27-40-March on Gravel Road.wav | boucle, mono |
| `battle/cavalry_bed.ogg` | [527430](https://freesound.org/people/bruno.auzet/sounds/527430/) bruno.auzet — 6 horses gallop.wav | boucle, mono |
| `battle/fire_bed.ogg` | [636178](https://freesound.org/people/jamesdrake89/sounds/636178/) jamesdrake89 — Fire - Crackling, Spitting, Roaring | boucle, mono |
| `ambience/battle_distant.ogg` | [376646](https://freesound.org/people/DeadVDI/sounds/376646/) DeadVDI — Vikings in battle (swords crossing, shields bashing, men yelling)<br>[384401](https://freesound.org/people/FillMat/sounds/384401/) FillMat — Crowd/Mob/Riot Noise (Voices Only) - 14 people, 2 minutes HENRY VI | boucle, stéréo ; muffled melee + clamour |
| `ambience/wind.ogg` | [760241](https://freesound.org/people/jackstraton/sounds/760241/) jackstraton — Medium Wind, Constant 2 | boucle, stéréo |
| `ambience/wind_strong.ogg` | [185070](https://freesound.org/people/juryduty/sounds/185070/) juryduty — howling_wind.wav | boucle, stéréo |
| `ambience/rain.ogg` | [157487](https://freesound.org/people/loopbasedmusic/sounds/157487/) loopbasedmusic — rain_near_smooth.aif | boucle, stéréo |
| `ambience/sea.ogg` | [534910](https://freesound.org/people/Lucas_Schacht/sounds/534910/) Lucas_Schacht — Ocean Waves 03.wav | boucle, stéréo |
| `ambience/countryside.ogg` | [514550](https://freesound.org/people/Kinoton/sounds/514550/) Kinoton — Countryside Ambience Spring | boucle, stéréo |
| `ambience/crickets.ogg` | [522299](https://freesound.org/people/Defelozedd94/sounds/522299/) Defelozedd94 — Crickets At Night - Raw sound | boucle, stéréo |
| `ambience/forest.ogg` | [474342](https://freesound.org/people/pborel/sounds/474342/) pborel — forest-birds3a.WAV | boucle, stéréo |
| `ambience/town.ogg` | [424790](https://freesound.org/people/bolkmar/sounds/424790/) bolkmar — Crowded street at medieval market<br>[444900](https://freesound.org/people/DigestContent/sounds/444900/) DigestContent — Crowd Murmuring | boucle, stéréo ; market street layered over a crowd murmur |

## Sons de siège SG2 (synthèse procédurale)

Générés par `tools/cent_ans_tools/sg2_sounds.py` (numpy/scipy, graines fixes, aucun échantillon
externe : `uv run --project tools --with soundfile python -m cent_ans_tools.sg2_sounds`). Si
`audio_bank.py` réécrit ce fichier, recopier cette section.

| Fichier | Recette |
|---|---|
| `battle/boiling_oil_1.ogg` | huile bouillante : coulée (bruit passe-bas modulé), éclaboussure à 0,85 s, grésillement et crépitements, bulles ; mono |
| `battle/boiling_oil_2.ogg` | même recette, autre graine |
