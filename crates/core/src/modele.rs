//! Éditeur de modèle : des pièces simples (boîte, cylindre, prisme) deviennent des faces convexes, rangées dans un
//! arbre BSP (partition binaire de l'espace) qui donne un ordre d'affichage EXACT, même quand des pièces se traversent.
//! L'arbre ne dépend pas de la caméra : il est construit une fois par modèle ; tourner la caméra ne fait que le parcourir.
//! Repère : x vers la droite, y vers le bas, z vers le spectateur.
use crate::vue::{focale_px, Cam, CADRE};
use std::collections::HashMap;
use std::f64::consts::PI;

pub type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn add(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn mulk(a: V3, k: f64) -> V3 { [a[0] * k, a[1] * k, a[2] * k] }
fn dot(a: V3, b: V3) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn long(a: V3) -> f64 { dot(a, a).sqrt() }
fn unit(a: V3) -> V3 { let l = long(a); if l < 1e-300 { [0.0; 3] } else { mulk(a, 1.0 / l) } }
fn newell(p: &[V3]) -> V3 {
    let mut n = [0.0; 3];
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        n = [n[0] + (a[1] - b[1]) * (a[2] + b[2]), n[1] + (a[2] - b[2]) * (a[0] + b[0]), n[2] + (a[0] - b[0]) * (a[1] + b[1])];
    }
    n
}
fn moyenne(p: &[V3]) -> V3 { mulk(p.iter().fold([0.0; 3], |m, q| add(m, *q)), 1.0 / p.len() as f64) }

// ---------- Pièces ----------
/// genre 0 : boîte (p = largeur, hauteur, profondeur, rayon des coins, bevel, segments des coins)
/// genre 1 : cylindre (p = rayon, longueur, axe 0/1/2 = x/y/z, bevel, segments, _)
/// genre 2 : prisme (p = profondeur, _, _, _, _, _ ; contour dans `pts`, convexe)
/// aretes : 0 = contours et arêtes vives, 1 = toutes les arêtes, 2 = contours seulement
/// Dessin posé à plat sur une face de pièce. genre 0 rectangle (a = largeur, b = hauteur, c = rayon des coins),
/// 1 cercle (a = rayon), 2 polygone ou ligne (points dans `pts`, c > 0,5 : fermé),
/// 3 texte (a = taille, b = graisse, c = ancre 0 début / 1 milieu / 2 fin, d = police 0 sans / 1 mono).
/// (x, y) : position dans le repère de la face (origine au centre, x à droite, y vers le bas) ; rot en degrés.
#[derive(Clone, Debug, Default)]
pub struct Decor {
    pub genre: u32,
    pub id: u32,
    pub cara: u8,
    pub x: f64,
    pub y: f64,
    pub rot: f64,
    pub p: [f64; 4],
    pub fill: Option<u32>,
    pub trait_: Option<u32>,
    pub ep: f64,
    pub pts: Vec<[f64; 2]>,
    pub texte: String,
}

#[derive(Clone, Debug, Default)]
pub struct Piece {
    pub id: u32,
    pub genre: u32,
    pub pos: V3,
    pub rot: V3,
    pub p: [f64; 6],
    pub rgb: u32,
    pub aretes: u32,
    pub pts: Vec<[f64; 2]>,
    pub decors: Vec<Decor>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Erreur { Dimensions(u32), NonConvexe(u32), Vide }
impl Erreur {
    /// Code pour la page : 1 dimensions invalides, 2 contour non convexe, 3 modèle vide.
    pub fn code(&self) -> (u32, u32) { match *self { Erreur::Dimensions(i) => (1, i), Erreur::NonConvexe(i) => (2, i), Erreur::Vide => (3, 0) } }
}

fn contour_rond(w: f64, h: f64, r: f64, n: usize) -> Vec<[f64; 2]> {
    let (x, y) = (-w / 2.0, -h / 2.0);
    let mut out = Vec::new();
    let mut arc = |cx: f64, cy: f64, a0: f64| {
        if r <= 0.0 { out.push([cx, cy]); return; }
        for i in 0..=n { let a = a0 + (PI / 2.0) * i as f64 / n as f64; out.push([cx + r * a.cos(), cy + r * a.sin()]); }
    };
    arc(x + w - r, y + r, -PI / 2.0);
    arc(x + w - r, y + h - r, 0.0);
    arc(x + r, y + h - r, PI / 2.0);
    arc(x + r, y + r, PI);
    out
}

fn en_v3(c: &[[f64; 2]], z: f64) -> Vec<V3> { c.iter().map(|p| [p[0], p[1], z]).collect() }

/// Anneau de quadrilatères entre deux contours de même nombre de points ; `cara(i)` repère les côtés plats.
fn anneau(a: &[V3], b: &[V3], cara: &dyn Fn(usize) -> Option<u8>) -> Vec<(Vec<V3>, Option<u8>)> {
    (0..a.len()).map(|i| { let j = (i + 1) % a.len(); (vec![a[i], a[j], b[j], b[i]], cara(i)) }).collect()
}

/// Faces repérées : boîte 0 devant (z+), 1 derrière, 2 droite (x+), 3 gauche, 4 dessous (y+), 5 dessus ;
/// cylindre et prisme : 0 et 1 pour les deux extrémités (z local +, z local −).
pub const NB_CARAS: [u8; 3] = [6, 2, 2];

/// Polygones (non orientés) de la pièce, dans son repère local, centrée sur l'origine, avec le repère de face éventuel.
fn polygones_locaux(p: &Piece) -> Result<Vec<(Vec<V3>, Option<u8>)>, Erreur> {
    let bad = Erreur::Dimensions(p.id);
    let f = p.p;
    if f.iter().any(|v| !v.is_finite()) { return Err(bad); }
    let aucune = |_: usize| -> Option<u8> { None };
    match p.genre {
        0 => {
            let (w, h, d, r) = (f[0], f[1], f[2], f[3].max(0.0));
            if w < 0.01 || h < 0.01 || d < 0.01 { return Err(bad); }
            let r = r.min(w.min(h) / 2.0 - 1e-3).max(0.0);
            let b = f[4].max(0.0).min(w.min(h) / 2.0 - 1e-3).min(d / 2.0 - 1e-3).max(0.0);
            let n = (f[5].round() as i64).clamp(1, 24) as usize;
            let ext = contour_rond(w, h, r, n);
            // côtés plats du contour : droite, bas, gauche, haut
            let plat = move |i: usize| -> Option<u8> {
                let k = if r > 0.0 { if i % (n + 1) == n { i / (n + 1) } else { return None } } else { i };
                Some([2u8, 4, 3, 5][k % 4])
            };
            let mut out: Vec<(Vec<V3>, Option<u8>)> = Vec::new();
            if b < 1e-6 {
                out.push((en_v3(&ext, d / 2.0), Some(0)));
                out.push((en_v3(&ext, -d / 2.0), Some(1)));
                out.extend(anneau(&en_v3(&ext, d / 2.0), &en_v3(&ext, -d / 2.0), &plat));
            } else {
                let ri = if r > 0.0 { (r - b).max(1e-3) } else { 0.0 };
                let int = contour_rond(w - 2.0 * b, h - 2.0 * b, ri, n);
                out.push((en_v3(&int, d / 2.0), Some(0)));
                out.push((en_v3(&int, -d / 2.0), Some(1)));
                out.extend(anneau(&en_v3(&int, d / 2.0), &en_v3(&ext, d / 2.0 - b), &aucune));
                out.extend(anneau(&en_v3(&ext, d / 2.0 - b), &en_v3(&ext, -d / 2.0 + b), &plat));
                out.extend(anneau(&en_v3(&ext, -d / 2.0 + b), &en_v3(&int, -d / 2.0), &aucune));
            }
            Ok(out)
        }
        1 => {
            let (r, l) = (f[0], f[1]);
            if r < 0.01 || l < 0.01 { return Err(bad); }
            let n = (f[4].round() as i64).clamp(3, 64) as usize;
            let b = f[3].max(0.0).min(r - 1e-3).min(l / 2.0 - 1e-3).max(0.0);
            let cercle = |rr: f64| -> Vec<[f64; 2]> { (0..n).map(|i| { let a = 2.0 * PI * i as f64 / n as f64; [rr * a.cos(), rr * a.sin()] }).collect() };
            let ext = cercle(r);
            let mut out: Vec<(Vec<V3>, Option<u8>)> = Vec::new();
            if b < 1e-6 {
                out.push((en_v3(&ext, l / 2.0), Some(0)));
                out.push((en_v3(&ext, -l / 2.0), Some(1)));
                out.extend(anneau(&en_v3(&ext, l / 2.0), &en_v3(&ext, -l / 2.0), &aucune));
            } else {
                let int = cercle(r - b);
                out.push((en_v3(&int, l / 2.0), Some(0)));
                out.push((en_v3(&int, -l / 2.0), Some(1)));
                out.extend(anneau(&en_v3(&int, l / 2.0), &en_v3(&ext, l / 2.0 - b), &aucune));
                out.extend(anneau(&en_v3(&ext, l / 2.0 - b), &en_v3(&ext, -l / 2.0 + b), &aucune));
                out.extend(anneau(&en_v3(&ext, -l / 2.0 + b), &en_v3(&int, -l / 2.0), &aucune));
            }
            // l'axe local est z ; permutations circulaires (rotations propres) pour x et y
            let axe = f[2].round() as i64;
            let perm = |v: V3| -> V3 { match axe { 0 => [v[2], v[0], v[1]], 1 => [v[1], v[2], v[0]], _ => v } };
            Ok(out.into_iter().map(|(poly, c)| (poly.into_iter().map(perm).collect(), c)).collect())
        }
        2 => {
            let d = f[0];
            let c = &p.pts;
            if d < 0.01 || c.len() < 3 || c.iter().any(|q| !q[0].is_finite() || !q[1].is_finite()) { return Err(bad); }
            // convexité : tous les produits vectoriels de même signe
            let mut signe = 0.0f64;
            for i in 0..c.len() {
                let (a, b, e) = (c[i], c[(i + 1) % c.len()], c[(i + 2) % c.len()]);
                let z = (b[0] - a[0]) * (e[1] - b[1]) - (b[1] - a[1]) * (e[0] - b[0]);
                if z.abs() < 1e-9 { continue; }
                if signe == 0.0 { signe = z.signum(); } else if z.signum() != signe { return Err(Erreur::NonConvexe(p.id)); }
            }
            if signe == 0.0 { return Err(bad); }
            let cx = c.iter().map(|q| q[0]).sum::<f64>() / c.len() as f64;
            let cy = c.iter().map(|q| q[1]).sum::<f64>() / c.len() as f64;
            let centre: Vec<[f64; 2]> = c.iter().map(|q| [q[0] - cx, q[1] - cy]).collect();
            let mut out = vec![(en_v3(&centre, d / 2.0), Some(0u8)), (en_v3(&centre, -d / 2.0), Some(1u8))];
            out.extend(anneau(&en_v3(&centre, d / 2.0), &en_v3(&centre, -d / 2.0), &aucune));
            Ok(out)
        }
        _ => Err(bad),
    }
}

fn rotation(r: V3) -> [[f64; 3]; 3] {
    let (a, b, c) = (r[0].to_radians(), r[1].to_radians(), r[2].to_radians());
    let (sx, cx, sy, cy, sz, cz) = (a.sin(), a.cos(), b.sin(), b.cos(), c.sin(), c.cos());
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rz = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    let mul = |p: [[f64; 3]; 3], q: [[f64; 3]; 3]| { let mut o = [[0.0; 3]; 3]; for i in 0..3 { for j in 0..3 { o[i][j] = (0..3).map(|k| p[i][k] * q[k][j]).sum(); } } o };
    mul(rz, mul(ry, rx))
}

// ---------- Faces et arêtes d'origine ----------
#[derive(Clone, Debug)]
pub struct Face {
    pub pts: Vec<V3>,
    /// pour chaque côté pts[i] → pts[i+1] : l'arête d'origine dont il fait partie ; `None` pour un côté créé par une coupe
    pub ef: Vec<Option<usize>>,
    pub n: V3,
    pub p0: V3,
    /// face d'origine (avant découpe)
    pub src: usize,
    pub piece: usize,
    /// repère de face (pour les décors) : 0 à 5 selon la pièce
    pub cara: Option<u8>,
}

/// Arête d'origine : faces voisines (indices de faces d'origine).
#[derive(Clone, Debug, Default)]
pub struct Arete { pub faces: Vec<usize> }

/// Repère d'une face plate : origine au centre, u vers la droite et v vers le bas (lus à l'endroit vus de l'extérieur), n vers l'extérieur.
#[derive(Clone, Copy, Debug)]
pub struct Cadre { pub o: V3, pub u: V3, pub v: V3, pub n: V3, pub larg: f64, pub haut: f64 }

/// Repère local d'une face de normale `n` (sortante) : « haut » = −y ; pour les faces horizontales, « haut » = −z.
/// u est choisi pour que la face se lise à l'endroit (jamais en miroir), c'est-à-dire (u × v) · n > 0.
fn repere_local(n: V3) -> (V3, V3) {
    let haut = if n[1].abs() > 0.99 { [0.0, 0.0, -1.0] } else { [0.0, -1.0, 0.0] };
    let bas = unit(mulk(sub(haut, mulk(n, dot(haut, n))), -1.0));
    let f = mulk(n, -1.0);
    let hx = [haut[1] * f[2] - haut[2] * f[1], haut[2] * f[0] - haut[0] * f[2], haut[0] * f[1] - haut[1] * f[0]];
    let mut u = unit(hx);
    let cp = [u[1] * bas[2] - u[2] * bas[1], u[2] * bas[0] - u[0] * bas[2], u[0] * bas[1] - u[1] * bas[0]];
    if dot(cp, n) < 0.0 { u = mulk(u, -1.0); }
    (u, bas)
}

pub struct Modele {
    pub pieces: Vec<Piece>,
    pub cadres: HashMap<(usize, u8), Cadre>,
    pub faces_origine: Vec<Face>,
    pub aretes: Vec<Arete>,
    pub arbre: Option<Box<Noeud>>,
    pub nb_fragments: usize,
    pub centre: V3,
    pub rayon: f64,
}

fn cle(v: V3) -> [i64; 3] { [(v[0] * 1000.0).round() as i64, (v[1] * 1000.0).round() as i64, (v[2] * 1000.0).round() as i64] }

/// Construit toutes les faces du modèle (monde), leurs arêtes et l'arbre BSP.
pub fn construire(pieces: &[Piece]) -> Result<Modele, Erreur> {
    if pieces.is_empty() { return Err(Erreur::Vide); }
    let mut faces: Vec<Face> = Vec::new();
    let mut cadres: HashMap<(usize, u8), Cadre> = HashMap::new();
    for (ip, p) in pieces.iter().enumerate() {
        let polys = polygones_locaux(p)?;
        let r = rotation(p.rot);
        let monde = |v: V3| add([dot(r[0], v), dot(r[1], v), dot(r[2], v)], p.pos);
        let tourne = |v: V3| [dot(r[0], v), dot(r[1], v), dot(r[2], v)];
        let tous: Vec<V3> = polys.iter().flat_map(|(q, _)| q.iter()).copied().collect();
        let centre_local = moyenne(&tous);
        for (mut loc, cara) in polys {
            // orientation vers l'extérieur, dans le repère local
            let mut nl = newell(&loc);
            let cl = moyenne(&loc);
            if dot(nl, sub(cl, centre_local)) < 0.0 { loc.reverse(); nl = mulk(nl, -1.0); }
            let nl = unit(nl);
            if long(nl) < 0.5 { continue; } // face dégénérée
            let pts: Vec<V3> = loc.iter().map(|v| monde(*v)).collect();
            let n = tourne(nl);
            if let Some(c) = cara {
                let (ul, vl) = repere_local(nl);
                let (u, v) = (tourne(ul), tourne(vl));
                let o = monde(cl);
                let (mut u0, mut u1, mut v0, mut v1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
                for q in &pts { let d = sub(*q, o); let (a, b) = (dot(d, u), dot(d, v)); u0 = u0.min(a); u1 = u1.max(a); v0 = v0.min(b); v1 = v1.max(b); }
                cadres.insert((ip, c), Cadre { o, u, v, n, larg: u1 - u0, haut: v1 - v0 });
            }
            let k = pts.len();
            faces.push(Face { p0: pts[0], pts, ef: vec![None; k], n, src: faces.len(), piece: ip, cara });
        }
    }
    // arêtes d'origine
    let mut index: HashMap<([i64; 3], [i64; 3]), usize> = HashMap::new();
    let mut aretes: Vec<Arete> = Vec::new();
    for (fi, f) in faces.iter_mut().enumerate() {
        let k = f.pts.len();
        for i in 0..k {
            let (a, b) = (cle(f.pts[i]), cle(f.pts[(i + 1) % k]));
            let kk = if a < b { (a, b) } else { (b, a) };
            let id = *index.entry(kk).or_insert_with(|| { aretes.push(Arete::default()); aretes.len() - 1 });
            aretes[id].faces.push(fi);
            f.ef[i] = Some(id);
        }
    }
    // centre et rayon
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for f in &faces { for v in &f.pts { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } } }
    let centre = mulk(add(lo, hi), 0.5);
    let rayon = faces.iter().flat_map(|f| f.pts.iter()).map(|v| long(sub(*v, centre))).fold(0.0, f64::max);
    let arbre = bsp(faces.clone(), 0);
    let nb = arbre.as_ref().map_or(0, |a| a.compte());
    Ok(Modele { pieces: pieces.to_vec(), cadres, faces_origine: faces, aretes, arbre, nb_fragments: nb, centre, rayon })
}

// ---------- Arbre BSP ----------
pub struct Noeud {
    pub p: V3,
    pub n: V3,
    pub coplanaires: Vec<Face>,
    pub avant: Option<Box<Noeud>>,
    pub arriere: Option<Box<Noeud>>,
}
impl Noeud {
    fn compte(&self) -> usize {
        self.coplanaires.len() + self.avant.as_ref().map_or(0, |n| n.compte()) + self.arriere.as_ref().map_or(0, |n| n.compte())
    }
    /// Fragments du plus loin au plus proche vu de `oeil`.
    pub fn parcourir<'a>(&'a self, oeil: V3, out: &mut Vec<&'a Face>) {
        let (loin, pres) = if dot(self.n, sub(oeil, self.p)) > 0.0 { (&self.arriere, &self.avant) } else { (&self.avant, &self.arriere) };
        if let Some(n) = loin { n.parcourir(oeil, out); }
        out.extend(self.coplanaires.iter());
        if let Some(n) = pres { n.parcourir(oeil, out); }
    }
}

const EPS: f64 = 1e-5;
#[derive(Clone, Copy, PartialEq)]
enum Cote { Avant, Arriere, Sur }

fn cote(d: f64) -> Cote { if d > EPS { Cote::Avant } else if d < -EPS { Cote::Arriere } else { Cote::Sur } }

/// Coupe un polygone convexe par un plan ; renvoie (partie avant, partie arrière) ; les côtés créés par la coupe n'ont pas d'arête d'origine.
fn couper(f: &Face, p: V3, n: V3) -> (Option<Face>, Option<Face>) {
    let k = f.pts.len();
    let d: Vec<f64> = f.pts.iter().map(|v| dot(n, sub(*v, p))).collect();
    let c: Vec<Cote> = d.iter().map(|&x| cote(x)).collect();
    if c.iter().all(|&x| x != Cote::Arriere) { return (Some(f.clone()), None); }
    if c.iter().all(|&x| x != Cote::Avant) { return (None, Some(f.clone())); }
    let (mut av, mut ar): (Vec<(V3, Option<usize>)>, Vec<(V3, Option<usize>)>) = (Vec::new(), Vec::new());
    for i in 0..k {
        let j = (i + 1) % k;
        let (a, b, fl) = (f.pts[i], f.pts[j], f.ef[i]);
        let (ca, cb) = (c[i], c[j]);
        match (ca, cb) {
            (Cote::Avant, Cote::Avant) | (Cote::Avant, Cote::Sur) => av.push((a, fl)),
            (Cote::Arriere, Cote::Arriere) | (Cote::Arriere, Cote::Sur) => ar.push((a, fl)),
            (Cote::Sur, Cote::Avant) => { av.push((a, fl)); ar.push((a, None)); }
            (Cote::Sur, Cote::Arriere) => { ar.push((a, fl)); av.push((a, None)); }
            (Cote::Sur, Cote::Sur) => { av.push((a, fl)); ar.push((a, fl)); }
            (Cote::Avant, Cote::Arriere) | (Cote::Arriere, Cote::Avant) => {
                let t = d[i] / (d[i] - d[j]);
                let m = add(a, mulk(sub(b, a), t));
                if ca == Cote::Avant { av.push((a, fl)); av.push((m, None)); ar.push((m, fl)); }
                else { ar.push((a, fl)); ar.push((m, None)); av.push((m, fl)); }
            }
        }
    }
    let faire = |l: Vec<(V3, Option<usize>)>| -> Option<Face> {
        if l.len() < 3 { return None; }
        let pts: Vec<V3> = l.iter().map(|x| x.0).collect();
        // fragment sans surface : on l'écarte
        if long(newell(&pts)) < 1e-9 { return None; }
        Some(Face { pts, ef: l.iter().map(|x| x.1).collect(), n: f.n, p0: f.p0, src: f.src, piece: f.piece, cara: f.cara })
    };
    (faire(av), faire(ar))
}

fn bsp(mut faces: Vec<Face>, profondeur: usize) -> Option<Box<Noeud>> {
    if faces.is_empty() { return None; }
    // trop profond : on s'arrête (reste exact pour ce qui est déjà ordonné)
    if profondeur > 4000 { let f = faces.remove(0); return Some(Box::new(Noeud { p: f.p0, n: f.n, coplanaires: std::iter::once(f).chain(faces).collect(), avant: None, arriere: None })); }
    // choix du plan : parmi quelques candidats, celui qui coupe le moins et équilibre le mieux
    let pas = (faces.len() / 12).max(1);
    let mut meilleur = (usize::MAX, 0usize);
    for ci in (0..faces.len()).step_by(pas) {
        let (p, n) = (faces[ci].p0, faces[ci].n);
        let (mut coupes, mut av, mut ar) = (0usize, 0usize, 0usize);
        for (i, f) in faces.iter().enumerate() {
            if i == ci { continue; }
            let cs: Vec<Cote> = f.pts.iter().map(|v| cote(dot(n, sub(*v, p)))).collect();
            let a = cs.iter().any(|&x| x == Cote::Avant);
            let r = cs.iter().any(|&x| x == Cote::Arriere);
            if a && r { coupes += 1; } else if a { av += 1; } else if r { ar += 1; }
        }
        let score = coupes * 8 + av.abs_diff(ar);
        if score < meilleur.0 { meilleur = (score, ci); }
    }
    let racine = faces.swap_remove(meilleur.1);
    let (p, n) = (racine.p0, racine.n);
    let (mut av, mut ar, mut cop) = (Vec::new(), Vec::new(), vec![racine]);
    for f in faces {
        let cs: Vec<Cote> = f.pts.iter().map(|v| cote(dot(n, sub(*v, p)))).collect();
        if cs.iter().all(|&x| x == Cote::Sur) { cop.push(f); continue; }
        let (a, r) = couper(&f, p, n);
        if let Some(x) = a { av.push(x); }
        if let Some(x) = r { ar.push(x); }
    }
    Some(Box::new(Noeud { p, n, coplanaires: cop, avant: bsp(av, profondeur + 1), arriere: bsp(ar, profondeur + 1) }))
}

// ---------- Rendu ----------
fn nombre(v: f64) -> String {
    let s = format!("{v:.1}");
    let s = s.strip_suffix(".0").unwrap_or(&s).to_string();
    if s == "-0" { "0".to_string() } else { s }
}

/// Éclairage des faces : direction de la lumière (vers la source) et part de lumière ambiante (0 à 1).
#[derive(Clone, Copy, Debug)]
pub struct Lumiere { pub dir: V3, pub ambiant: f64 }
impl Default for Lumiere {
    fn default() -> Self { Lumiere { dir: unit([-0.6, -0.5, 0.62]), ambiant: 0.22 } }
}

fn teinte(rgb: u32, t: f64, ambiant: f64) -> String {
    let k = ambiant + (1.0 - ambiant) * t.clamp(0.0, 1.0);
    let c = |sh: u32| ((((rgb >> sh) & 255) as f64) * k).round() as u8;
    format!("#{:02x}{:02x}{:02x}", c(16), c(8), c(0))
}

/// Caméra qui tourne autour du centre du modèle ; `dist` ≤ 0 : cadrage automatique pour que le modèle tienne dans le cadre.
pub fn camera_du_modele(m: &Modele, az: f64, el: f64, dist: f64, focale_mm: f64) -> Cam {
    let f = focale_px(focale_mm);
    let rb = m.rayon.max(1.0);
    let d = if dist <= 0.0 { (rb * rb + (f * rb / (0.45 * CADRE)).powi(2)).sqrt().max(rb * 1.08) } else { dist.max(rb * 1.05) };
    let (a, e) = (az.to_radians(), el.clamp(-89.0, 89.0).to_radians());
    let eye = [m.centre[0] + d * e.cos() * a.sin(), m.centre[1] - d * e.sin(), m.centre[2] + d * e.cos() * a.cos()];
    Cam::look_at(eye, m.centre, f)
}

pub fn distance_auto(m: &Modele, focale_mm: f64) -> f64 {
    let f = focale_px(focale_mm);
    let rb = m.rayon.max(1.0);
    (rb * rb + (f * rb / (0.45 * CADRE)).powi(2)).sqrt().max(rb * 1.08)
}

/// Fragments visibles dans l'ordre de dessin (du plus loin au plus proche).
pub fn ordre_de_dessin<'a>(m: &'a Modele, cam: &Cam) -> Vec<&'a Face> {
    let mut tous = Vec::new();
    if let Some(a) = &m.arbre { a.parcourir(cam.center, &mut tous); }
    tous.retain(|f| dot(f.n, sub(cam.center, f.p0)) > 0.0);
    tous
}

fn echapper(t: &str) -> String {
    t.chars().map(|c| match c { '&' => "&amp;".to_string(), '<' => "&lt;".to_string(), '>' => "&gt;".to_string(), '"' => "&quot;".to_string(), '\'' => "&#39;".to_string(), c if (c as u32) < 32 && c != ' ' => " ".to_string(), c => c.to_string() }).collect()
}
fn hex(rgb: Option<u32>) -> String { rgb.map_or("none".to_string(), |c| format!("#{:06x}", c & 0xFF_FFFF)) }

/// Point (monde) du repère de face.
fn sur_la_face(c: &Cadre, fx: f64, fy: f64) -> V3 { add(c.o, add(mulk(c.u, fx), mulk(c.v, fy))) }

/// Contour d'un décor dans son propre repère (avant rotation et position).
fn contour_decor(d: &Decor) -> Vec<[f64; 2]> {
    match d.genre {
        0 => {
            let (w, h) = (d.p[0].max(0.01), d.p[1].max(0.01));
            let r = d.p[2].max(0.0).min(w.min(h) / 2.0 - 1e-3).max(0.0);
            contour_rond(w, h, r, 6)
        }
        1 => { let r = d.p[0].max(0.01); (0..48).map(|i| { let a = 2.0 * PI * i as f64 / 48.0; [r * a.cos(), r * a.sin()] }).collect() }
        _ => d.pts.clone(),
    }
}

/// SVG d'un décor, projeté avec la caméra. `cadre_ecran` : rectangle de l'écran du morceau de face qui le porte (pour ignorer ce qui n'y touche pas).
fn decor_svg(cam: &Cam, c: &Cadre, d: &Decor, cadre_ecran: [f64; 4]) -> Option<String> {
    let (s, co) = d.rot.to_radians().sin_cos();
    let en_face = |sx: f64, sy: f64| sur_la_face(c, d.x + sx * co - sy * s, d.y + sx * s + sy * co);
    let trait_attrs = format!("stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\"", hex(d.trait_), nombre(d.ep.max(0.0)));
    let id = d.id;
    if d.genre == 3 {
        if d.texte.is_empty() { return None; }
        let e = 0.5;
        let o = cam.project(en_face(0.0, 0.0));
        let ux = cam.project(en_face(e, 0.0));
        let uy = cam.project(en_face(0.0, e));
        let m = [(ux[0] - o[0]) / e, (ux[1] - o[1]) / e, (uy[0] - o[0]) / e, (uy[1] - o[1]) / e, o[0], o[1]];
        let mm = m.iter().map(|v| format!("{v:.4}").trim_end_matches('0').trim_end_matches('.').to_string()).map(|t| if t == "-0" || t.is_empty() { "0".to_string() } else { t }).collect::<Vec<_>>().join(" ");
        let ancre = ["start", "middle", "end"][(d.p[2].round() as usize).min(2)];
        let police = if d.p[3] > 0.5 { "ui-monospace,Menlo,monospace" } else { "&quot;Helvetica Neue&quot;,Helvetica,Arial,sans-serif" };
        return Some(format!("<text data-d=\"{id}\" transform=\"matrix({mm})\" font-size=\"{}\" font-weight=\"{}\" text-anchor=\"{ancre}\" font-family=\"{police}\" fill=\"{}\" {trait_attrs}>{}</text>",
            nombre(d.p[0].max(0.1)), nombre(d.p[1].clamp(100.0, 900.0)), hex(d.fill), echapper(&d.texte)));
    }
    if d.genre == 4 {
        // grille de trous ronds : a = rayon, b = pas, c = colonnes, d = lignes ; un seul tracé pour tous les trous
        let (r, pas) = (d.p[0].max(0.05), d.p[1].max(0.1));
        let (nx, ny) = ((d.p[2].round() as i64).clamp(1, 40) as usize, (d.p[3].round() as i64).clamp(1, 40) as usize);
        let mut dd = String::new();
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for j in 0..ny {
            for i in 0..nx {
                let (cx, cy) = ((i as f64 - (nx as f64 - 1.0) / 2.0) * pas, (j as f64 - (ny as f64 - 1.0) / 2.0) * pas);
                for k in 0..12 {
                    let a = 2.0 * PI * k as f64 / 12.0;
                    let (px, py) = (cx + r * a.cos(), cy + r * a.sin());
                    let q = cam.project(en_face(px, py));
                    x0 = x0.min(q[0]); x1 = x1.max(q[0]); y0 = y0.min(q[1]); y1 = y1.max(q[1]);
                    dd += &format!("{}{} {}", if k == 0 { "M" } else { " L" }, nombre(q[0]), nombre(q[1]));
                }
                dd += " Z";
            }
        }
        if x1 < cadre_ecran[0] || x0 > cadre_ecran[2] || y1 < cadre_ecran[1] || y0 > cadre_ecran[3] { return None; }
        return Some(format!("<path data-d=\"{id}\" d=\"{dd}\" fill=\"{}\" {trait_attrs}/>", hex(d.fill)));
    }
    let loc = contour_decor(d);
    if loc.len() < 2 { return None; }
    let pr: Vec<V3> = loc.iter().map(|q| cam.project(en_face(q[0], q[1]))).collect();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for q in &pr { x0 = x0.min(q[0]); x1 = x1.max(q[0]); y0 = y0.min(q[1]); y1 = y1.max(q[1]); }
    if x1 < cadre_ecran[0] || x0 > cadre_ecran[2] || y1 < cadre_ecran[1] || y0 > cadre_ecran[3] { return None; }
    let ferme = d.genre != 2 || d.p[2] > 0.5;
    let dd = pr.iter().map(|q| format!("{} {}", nombre(q[0]), nombre(q[1]))).collect::<Vec<_>>().join(" L");
    Some(format!("<path data-d=\"{id}\" d=\"M{dd}{}\" fill=\"{}\" {trait_attrs}/>", if ferme { " Z" } else { "" }, hex(if ferme || d.fill.is_some() { d.fill } else { None })))
}

/// Dimensions (largeur, hauteur) d'une face repérée, pour guider le placement des décors.
pub fn taille_de_face(m: &Modele, id: u32, cara: u8) -> Option<(f64, f64)> {
    let ip = m.pieces.iter().position(|p| p.id == id)?;
    m.cadres.get(&(ip, cara)).map(|c| (c.larg, c.haut))
}

pub fn svg(m: &Modele, cam: &Cam) -> String { svg_avec(m, cam, &Lumiere::default()) }

pub fn svg_avec(m: &Modele, cam: &Cam, lum: &Lumiere) -> String {
    let lumiere = lum.dir;
    // quelles faces d'origine regardent la caméra
    let vis: Vec<bool> = m.faces_origine.iter().map(|f| dot(f.n, sub(cam.center, f.p0)) > 0.0).collect();
    let mut nb_clips = 0usize;
    let mut out = String::from("<svg id=\"mod\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 736 736\" role=\"img\" aria-label=\"Modèle 3D\">");
    for f in ordre_de_dessin(m, cam) {
        let piece = &m.pieces[f.piece];
        let fill = teinte(piece.rgb, dot(f.n, lumiere), lum.ambiant);
        let pr: Vec<V3> = f.pts.iter().map(|v| cam.project(*v)).collect();
        let d = pr.iter().map(|q| format!("{} {}", nombre(q[0]), nombre(q[1]))).collect::<Vec<_>>().join(" L");
        out += &format!("<path class=\"m\" data-p=\"{}\" d=\"M{d} Z\" style=\"fill:{fill};stroke:{fill}\"/>", piece.id);
        // arêtes d'origine de ce fragment : contours, arêtes vives, ou toutes selon le réglage de la pièce
        let mut seg = String::new();
        for i in 0..f.pts.len() {
            let Some(e) = f.ef[i] else { continue };
            let voisines: Vec<usize> = m.aretes[e].faces.iter().copied().filter(|&g| vis[g]).collect();
            let contour = voisines.len() == 1;
            let vive = voisines.len() >= 2 && voisines.iter().any(|&g| voisines.iter().any(|&h| dot(m.faces_origine[g].n, m.faces_origine[h].n) < 0.82));
            let trace = match piece.aretes { 1 => true, 2 => contour, _ => contour || vive };
            if trace {
                let (a, b) = (pr[i], pr[(i + 1) % pr.len()]);
                seg += &format!("M{} {} L{} {}", nombre(a[0]), nombre(a[1]), nombre(b[0]), nombre(b[1]));
            }
        }
        if !seg.is_empty() { out += &format!("<path class=\"e\" data-p=\"{}\" d=\"{seg}\"/>", piece.id); }
        // décors de cette face : découpés selon CE morceau visible, juste après lui ; ce qui est dessiné plus tard (plus proche) les recouvre
        if let (Some(c), true) = (f.cara, !piece.decors.is_empty()) {
            if let Some(cadre) = m.cadres.get(&(f.piece, c)) {
                let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
                for q in &pr { x0 = x0.min(q[0]); x1 = x1.max(q[0]); y0 = y0.min(q[1]); y1 = y1.max(q[1]); }
                let dedans: String = piece.decors.iter().filter(|d| d.cara == c).filter_map(|d| decor_svg(cam, cadre, d, [x0, y0, x1, y1])).collect();
                if !dedans.is_empty() {
                    nb_clips += 1;
                    // le découpage est agrandi d'un demi-pixel : deux morceaux voisins d'une même face se chevauchent, sans fente claire entre eux
                    let (mx, my) = (pr.iter().map(|q| q[0]).sum::<f64>() / pr.len() as f64, pr.iter().map(|q| q[1]).sum::<f64>() / pr.len() as f64);
                    let large = pr.iter().map(|q| {
                        let (dx, dy) = (q[0] - mx, q[1] - my);
                        let l = (dx * dx + dy * dy).sqrt().max(1e-9);
                        format!("{} {}", nombre(q[0] + dx / l * 0.5), nombre(q[1] + dy / l * 0.5))
                    }).collect::<Vec<_>>().join(" L");
                    out += &format!("<clipPath id=\"c{nb_clips}\"><path d=\"M{large} Z\"/></clipPath><g clip-path=\"url(#c{nb_clips})\" data-p=\"{}\">{dedans}</g>", piece.id);
                }
            }
        }
    }
    out + "</svg>"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boite(id: u32, pos: V3, rot: V3, dims: [f64; 3], r: f64, b: f64) -> Piece {
        Piece { id, genre: 0, pos, rot, p: [dims[0], dims[1], dims[2], r, b, 4.0], rgb: 0x808890, aretes: 0, pts: vec![], decors: vec![] }
    }
    fn cylindre(id: u32, pos: V3, rot: V3, r: f64, l: f64, axe: f64, b: f64) -> Piece {
        Piece { id, genre: 1, pos, rot, p: [r, l, axe, b, 16.0, 0.0], rgb: 0x8890a0, aretes: 0, pts: vec![], decors: vec![] }
    }
    fn prisme(id: u32, pos: V3, d: f64, pts: Vec<[f64; 2]>) -> Piece {
        Piece { id, genre: 2, pos, rot: [0.0; 3], p: [d, 0.0, 0.0, 0.0, 0.0, 0.0], rgb: 0xa09088, aretes: 0, pts, decors: vec![] }
    }
    /// Scène avec des pièces qui se traversent : boîte, cylindre enfoncé dedans, prisme planté, boîte tournée.
    fn scene() -> Vec<Piece> {
        vec![
            boite(1, [0.0; 3], [0.0; 3], [200.0, 120.0, 80.0], 12.0, 6.0),
            cylindre(2, [60.0, 0.0, 40.0], [0.0; 3], 25.0, 90.0, 2.0, 3.0),
            prisme(3, [-70.0, -60.0, 0.0], 40.0, vec![[-20.0, 0.0], [20.0, 0.0], [0.0, 50.0]]),
            boite(4, [10.0, 20.0, 0.0], [20.0, 35.0, 15.0], [60.0, 60.0, 60.0], 0.0, 0.0),
        ]
    }

    fn aire(pts: &[V3]) -> f64 { long(newell(pts)) / 2.0 }

    #[test]
    fn la_decoupe_conserve_la_surface_de_chaque_face() {
        let m = construire(&scene()).unwrap();
        let mut par_src: HashMap<usize, f64> = HashMap::new();
        fn collecter(n: &Noeud, acc: &mut HashMap<usize, f64>) {
            for f in &n.coplanaires { *acc.entry(f.src).or_insert(0.0) += aire(&f.pts); }
            if let Some(x) = &n.avant { collecter(x, acc); }
            if let Some(x) = &n.arriere { collecter(x, acc); }
        }
        collecter(m.arbre.as_ref().unwrap(), &mut par_src);
        for (i, f) in m.faces_origine.iter().enumerate() {
            let (a0, a1) = (aire(&f.pts), par_src.get(&i).copied().unwrap_or(0.0));
            assert!((a0 - a1).abs() <= 1e-6 * a0.max(1.0), "face {i} : aire {a0} avant coupe, {a1} après");
        }
        assert!(m.nb_fragments > m.faces_origine.len(), "des pièces qui se traversent doivent produire des coupes");
    }

    fn direction_pixel(cam: &Cam, u: f64, v: f64) -> V3 {
        let d = [(u - 368.0) / cam.f, -(v - 368.0) / cam.f, 1.0];
        let r = &cam.r;
        let w = [r[0][0] * d[0] + r[1][0] * d[1] + r[2][0] * d[2], r[0][1] * d[0] + r[1][1] * d[1] + r[2][1] * d[2], r[0][2] * d[0] + r[1][2] * d[1] + r[2][2] * d[2]];
        [w[0], -w[1], -w[2]]
    }
    fn dans_polygone(poly: &[[f64; 2]], x: f64, y: f64) -> bool {
        let mut dedans = false;
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            let (pi, pj) = (poly[i], poly[j]);
            if (pi[1] > y) != (pj[1] > y) && x < (pj[0] - pi[0]) * (y - pi[1]) / (pj[1] - pi[1]) + pi[0] { dedans = !dedans; }
            j = i;
        }
        dedans
    }
    /// En chaque point d'écran, le fragment dessiné en dernier doit être le plus proche de la caméra.
    fn erreurs(m: &Modele, cam: &Cam, n: usize) -> (usize, String) {
        let ordre = ordre_de_dessin(m, cam);
        let ecrans: Vec<Vec<[f64; 2]>> = ordre.iter().map(|f| f.pts.iter().map(|q| { let u = cam.project(*q); [u[0], u[1]] }).collect()).collect();
        let (mut faux, mut ex) = (0, String::new());
        for a in 0..n {
            for b in 0..n {
                let (u, v) = (CADRE * (a as f64 + 0.37) / n as f64, CADRE * (b as f64 + 0.61) / n as f64);
                let dir = direction_pixel(cam, u, v);
                let (mut dernier, mut proche): (Option<usize>, Option<f64>) = (None, None);
                for (k, f) in ordre.iter().enumerate() {
                    if !dans_polygone(&ecrans[k], u, v) { continue; }
                    dernier = Some(k);
                    let nd = dot(f.n, dir);
                    if nd.abs() < 1e-12 { continue; }
                    let t = dot(f.n, sub(f.p0, cam.center)) / nd;
                    if t > 0.0 && proche.map_or(true, |p| t < p) { proche = Some(t); }
                }
                if let (Some(k), Some(tp)) = (dernier, proche) {
                    let f = ordre[k];
                    let t = dot(f.n, sub(f.p0, cam.center)) / dot(f.n, dir);
                    if t > tp + 1e-4 * tp.abs() + 1e-3 {
                        faux += 1;
                        if ex.is_empty() { ex = format!("point ({u:.0},{v:.0}) : pièce {} (t={t:.1}) dessinée par-dessus pièce plus proche (t={tp:.1})", m.pieces[f.piece].id); }
                    }
                }
            }
        }
        (faux, ex)
    }

    #[test]
    fn aucun_defaut_d_ordre_meme_avec_des_pieces_qui_se_traversent() {
        let m = construire(&scene()).unwrap();
        let mut echecs = Vec::new();
        for az in (-180..180).step_by(20) {
            for el in [-85, -60, -30, 0, 30, 60, 85] {
                for mm in [14.0, 35.0, 135.0] {
                    let cam = camera_du_modele(&m, az as f64, el as f64, 0.0, mm);
                    let (faux, ex) = erreurs(&m, &cam, 40);
                    if faux > 0 { echecs.push(format!("az {az} el {el} {mm} mm : {faux} points, {ex}")); }
                }
            }
        }
        assert!(echecs.is_empty(), "{} vues fausses, par ex. :\n{}", echecs.len(), echecs.iter().take(8).cloned().collect::<Vec<_>>().join("\n"));
    }

    /// Contrôle négatif : un tri par profondeur des centres (l'ancienne méthode) se trompe sur cette même scène.
    #[test]
    fn le_verificateur_voit_les_erreurs_d_un_tri_par_centres() {
        let m = construire(&scene()).unwrap();
        let mut total = 0;
        for az in (-180..180).step_by(30) {
            let cam = camera_du_modele(&m, az as f64, 25.0, 0.0, 35.0);
            let mut faux_ordre: Vec<&Face> = m.faces_origine.iter().filter(|f| dot(f.n, sub(cam.center, f.p0)) > 0.0).collect();
            faux_ordre.sort_by(|a, b| cam.project(moyenne(&b.pts))[2].partial_cmp(&cam.project(moyenne(&a.pts))[2]).unwrap());
            // même vérificateur, ordre faux
            let ecrans: Vec<Vec<[f64; 2]>> = faux_ordre.iter().map(|f| f.pts.iter().map(|q| { let u = cam.project(*q); [u[0], u[1]] }).collect()).collect();
            for a in 0..40 { for b in 0..40 {
                let (u, v) = (CADRE * (a as f64 + 0.37) / 40.0, CADRE * (b as f64 + 0.61) / 40.0);
                let dir = direction_pixel(&cam, u, v);
                let (mut dernier, mut proche) = (None, None::<f64>);
                for (k, f) in faux_ordre.iter().enumerate() {
                    if !dans_polygone(&ecrans[k], u, v) { continue; }
                    dernier = Some(k);
                    let t = dot(f.n, sub(f.p0, cam.center)) / dot(f.n, dir);
                    if t > 0.0 && proche.map_or(true, |p| t < p) { proche = Some(t); }
                }
                if let (Some(k), Some(tp)) = (dernier, proche) {
                    let f = faux_ordre[k];
                    let t = dot(f.n, sub(f.p0, cam.center)) / dot(f.n, dir);
                    if t > tp + 1e-4 * tp.abs() + 1e-3 { total += 1; }
                }
            } }
        }
        assert!(total > 0, "le vérificateur aurait dû voir les erreurs du tri par centres");
    }

    #[test]
    fn prisme_non_convexe_refuse_et_dimensions_invalides_refusees() {
        let creux = prisme(7, [0.0; 3], 10.0, vec![[0.0, 0.0], [10.0, 0.0], [5.0, 4.0], [10.0, 10.0], [0.0, 10.0]]);
        assert_eq!(construire(&[creux]).err(), Some(Erreur::NonConvexe(7)));
        let mauvais = boite(3, [0.0; 3], [0.0; 3], [0.0, 10.0, 10.0], 0.0, 0.0);
        assert_eq!(construire(&[mauvais]).err(), Some(Erreur::Dimensions(3)));
        assert_eq!(construire(&[]).err(), Some(Erreur::Vide));
    }

    #[test]
    fn une_boite_simple_a_six_faces_et_douze_aretes() {
        let m = construire(&[boite(1, [0.0; 3], [0.0; 3], [10.0, 20.0, 30.0], 0.0, 0.0)]).unwrap();
        assert_eq!(m.faces_origine.len(), 6);
        assert_eq!(m.aretes.len(), 12);
        assert_eq!(m.nb_fragments, 6, "une boîte seule ne doit pas être coupée");
        assert!(m.aretes.iter().all(|a| a.faces.len() == 2));
    }

    #[test]
    fn le_modele_se_dessine_et_reste_dans_le_cadre() {
        let m = construire(&scene()).unwrap();
        let cam = camera_du_modele(&m, 30.0, 20.0, 0.0, 35.0);
        let s = svg(&m, &cam);
        assert!(s.contains("data-p=\"1\"") && s.contains("data-p=\"2\"") && s.contains("data-p=\"3\"") && s.contains("data-p=\"4\""));
        for f in ordre_de_dessin(&m, &cam) { for q in &f.pts { let u = cam.project(*q); assert!(u[0] > 0.0 && u[0] < CADRE && u[1] > 0.0 && u[1] < CADRE, "sort du cadre : {u:?}"); } }
    }

    // ---------- Décors ----------
    fn texte_decor(id: u32, cara: u8, texte: &str) -> Decor {
        Decor { genre: 3, id, cara, x: 0.0, y: 0.0, rot: 0.0, p: [10.0, 400.0, 1.0, 0.0], fill: Some(0xffffff), trait_: None, ep: 0.0, pts: vec![], texte: texte.to_string() }
    }
    fn rectangle_decor(id: u32, cara: u8) -> Decor {
        Decor { genre: 0, id, cara, x: 5.0, y: -3.0, rot: 20.0, p: [20.0, 10.0, 3.0, 0.0], fill: Some(0x112233), trait_: Some(0xffffff), ep: 1.0, pts: vec![], texte: String::new() }
    }
    fn matrice_du_texte(svg: &str, id: u32) -> Option<[f64; 6]> {
        let debut = svg.find(&format!("<text data-d=\"{id}\" transform=\"matrix("))? + format!("<text data-d=\"{id}\" transform=\"matrix(").len();
        let fin = svg[debut..].find(')')?;
        let v: Vec<f64> = svg[debut..debut + fin].split(' ').filter_map(|t| t.parse().ok()).collect();
        if v.len() == 6 { Some([v[0], v[1], v[2], v[3], v[4], v[5]]) } else { None }
    }

    /// Pour toutes les faces de toutes les pièces (tournées) : repère orthonormé, normale sortante, lecture jamais en miroir.
    #[test]
    fn les_reperes_de_face_sont_orthonormes_et_jamais_en_miroir() {
        let pieces = vec![
            boite(1, [10.0, 5.0, -3.0], [20.0, 35.0, 15.0], [60.0, 40.0, 30.0], 6.0, 2.0),
            boite(2, [0.0; 3], [0.0; 3], [30.0, 30.0, 30.0], 0.0, 0.0),
            cylindre(3, [0.0; 3], [0.0; 3], 10.0, 40.0, 0.0, 1.0),
            cylindre(4, [0.0; 3], [10.0, 20.0, 30.0], 10.0, 40.0, 1.0, 0.0),
            cylindre(5, [0.0; 3], [0.0; 3], 10.0, 40.0, 2.0, 2.0),
            prisme(6, [0.0; 3], 20.0, vec![[0.0, 0.0], [20.0, 0.0], [10.0, 15.0]]),
        ];
        let m = construire(&pieces).unwrap();
        assert_eq!(m.cadres.len(), 6 + 6 + 3 * 2 + 2, "toutes les faces repérées doivent avoir un repère ({})", m.cadres.len());
        for ((ip, c), k) in &m.cadres {
            let (u, v, n) = (k.u, k.v, k.n);
            assert!((long(u) - 1.0).abs() < 1e-9 && (long(v) - 1.0).abs() < 1e-9 && (long(n) - 1.0).abs() < 1e-9, "pièce {ip} cara {c} : vecteurs non unitaires");
            assert!(dot(u, v).abs() < 1e-9 && dot(u, n).abs() < 1e-9 && dot(v, n).abs() < 1e-9, "pièce {ip} cara {c} : repère non perpendiculaire");
            let cp = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            assert!(dot(cp, n) > 0.99, "pièce {ip} cara {c} : lecture en miroir");
        }
    }

    /// Vu de l'extérieur, le texte posé sur chacune des 6 faces d'une boîte n'est jamais en miroir (déterminant > 0),
    /// et sur les 4 faces verticales il est à l'endroit (x vers la droite, y vers le bas).
    #[test]
    fn le_texte_se_lit_a_l_endroit_sur_les_six_faces() {
        let mut b = boite(1, [0.0; 3], [0.0; 3], [80.0, 60.0, 40.0], 0.0, 0.0);
        for c in 0..6u8 { b.decors.push(texte_decor(10 + c as u32, c, "Aa")); }
        let m = construire(&[b]).unwrap();
        for c in 0..6u8 {
            let k = m.cadres[&(0usize, c)];
            // caméra devant la face (légèrement décalée pour ne jamais regarder exactement le long de l'axe vertical)
            let eye = add(k.o, add(mulk(k.n, 300.0), mulk(k.u, 6.0)));
            let cam = Cam::look_at(eye, k.o, focale_px(35.0));
            let s = svg(&m, &cam);
            let t = matrice_du_texte(&s, 10 + c as u32).unwrap_or_else(|| panic!("face {c} : texte absent"));
            let det = t[0] * t[3] - t[1] * t[2];
            assert!(det > 0.0, "face {c} : texte en miroir (déterminant {det})");
            if c < 4 { assert!(t[0] > 0.0 && t[3] > 0.0 && t[1].abs() < 0.3 * t[0], "face {c} : texte pas à l'endroit {t:?}"); }
        }
    }

    #[test]
    fn le_texte_des_decors_ne_peut_pas_injecter_de_code() {
        let mut b = boite(1, [0.0; 3], [0.0; 3], [80.0, 60.0, 40.0], 0.0, 0.0);
        b.decors.push(texte_decor(5, 0, "<b>&\"'x</b><script>alert(1)</script>"));
        let m = construire(&[b]).unwrap();
        let cam = camera_du_modele(&m, 20.0, 10.0, 0.0, 35.0);
        let s = svg(&m, &cam);
        assert!(s.contains("&lt;b&gt;&amp;&quot;&#39;x&lt;/b&gt;&lt;script&gt;"), "texte non échappé : {}", &s[s.find("<text").unwrap_or(0)..].chars().take(300).collect::<String>());
        assert!(!s.contains("<script") && !s.contains("<b>"));
    }

    /// Les décors sont découpés selon les morceaux visibles de leur face et dessinés juste après eux ;
    /// une pièce posée devant est dessinée plus tard et recouvre donc le décor.
    #[test]
    fn un_decor_est_recouvert_par_une_piece_placee_devant() {
        let mut a = boite(1, [0.0; 3], [0.0; 3], [80.0, 60.0, 20.0], 0.0, 0.0);
        a.decors.push(rectangle_decor(7, 0));
        let b = boite(2, [0.0, 0.0, 14.0], [0.0; 3], [100.0, 80.0, 10.0], 0.0, 0.0); // couvre entièrement la face avant de A
        let m = construire(&[a, b]).unwrap();
        let cam = camera_du_modele(&m, 0.0, 0.0, 0.0, 35.0);
        let s = svg(&m, &cam);
        let pos_decor = s.find("data-d=\"7\"").expect("le décor est tout de même émis (il est recouvert par la suite)");
        let derniere_b = s.rfind("class=\"m\" data-p=\"2\"").unwrap();
        assert!(pos_decor < derniere_b, "la pièce de devant doit être dessinée après le décor");
        assert_eq!(s.matches("<clipPath").count(), s.matches("<g clip-path").count());
        assert!(s.matches("<clipPath").count() >= 1);
    }

    #[test]
    fn la_taille_d_une_face_est_connue() {
        let m = construire(&[boite(3, [0.0; 3], [0.0; 3], [80.0, 60.0, 20.0], 0.0, 0.0)]).unwrap();
        let (w, h) = taille_de_face(&m, 3, 0).unwrap();
        assert!((w - 80.0).abs() < 1e-9 && (h - 60.0).abs() < 1e-9);
        let (w, h) = taille_de_face(&m, 3, 2).unwrap();
        assert!((w - 20.0).abs() < 1e-9 && (h - 60.0).abs() < 1e-9, "face droite : profondeur × hauteur ({w}, {h})");
        assert!(taille_de_face(&m, 3, 9).is_none() && taille_de_face(&m, 99, 0).is_none());
    }

    #[test]
    fn la_grille_de_trous_est_un_seul_trace_et_suit_la_face() {
        let mut a = boite(1, [0.0; 3], [0.0; 3], [80.0, 60.0, 20.0], 0.0, 0.0);
        a.decors.push(Decor { genre: 4, id: 9, cara: 0, p: [1.0, 4.0, 5.0, 3.0], fill: Some(0x112233), ..Default::default() });
        let m = construire(&[a]).unwrap();
        let s = svg(&m, &camera_du_modele(&m, 20.0, -15.0, 0.0, 35.0));
        let tag = s.split("<path data-d=\"9\"").nth(1).expect("grille dessinée").split("/>").next().unwrap();
        assert_eq!(tag.matches('M').count(), 15, "5 colonnes × 3 lignes de trous");
        assert!(tag.contains("#112233"));
        // vue de dos : la face avant est cachée, la grille aussi
        let dos = svg(&m, &camera_du_modele(&m, 180.0, 0.0, 0.0, 35.0));
        assert!(!dos.contains("data-d=\"9\""));
    }

    #[test]
    fn l_eclairage_par_defaut_est_celui_d_origine_et_l_ambiant_eclaircit() {
        let m = construire(&[boite(1, [0.0; 3], [0.0; 3], [80.0, 60.0, 20.0], 0.0, 0.0)]).unwrap();
        let cam = camera_du_modele(&m, 30.0, -20.0, 0.0, 35.0);
        assert_eq!(svg(&m, &cam), svg_avec(&m, &cam, &Lumiere::default()));
        let clair = svg_avec(&m, &cam, &Lumiere { ambiant: 0.9, ..Lumiere::default() });
        assert_ne!(clair, svg(&m, &cam));
        let (a, b) = (teinte(0x808080, 0.0, 0.22), teinte(0x808080, 0.0, 0.9));
        assert!(b > a, "{a} {b}");
    }
}
