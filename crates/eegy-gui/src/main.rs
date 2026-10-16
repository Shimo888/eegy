mod node_editor;

use eframe::egui::{self, Pos2};
use eegy::core::graph_engine::GraphEngine;
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
    node_editor: NodeEditor,
}

impl EegyApp {
    fn new() -> Self {
        let mut engine = GraphEngine::new();
        let mut node_editor = NodeEditor::new();

        // 初期サンプルノードを 3 つ配置して接続
        let n1 = engine.add_node(Box::new(GainProcessor::new(2.0)));
        let n2 = engine.add_node(Box::new(GainProcessor::new(0.5)));
        let n3 = engine.add_node(Box::new(GainProcessor::new(1.5)));

        node_editor.register_node(n1, Pos2::new(80.0, 120.0));
        node_editor.register_node(n2, Pos2::new(340.0, 80.0));
        node_editor.register_node(n3, Pos2::new(340.0, 200.0));

        let _ = engine.connect(n1, 0, n2, 0);
        let _ = engine.connect(n1, 0, n3, 0);

        Self {
            engine,
            node_editor,
        }
    }
}

impl eframe::App for EegyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                // 上部ツールバー
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(32, 35, 42))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading("Eegy Node Editor");
                            ui.separator();

                            if ui.button("+ Add Gain Node").clicked() {
                                let node_id = self.engine.add_node(Box::new(GainProcessor::new(1.0)));
                                let offset = (node_id as f32 * 30.0) % 300.0;
                                self.node_editor.register_node(
                                    node_id,
                                    Pos2::new(150.0 + offset, 150.0 + offset * 0.6),
                                );
                            }

                            ui.separator();
                            if let Some(selected_id) = self.node_editor.selected_node() {
                                ui.label(format!("Selected Node: #{selected_id}"));
                            } else {
                                ui.label("Selected Node: None (Drag nodes to move, drag canvas to pan)");
                            }
                        });
                    });

                // メインのノードキャンバス領域
                self.node_editor.show(ui, &self.engine);
            });
    }
}
