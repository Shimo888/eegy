mod inspector_view;
mod node_editor;

use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use eegy::core::graph_engine::GraphEngine;
use eegy::core::registry::Registry;
use eegy::processors::gain::GainProcessor;
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
}

impl EegyApp {
    fn new() -> Self {
        let mut registry = Registry::new();
        registry.register::<GainProcessor>("Gain");

        Self {
            engine: GraphEngine::new(),
            registry,
            node_editor: NodeEditor::new(),
            inspector_view: InspectorView::new(),
            dock_state: create_default_dock_state(),
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
