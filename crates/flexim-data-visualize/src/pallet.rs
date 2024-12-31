use egui::epaint::ahash::AHasher;
use egui::Color32;
use scarlet::material_colors::{AccentTone, MaterialPrimary, MaterialTone};
use scarlet::prelude::RGBColor;
use std::hash::{Hash, Hasher};

pub fn pallet(id: impl Hash) -> Color32 {
    static COLORS: &[MaterialPrimary] = &[
        MaterialPrimary::Red(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Purple(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Indigo(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Blue(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::LightBlue(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Cyan(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Teal(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::LightGreen(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Yellow(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Orange(MaterialTone::Accent(AccentTone::A400)),
    ];

    let mut hasher = AHasher::default();
    id.hash(&mut hasher);
    let hashed = hasher.finish();
    let index = (hashed % COLORS.len() as u64) as usize;
    let color = RGBColor::from_material_palette(COLORS[index]);
    Color32::from_rgb(color.int_r(), color.int_g(), color.int_b())
}
