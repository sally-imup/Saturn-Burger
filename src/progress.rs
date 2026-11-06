//! Pesan progres dari proses install dan launch ke UI.

#[derive(Clone)]
pub struct Progress {
    pub text: String,
    /// 0.0 sampai 1.0, atau None jika tidak diketahui (cukup tampilkan teks).
    pub fraction: Option<f32>,
}

impl Progress {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            text: s.into(),
            fraction: None,
        }
    }

    pub fn at(s: impl Into<String>, fraction: f32) -> Self {
        Self {
            text: s.into(),
            fraction: Some(fraction.clamp(0.0, 1.0)),
        }
    }
}
