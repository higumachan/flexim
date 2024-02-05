use crate::pane::{into_pane_content, Pane, PaneContent};
use crate::{
    insert_root_tile, left_and_right_layout, left_and_right_layout_dummy, App, FlLayout, Managed,
};
use chrono::Local;
use egui::{CollapsingHeader, Id, ScrollArea, Ui};
use egui_tiles::Tile;
use flexim_data_type::FlDataReference;
use flexim_data_view::DataViewCreatable;
use flexim_data_visualize::data_visualizable::DataVisualizable;
use flexim_storage::{Bag, StorageQuery};
use itertools::Itertools;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

pub fn left_panel(app: &mut App, ui: &mut Ui, bag: &Bag) {
    puffin::profile_function!();
    data_bag_list_view(app, ui);
    ui.separator();
    data_list_view(app, ui);
    ui.separator();
    data_view_list_view(app, ui, bag);
    ui.separator();
    visualize_list_view(app, ui);
    ui.separator();
    layout_list_view(app, ui);
}

fn data_bag_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    ScrollArea::vertical()
        .id_source("data_bag_list")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data Bag");
            let bag_groups = app.storage.bag_groups().unwrap();
            for (bag_name, bag_group) in bag_groups {
                if bag_group.len() > 1 {
                    CollapsingHeader::new(&bag_name).show(ui, |ui| {
                        for bag in bag_group {
                            let bag = bag.read().unwrap();
                            left_and_right_layout(
                                ui,
                                app,
                                |_app, ui| {
                                    ui.label(
                                        &bag.created_at
                                            .with_timezone(&Local)
                                            .format("%Y-%m-%d %H:%M:%S")
                                            .to_string(),
                                    );
                                },
                                |app, ui| {
                                    if ui.button("+").clicked() {
                                        app.replace_bag_id = Some(bag.id);
                                    }
                                },
                            )
                        }
                    });
                } else {
                    let bag = bag_group.first().unwrap().read().unwrap();
                    left_and_right_layout(
                        ui,
                        app,
                        |_app, ui| {
                            ui.label(&bag.name);
                        },
                        |app, ui| {
                            if ui.button("+").clicked() {
                                app.replace_bag_id = Some(bag.id);
                            }
                        },
                    )
                }
            }
        });
}

fn data_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    ScrollArea::vertical()
        .id_source("data_list")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data");
            let bind = app.storage.get_bag(app.current_bag_id).unwrap();
            let bag = bind.read().unwrap();
            for (name, data_group) in &bag.data_groups() {
                if data_group.len() > 1 {
                    CollapsingHeader::new(name).show(ui, |ui| {
                        for &d in data_group {
                            data_list_content_view(
                                app,
                                ui,
                                format!("#{}", d.generation).as_str(),
                                format!("{} #{}", &d.name, d.generation).as_str(),
                                FlDataReference::from(d.clone()),
                            );
                        }
                    });
                } else {
                    let &d = data_group.first().unwrap();
                    data_list_content_view(
                        app,
                        ui,
                        &d.name,
                        format!("{} #{}", &d.name, d.generation).as_str(),
                        FlDataReference::from(d.clone()),
                    );
                }
            }
        });
}

#[allow(clippy::collapsible_if)]
fn data_list_content_view(
    app: &mut App,
    ui: &mut Ui,
    display_label: &str,
    title: &str,
    data_ref: FlDataReference,
) {
    let data = app
        .storage
        .get_bag(app.current_bag_id)
        .unwrap()
        .read()
        .unwrap()
        .data_by_reference(&data_ref)
        .unwrap();

    left_and_right_layout(
        ui,
        app,
        |_app, ui| {
            ui.label(display_label);
        },
        |app, ui| {
            if data.is_visualizable() || data.data_view_creatable() {
                if ui.button("+").clicked() {
                    let content = into_pane_content(data_ref).unwrap();
                    let _tile_id = insert_root_tile(&mut app.tree, title, content.clone());
                }
            }
        },
    )
}

fn data_view_list_view(app: &mut App, ui: &mut Ui, bag: &Bag) {
    let width = ui.available_width();
    let data_views = app
        .tree
        .tiles
        .iter()
        .filter_map(|(tile_id, tile)| match tile {
            Tile::Pane(Pane {
                content: PaneContent::DataView(v),
                name,
            }) => Some(Managed::new(*tile_id, name.clone(), v.clone())),
            _ => None,
        })
        .collect_vec();

    ScrollArea::vertical()
        .id_source("data_view_list")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data View");
            for m in &data_views {
                left_and_right_layout(
                    ui,
                    app,
                    |app, ui| {
                        CollapsingHeader::new(&m.name)
                            .id_source(m.tile_id)
                            .show(ui, |ui| {
                                for attr in m.data.visualizeable_attributes(bag) {
                                    left_and_right_layout_dummy(
                                        ui,
                                        app,
                                        |_app, ui| {
                                            ui.label(attr.to_string());
                                        },
                                        |app, ui| {
                                            if ui.button("+").clicked() {
                                                let render = m.data.create_visualize(attr.clone());
                                                insert_root_tile(
                                                    &mut app.tree,
                                                    format!("{} {}", m.name, attr).as_str(),
                                                    PaneContent::Visualize(render),
                                                );
                                            }
                                        },
                                    );
                                }
                            });
                    },
                    |app, ui| {
                        let tile_visible = app.tree.tiles.is_visible(m.tile_id);
                        if ui.button(if tile_visible { "👁" } else { "‿" }).clicked() {
                            app.tree.tiles.set_visible(m.tile_id, !tile_visible);
                        }
                        if ui.button("➖").clicked() {
                            app.removing_tiles.push(m.tile_id);
                        }
                    },
                );
            }
        });
}

fn visualize_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    let visualizes = app
        .tree
        .tiles
        .iter()
        .filter_map(|(tile_id, tile)| match tile {
            Tile::Pane(Pane {
                content: PaneContent::Visualize(v),
                name,
            }) => Some(Managed::new(*tile_id, name.clone(), v.clone())),
            _ => None,
        })
        .collect_vec();

    ScrollArea::vertical()
        .id_source("visualize list")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data Visualize");
            for m in visualizes {
                left_and_right_layout(
                    ui,
                    app,
                    |_app, ui| {
                        ui.label(m.name);
                    },
                    |app, ui| {
                        let tile_visible = app.tree.tiles.is_visible(m.tile_id);
                        if ui.button(if tile_visible { "👁" } else { "‿" }).clicked() {
                            app.tree.tiles.set_visible(m.tile_id, !tile_visible);
                        }
                        if ui.button("➖").clicked() {
                            app.removing_tiles.push(m.tile_id);
                        }
                    },
                );
            }
        });
}

fn layout_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    ScrollArea::vertical()
        .id_source("layout_list_view")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            left_and_right_layout(
                ui,
                &mut (),
                |_, ui| ui.label("Layout"),
                |_, ui| {
                    let button = ui.button("💾");
                    if button.clicked() {
                        ui.ctx().memory_mut(|mem| {
                            mem.data.insert_temp(Id::new("layout save dialog"), true);
                        })
                    }
                    button
                },
            );
            let mut remove_layout_id = None;
            for l in &app.layouts {
                left_and_right_layout(
                    ui,
                    &mut app.tree,
                    |_app, ui| {
                        ui.label(&l.name);
                    },
                    |tree, ui| {
                        if ui.button("📲").clicked() {
                            *tree = l.tree.clone();
                        }
                        if ui.button("➖").clicked() {
                            remove_layout_id.replace(l.id);
                        }
                    },
                );
            }
            if let Some(id) = remove_layout_id {
                app.layouts.retain(|l| l.id != id);
            }
        });
    let mut layout_save_dialog = ui
        .ctx()
        .memory(|mem| mem.data.get_temp(Id::new("layout save dialog")))
        .unwrap_or(false);
    let mut saved = false;
    egui::Window::new("Save Layout")
        .open(&mut layout_save_dialog)
        .show(ui.ctx(), |ui| {
            ui.label("保存するLayoutの名前を入力してください");
            let layout_name = ui.ctx().memory_mut(|mem| {
                mem.data
                    .get_temp_mut_or_insert_with(Id::new("layout save dialog layout name"), || {
                        Arc::new(Mutex::new("".to_string()))
                    })
                    .clone()
            });
            ui.text_edit_singleline(layout_name.lock().unwrap().deref_mut());
            let layout_name = layout_name.clone();
            if ui.button("Save").clicked() {
                saved = true;
                app.layouts.push(FlLayout::new(
                    layout_name.lock().unwrap().clone(),
                    app.tree.clone(),
                ))
            }
        });
    if saved {
        layout_save_dialog = false;
    }
    ui.ctx().memory_mut(|mem| {
        mem.data
            .insert_temp(Id::new("layout save dialog"), layout_save_dialog);
        if !layout_save_dialog {
            mem.data
                .remove_temp::<Arc<Mutex<String>>>(Id::new("layout save dialog layout name"));
        }
    });
}
