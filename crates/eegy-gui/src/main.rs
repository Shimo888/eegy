mod node_editor;

use eframe::egui;
use eegy::core::graph_engine::GraphEngine;
use eegy::core::registry::Registry;
use eegy::processors::gain::GainProcessor;
use node_editor::NodeEditor;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 680.0]),
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
}

impl EegyApp {
    fn new() -> Self {
        let mut registry = Registry::new();
        registry.register::<GainProcessor>("Gain");

        Self {
            engine: GraphEngine::new(),
            registry,
            node_editor: NodeEditor::new(),
        }
    }
}

impl eframe::App for EegyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                // 上部ステータスバー
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(32, 35, 42))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading("Eegy Node Editor");
                            ui.separator();
                            if let Some(selected_id) = self.node_editor.selected_node() {
                                ui.label(format!(
                                    "Selected Node: #{selected_id} (Press Delete/Backspace or Right-click to delete)"
                                ));
                            } else {
                                ui.label("Right-click canvas to add node | Drag ports to connect/disconnect");
                            }
                        });
                    });

                // メインのノードキャンバス領域
                self.node_editor.show(ui, &mut self.engine, &self.registry);
            });
    }
}
