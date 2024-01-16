use egui::Color32;
use polars::export::ahash::AHasher;
use scarlet::material_colors::{AccentTone, MaterialPrimary, MaterialTone, NeutralTone};
use scarlet::prelude::RGBColor;
use std::hash::{Hash, Hasher};

pub fn pallet(id: impl Hash) -> Color32 {
    static COLORS: &'static [MaterialPrimary] = &[
        MaterialPrimary::Red(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Pink(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Purple(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::DeepPurple(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Indigo(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Blue(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::LightBlue(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Cyan(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Teal(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Green(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::LightGreen(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Lime(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Yellow(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Amber(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::Orange(MaterialTone::Accent(AccentTone::A400)),
        MaterialPrimary::DeepOrange(MaterialTone::Accent(AccentTone::A400)),
    ];

    let mut hasher = AHasher::default();
    id.hash(&mut hasher);
    let hashed = hasher.finish();
    let index = (hashed % COLORS.len() as u64) as usize;
    let color = RGBColor::from_material_palette(COLORS[index]);
    Color32::from_rgb(color.int_r(), color.int_g(), color.int_b())
}
