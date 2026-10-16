use std::collections::BTreeMap;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, epaint::CubicBezierShape,
};
use eegy::core::graph_engine::{GraphEngine, NodeId};
use eegy::core::processor::IOType;

/// ノードエディタの状態管理と描画を担当する構造体
/// （グラフのポートや接続情報は持たず、UI上の座標と選択状態のみを管理する）
pub struct NodeEditor {
    /// 各 NodeId のキャンバス上の左上座標（ローカル座標）
    node_positions: BTreeMap<NodeId, Pos2>,
    /// キャンバス全体のパン（スクロール）オフセット
    pan_offset: Vec2,
    /// 現在選択されているノードID
    selected_node: Option<NodeId>,
}

impl NodeEditor {
    const NODE_WIDTH: f32 = 170.0;
    const HEADER_HEIGHT: f32 = 28.0;
    const PORT_ROW_HEIGHT: f32 = 24.0;
    const NODE_PADDING_BOTTOM: f32 = 10.0;
    const PORT_RADIUS: f32 = 5.5;

    pub fn new() -> Self {
        Self {
            node_positions: BTreeMap::new(),
            pan_offset: Vec2::ZERO,
            selected_node: None,
        }
    }

    /// ノードのキャンバス上の座標を登録する
    pub fn register_node(&mut self, node_id: NodeId, pos: Pos2) {
        self.node_positions.insert(node_id, pos);
    }

    pub fn selected_node(&self) -> Option<NodeId> {
        self.selected_node
    }

    /// ノードエディタ全体の描画
    pub fn show(&mut self, ui: &mut Ui, engine: &GraphEngine) {
        let (canvas_rect, canvas_response) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

        // 背景のドラッグ（または中ボタンドラッグ）でキャンバスをパン移動
        if canvas_response.dragged_by(egui::PointerButton::Primary)
            || canvas_response.dragged_by(egui::PointerButton::Middle)
        {
            self.pan_offset += canvas_response.drag_delta();
        }
        if canvas_response.clicked() {
            self.selected_node = None;
        }

        let painter = ui.painter_at(canvas_rect);

        // 1. 背景色とグリッド線の描画
        painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(24, 26, 31));
        self.draw_grid(&painter, canvas_rect);

        let canvas_origin = canvas_rect.min + self.pan_offset;

        // 2. 各ノードのドラッグ判定と矩形計算
        let node_ids: Vec<NodeId> = self.node_positions.keys().copied().collect();
        for &node_id in &node_ids {
            let Some(processor) = engine.get_processor(node_id) else {
                continue;
            };
            let num_ports = processor
                .get_input_ports()
                .len()
                .max(processor.get_output_ports().len())
                .max(1);
            let node_height =
                Self::HEADER_HEIGHT + (num_ports as f32) * Self::PORT_ROW_HEIGHT + Self::NODE_PADDING_BOTTOM;

            let local_pos = self.node_positions[&node_id];
            let screen_pos = canvas_origin + local_pos.to_vec2();
            let node_rect = Rect::from_min_size(screen_pos, Vec2::new(Self::NODE_WIDTH, node_height));

            let node_id_widget = ui.make_persistent_id(("eegy_node", node_id));
            let node_response = ui.interact(
                node_rect,
                node_id_widget,
                Sense::click_and_drag(),
            );

            if node_response.clicked() || node_response.drag_started() {
                self.selected_node = Some(node_id);
            }
            if node_response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = self.node_positions.get_mut(&node_id) {
                    *pos += node_response.drag_delta();
                }
            }
        }

        // 3. エッジ（ベジェ曲線）の描画（GraphEngine のエッジ情報を直接参照）
        for edge in engine.get_edges() {
            if let (Some(src_pos), Some(dst_pos)) = (
                self.port_screen_pos(canvas_origin, edge.src_node, edge.src_port, false),
                self.port_screen_pos(canvas_origin, edge.dst_node, edge.dst_port, true),
            ) {
                self.draw_bezier_edge(&painter, src_pos, dst_pos);
            }
        }

        // 4. ノード本体とポートピンの描画
        for &node_id in &node_ids {
            if let Some(processor) = engine.get_processor(node_id) {
                let local_pos = self.node_positions[&node_id];
                let screen_pos = canvas_origin + local_pos.to_vec2();
                self.draw_node(
                    &painter,
                    node_id,
                    screen_pos,
                    processor.get_type(),
                    processor.get_input_ports(),
                    processor.get_output_ports(),
                );
            }
        }
    }

    /// ポートの画面上の中心座標を計算する
    fn port_screen_pos(
        &self,
        canvas_origin: Pos2,
        node_id: NodeId,
        port_index: usize,
        is_input: bool,
    ) -> Option<Pos2> {
        let local_pos = self.node_positions.get(&node_id)?;
        let screen_pos = canvas_origin + local_pos.to_vec2();
        let x = if is_input {
            screen_pos.x
        } else {
            screen_pos.x + Self::NODE_WIDTH
        };
        let y = screen_pos.y
            + Self::HEADER_HEIGHT
            + (port_index as f32 + 0.5) * Self::PORT_ROW_HEIGHT;
        Some(Pos2::new(x, y))
    }

    /// 背景のグリッド線を描画
    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let grid_spacing = 32.0;
        let grid_color = Color32::from_rgb(36, 39, 46);
        let stroke = Stroke::new(1.0, grid_color);

        let offset_x = self.pan_offset.x.rem_euclid(grid_spacing);
        let offset_y = self.pan_offset.y.rem_euclid(grid_spacing);

        let mut x = rect.left() + offset_x;
        while x < rect.right() {
            painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], stroke);
            x += grid_spacing;
        }

        let mut y = rect.top() + offset_y;
        while y < rect.bottom() {
            painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], stroke);
            y += grid_spacing;
        }
    }

    /// 2点間を結ぶベジェ曲線を描画
    fn draw_bezier_edge(&self, painter: &egui::Painter, src: Pos2, dst: Pos2) {
        let ctrl_offset = ((dst.x - src.x).abs() * 0.5).max(50.0);
        let ctrl1 = Pos2::new(src.x + ctrl_offset, src.y);
        let ctrl2 = Pos2::new(dst.x - ctrl_offset, dst.y);

        let bezier = CubicBezierShape::from_points_stroke(
            [src, ctrl1, ctrl2, dst],
            false,
            Color32::TRANSPARENT,
            Stroke::new(2.5, Color32::from_rgb(100, 185, 255)),
        );
        painter.add(Shape::CubicBezier(bezier));
    }

    /// 1つのノードを描画
    fn draw_node(
        &self,
        painter: &egui::Painter,
        node_id: NodeId,
        screen_pos: Pos2,
        type_name: &str,
        inputs: &[eegy::core::processor::IOPort],
        outputs: &[eegy::core::processor::IOPort],
    ) {
        let num_ports = inputs.len().max(outputs.len()).max(1);
        let node_height =
            Self::HEADER_HEIGHT + (num_ports as f32) * Self::PORT_ROW_HEIGHT + Self::NODE_PADDING_BOTTOM;
        let node_rect = Rect::from_min_size(screen_pos, Vec2::new(Self::NODE_WIDTH, node_height));
        let header_rect =
            Rect::from_min_size(screen_pos, Vec2::new(Self::NODE_WIDTH, Self::HEADER_HEIGHT));

        let is_selected = self.selected_node == Some(node_id);

        // ノード全体の背景
        painter.rect_filled(
            node_rect,
            CornerRadius::same(6),
            Color32::from_rgb(43, 47, 56),
        );

        // ヘッダー背景（上角のみ丸める）
        painter.rect_filled(
            header_rect,
            CornerRadius {
                nw: 6,
                ne: 6,
                sw: 0,
                se: 0,
            },
            Color32::from_rgb(62, 95, 138),
        );

        // 枠線（選択時は明るくハイライト）
        let border_stroke = if is_selected {
            Stroke::new(2.0, Color32::from_rgb(255, 200, 80))
        } else {
            Stroke::new(1.0, Color32::from_rgb(80, 86, 98))
        };
        painter.rect_stroke(
            node_rect,
            CornerRadius::same(6),
            border_stroke,
            StrokeKind::Outside,
        );

        // ヘッダータイトル ("#1 Gain")
        painter.text(
            header_rect.left_center() + Vec2::new(10.0, 0.0),
            Align2::LEFT_CENTER,
            format!("#{node_id}  {type_name}"),
            FontId::proportional(14.0),
            Color32::from_rgb(240, 244, 250),
        );

        // 入力ポートの描画（左側）
        for (idx, port) in inputs.iter().enumerate() {
            let y = screen_pos.y + Self::HEADER_HEIGHT + (idx as f32 + 0.5) * Self::PORT_ROW_HEIGHT;
            let pin_pos = Pos2::new(screen_pos.x, y);
            let pin_color = Self::port_color(port.types.first());

            painter.circle_filled(pin_pos, Self::PORT_RADIUS, pin_color);
            painter.circle_stroke(
                pin_pos,
                Self::PORT_RADIUS,
                Stroke::new(1.2, Color32::from_rgb(25, 28, 34)),
            );

            painter.text(
                pin_pos + Vec2::new(10.0, 0.0),
                Align2::LEFT_CENTER,
                port.name,
                FontId::proportional(12.5),
                Color32::from_rgb(210, 215, 225),
            );
        }

        // 出力ポートの描画（右側）
        for (idx, port) in outputs.iter().enumerate() {
            let y = screen_pos.y + Self::HEADER_HEIGHT + (idx as f32 + 0.5) * Self::PORT_ROW_HEIGHT;
            let pin_pos = Pos2::new(screen_pos.x + Self::NODE_WIDTH, y);
            let pin_color = Self::port_color(port.types.first());

            painter.circle_filled(pin_pos, Self::PORT_RADIUS, pin_color);
            painter.circle_stroke(
                pin_pos,
                Self::PORT_RADIUS,
                Stroke::new(1.2, Color32::from_rgb(25, 28, 34)),
            );

            painter.text(
                pin_pos - Vec2::new(10.0, 0.0),
                Align2::RIGHT_CENTER,
                port.name,
                FontId::proportional(12.5),
                Color32::from_rgb(210, 215, 225),
            );
        }
    }

    /// IOType に応じたポートピンの色分け
    fn port_color(io_type: Option<&IOType>) -> Color32 {
        match io_type {
            Some(IOType::Time) => Color32::from_rgb(100, 210, 140),
            Some(IOType::Frequency) => Color32::from_rgb(240, 160, 80),
            Some(IOType::TimeFrequency) => Color32::from_rgb(200, 120, 240),
            _ => Color32::from_rgb(160, 165, 175),
        }
    }
}
