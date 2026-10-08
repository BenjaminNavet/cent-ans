# Mocap gratuite : sources, licences et essai de reciblage (lot NT12, 30/09)

Question : de la vraie capture de mouvement gratuite, reciblée sur nos figurines fines, bat-elle
nettement nos clips keyframés (AN1b/NT7, ADR 0096 et 0129) ? Coût : 0 $, aucun compte créé.
Dossier des fichiers bruts, **hors dépôt** : `~/dev/cent-ans-mocap-src/` (le dépôt est public).

## Sources

| Source | Obtention | Licence (résumé) | Redistribution de dérivés cuits dans un dépôt public |
|---|---|---|---|
| CMU Graphics Lab Motion Capture Database (ajout NT12) | curl, sans compte : `mocap.cs.cmu.edu/subjects/NN/NN_TT.amc` + `NN.asf` | « free for all uses » ; inclusion dans un produit commercial permise ; interdit de revendre les données elles-mêmes, même converties | **Oui** : FAQ du site, « The motion capture data may be copied, modified, or redistributed without permission. » Remerciement demandé (NSF EIA-0196217) |
| MoCap Online, « FREE T.C. Sword » | **À la main** : formulaire e-mail (lien envoyé par courriel), https://mocaponline.com/products/tc-sword ; autres packs gratuits : https://mocaponline.com/collections/free-mocap-animations | Standard : libre de redevance, projets < 1 M$ de revenu et < 1 M d'utilisateurs, sans attribution | **Non** : distribution « binary-only » ; interdit dans un dépôt open source ou public, même modifié (https://mocaponline.com/pages/license). Clips cuits à garder hors dépôt (ou dans l'export seulement) |
| Rokoko, 13 combats gratuits | **À la main** : formulaire nom + e-mail, https://www.rokoko.com/resources/rokoko-mocap-13-free-fight-animations (FBX, squelette Mixamo, 30 i/s) | Usage commercial permis (« from passion project to commercial use ») | **Non** pour les fichiers bruts (EULA Rokoko : pas de reproduction ni distribution des assets) ; dérivés cuits : non précisé, donc à garder hors dépôt |
| Rokoko Motion Library, 10 armes | **À la main** : même formulaire, https://www.rokoko.com/resources/motion-library-10-free-fight-and-weapon-animations | Idem | Idem. Contenu : pistolets, fusil, couteau, grenade, arc (aucune épée) : peu utile |
| Quaternius Universal Animation Library 2 | **À la main** : https://quaternius.itch.io/universal-animation-library-2 (« Download » puis « No thanks, just take me to the downloads », sans compte) ; curl refusé (défi Cloudflare, 403) ; aussi https://quaternius.com/packs/universalanimationlibrary2.html | CC0 1.0 | **Oui** (domaine public). Animations faites main sur un rig humanoïde, pas de la mocap |

Exclus : Mixamo (licence Adobe), Bandai Namco Research (non commercial).

Note : le lien direct du zip Rokoko « 10 armes » figure dans la page, derrière le formulaire ;
il a été récupéré puis **supprimé** sans usage (la page exige un formulaire, et le contenu était
sans épée). Rien de Rokoko ni de MoCap Online n'a servi à l'essai.

## Essai (CMU, faute des autres sources sans formulaire)

Prises : sujet 2 « swordplay » (02_07, 02_08, 02_09), sujet 90 « RugPullFall » (90_18).
Pipeline : `tools/blender_scripts/mocap_asf.py` (lecteur ASF/AMC) et
`tools/blender_scripts/nt12_mocap_trial.py` (reciblage par rotations, cuisson `CAB1`, rendu
Blender de contrôle avec `-- render DIR`). Six clips substitués au rig fin `human` avec
`--mocap-trial` après `--` : `guard`, `slash`, `overhead`, `parry`, `hit`, `death`. Les clips
cuits (156 Ko) sont versionnés dans `game/assets/models/battle_fine/mocap_trial/` (licence CMU).

Défauts observés (mesures dans `mocap_trial/manifest.json`, planche Blender) :
- **Épée à deux mains** : le sujet 2 tient la poignée des deux mains ; sur une figurine épée et
  bouclier, le bouclier passait devant le visage. Correctif : le bras gauche garde la garde
  keyframée (`Idle_Sword`) posée sur le buste mocap (clips d'escrime). Le coup reste un geste
  d'épée longue, pas d'épée et bouclier.
- **Pieds qui glissent** : 2 à 18 cm par clip (pas d'IK de pied ; le sujet se déplace, la dérive
  de racine est ôtée des boucles).
- **Poignets** : le poignet CMU n'a qu'un ou deux degrés de liberté et la main est bruitée ;
  l'épée suit l'avant-bras de façon plausible (≤ 15° par image), sans torsion fine.
- **Impacts et morts** : la base CMU n'a ni coup reçu ni mort ; `hit` est un recul-esquive,
  `death` une chute sur le dos qui finit à demi assise.
- Gain réel : poids du corps, pas d'appui, amplitude des coups (diagonale en `slash`).

Verdict provisoire (à confirmer par la session principale sur `docs/audit/captures/nt/nt12_*.png`) :
la mocap CMU ne bat **pas nettement** nos clips — plus vivante en mouvement de jambes et de
buste, mais geste inadapté (deux mains), sans vrais impacts. Un pack dédié épée et bouclier
(payant, voir `docs/archive/chantiers.md`) ou T.C. Sword (à télécharger à la main) reste le vrai test ; le
pipeline de reciblage est prêt pour du FBX à condition d'ajouter un lecteur FBX (import Blender
natif) à la place du lecteur ASF/AMC.
