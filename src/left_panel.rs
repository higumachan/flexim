use crate::{insert_root_tile, App, Managed};
use chrono::Local;
use egui::menu::menu_image_button;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    global_dark_light_mode_switch, CollapsingHeader, Id, Image, ImageButton, Label, Response,
    ScrollArea, Sense, Ui, Vec2, Widget,
};
use egui_tiles::Tile;
use flexim_data_type::{FlDataReference, FlDataType};
use flexim_data_view::DataViewCreatable;
use flexim_data_visualize::data_visualizable::DataVisualizable;
use flexim_layout::check::check_applicable;
use flexim_layout::pane::{into_pane_content, Pane, PaneContent};
use flexim_layout::FlLayout;
use flexim_storage::{Bag, StorageQuery};
use flexim_utility::left_and_right_layout;
use itertools::Itertools;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn left_panel(app: &mut App, ui: &mut Ui, bag: &Bag) {
    puffin::profile_function!();
    menu_image_button(
        ui,
        ImageButton::new(
            Image::from_bytes("bytes://logo.png", include_bytes!("../assets/logo.png"))
                .max_size(Vec2::new(12.0, 12.0)),
        ),
        |ui| {
            let layout_file_path_id = Id::new("layout_file_path");
            let path = ui
                .ctx()
                .memory_mut(|mem| mem.data.get_persisted::<PathBuf>(layout_file_path_id));

            global_dark_light_mode_switch(ui);
            if ui.button("Save Layout").clicked() {
                let fd = rfd::FileDialog::new();
                let fd = if let Some(path) = path.as_ref() {
                    fd.set_directory(path)
                } else {
                    fd
                };

                if let Some(path) = fd.save_file() {
                    let mut buf_writer =
                        std::io::BufWriter::new(std::fs::File::create(path.clone()).unwrap());
                    serde_json::to_writer(&mut buf_writer, &app.layouts).unwrap();

                    ui.ctx().memory_mut(|mem| {
                        mem.data.insert_persisted(
                            layout_file_path_id,
                            path.parent().unwrap().to_path_buf(),
                        );
                    });
                }
            }
            if ui.button("Load Layout").clicked() {
                let fd = rfd::FileDialog::new();
                let fd = if let Some(path) = path.as_ref() {
                    fd.set_directory(path)
                } else {
                    fd
                };
                if let Some(path) = fd.pick_file() {
                    let buf_reader = std::io::BufReader::new(std::fs::File::open(&path).unwrap());
                    let add_layouts: Vec<FlLayout> = serde_json::from_reader(buf_reader).unwrap();

                    app.layouts.extend(add_layouts);
                    app.layouts = app
                        .layouts
                        .iter()
                        .unique_by(|l| &l.name)
                        .cloned()
                        .collect_vec();

                    ui.ctx().memory_mut(|mem| {
                        mem.data.insert_persisted(
                            layout_file_path_id,
                            path.parent().unwrap().to_owned(),
                        );
                    });
                }
            }
        },
    );
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
    default_scroll_area(ui, "data_bag_list").show(ui, |ui| {
        ui.set_width(width);
        left_and_right_layout(
            ui,
            app,
            |_app, ui| {
                ui.label("Data Bag");
            },
            |app, ui| {
                let bag_file_path_id = Id::new("bag_file_path");

                if ui.button("📲").clicked() {
                    let fd = rfd::FileDialog::new();
                    let file_path =
                        ui.memory_mut(|mem| mem.data.get_persisted::<PathBuf>(bag_file_path_id));
                    let fd = if let Some(path) = file_path.as_ref() {
                        fd.set_directory(path)
                    } else {
                        fd
                    };
                    if let Some(file_path) = fd.pick_file() {
                        let buf_reader =
                            std::io::BufReader::new(std::fs::File::open(&file_path).unwrap());
                        let bag: Bag = bincode::deserialize_from(buf_reader).unwrap();
                        if !app.storage.load_bag(bag) {
                            log::error!("bag already exists");
                        }

                        ui.ctx().memory_mut(|mem| {
                            mem.data.insert_persisted(
                                bag_file_path_id,
                                file_path.parent().unwrap().to_owned(),
                            );
                        });
                    }
                }
            },
        );
        let bag_groups = app.storage.bag_groups().unwrap();
        for (bag_name, bag_group) in bag_groups {
            if bag_group.len() > 1 {
                CollapsingHeader::new(bag_name)
                    .header_truncate(true)
                    .show(ui, |ui| {
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
                                    if ui.button("💾").clicked() {
                                        if let Some(file_path) = rfd::FileDialog::new().save_file()
                                        {
                                            let mut buf_writer = std::io::BufWriter::new(
                                                std::fs::File::create(file_path).unwrap(),
                                            );
                                            bincode::serialize_into(&mut buf_writer, bag.deref())
                                                .unwrap();
                                        }
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
                        Label::new(bag_name).truncate(true).ui(ui);
                    },
                    |app, ui| {
                        if ui.button("+").clicked() {
                            app.replace_bag_id = Some(bag.id);
                        }
                        if ui.button("💾").clicked() {
                            if let Some(file_path) = rfd::FileDialog::new().save_file() {
                                let mut buf_writer = std::io::BufWriter::new(
                                    std::fs::File::create(file_path).unwrap(),
                                );
                                bincode::serialize_into(&mut buf_writer, bag.deref()).unwrap();
                            }
                        }
                    },
                )
            }
        }
    });
}

fn data_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    default_scroll_area(ui, "data_list").show(ui, |ui| {
        ui.set_width(width);
        ui.label("Data");
        let bind = app.storage.get_bag(app.current_bag_id).unwrap();
        let bag = bind.read().unwrap();
        for (name, data_group) in &bag.data_groups() {
            let icon = data_type_to_icon(data_group.first().unwrap().data.data_type());

            if data_group.len() > 1 {
                CollapsingHeader::new(format!("{} {}", icon, name))
                    .header_truncate(true)
                    .show(ui, |ui| {
                        for &d in data_group {
                            data_list_content_view(
                                app,
                                ui,
                                format!("#{}", d.generation).as_str(),
                                format!("{} {} #{}", icon, &d.name, d.generation).as_str(),
                                FlDataReference::from(d.clone()),
                            );
                        }
                    });
            } else {
                let &d = data_group.first().unwrap();
                data_list_content_view(
                    app,
                    ui,
                    format!("{} {}", icon, &d.name).as_str(),
                    format!("{} {} #{}", icon, &d.name, d.generation).as_str(),
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
            list_item_label(ui, display_label);
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

    default_scroll_area(ui, "data_view_list").show(ui, |ui| {
        ui.set_width(width);
        ui.label("Data View");

        for m in &data_views {
            let parent_width = ui.available_width();
            left_and_right_layout(
                ui,
                app,
                |app, ui| {
                    CollapsingHeader::new(&m.name)
                        .header_truncate(true)
                        .show(ui, |ui| {
                            ui.set_width(parent_width - ui.spacing().indent);
                            for attr in m.data.visualizeable_attributes(bag) {
                                left_and_right_layout(
                                    ui,
                                    app,
                                    |_app, ui| list_item_label(ui, attr.as_str()),
                                    |app, ui| {
                                        if ui.button("+").clicked() {
                                            let render = m.data.create_visualize(attr.clone());
                                            insert_root_tile(
                                                &mut app.tree,
                                                format!("{} {}", attr, m.name).as_str(),
                                                PaneContent::Visualize(render),
                                            );
                                        }
                                    },
                                );
                            }
                        })
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

    default_scroll_area(ui, "visualize list").show(ui, |ui| {
        ui.set_width(width);
        ui.label("Data Visualize");
        for m in visualizes {
            left_and_right_layout(
                ui,
                app,
                |_app, ui| {
                    list_item_label(ui, &m.name);
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
    default_scroll_area(ui, "layout_list_view").show(ui, |ui| {
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
                    list_item_label(ui, &l.name);
                },
                |tree, ui| {
                    if check_applicable(
                        &app.storage
                            .get_bag(app.current_bag_id)
                            .unwrap()
                            .read()
                            .unwrap(),
                        l,
                    ) {
                        if ui.button("📲").clicked() {
                            *tree = l.tree.clone();
                        }
                    } else {
                        ui.button("🚫").on_hover_text("Not applicable");
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

fn list_item_label(ui: &mut Ui, name: &str) -> Response {
    Label::new(name).truncate(true).sense(Sense::click()).ui(ui)
}

fn data_type_to_icon(data_type: FlDataType) -> &'static str {
    match data_type {
        FlDataType::Image => "🖼",
        FlDataType::DataFrame => "📊",
        FlDataType::Tensor => "🔢",
    }
}

fn default_scroll_area(ui: &mut Ui, id: &str) -> ScrollArea {
    ScrollArea::vertical()
        .id_source(id)
        .max_height(ui.available_height() / 5.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
}
