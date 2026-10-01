//! Primary Arms ACSS Raptor 5.56/.308 Yard G2 (PLxC 1-8x24 FFP RDB) subtensions.
//!
//! Extracted from the vector artwork in Primary Arms' reticle manual
//! (PLxC-1-8x24-FFP-RDB-RAPTOR-RETICLE-MANUAL_WEB.pdf, (c) 2025), page 9 "MILS".
//! In that drawing the 1-MIL ticks on the side ranging scale are 13.609 units apart
//! and the horizontal crosshair is centered at y = 130.833. The scale was cross-checked
//! three ways against design intent stated in the manual:
//!   - each vertical ranging bar is 5'10" at its labeled range (400 yd bar: 4.857 vs 4.861 MIL),
//!   - each BDC hash is 18" wide at its range (500/600/700/800: 1.000/0.830/0.713/0.626 MIL),
//!   - 5 and 10 mph wind dots scale exactly 1:2 on every row.
//!
//! The manual states the BDC is "primarily optimized around Mk262 (77gr) 5.56x45mm or
//! M80 (147gr) 7.62x51mm with a 50-yard zero", and that M193/M855 should use a 100-yard
//! zero with altitude-dependent offsets.

/// A BDC aiming point, MILs below the chevron tip.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    pub range_yd: f64,
    pub mil: f64,
    pub label: &'static str,
}

pub const BDC: &[Mark] = &[
    Mark {
        range_yd: 100.0,
        mil: 0.000,
        label: "chevron tip",
    },
    // Read from the leader line in the manual's "BDC AUTO RANGING" figure (schematic,
    // ~+/-0.04 MIL); there is no physical mark here, it is just under the chevron.
    Mark {
        range_yd: 200.0,
        mil: 0.34,
        label: "below chevron",
    },
    // Pointed top of the BDC stem: tip at 0.931, full width at 0.998.
    Mark {
        range_yd: 300.0,
        mil: 0.931,
        label: "top of stem",
    },
    Mark {
        range_yd: 350.0,
        mil: 1.375,
        label: "small hash",
    },
    Mark {
        range_yd: 400.0,
        mil: 1.739,
        label: "4 hash",
    },
    Mark {
        range_yd: 450.0,
        mil: 2.298,
        label: "small hash",
    },
    Mark {
        range_yd: 500.0,
        mil: 2.806,
        label: "5 hash",
    },
    Mark {
        range_yd: 550.0,
        mil: 3.416,
        label: "small hash",
    },
    Mark {
        range_yd: 600.0,
        mil: 4.119,
        label: "6 hash",
    },
    Mark {
        range_yd: 650.0,
        mil: 4.757,
        label: "small hash",
    },
    Mark {
        range_yd: 700.0,
        mil: 5.560,
        label: "7 hash",
    },
    Mark {
        range_yd: 750.0,
        mil: 6.388,
        label: "small hash",
    },
    Mark {
        range_yd: 800.0,
        mil: 7.360,
        label: "8 hash",
    },
];

/// Wind holds per BDC row: (range, 5 mph hold, 10 mph hold), MILs from the stem.
/// At 400 the 5 mph hold is the end of the hash itself (no separate dot).
#[allow(clippy::approx_constant)] // measured subtensions, not constants
pub const WIND: &[(f64, f64, f64)] = &[
    (400.0, 0.626, 1.250),
    (500.0, 0.814, 1.658),
    (600.0, 1.048, 2.094),
    (700.0, 1.309, 2.617),
    (800.0, 1.571, 3.140),
];

/// Moving-target lead dots on the horizontal crosshair: (target speed mph, MILs).
pub const LEADS: &[(f64, f64, &str)] = &[
    (3.0, 1.700, "walk"),
    (6.0, 3.300, "jog"),
    (9.0, 4.998, "sprint"),
];
