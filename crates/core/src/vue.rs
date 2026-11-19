//! Caméra perspective définie par sa focale, une rotation et une translation (point principal au centre du cadre de 736 px).
//! Repère de la scène : x vers la droite, y vers le bas, z vers le spectateur.

pub type V3 = [f64; 3];
type M3 = [[f64; 3]; 3];

const CX: f64 = 368.0;
const CY: f64 = 368.0;
/// Côté du cadre carré de l'image, en pixels (équivaut à 36 mm de capteur pour convertir une focale).
pub const CADRE: f64 = 736.0;
/// Focale en pixels pour une focale en mm (équivalent plein format 24×36, côté de 36 mm).
pub fn focale_px(mm: f64) -> f64 { mm * CADRE / 36.0 }

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn crs(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn unit(a: V3) -> V3 {
    let l = a[0].hypot(a[1]).hypot(a[2]);
    let l = if l == 0.0 { 1.0 } else { l };
    [a[0] / l, a[1] / l, a[2] / l]
}

pub struct Cam {
    pub f: f64,
    pub r: M3,
    pub t: V3,
    pub center: V3,
}

impl Cam {
    fn from_params(f: f64, r: M3, t: V3) -> Cam {
        let c: Vec<f64> = (0..3).map(|j| -(r[0][j] * t[0] + r[1][j] * t[1] + r[2][j] * t[2])).collect();
        Cam { f, r, t, center: [c[0], -c[1], -c[2]] }
    }

    /// Caméra « regarder vers » : position de l'œil et point visé, dans le repère de la scène.
    pub fn look_at(eye: V3, target: V3, f: f64) -> Cam {
        let e = [eye[0], -eye[1], -eye[2]];
        let g = [target[0], -target[1], -target[2]];
        let zc = unit(sub(g, e));
        let xc = unit(crs([0.0, 1.0, 0.0], zc));
        let yc = crs(zc, xc);
        let r = [xc, yc, zc];
        let t = [
            -(r[0][0] * e[0] + r[0][1] * e[1] + r[0][2] * e[2]),
            -(r[1][0] * e[0] + r[1][1] * e[1] + r[1][2] * e[2]),
            -(r[2][0] * e[0] + r[2][1] * e[1] + r[2][2] * e[2]),
        ];
        Cam::from_params(f, r, t)
    }

    /// Renvoie [u, v, profondeur].
    pub fn project(&self, p: V3) -> V3 {
        let (x, y, z) = (p[0], -p[1], -p[2]);
        let r = &self.r;
        let c0 = r[0][0] * x + r[0][1] * y + r[0][2] * z + self.t[0];
        let c1 = r[1][0] * x + r[1][1] * y + r[1][2] * z + self.t[1];
        let c2 = r[2][0] * x + r[2][1] * y + r[2][2] * z + self.t[2];
        [CX + self.f * c0 / c2, CY - self.f * c1 / c2, c2]
    }
}
