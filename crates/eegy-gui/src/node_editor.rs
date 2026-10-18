use std::collections::BTreeMap;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, epaint::CubicBezierShape,
};
use eegy::core::graph_engine::{GraphEngine, NodeId};
use eegy::core::processor::IOType;
use eegy::core::registry::Registry;

/// ドラッグ中のワイヤーの起点情報
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragSource {
    /// 出力ポートからドラッグ中（入力ポートへドロップして接続）
    FromOutput { src_node: NodeId, src_port: usize },
    /// 未接続の入力ポートからドラッグ中（出力ポートへドロップして接続）
    FromInput { dst_node: NodeId, dst_port: usize },
}

/// ノードエディタの状態管理と描画を担当する構造体
/// （グラフのポートや接続情報は持たず、UI上の座標と操作状態のみを管理する）
pub struct NodeEditor {
    /// 各 NodeId のキャンバス上の左上座標（ローカル座標）
    node_positions: BTreeMap<NodeId, Pos2>,
    /// キャンバス全体のパン（スクロール）オフセット
    pan_offset: Vec2,
    /// キャンバスのズーム倍率（0.2 ..= 3.0, デフォルト 1.0）
    zoom: f32,
    /// 現在選択されているノードID
    selected_node: Option<NodeId>,
    /// 現在マウスドラッグで配線中のワイヤー起点
    dragging_wire: Option<DragSource>,
    /// キャンバス右クリック時のローカル座標（ノード生成位置）
    context_menu_pos: Pos2,
}

impl NodeEditor {
    const NODE_WIDTH: f32 = 170.0;
    const HEADER_HEIGHT: f32 = 28.0;
    const PORT_ROW_HEIGHT: f32 = 24.0;
    const NODE_PADDING_BOTTOM: f32 = 10.0;
    const PORT_RADIUS: f32 = 5.5;
    const PORT_HIT_RADIUS: f32 = 10.0;

    pub fn new() -> Self {
        Self {
            node_positions: BTreeMap::new(),
            pan_offset: Vec2::ZERO,
            zoom: 1.0,
            selected_node: None,
            dragging_wire: None,
            context_menu_pos: Pos2::ZERO,
        }
    }

    /// ノードのキャンバス上の座標を登録する
    pub fn register_node(&mut self, node_id: NodeId, pos: Pos2) {
        self.node_positions.insert(node_id, pos);
    }

    /// 指定したアンカー座標を中心にズーム率を変更する
    pub fn zoom_around(&mut self, anchor: Pos2, new_zoom: f32, canvas_min: Pos2) {
        let old_zoom = self.zoom;
        if (new_zoom - old_zoom).abs() > 0.0001 {
            let local_vec = (anchor - canvas_min - self.pan_offset) / old_zoom;
            self.pan_offset = anchor - canvas_min - local_vec * new_zoom;
            self.zoom = new_zoom;
        }
    }

    /// ノードを GraphEngine と UI 座標マップの両方から削除する
    pub fn delete_node(&mut self, engine: &mut GraphEngine, node_id: NodeId) {
        let _ = engine.remove_node(node_id);
        self.node_positions.remove(&node_id);
        if self.selected_node == Some(node_id) {
            self.selected_node = None;
        }
        self.dragging_wire = None;
    }

    pub fn selected_node(&self) -> Option<NodeId> {
        self.selected_node
    }

    /// 全ノードの座標マップへの参照を取得（シリアライズ用）
    #[allow(dead_code)]
    pub fn node_positions(&self) -> &BTreeMap<NodeId, Pos2> {
        &self.node_positions
    }

    /// 座標マップを一括設定・クリアする（デシリアライズ・初期化用）
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.node_positions.clear();
        self.selected_node = None;
        self.dragging_wire = None;
    }

    #[allow(dead_code)]
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// ノードエディタ全体の描画と入力処理
    pub fn show(&mut self, ui: &mut Ui, engine: &mut GraphEngine, registry: &Registry) {
        let (canvas_rect, canvas_response) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

        let pointer_pos = ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.latest_pos()));

        // マウスホイールまたはトラックパッドピンチによるズーム処理
        if canvas_response.hovered() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            let mut zoom_factor = 1.0;

            if zoom_delta != 1.0 {
                zoom_factor *= zoom_delta;
            }
            if scroll_delta != 0.0 {
                zoom_factor *= (scroll_delta * 0.002).exp();
            }

            if (zoom_factor - 1.0).abs() > 0.0001 {
                let old_zoom = self.zoom;
                let new_zoom = (old_zoom * zoom_factor).clamp(0.2, 3.0);
                if (new_zoom - old_zoom).abs() > 0.0001 {
                    if let Some(mouse_pos) = pointer_pos {
                        self.zoom_around(mouse_pos, new_zoom, canvas_rect.min);
                    }
                }
            }
        }

        // ワイヤーをドラッグ中でなければ、背景ドラッグでキャンバスをパン移動
        if self.dragging_wire.is_none()
            && (canvas_response.dragged_by(egui::PointerButton::Primary)
                || canvas_response.dragged_by(egui::PointerButton::Middle))
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

        // キャンバス右クリック開始時のローカル座標を記録（ズームに影響されないローカル座標系）
        if canvas_response.secondary_clicked() {
            if let Some(pos) = pointer_pos {
                self.context_menu_pos = ((pos - canvas_origin) / self.zoom).to_pos2();
            }
        }

        // キャンバス右クリックメニュー：Registry に登録されているノードを作成
        canvas_response.context_menu(|ui| {
            ui.label("Add Node");
            ui.separator();
            for processor_type in registry.get_processor_types() {
                if ui.button(format!("+ {processor_type}")).clicked() {
                    if let Ok(processor) = registry.create_default(processor_type) {
                        let node_id = engine.add_node(processor);
                        self.register_node(node_id, self.context_menu_pos);
                        self.selected_node = Some(node_id);
                    }
                    ui.close();
                }
            }
        });

        // 2. 各ノードおよびポートのインタラクション判定
        // 選択中のノードを末尾（最前面）に回して優先度を高める
        let mut node_ids: Vec<NodeId> = self.node_positions.keys().copied().collect();
        if let Some(selected) = self.selected_node {
            if let Some(pos) = node_ids.iter().position(|&id| id == selected) {
                let id = node_ids.remove(pos);
                node_ids.push(id);
            }
        }
        let mut hovered_port: Option<(NodeId, usize, bool)> = None;
        let mut node_to_delete: Option<NodeId> = None;

        let port_hit_radius = (Self::PORT_HIT_RADIUS * self.zoom).max(6.0);

        for &node_id in &node_ids {
            let Some((num_inputs, num_outputs)) = engine
                .get_processor(node_id)
                .map(|p| (p.get_input_ports().len(), p.get_output_ports().len()))
            else {
                continue;
            };
            let num_ports = num_inputs.max(num_outputs).max(1);
            let Some(node_rect) = self.node_screen_rect(canvas_origin, node_id, num_ports) else {
                continue;
            };

            let node_id_widget = ui.make_persistent_id(("eegy_node", node_id));
            let node_response = ui.interact(
                node_rect,
                node_id_widget,
                Sense::click_and_drag(),
            );

            // 入力ポートのインタラクション（ノード矩形より後に登録して優先度を高くする）
            for port_idx in 0..num_inputs {
                if let Some(pin_pos) = self.port_screen_pos(canvas_origin, node_id, port_idx, true) {
                    let hit_rect = Rect::from_center_size(pin_pos, Vec2::splat(port_hit_radius * 2.0));
                    let port_id = ui.make_persistent_id(("eegy_in_port", node_id, port_idx));
                    let port_resp = ui.interact(hit_rect, port_id, Sense::click_and_drag());

                    if port_resp.hovered() || pointer_pos.is_some_and(|p| p.distance(pin_pos) <= port_hit_radius) {
                        hovered_port = Some((node_id, port_idx, true));
                    }

                    // 入力ポートからドラッグ開始：
                    // 既に接続がある場合はそのエッジを取り外し、元の出力ポートからのドラッグに移行する
                    if port_resp.drag_started_by(egui::PointerButton::Primary) {
                        let existing_edge = engine
                            .get_edges()
                            .find(|e| e.dst_node == node_id && e.dst_port == port_idx)
                            .cloned();

                        if let Some(edge) = existing_edge {
                            let _ = engine.disconnect(edge.id);
                            self.dragging_wire = Some(DragSource::FromOutput {
                                src_node: edge.src_node,
                                src_port: edge.src_port,
                            });
                        } else {
                            self.dragging_wire = Some(DragSource::FromInput {
                                dst_node: node_id,
                                dst_port: port_idx,
                            });
                        }
                    }

                    // 右クリックでも接続を切断可能にする
                    if port_resp.clicked_by(egui::PointerButton::Secondary) {
                        let edge_id = engine
                            .get_edges()
                            .find(|e| e.dst_node == node_id && e.dst_port == port_idx)
                            .map(|e| e.id);
                        if let Some(edge_id) = edge_id {
                            let _ = engine.disconnect(edge_id);
                        }
                    }
                }
            }

            // 出力ポートのインタラクション
            for port_idx in 0..num_outputs {
                if let Some(pin_pos) = self.port_screen_pos(canvas_origin, node_id, port_idx, false) {
                    let hit_rect = Rect::from_center_size(pin_pos, Vec2::splat(port_hit_radius * 2.0));
                    let port_id = ui.make_persistent_id(("eegy_out_port", node_id, port_idx));
                    let port_resp = ui.interact(hit_rect, port_id, Sense::click_and_drag());

                    if port_resp.hovered() || pointer_pos.is_some_and(|p| p.distance(pin_pos) <= port_hit_radius) {
                        hovered_port = Some((node_id, port_idx, false));
                    }

                    if port_resp.drag_started_by(egui::PointerButton::Primary) {
                        self.dragging_wire = Some(DragSource::FromOutput {
                            src_node: node_id,
                            src_port: port_idx,
                        });
                    }

                    // 右クリックでその出力ポートから出ている全エッジを切断
                    if port_resp.clicked_by(egui::PointerButton::Secondary) {
                        let edge_ids: Vec<_> = engine
                            .get_edges()
                            .filter(|e| e.src_node == node_id && e.src_port == port_idx)
                            .map(|e| e.id)
                            .collect();
                        for edge_id in edge_ids {
                            let _ = engine.disconnect(edge_id);
                        }
                    }
                }
            }

            // ノード本体の選択と移動（ワイヤードラッグ中は移動しない）
            if self.dragging_wire.is_none() {
                if node_response.clicked()
                    || node_response.secondary_clicked()
                    || node_response.drag_started()
                {
                    self.selected_node = Some(node_id);
                }
                if node_response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pos) = self.node_positions.get_mut(&node_id) {
                        *pos += node_response.drag_delta() / self.zoom;
                    }
                }
            }

            // ノード上の右クリックメニュー（ノード削除）
            node_response.context_menu(|ui| {
                if ui.button(format!("Delete Node #{node_id}")).clicked() {
                    node_to_delete = Some(node_id);
                    ui.close();
                }
            });
        }

        // Delete / Backspace キーによる選択中ノードの削除（テキスト入力中の暴発を防止）
        if !ui.ctx().egui_wants_keyboard_input()
            && ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
        {
            if let Some(selected_id) = self.selected_node {
                node_to_delete = Some(selected_id);
            }
        }

        if let Some(target_id) = node_to_delete {
            self.delete_node(engine, target_id);
        }

        // 3. 確定済みエッジ（ベジェ曲線）の描画（GraphEngine のエッジ情報を直接参照）
        for edge in engine.get_edges() {
            if let (Some(src_pos), Some(dst_pos)) = (
                self.port_screen_pos(canvas_origin, edge.src_node, edge.src_port, false),
                self.port_screen_pos(canvas_origin, edge.dst_node, edge.dst_port, true),
            ) {
                self.draw_bezier_edge(
                    &painter,
                    src_pos,
                    dst_pos,
                    Color32::from_rgb(100, 185, 255),
                );
            }
        }

        // 4. ドラッグ中の仮ワイヤー描画と、ドロップ時の接続確定処理
        if let Some(drag_source) = self.dragging_wire {
            if let Some(cursor_pos) = pointer_pos {
                match drag_source {
                    DragSource::FromOutput { src_node, src_port } => {
                        if let Some(src_pos) = self.port_screen_pos(canvas_origin, src_node, src_port, false) {
                            // ホバー中の入力ポートがあればそこにスナップ表示
                            let dst_pos = match hovered_port {
                                Some((n, p, true)) if n != src_node => {
                                    self.port_screen_pos(canvas_origin, n, p, true).unwrap_or(cursor_pos)
                                }
                                _ => cursor_pos,
                            };
                            self.draw_bezier_edge(
                                &painter,
                                src_pos,
                                dst_pos,
                                Color32::from_rgb(180, 225, 255),
                            );
                        }
                    }
                    DragSource::FromInput { dst_node, dst_port } => {
                        if let Some(dst_pos) = self.port_screen_pos(canvas_origin, dst_node, dst_port, true) {
                            let src_pos = match hovered_port {
                                Some((n, p, false)) if n != dst_node => {
                                    self.port_screen_pos(canvas_origin, n, p, false).unwrap_or(cursor_pos)
                                }
                                _ => cursor_pos,
                            };
                            self.draw_bezier_edge(
                                &painter,
                                src_pos,
                                dst_pos,
                                Color32::from_rgb(180, 225, 255),
                            );
                        }
                    }
                }
            }

            // マウスボタンが離されたらドロップ判定を行う
            if !ui.input(|i| i.pointer.primary_down()) {
                self.dragging_wire = None;
                if let Some((target_node, target_port, is_target_input)) = hovered_port {
                    match drag_source {
                        DragSource::FromOutput { src_node, src_port }
                            if is_target_input && target_node != src_node =>
                        {
                            self.connect_replacing_existing(
                                engine,
                                src_node,
                                src_port,
                                target_node,
                                target_port,
                            );
                        }
                        DragSource::FromInput { dst_node, dst_port }
                            if !is_target_input && target_node != dst_node =>
                        {
                            self.connect_replacing_existing(
                                engine,
                                target_node,
                                target_port,
                                dst_node,
                                dst_port,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        // 5. ノード本体とポートピンの描画
        // インタラクション中に選択ノードが変化した場合に備えて最新の選択順を反映
        if let Some(selected) = self.selected_node {
            if let Some(pos) = node_ids.iter().position(|&id| id == selected) {
                let id = node_ids.remove(pos);
                node_ids.push(id);
            }
        }
        for &node_id in &node_ids {
            if let Some(processor) = engine.get_processor(node_id) {
                self.draw_node(
                    &painter,
                    canvas_origin,
                    node_id,
                    processor.get_type(),
                    processor.get_input_ports(),
                    processor.get_output_ports(),
                    hovered_port,
                );
            }
        }

        // 6. ズームコントロールHUD（右下）
        let hud_pos = canvas_rect.right_bottom() - Vec2::new(175.0, 42.0);
        let hud_rect = Rect::from_min_size(hud_pos, Vec2::new(165.0, 32.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(hud_rect), |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgba_premultiplied(30, 33, 40, 220))
                .corner_radius(CornerRadius::same(6))
                .stroke(Stroke::new(1.0, Color32::from_rgb(60, 65, 75)))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button("−").on_hover_text("Zoom Out").clicked() {
                            self.zoom_around(canvas_rect.center(), (self.zoom * 0.85).clamp(0.2, 3.0), canvas_rect.min);
                        }
                        let zoom_percent = (self.zoom * 100.0).round() as i32;
                        if ui.small_button(format!("{zoom_percent}%")).on_hover_text("Reset Zoom to 100%").clicked() {
                            self.zoom_around(canvas_rect.center(), 1.0, canvas_rect.min);
                        }
                        if ui.small_button("+").on_hover_text("Zoom In").clicked() {
                            self.zoom_around(canvas_rect.center(), (self.zoom * 1.15).clamp(0.2, 3.0), canvas_rect.min);
                        }
                    });
                });
        });
    }

    /// 入力ポートに既存のエッジがあれば差し替えて接続する（失敗時は元の接続を復元）
    ///
    /// NOTE: 現在の実装は、安全のために一旦既存エッジを切断した上で接続を試み、
    /// 失敗した場合に元に戻すロールバック方式をとっています。
    /// 将来 GraphEngine 側に事前検証メソッド（例: `can_connect`）が追加された場合は、
    /// 切断前に検証を行うアトミックな差し替え方式への移行を推奨します。
    fn connect_replacing_existing(
        &self,
        engine: &mut GraphEngine,
        src_node: NodeId,
        src_port: usize,
        dst_node: NodeId,
        dst_port: usize,
    ) {
        let prev_edge = engine
            .get_edges()
            .find(|e| e.dst_node == dst_node && e.dst_port == dst_port)
            .cloned();

        // 既存接続がある場合は一旦切断して新規接続を試行
        if let Some(ref edge) = prev_edge {
            let _ = engine.disconnect(edge.id);
        }

        if engine.connect(src_node, src_port, dst_node, dst_port).is_err() {
            // 接続失敗時は元のエッジを再接続（ロールバック）
            if let Some(edge) = prev_edge {
                let _ = engine.connect(edge.src_node, edge.src_port, edge.dst_node, edge.dst_port);
            }
        }
    }

    /// ノードの画面上の矩形（Rect）を計算する
    fn node_screen_rect(
        &self,
        canvas_origin: Pos2,
        node_id: NodeId,
        num_ports: usize,
    ) -> Option<Rect> {
        let local_pos = self.node_positions.get(&node_id)?;
        let screen_pos = canvas_origin + local_pos.to_vec2() * self.zoom;
        let node_width = Self::NODE_WIDTH * self.zoom;
        let header_height = Self::HEADER_HEIGHT * self.zoom;
        let port_row_height = Self::PORT_ROW_HEIGHT * self.zoom;
        let node_padding_bottom = Self::NODE_PADDING_BOTTOM * self.zoom;
        let node_height =
            header_height + (num_ports.max(1) as f32) * port_row_height + node_padding_bottom;
        Some(Rect::from_min_size(screen_pos, Vec2::new(node_width, node_height)))
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
        let screen_pos = canvas_origin + local_pos.to_vec2() * self.zoom;
        let x = if is_input {
            screen_pos.x
        } else {
            screen_pos.x + Self::NODE_WIDTH * self.zoom
        };
        let y = screen_pos.y
            + (Self::HEADER_HEIGHT + (port_index as f32 + 0.5) * Self::PORT_ROW_HEIGHT) * self.zoom;
        Some(Pos2::new(x, y))
    }

    /// 背景のグリッド線を描画
    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let base_spacing = 32.0;
        let mut grid_spacing = base_spacing * self.zoom;
        while grid_spacing < 16.0 {
            grid_spacing *= 2.0;
        }
        while grid_spacing > 64.0 {
            grid_spacing /= 2.0;
        }
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
    fn draw_bezier_edge(&self, painter: &egui::Painter, src: Pos2, dst: Pos2, color: Color32) {
        let ctrl_offset = ((dst.x - src.x).abs() * 0.5).max(50.0 * self.zoom);
        let ctrl1 = Pos2::new(src.x + ctrl_offset, src.y);
        let ctrl2 = Pos2::new(dst.x - ctrl_offset, dst.y);

        let bezier = CubicBezierShape::from_points_stroke(
            [src, ctrl1, ctrl2, dst],
            false,
            Color32::TRANSPARENT,
            Stroke::new((2.5 * self.zoom).clamp(1.2, 5.0), color),
        );
        painter.add(Shape::CubicBezier(bezier));
    }

    /// 1つのノードを描画
    fn draw_node(
        &self,
        painter: &egui::Painter,
        canvas_origin: Pos2,
        node_id: NodeId,
        type_name: &str,
        inputs: &[eegy::core::processor::IOPort],
        outputs: &[eegy::core::processor::IOPort],
        hovered_port: Option<(NodeId, usize, bool)>,
    ) {
        let num_ports = inputs.len().max(outputs.len()).max(1);
        let Some(node_rect) = self.node_screen_rect(canvas_origin, node_id, num_ports) else {
            return;
        };
        let header_height = Self::HEADER_HEIGHT * self.zoom;
        let header_rect =
            Rect::from_min_size(node_rect.min, Vec2::new(node_rect.width(), header_height));

        let is_selected = self.selected_node == Some(node_id);
        let corner_r = (6.0 * self.zoom).clamp(2.0, 6.0) as u8;

        // ノード全体の背景
        painter.rect_filled(
            node_rect,
            CornerRadius::same(corner_r),
            Color32::from_rgb(43, 47, 56),
        );

        // ヘッダー背景（上角のみ丸める）
        painter.rect_filled(
            header_rect,
            CornerRadius {
                nw: corner_r,
                ne: corner_r,
                sw: 0,
                se: 0,
            },
            Color32::from_rgb(62, 95, 138),
        );

        // 枠線（選択時は明るくハイライト）
        let border_stroke = if is_selected {
            Stroke::new((2.0 * self.zoom).clamp(1.5, 3.5), Color32::from_rgb(255, 200, 80))
        } else {
            Stroke::new((1.0 * self.zoom).clamp(0.8, 2.0), Color32::from_rgb(80, 86, 98))
        };
        painter.rect_stroke(
            node_rect,
            CornerRadius::same(corner_r),
            border_stroke,
            StrokeKind::Outside,
        );

        // ヘッダータイトル ("#1 Gain")
        painter.text(
            header_rect.left_center() + Vec2::new(10.0 * self.zoom, 0.0),
            Align2::LEFT_CENTER,
            format!("#{node_id}  {type_name}"),
            FontId::proportional((14.0 * self.zoom).max(8.0)),
            Color32::from_rgb(240, 244, 250),
        );

        // 入力ポートの描画（左側）
        for (idx, port) in inputs.iter().enumerate() {
            if let Some(pin_pos) = self.port_screen_pos(canvas_origin, node_id, idx, true) {
                let is_hovered = hovered_port == Some((node_id, idx, true));
                self.draw_port(painter, pin_pos, port, true, is_hovered);
            }
        }

        // 出力ポートの描画（右側）
        for (idx, port) in outputs.iter().enumerate() {
            if let Some(pin_pos) = self.port_screen_pos(canvas_origin, node_id, idx, false) {
                let is_hovered = hovered_port == Some((node_id, idx, false));
                self.draw_port(painter, pin_pos, port, false, is_hovered);
            }
        }
    }

    /// 1つのポートピンとラベルを描画
    fn draw_port(
        &self,
        painter: &egui::Painter,
        pin_pos: Pos2,
        port: &eegy::core::processor::IOPort,
        is_input: bool,
        is_hovered: bool,
    ) {
        let pin_color = Self::port_color(port.types.first());
        let base_radius = (Self::PORT_RADIUS * self.zoom).max(2.5);
        let radius = if is_hovered {
            base_radius + 2.0 * self.zoom
        } else {
            base_radius
        };

        painter.circle_filled(pin_pos, radius, pin_color);
        painter.circle_stroke(
            pin_pos,
            radius,
            Stroke::new(
                if is_hovered { 2.0 * self.zoom } else { 1.2 * self.zoom },
                if is_hovered {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(25, 28, 34)
                },
            ),
        );

        let font_size = (12.5 * self.zoom).max(7.0);
        let (text_pos, align) = if is_input {
            (pin_pos + Vec2::new(10.0 * self.zoom, 0.0), Align2::LEFT_CENTER)
        } else {
            (pin_pos - Vec2::new(10.0 * self.zoom, 0.0), Align2::RIGHT_CENTER)
        };
        painter.text(
            text_pos,
            align,
            port.name,
            FontId::proportional(font_size),
            Color32::from_rgb(210, 215, 225),
        );
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
