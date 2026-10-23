mod inspector_view;
mod node_editor;
mod project_io;

use std::path::PathBuf;
use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use eegy::core::graph_engine::GraphEngine;
use eegy::core::registry::Registry;
use inspector_view::InspectorView;
use node_editor::NodeEditor;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Tab {
    NodeEditor,
    Inspector,
}

fn create_default_dock_state() -> DockState<Tab> {
    let mut state = DockState::new(vec![Tab::NodeEditor]);
    let [_, _] = state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.75,
        vec![Tab::Inspector],
    );
    state
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Eegy Node Editor",
        options,
        Box::new(|_cc| Ok(Box::new(EegyApp::new()))),
    )
}

struct EegyApp {
    engine: GraphEngine,
    registry: Registry,
    node_editor: NodeEditor,
    inspector_view: InspectorView,
    dock_state: DockState<Tab>,
    current_project_path: Option<PathBuf>,
    status_message: Option<String>,
}

impl EegyApp {
    fn new() -> Self {
        Self {
            engine: GraphEngine::new(),
            registry: Registry::new(),
            node_editor: NodeEditor::new(),
            inspector_view: InspectorView::new(),
            dock_state: create_default_dock_state(),
            current_project_path: None,
            status_message: None,
        }
    }

    fn new_project(&mut self) {
        self.engine = GraphEngine::new();
        self.node_editor.clear();
        self.inspector_view.invalidate_cache();
        self.current_project_path = None;
        self.status_message = Some("Created new project.".to_string());
    }

    fn open_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Eegy Project (*.json)", &["json"])
            .pick_file()
        {
            match project_io::load_project_from_path(&path, &self.registry, &mut self.node_editor) {
                Ok(loaded_engine) => {
                    self.engine = loaded_engine;
                    self.inspector_view.invalidate_cache();
                    self.status_message = Some(format!("Loaded: {}", path.display()));
                    self.current_project_path = Some(path);
                }
                Err(err) => {
                    self.status_message = Some(format!("Load error: {err}"));
                }
            }
        }
    }

    fn save_project(&mut self) {
        if let Some(path) = self.current_project_path.clone() {
            self.save_to_path(path);
        } else {
            self.save_project_as_dialog();
        }
    }

    fn save_project_as_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Eegy Project (*.json)", &["json"])
            .set_file_name("project.json")
            .save_file()
        {
            self.save_to_path(path);
        }
    }

    fn save_to_path(&mut self, path: PathBuf) {
        match project_io::save_project_to_path(&path, &self.engine, &self.node_editor) {
            Ok(()) => {
                self.status_message = Some(format!("Saved: {}", path.display()));
                self.current_project_path = Some(path);
            }
            Err(err) => {
                self.status_message = Some(format!("Save error: {err}"));
            }
        }
    }
}

struct EegyTabViewer<'a> {
    engine: &'a mut GraphEngine,
    registry: &'a Registry,
    node_editor: &'a mut NodeEditor,
    inspector_view: &'a mut InspectorView,
}

impl TabViewer for EegyTabViewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(format!("{tab:?}"))
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::NodeEditor => "Node Editor".into(),
            Tab::Inspector => "Inspector".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::NodeEditor => {
                self.node_editor.show(ui, self.engine, self.registry);
            }
            Tab::Inspector => {
                let selected_node = self.node_editor.selected_node();
                self.inspector_view.show(ui, self.engine, selected_node);
            }
        }
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        true
    }
}

impl eframe::App for EegyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 1. 上部メニューバー / ステータスバー
        egui::Panel::top("eegy_top_bar")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(32, 35, 42))
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Eegy");
                    ui.separator();

                    ui.menu_button("File", |ui| {
                        if ui.button("New Project").clicked() {
                            self.new_project();
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Open Project...").clicked() {
                            self.open_project_dialog();
                            ui.close();
                        }
                        if ui.button("Save Project").clicked() {
                            self.save_project();
                            ui.close();
                        }
                        if ui.button("Save Project As...").clicked() {
                            self.save_project_as_dialog();
                            ui.close();
                        }
                    });

                    ui.menu_button("View", |ui| {
                        let mut has_node_editor = self.dock_state.find_tab(&Tab::NodeEditor).is_some();
                        if ui.checkbox(&mut has_node_editor, "Node Editor").clicked() {
                            if has_node_editor {
                                self.dock_state.push_to_first_leaf(Tab::NodeEditor);
                            } else if let Some(locator) = self.dock_state.find_tab(&Tab::NodeEditor) {
                                self.dock_state.remove_tab(locator);
                            }
                            ui.close();
                        }

                        let mut has_inspector = self.dock_state.find_tab(&Tab::Inspector).is_some();
                        if ui.checkbox(&mut has_inspector, "Inspector").clicked() {
                            if has_inspector {
                                self.dock_state.push_to_focused_leaf(Tab::Inspector);
                            } else if let Some(locator) = self.dock_state.find_tab(&Tab::Inspector) {
                                self.dock_state.remove_tab(locator);
                            }
                            ui.close();
                        }

                        ui.separator();
                        if ui.button("Reset Layout").clicked() {
                            self.dock_state = create_default_dock_state();
                            ui.close();
                        }
                    });

                    ui.separator();
                    if let Some(selected_id) = self.node_editor.selected_node() {
                        ui.label(format!(
                            "Selected Node: #{selected_id} (Delete/Backspace to delete)"
                        ));
                    } else if let Some(ref msg) = self.status_message {
                        ui.label(msg);
                    } else {
                        ui.label("Right-click canvas to add node | Drag ports to connect/disconnect | Drag tabs to dock/split");
                    }
                });
            });

        // 2. メインのドッキングエリア
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let mut tab_viewer = EegyTabViewer {
                    engine: &mut self.engine,
                    registry: &self.registry,
                    node_editor: &mut self.node_editor,
                    inspector_view: &mut self.inspector_view,
                };

                DockArea::new(&mut self.dock_state)
                    .show_close_buttons(true)
                    .show_inside(ui, &mut tab_viewer);
            });
    }
}
