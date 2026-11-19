//! Perspective Studio : moteur. Aucune dépendance externe, aucun bloc `unsafe`.
//!
//! Liaison manuelle avec le navigateur : le JavaScript décrit le modèle par petits appels numériques, appelle
//! `modele_rendre(...)`, puis lit la longueur renvoyée en octets à partir de `out_ptr()` (texte SVG en UTF-8).

pub mod modele;
pub mod vue;

use std::sync::Mutex;

static SORTIE: Mutex<Vec<u8>> = Mutex::new(Vec::new());

// ---------- Éditeur de modèle ----------
// Le modèle est envoyé pièce par pièce, par de petits appels numériques (pas de mémoire partagée, pas de JSON dans le moteur).
struct EtatModele { pieces: Vec<modele::Piece>, cache: Option<modele::Modele>, erreur: (u32, u32) }
static LUMIERE: Mutex<modele::Lumiere> = Mutex::new(modele::Lumiere { dir: [-0.601690, -0.501408, 0.621746], ambiant: 0.22 });
static MODELE: Mutex<EtatModele> = Mutex::new(EtatModele { pieces: Vec::new(), cache: None, erreur: (0, 0) });

fn modele_verrou() -> std::sync::MutexGuard<'static, EtatModele> { MODELE.lock().unwrap_or_else(|e| e.into_inner()) }

/// Construit (ou réutilise) le modèle ; met à jour le code d'erreur.
fn preparer(e: &mut EtatModele) -> bool {
    if e.cache.is_some() { return true; }
    match modele::construire(&e.pieces) {
        Ok(m) => { e.cache = Some(m); e.erreur = (0, 0); true }
        Err(err) => { e.erreur = err.code(); false }
    }
}

/// Efface le modèle.
#[unsafe(no_mangle)]
pub extern "C" fn modele_vider() {
    let mut e = modele_verrou();
    e.pieces.clear();
    e.cache = None;
    e.erreur = (0, 0);
}

/// Ajoute une pièce. genre 0 boîte (a = largeur, b = hauteur, c = profondeur, d = rayon des coins, e = bevel, f = segments),
/// 1 cylindre (a = rayon, b = longueur, c = axe 0/1/2, d = bevel, e = segments), 2 prisme (a = profondeur ; points avec `modele_point`).
/// `rgb` : 0xRRGGBB ; `aretes` : 0 contours et arêtes vives, 1 toutes, 2 contours seulement. Renvoie le nombre de pièces.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn modele_piece(genre: u32, id: u32, px: f64, py: f64, pz: f64, rx: f64, ry: f64, rz: f64, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, rgb: u32, aretes: u32) -> u32 {
    let mut m = modele_verrou();
    if m.pieces.len() >= 400 { return m.pieces.len() as u32; }
    m.pieces.push(modele::Piece { id, genre, pos: [px, py, pz], rot: [rx, ry, rz], p: [a, b, c, d, e, f], rgb: rgb & 0xFF_FFFF, aretes: aretes.min(2), pts: Vec::new(), decors: Vec::new() });
    m.cache = None;
    m.pieces.len() as u32
}

/// Ajoute un point au contour de la dernière pièce (prisme). 64 points au plus.
#[unsafe(no_mangle)]
pub extern "C" fn modele_point(x: f64, y: f64) {
    let mut m = modele_verrou();
    if let Some(p) = m.pieces.last_mut() { if p.pts.len() < 64 { p.pts.push([x, y]); } }
    m.cache = None;
}

/// Ajoute un décor à la dernière pièce. genre 0 rectangle (a, b, c = largeur, hauteur, rayon des coins), 1 cercle (a = rayon),
/// 2 polygone ou ligne (points avec `modele_decor_point` ; c > 0,5 : fermé), 3 texte (a = taille, b = graisse, c = ancre 0/1/2,
/// d = police 0/1 ; caractères avec `modele_decor_char`). `cara` : face de la pièce. `fill` et `trait` : 0xRRGGBB, ou < 0 pour aucun.
/// Renvoie le nombre de décors de la pièce (60 au plus).
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn modele_decor(genre: u32, id: u32, cara: u32, x: f64, y: f64, rot: f64, a: f64, b: f64, c: f64, d: f64, fill: f64, trait_: f64, ep: f64) -> u32 {
    let mut m = modele_verrou();
    let Some(p) = m.pieces.last_mut() else { return 0 };
    if genre > 4 || cara > 5 || p.decors.len() >= 60 { return p.decors.len() as u32; }
    let col = |v: f64| if v.is_finite() && v >= 0.0 { Some((v as u32) & 0xFF_FFFF) } else { None };
    p.decors.push(modele::Decor { genre, id, cara: cara as u8, x, y, rot, p: [a, b, c, d], fill: col(fill), trait_: col(trait_), ep, pts: Vec::new(), texte: String::new() });
    let n = p.decors.len() as u32;
    m.cache = None;
    n
}
/// Ajoute un point au décor courant (polygone). 64 points au plus.
#[unsafe(no_mangle)]
pub extern "C" fn modele_decor_point(x: f64, y: f64) {
    let mut m = modele_verrou();
    if let Some(d) = m.pieces.last_mut().and_then(|p| p.decors.last_mut()) { if d.pts.len() < 64 && x.is_finite() && y.is_finite() { d.pts.push([x, y]); } }
    m.cache = None;
}
/// Ajoute un caractère (point de code Unicode) au texte du décor courant. 120 caractères au plus.
#[unsafe(no_mangle)]
pub extern "C" fn modele_decor_char(code: u32) {
    let mut m = modele_verrou();
    if let (Some(d), Some(c)) = (m.pieces.last_mut().and_then(|p| p.decors.last_mut()), char::from_u32(code)) { if d.texte.chars().count() < 120 { d.texte.push(c); } }
    m.cache = None;
}
/// Taille d'une face repérée : quoi = 0 largeur, 1 hauteur ; renvoie −1 si la pièce n'a pas cette face (ou si le modèle est invalide).
#[unsafe(no_mangle)]
pub extern "C" fn modele_face(id: u32, cara: u32, quoi: u32) -> f64 {
    let mut e = modele_verrou();
    if !preparer(&mut e) || cara > 5 { return -1.0; }
    e.cache.as_ref().and_then(|m| modele::taille_de_face(m, id, cara as u8)).map_or(-1.0, |(w, h)| if quoi == 0 { w } else { h })
}

/// Dessine le modèle (caméra en orbite autour de son centre). `dist` ≤ 0 : cadrage automatique. Renvoie la longueur du SVG, 0 si erreur
/// (voir `modele_erreur`).
#[unsafe(no_mangle)]
pub extern "C" fn modele_rendre(az: f64, el: f64, dist: f64, focale_mm: f64) -> usize {
    if ![az, el, dist, focale_mm].iter().all(|v| v.is_finite()) || !(8.0..=1000.0).contains(&focale_mm) { return 0; }
    let mut e = modele_verrou();
    if !preparer(&mut e) { return 0; }
    let Some(m) = e.cache.as_ref() else { return 0 };
    let cam = modele::camera_du_modele(m, az, el, dist, focale_mm);
    let lum = *LUMIERE.lock().unwrap_or_else(|x| x.into_inner());
    let texte = modele::svg_avec(m, &cam, &lum);
    drop(e);
    let mut sortie = SORTIE.lock().unwrap_or_else(|x| x.into_inner());
    *sortie = texte.into_bytes();
    sortie.len()
}

/// Éclairage du modèle : direction de la lumière (vers la source ; ne doit pas être nulle) et lumière ambiante entre 0 et 1.
/// Sans appel, l'éclairage reste celui d'origine. Renvoie 1 si accepté.
#[unsafe(no_mangle)]
pub extern "C" fn modele_lumiere(x: f64, y: f64, z: f64, ambiant: f64) -> u32 {
    let n = (x * x + y * y + z * z).sqrt();
    if ![x, y, z, ambiant].iter().all(|v| v.is_finite()) || n < 1e-9 { return 0; }
    *LUMIERE.lock().unwrap_or_else(|e| e.into_inner()) = modele::Lumiere { dir: [x / n, y / n, z / n], ambiant: ambiant.clamp(0.0, 1.0) };
    1
}

/// Dernière erreur : 0 aucune, 1 dimensions invalides, 2 contour non convexe, 3 modèle vide.
#[unsafe(no_mangle)]
pub extern "C" fn modele_erreur() -> u32 { modele_verrou().erreur.0 }
/// Identifiant de la pièce en cause.
#[unsafe(no_mangle)]
pub extern "C" fn modele_erreur_piece() -> u32 { modele_verrou().erreur.1 }

/// Rayon (unités du modèle) de la sphère qui contient le modèle ; 0 si le modèle est invalide.
#[unsafe(no_mangle)]
pub extern "C" fn modele_rayon() -> f64 {
    let mut e = modele_verrou();
    if !preparer(&mut e) { return 0.0; }
    e.cache.as_ref().map_or(0.0, |m| m.rayon)
}
/// Distance automatique pour une focale (mm).
#[unsafe(no_mangle)]
pub extern "C" fn modele_distance_auto(focale_mm: f64) -> f64 {
    let mut e = modele_verrou();
    if !focale_mm.is_finite() || !preparer(&mut e) { return 0.0; }
    e.cache.as_ref().map_or(0.0, |m| modele::distance_auto(m, focale_mm))
}
/// Statistiques : 0 = faces d'origine, 1 = fragments après découpe de l'arbre BSP.
#[unsafe(no_mangle)]
pub extern "C" fn modele_stats(quoi: u32) -> u32 {
    let mut e = modele_verrou();
    if !preparer(&mut e) { return 0; }
    e.cache.as_ref().map_or(0, |m| if quoi == 0 { m.faces_origine.len() as u32 } else { m.nb_fragments as u32 })
}

/// Adresse (dans la mémoire WebAssembly) du dernier SVG produit.
#[unsafe(no_mangle)]
pub extern "C" fn out_ptr() -> *const u8 {
    SORTIE.lock().unwrap_or_else(|e| e.into_inner()).as_ptr()
}

#[cfg(test)]
mod tests {
    use super::vue::{focale_px, CADRE};

    #[test]
    fn la_focale_en_mm_donne_un_champ_de_camera_reel() {
        // 50 mm sur capteur 36 mm de large : champ horizontal d'environ 39,6 degrés
        let champ = 2.0 * ((CADRE / 2.0) / focale_px(50.0)).atan().to_degrees();
        assert!((champ - 39.6).abs() < 0.1, "{champ}");
        let champ18 = 2.0 * ((CADRE / 2.0) / focale_px(18.0)).atan().to_degrees();
        assert!((champ18 - 90.0).abs() < 0.1, "{champ18}");
    }

    #[test]
    fn un_modele_vide_ou_invalide_donne_un_rendu_vide() {
        super::modele_vider();
        assert_eq!(super::modele_rendre(0.0, 0.0, 0.0, 50.0), 0, "modèle vide");
        assert_eq!(super::modele_rendre(f64::NAN, 0.0, 0.0, 50.0), 0);
        assert_eq!(super::modele_rendre(0.0, 0.0, 0.0, 2.0), 0, "focale hors limites");
    }
}
