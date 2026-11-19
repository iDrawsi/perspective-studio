# Éditeur de modèle : conception et fonctionnement

## À quoi il sert
Décrire un objet depuis l'interface (sans toucher au code) et le voir tourner en perspective avec la caméra en millimètres, le bevel et les exports. Page : `web/editeur.html`.

## Le modèle : une liste de pièces simples
Repère : x vers la droite, y vers le bas, z vers le spectateur. Chaque pièce a un nom, une position, une rotation (degrés), une couleur, un réglage d'arêtes et des dimensions.

| Pièce | Dimensions | Remarque |
|---|---|---|
| Boîte | largeur, hauteur, profondeur, rayon des coins, bevel, segments des coins | le bevel est un chanfrein à 45° sur les deux faces (avant et arrière) |
| Cylindre | rayon, longueur, axe (x, y ou z), bevel, segments | le bevel chanfreine les deux extrémités |
| Prisme | contour (3 à 64 points, **convexe**), profondeur | contour tapé « x, y » par ligne |

Arêtes tracées : « contours et arêtes vives » (par défaut), « toutes », « contours seulement ».
Fichier : JSON `{"format":"perspective-studio-modele","version":1,"nom":…,"pieces":[…]}`. À la lecture, tout est borné et nettoyé (noms de 40 caractères, nombres limités, couleurs `#rrggbb`, 200 pièces au plus, type inconnu refusé) ; les noms sont affichés comme du texte, jamais comme du HTML.

## Pourquoi un nouvel ordre d'affichage (arbre BSP)
Un tri objet par objet n'est exact que si les pièces ne se traversent pas. Dans un éditeur, on enfonce un bouton dans un boîtier : c'est exactement ce qui produisait des « transparences ». Le moteur (`crates/core/src/modele.rs`) range donc toutes les faces dans un **arbre BSP** (partition binaire de l'espace) :
1. chaque pièce devient des faces plates convexes, orientées vers l'extérieur ;
2. on choisit une face comme plan de coupe ; les autres faces sont réparties devant / derrière, et celles qui le traversent sont **coupées** le long de ce plan (les côtés créés par la coupe n'ont pas d'arête d'origine) ;
3. on recommence dans chaque moitié ;
4. pour dessiner : parcours de l'arbre du plus loin au plus proche de la caméra. L'ordre est exact, pour n'importe quel assemblage.
L'arbre ne dépend pas de la caméra : construit une fois par modèle (3,6 ms pour 4 pièces / 164 faces), puis tourner la caméra ne fait que le parcourir (1,3 ms).

## Arêtes
Chaque côté d'un fragment garde l'identité de l'arête d'origine dont il vient. Une arête est tracée si elle borde le contour de la silhouette (une seule face voisine regarde la caméra) ou si c'est une arête vive (angle entre faces voisines > 35°), selon le réglage de la pièce. Les arêtes sont tracées juste après le fragment qui les porte, donc cachées par ce qui passe devant.

## Échange entre l'interface et le moteur
Pas de bibliothèque JSON dans le moteur : la page envoie le modèle par de petits appels numériques (`modele_vider`, `modele_piece` avec 16 nombres, `modele_point`, `modele_rendre`), le moteur renvoie du SVG avec un `data-p` par pièce, ce qui permet de **sélectionner une pièce en cliquant dessus**. Annuler / rétablir (Cmd+Z, Cmd+Maj+Z), sauvegarde automatique dans le navigateur, exemples (télécommande, pièces qui se traversent).

## Vérifications
- `la_decoupe_conserve_la_surface_de_chaque_face` : après découpe, la somme des aires des fragments égale l'aire de chaque face d'origine.
- `aucun_defaut_d_ordre_meme_avec_des_pieces_qui_se_traversent` : sur 378 vues (azimuts, élévations jusqu'à ±85°, focales 14, 35 et 135 mm) d'une scène avec boîte, cylindre enfoncé, prisme planté et cube tourné, en chaque point d'écran le fragment dessiné en dernier est le plus proche.
- `le_verificateur_voit_les_erreurs_d_un_tri_par_centres` : contrôle négatif, le même vérificateur détecte les erreurs de l'ancien tri sur la même scène.
- Contours non convexes et dimensions invalides refusés avec le nom de la pièce ; une boîte seule a 6 faces, 12 arêtes et n'est pas coupée.
- Dans le navigateur : ajout, modification, clic de sélection, annuler, suppression au clavier, enregistrement et relecture, fichier piégé (valeurs absurdes, HTML dans un nom), 390 px sans débordement.

## Dessin à plat sur les faces (décors)
Un **décor** est un dessin posé sur **une face plate d'une pièce**. Il suit la pièce (position, rotation) et ne dépasse jamais de sa face.
- **Faces** : boîte = devant (z+), derrière, droite, gauche, dessous, dessus (les faces des côtés sont les parties planes, hors coins arrondis et chanfreins) ; cylindre et prisme = les deux extrémités. Les côtés courbes ne sont pas gérés.
- **Repère d'une face** : origine au centre, x à droite, y vers le bas, **lus à l'endroit vus de l'extérieur** (jamais en miroir). Pour les faces verticales, « haut » est −y ; pour les faces horizontales, « haut » est −z ; on garde (u × v) · n > 0.
- **Types** : rectangle (coins arrondis), cercle, ligne ou polygone (ouvert ou fermé), texte (taille, graisse, alignement, police sans empattement ou chasse fixe). Remplissage et trait facultatifs (couleur, épaisseur). Ordre de la liste = ordre de superposition.
- **Cacher correctement** : un texte ne se coupe pas dans l'arbre BSP. Chaque décor est donc dessiné **découpé selon chaque morceau visible de sa face, juste après ce morceau** ; ce que dessine une pièce plus proche, tracé plus tard, le recouvre exactement. Les formes sont projetées point par point (perspective exacte) ; un texte est orienté par la déformation locale de la caméra à son point d'ancrage.
- **Sécurité** : le texte est échappé (`& < > " '`) dans le moteur ; à la lecture d'un fichier, type, face, nombres, couleurs et longueurs (60 décors par pièce, 120 caractères, 64 points) sont bornés.
- Clic sur un décor : sélectionne la pièce et le décor ; le panneau propose la face, la position, la rotation, les dimensions, les couleurs, monter / descendre / dupliquer / supprimer.

Vérifications supplémentaires : `les_reperes_de_face_sont_orthonormes_et_jamais_en_miroir` (20 faces de boîtes, cylindres sur les trois axes et prisme, tous tournés), `le_texte_se_lit_a_l_endroit_sur_les_six_faces` (vu de l'extérieur de chaque face : déterminant > 0 ; à l'endroit sur les 4 faces verticales), `le_texte_des_decors_ne_peut_pas_injecter_de_code`, `un_decor_est_recouvert_par_une_piece_placee_devant`, `la_taille_d_une_face_est_connue`. 15 tests Rust au total. Dans le navigateur : décors à plat (cercles, rectangles, polygones, textes), lettres sur des boutons cylindriques, texte sur 3 faces d'un cube vu en perspective, texte avec `<b>&"'` affiché tel quel, fichier piégé (face inexistante, valeurs absurdes, 200 rectangles, balises script) borné et sans injection, clic de sélection, suppression puis annulation.

## Limites connues
- **Pas de ligne d'intersection** entre deux pièces qui se traversent : seule la différence de teinte la montre.
- Pas encore : décors sur les côtés courbes, décors qui dépassent sur plusieurs faces, dégradés, images, rotation autour d'un point autre que le centre de la pièce, pièces non convexes (prisme creux, tore), groupes de pièces, symétries, import de photo.
- Les faces d'une pièce sont supposées fermées (solides) : une pièce ouverte n'est pas gérée.
