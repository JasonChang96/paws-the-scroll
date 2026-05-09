//! Hand-drawn base portraits for each `CatType`. Embedded into the binary
//! via `include_bytes!` so they ship inside the bundle without needing a
//! resources entry in `tauri.conf.json`. These are the "before" image fed
//! to the `gpt-image-2` edits endpoint — every generated portrait is a
//! variation of one of these, which keeps the visual style consistent
//! across mood/tier/skill combinations without us having to re-describe
//! the style in the prompt every time.
//!
//! The image bytes are also exposed so the cat picker can render the same
//! base portrait in its card before the user has adopted anything.

use crate::model::CatType;

const MANGO_PNG: &[u8] = include_bytes!("../../assets/mango.png");
const PLUTO_PNG: &[u8] = include_bytes!("../../assets/pluto.png");
const BEAN_PNG: &[u8] = include_bytes!("../../assets/bean.png");

pub fn bytes_for(cat_type: CatType) -> &'static [u8] {
    match cat_type {
        CatType::OrangeFat => MANGO_PNG,
        CatType::Void => PLUTO_PNG,
        CatType::ScrunglyStreet => BEAN_PNG,
    }
}
