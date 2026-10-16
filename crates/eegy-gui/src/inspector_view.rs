use eframe::egui::{self, Color32, RichText, Ui};
use eegy::core::graph_engine::{GraphEngine, NodeId};
use eegy::core::processor::{IOPort, IOType};
use serde_json::Value;

/// 選択中のノードの情報・パラメータ・ポートを表示・編集するビュー
pub struct InspectorView {
    cached_node_id: Option<NodeId>,
    cached_params: Option<Value>,
}

impl InspectorView {
    pub fn new() -> Self {
        Self {
            cached_node_id: None,
            cached_params: None,
        }
    }

    /// キャッシュを破棄して強制的に再読み込みさせる
    #[allow(dead_code)]
    pub fn invalidate_cache(&mut self) {
        self.cached_node_id = None;
        self.cached_params = None;
    }

    /// Inspector の中身を描画
    pub fn show(
        &mut self,
        ui: &mut Ui,
        engine: &mut GraphEngine,
        selected_node: Option<NodeId>,
    ) {
        let Some(node_id) = selected_node else {
            self.cached_node_id = None;
            self.cached_params = None;
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                ui.weak("Select a node to inspect and edit its parameters.");
            });
            return;
        };

        let Some(processor) = engine.get_processor(node_id) else {
            self.cached_node_id = None;
            self.cached_params = None;
            ui.weak("Selected node no longer exists.");
            return;
        };

        // 選択中ノードが切り替わった場合、またはキャッシュ未設定時のみシリアライズしてキャッシュ更新
        if self.cached_node_id != Some(node_id) || self.cached_params.is_none() {
            self.cached_node_id = Some(node_id);
            self.cached_params = serde_json::to_value(processor).ok();
        }

        let type_name = processor.get_type();
        let input_ports = processor.get_input_ports();
        let output_ports = processor.get_output_ports();

        let mut changed = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            // 2. ノード基本情報（node_id を含めたユニーク ID で衝突防止）
            egui::Grid::new(("inspector_basic_info", node_id))
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Node ID:");
                    ui.label(format!("#{node_id}"));
                    ui.end_row();

                    ui.strong("Type:");
                    ui.label(type_name);
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            // 3. パラメータ編集（キャッシュされた Value を編集）
            ui.strong("Parameters");
            ui.add_space(4.0);

            if let Some(ref mut json_val) = self.cached_params {
                match json_val {
                    Value::Object(map) if !map.is_empty() => {
                        egui::Grid::new(("inspector_params", node_id))
                            .num_columns(2)
                            .spacing([12.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (key, val) in map.iter_mut() {
                                    ui.label(key);
                                    if Self::draw_value_editor(ui, val) {
                                        changed = true;
                                    }
                                    ui.end_row();
                                }
                            });
                    }
                    _ => {
                        ui.weak("No configurable parameters.");
                    }
                }
            } else {
                ui.weak("Failed to read parameters.");
            }

            ui.add_space(10.0);
            ui.separator();

            // 4. 入出力ポート情報
            ui.strong("Input Ports");
            ui.add_space(4.0);
            Self::draw_ports_section(ui, input_ports);

            ui.add_space(8.0);
            ui.strong("Output Ports");
            ui.add_space(4.0);
            Self::draw_ports_section(ui, output_ports);
        });

        // パラメータが編集された時のみ GraphEngine 内の Processor に反映
        if changed {
            if let Some(ref new_val) = self.cached_params {
                if let Some(processor_mut) = engine.get_processor_mut(node_id) {
                    let _ = processor_mut.set_params(new_val.clone());
                }
            }
        }
    }

    /// serde_json::Value の型に応じた編集ウィジェットを描画し、変更があれば true を返す
    fn draw_value_editor(ui: &mut Ui, val: &mut Value) -> bool {
        match val {
            Value::Number(num) => {
                // 浮動小数点（f64）を最優先で判定し、整数解釈への吸い込まれを防ぐ
                if num.is_f64() {
                    if let Some(mut f) = num.as_f64() {
                        if ui
                            .add(egui::DragValue::new(&mut f).speed(0.05).max_decimals(4))
                            .changed()
                        {
                            if let Some(n) = serde_json::Number::from_f64(f) {
                                *val = Value::Number(n);
                                return true;
                            }
                        }
                    }
                } else if let Some(mut i) = num.as_i64() {
                    if ui.add(egui::DragValue::new(&mut i)).changed() {
                        *val = Value::Number(i.into());
                        return true;
                    }
                } else if let Some(mut u) = num.as_u64() {
                    if ui.add(egui::DragValue::new(&mut u)).changed() {
                        *val = Value::Number(u.into());
                        return true;
                    }
                }
                false
            }
            Value::Bool(b) => ui.checkbox(b, "").changed(),
            Value::String(s) => ui.text_edit_singleline(s).changed(),
            Value::Array(arr) => {
                let mut changed = false;
                ui.vertical(|ui| {
                    let mut remove_idx = None;
                    for (idx, item) in arr.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("[{idx}]"));
                            if Self::draw_value_editor(ui, item) {
                                changed = true;
                            }
                            if ui.small_button("✖").clicked() {
                                remove_idx = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = remove_idx {
                        arr.remove(idx);
                        changed = true;
                    }
                    if ui.small_button("+ Add Item").clicked() {
                        let default_item = arr
                            .last()
                            .map(|last| match last {
                                Value::Number(n) if n.is_f64() => {
                                    Value::Number(serde_json::Number::from_f64(0.0).unwrap())
                                }
                                Value::Number(_) => {
                                    // i64 または u64 の場合は整数の 0 を入れる
                                    Value::Number(0.into())
                                }
                                Value::Bool(_) => Value::Bool(false),
                                _ => Value::String(String::new()),
                            })
                            .unwrap_or_else(|| Value::String(String::new()));

                        arr.push(default_item);
                        changed = true;
                    }
                });
                changed
            }
            other => {
                ui.weak(other.to_string());
                false
            }
        }
    }

    /// 入出力ポート一覧を描画
    fn draw_ports_section(ui: &mut Ui, ports: &[IOPort]) {
        if ports.is_empty() {
            ui.weak("None");
            return;
        }

        for (idx, port) in ports.iter().enumerate() {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("[{idx}] {}", port.name));
                for io_type in port.types {
                    let (label, color) = Self::io_type_badge(io_type);
                    ui.label(RichText::new(label).color(color).small());
                }
                if port.optional {
                    ui.weak("(optional)");
                }
            });
        }
    }

    fn io_type_badge(io_type: &IOType) -> (&'static str, Color32) {
        match io_type {
            IOType::Time => ("Time", Color32::from_rgb(100, 210, 140)),
            IOType::Frequency => ("Freq", Color32::from_rgb(240, 160, 80)),
            IOType::TimeFrequency => ("TimeFreq", Color32::from_rgb(200, 120, 240)),
            IOType::None => ("None", Color32::from_rgb(160, 165, 175)),
        }
    }
}

