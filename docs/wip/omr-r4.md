# OMR R4 — relecture historienne de l'Est (printemps 1337)

Branche `feat/omr-r4`, worktree `../gp-omr-r4`. Ne touche ni revenus, ni garnisons, ni bâtiments
(R3) : les colonies remplacées gardent le profil de bâtiments de celles qu'elles remplacent.

## État
- [x] 1. Objectifs ajoutés par I1 (30 objectifs nouveaux sur 31 titres ; Galicie : retrait de
  « rester indépendant », Horde : Caffa fusionné dans « Cœur de l'empire »).
- [x] 2. Incertitudes om-d1..d6 : points tranchés ci-dessous.
- [x] 3. Colonies ramenées de loin.
- [ ] 4. Régénération géo (colonies déplacées/ajoutées, propriétaires changés) puis tests.

## Décisions (sources : Wikipédia en/ru/tr sauf mention)
### Objectifs I1
- Remplacés :
  - Cilicie « Séleucie » (visait `prov_karaman`, donc Larende, capitale karamanide ; Silifke est
    karamanide depuis la fin du XIIIe s.) → **Marash** (`prov_elbistan`), place arménienne perdue
    au profit des Mamelouks à la fin du XIIIe s.
  - Hama « Alep » (un vassal mamelouk loyal n'a aucune prétention sur Alep) → **secouer la tutelle
    mamelouke** (`be_independent` ; la principauté est supprimée en 1341).
  - Trébizonde « Limnia et Oinaion » visait `prov_amasya` (Amasya, Niksar, Çorum : intérieur, pas
    les bandons côtiers) → **Sinope et Amisos** (`prov_sinope`) : Sinope trapézontine 1254-1265.
  - Eşrefoğulları « Venger Süleyman » : faction supprimée (voir plus bas).
- Reformulés : Archipel/Chios (les Zaccaria n'étaient pas « alliés de Naxos » : Chios latine
  1304-1329), Barqa/Tripoli (les Sulaym de Barqa et les Dabbab de Tripolitaine sont une même
  confédération), Karaman/Beyşehir (tenue par les Hamidides), Bulgarie (Philippopolis, voir plus bas).
- Gardés (plausibles et réalisables) : Artuqides/Mossoul, Aydın/Chios, Candar/Amasya,
  Dulkadir/Kayseri, Épire/Thessalie (perdue en 1333), Eretna/Konya, Galicie/Brest, Géorgie/Samtskhé
  (soumission 1334), Germiyan suzerain d'Aydın, Hafsides/Tripoli, Hamid↔Teke, Hongrie/Halych,
  Djalayirides/Mardin, Karasi/Lesbos, Kiev/Tchernigov, Menteşe/Rhodes, Hospitaliers/Smyrne (1344),
  Saruhan/Smyrne, Slavonie/Zara (1358), Świdnica/Legnica, Ordre/Samogitie, Tripoli/Syrte.

### Propriétaires et suzerainetés
- **Eşrefoğulları supprimés** : beylik éteint en 1326 (Timurtaş fait exécuter le bey) ; Beyşehir
  passe aux Hamidides (qui la vendront à Murad Ier). `prov_beysehir` → `fac_hamid` (titre
  `tit_hamid`), faction/titre/personnage/maison/fiche/écus/portraits supprimés. Ajout de
  `chr_hizir_hamid` (Hızır Bey, bey de Hamid vers 1335-1358), souverain de `fac_hamid` (sans
  portrait : à générer, R6 ou plus tard).
- **Anchialos bulgare, Philippopolis byzantine** (les lots D avaient inversé) : Ivan Alexandre
  reprend la côte pontique en 1331 (Roussokastro 1332, statu quo) ; Philippopolis, prise en 1322,
  est reprise par Byzance en 1323 et cédée à la Bulgarie seulement en 1344. Titres de jure et
  revendication bulgare échangés en conséquence.
- **Mazovie** : seul Wenceslas de Płock a fait hommage à Jean de Bohême (1329) ; Trojden (Czersk-
  Varsovie) et Siemowit II (Rawa) restent neutres → `fac_warsaw`, `fac_rawa` sans suzerain, titres
  sans liège de jure ; objectif ajouté à Varsovie : Rawa (léguée aux fils de Trojden en 1345).
- **Głogów** : hommage d'Henri IV de Żagań en 1329 (certain), mort le 22 janvier 1342 ; la ville
  de Głogów est au roi de Bohême depuis octobre 1331 (textes corrigés ; la province reste au duc,
  simplification : pas de scission possible sans casser la capitale).
- **Dalmatie** : Trogir et Šibenik vénitiens depuis 1322, Split 1327 (certain) ; cathédrale de
  Šibenik (1431) retirée de la description.
- **Smolensk** : gardé vassal de la Horde, allié de la Lituanie. Condominium de tribut : Ivan
  Alexandrovitch reconnaît la primauté de Gediminas, mais l'expédition punitive d'Özbeg (1339)
  montre que le tribut est encore dû en 1337.
- **Kuyavie** : déjà à l'Ordre (occupation 1332-1343) : rien à faire.
- **Épire** : Jean II mort en 1335, régence d'Anna pour Nicéphore II : déjà conforme.

### Colonies
- Volok Lamski (Torjok, 75 km) → `prov_moscow` (≈ 9 km du polygone) : ancien condominium
  Novgorod/grand-prince, dont le lieutenant d'Ivan Kalita a chassé le représentant novgorodien.
- Bejetsk (Tver) → `prov_torjok` sous le nom de Bejitchi (Bejetski Verkh) : volost novgorodienne
  (1137) disputée par Tver et Moscou au XIVe s. (≈ 26 km du polygone).
- Tver reçoit Kachine (apanage tverien de Vassili Mikhaïlovitch, cité en 1238 ; ≈ 19 km).
- Gorokhovets (Vladimir, 82 km) → `prov_murom` (Souzdal) : rattachée avec Nijni et Gorodets à la
  maison de Souzdal ; Vladimir reçoit Iouriev-Polski (1152, apanage uni à Moscou vers 1340).
- Kamianiets (Hrodna, 57 km) → `prov_brest` (tour de Vladimir Vassilkovitch, terre de Brest) ;
  Hrodna reçoit Zelva (chronique hypatienne, 1258).
- Anachronismes signalés par D1/D3 corrigés : Raseborg (vers 1370) → Junkarsborg (fort de Karis,
  début XIVe s.) ; Kastelholm (cité en 1388) → Kuusisto (château épiscopal, 1317) ; Kalouga
  (1371) → Vorotynsk (1155) ; Khlynov (nom du XVe s.) → Viatka ; « Laure de la Trinité-Saint-Serge »
  → « Ermitage de la Trinité (Makovets) ».

## Restes incertains (non tranchés, sources insuffisantes)
- Kholmogory (1355), Kotelnitch, Vychni Volotchek (cité au XVe s.) : gardés comme localités
  probables de la colonisation novgorodienne.
- Sozopolis : bulgare ou byzantine en 1337 (laissée avec Anchialos, bulgare).
- Mourom rattachée à Souzdal (principauté de Mourom autonome), Tchernigov à Briansk.
- Alexandre de Tver au printemps 1337 (Pskov ou déjà en route vers la Horde).
- Nicomédie (chute en 1337, avant ou après le printemps), Dulkadir (1337 ou 1348), Konya
  karamanide, Belaur de Vidin, évêque Jakob II d'Ösel-Wiek (mort en 1337 ?), Halland suédois.
- Souverains non nommés : Teke, Karasi, Circassie, Alanie, Rostov, Beloozero, Haute-Oka, Perm,
  Viatka, Théodoro, Gabès.

## Prochaine étape
Régénération géo (ordre `om-om2.md`), commit dédié aux artefacts, puis cargo test --workspace
et pytest, recalage des tests liés aux données (compteurs de factions : −1).
