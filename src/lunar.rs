//! Moon-phase & low-precision lunar ephemeris (Schaefer 1985/1994)
//!
//! Ported to Rust for the Tide-Tracker project.
//! Accuracy: ±1 day for phase index; a few degrees for λ, β; ~6 % for Δ.
//! References: Sky & Telescope BASIC “MOONFX.BAS” (Apr 1994) and
//! original phase routine (Mar 1985).  See docs for citation list.

use core::f64::consts::PI;

/// Return type holding everything Schaefer’s 1994 routine can compute.
#[derive(Debug, Clone, Copy)]
pub struct LunarEphemeris {
    /// Phase index 0 – 7 (0 =new, 4 =full).
    pub phase_index: u8,
    /// Age of the Moon in civil days since New.
    pub age_days: f64,
    /// Illuminated fraction (0–1).
    pub illum_frac: f64,
    /// Geocentric distance in Earth radii.
    pub distance_er: f64,
    /// Ecliptic longitude (deg, low precision).
    pub lon_deg: f64,
    /// Ecliptic latitude (deg, low precision).
    pub lat_deg: f64,
}

/// Compute Schaefer’s phase & ephemeris for a proleptic-Gregorian Y-M-D.
///
/// `year` is astronomer’s year (e.g. 2000).  
/// `month` is 1 = Jan … 12 = Dec.  
/// `day` can be fractional (UTC noon = 0.5).
pub fn schaefer_moon(year: i32, month: u32, day: f64) -> LunarEphemeris {
    // ---------- 1. Calendar → “March-based” year to simplify JD math ----------
    let (mut y, mut m) = (year, month as i32);
    if m < 3 {
        y -= 1;
        m += 12;
    } // Jan/Feb treated as months 13/14
    m += 1; // shift so Mar = 1, Apr = 2 …

    // ---------- 2. Julian-day offset from 1900-01-00 12 UT new moon ----------
    // 694 039.09 d from JD 0 to that epoch  (S&T 1985)  ⟹  J-base constant
    let days = (365.25 * y as f64).floor() + (30.6 * m as f64).floor() + day - 694_039.09;

    // ---------- 3. Phase index (0–7)  ----------------------------------------
    // Divide by synodic month length; drop integer cycles; scale ×8 & round.
    let mut jd_norm = days / 29.530_588_2; // mean synodic month length
    jd_norm -= jd_norm.floor(); // keep fractional part only
    let phase_index = ((jd_norm * 8.0) + 0.5).floor() as u8 & 7;

    // Extra goodies: illuminated fraction & age
    let age_days = jd_norm * 29.530_588_2;
    let illum_frac = (1.0 - (age_days - 14.765_294_1).abs() / 14.765_294_1).clamp(0.0, 1.0); // simple cosine proxy

    //  ---------- 4. 1994 add-ons (four separate lunar “cycles”) --------------
    // All periods and epochs are straight from MOONFX.BAS.
    fn frac(mut v: f64) -> f64 {
        v -= v.floor();
        if v < 0.0 {
            v + 1.0
        } else {
            v
        }
    }

    // Anomalistic phase → Moon-perigee distance term
    let dp = frac((days + 245_1550.1 - 245_1562.2) / 27.554_549_88) * 2.0 * PI;
    let distance_er = 60.4
        - 3.3 * (dp).cos()
        - 0.6 * ((2.0 * jd_norm * 2.0 * PI) - dp).cos()
        - 0.5 * (2.0 * jd_norm * 2.0 * PI).cos();

    // Draconic (nodal) → ecliptic latitude
    let np = frac((days + 245_1550.1 - 245_1565.2) / 27.212_220_817) * 2.0 * PI;
    let lat_deg = 5.1 * np.sin();

    // Sidereal → ecliptic longitude
    let rp = frac((days + 245_1550.1 - 245_1555.8) / 27.321_582_241);
    let lon_deg = (360.0 * rp
        + 6.3 * (dp).sin()
        + 1.3 * ((2.0 * jd_norm * 2.0 * PI) - dp).sin()
        + 0.7 * (2.0 * jd_norm * 2.0 * PI).sin())
        % 360.0;

    LunarEphemeris {
        phase_index,
        age_days,
        illum_frac,
        distance_er,
        lon_deg,
        lat_deg,
    }
}
