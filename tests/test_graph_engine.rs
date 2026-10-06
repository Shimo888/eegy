use eegy::core::buffer::Buffer;
use eegy::core::graph_engine::GraphEngine;
use eegy::core::processor::{IOPort, IOType, Processor, ProcessorMeta};
use eegy::processors::gain::GainProcessor;

// 型不一致テスト用のダミープロセッサ（周波数領域専用）
struct FrequencyDummyProcessor;
impl ProcessorMeta for FrequencyDummyProcessor {
    fn get_type(&self) -> &'static str {
        "FrequencyDummy"
    }
    fn get_input_ports(&self) -> &'static [IOPort] {
        &[IOPort {
            name: "freq_in",
            optional: false,
            types: &[IOType::Frequency],
        }]
    }
    fn get_output_ports(&self) -> &'static [IOPort] {
        &[IOPort {
            name: "freq_out",
            optional: false,
            types: &[IOType::Frequency],
        }]
    }
}
impl Processor for FrequencyDummyProcessor {
    fn setup(&mut self, _inputs: &[&Buffer]) -> Result<(), String> {
        Ok(())
    }
    fn process(&mut self, _inputs: &[&Buffer]) -> Result<(), String> {
        Ok(())
    }
    fn get_outputs(&self, _port: usize) -> Option<&Buffer> {
        None
    }
}

// ==========================================
// 1. add_node / 基本テスト
// ==========================================

/// ノード追加の正常系テスト:
/// - ノードを追加すると NodeId が 1 からインクリメントされて採番されること
/// - 追加されたノードが nodes マップに正しく登録されていること
#[test]
fn test_add_node() {
    let mut engine = GraphEngine::new();
    let node_id = engine.add_node(Box::new(GainProcessor::new(1.0)));
    assert_eq!(node_id, 1);
    assert!(engine.nodes.get(&node_id).is_some());

    let node2_id = engine.add_node(Box::new(GainProcessor::new(2.0)));
    assert_eq!(node2_id, 2);
}

// ==========================================
// 2. remove_node テスト
// ==========================================

/// 孤立ノード削除テスト:
/// - エッジが繋がっていないノードを削除したとき、nodes から綺麗に消えること
#[test]
fn test_remove_node_isolated() {
    let mut engine = GraphEngine::new();
    let node_id = engine.add_node(Box::new(GainProcessor::new(1.0)));

    assert!(engine.remove_node(node_id).is_ok());
    // nodes から消えていること
    assert!(engine.nodes.get(&node_id).is_none());
}

/// カスケード切断（配線の道連れ削除）テスト:
/// - ノードを削除した際、そのノードに接続されていた全エッジが edges から削除されること
/// - 親ノードの out_edges から対象エッジが消えていること
/// - 子ノードの in_edges が None にリセットされていること
#[test]
fn test_remove_node_cascades_edge_deletion() {
    let mut engine = GraphEngine::new();
    let n_prev = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n_target = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n_next = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // n_prev -> n_target -> n_next
    let edge_in = engine.connect(n_prev, 0, n_target, 0).unwrap();
    let edge_out = engine.connect(n_target, 0, n_next, 0).unwrap();

    // ターゲットノードを削除
    assert!(engine.remove_node(n_target).is_ok());

    // 1. target ノード自身が消えていること
    assert!(engine.nodes.get(&n_target).is_none());

    // 2. target に繋がっていたエッジが両方とも edges から消えていること
    assert!(engine.edges.get(&edge_in).is_none());
    assert!(engine.edges.get(&edge_out).is_none());

    // 3. 親ノード (n_prev) の out_edges から edge_in が消えていること
    assert!(engine.nodes.get(&n_prev).unwrap().out_edges[0].is_empty());

    // 4. 子ノード (n_next) の in_edges が None に戻っていること
    assert_eq!(engine.nodes.get(&n_next).unwrap().in_edges[0], None);
}

/// 他ノードへの非干渉テスト:
/// - あるノードを削除しても、それとは無関係な別ノード間の接続状態は破壊されないこと
#[test]
fn test_remove_node_does_not_affect_other_nodes() {
    let mut engine = GraphEngine::new();
    let n_unrelated = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n2 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    let edge_id = engine.connect(n1, 0, n2, 0).unwrap();

    // 無関係なノードを削除
    assert!(engine.remove_node(n_unrelated).is_ok());

    // n1 -> n2 の接続は維持されていること
    assert!(engine.edges.get(&edge_id).is_some());
    assert_eq!(engine.nodes.get(&n1).unwrap().out_edges[0], vec![edge_id]);
    assert_eq!(engine.nodes.get(&n2).unwrap().in_edges[0], Some(edge_id));
}

/// 存在しないノード削除テスト:
/// - 存在しない NodeId を削除しようとしたら Err("Cannot find node") が返ること
#[test]
fn test_remove_node_invalid_id() {
    let mut engine = GraphEngine::new();

    // 存在しないノードの削除
    let res = engine.remove_node(999);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Cannot find node");
}

/// 二重削除防止テスト:
/// - 一度削除したノードを再度削除しようとしたら Err("Cannot find node") が返ること
#[test]
fn test_remove_node_double_remove() {
    let mut engine = GraphEngine::new();
    let node_id = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // 1回目は成功
    assert!(engine.remove_node(node_id).is_ok());

    // 2回目はエラー
    let res = engine.remove_node(node_id);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Cannot find node");
}

// ==========================================
// 3. connect テスト
// ==========================================

/// 接続の正常系と内部状態の反映テスト:
/// - ノード間を connect したとき Edge が生成されること
/// - src_node.out_edges に edge_id が追加されること
/// - dst_node.in_edges に Some(edge_id) が格納されること
#[test]
fn test_connect_success_and_state_verification() {
    let mut engine = GraphEngine::new();
    let node1 = engine.add_node(Box::new(GainProcessor::new(2.0)));
    let node2 = engine.add_node(Box::new(GainProcessor::new(0.5)));

    // 正常系: node1:0 -> node2:0
    let edge_id = engine.connect(node1, 0, node2, 0).expect("Connect should succeed");

    // Edge が登録されているか
    let edge = engine.edges.get(&edge_id).expect("Edge must exist in edges");
    assert_eq!(edge.src_node, node1);
    assert_eq!(edge.src_port, 0);
    assert_eq!(edge.dst_node, node2);
    assert_eq!(edge.dst_port, 0);

    // 各ノードの配線状態が反映されているか
    let src = engine.nodes.get(&node1).unwrap();
    assert_eq!(src.out_edges[0], vec![edge_id]);

    let dst = engine.nodes.get(&node2).unwrap();
    assert_eq!(dst.in_edges[0], Some(edge_id));
}

/// 分岐接続（Fan-out）テスト:
/// - 1つの出力ポートから複数の異なるノードの入力ポートへ接続できること
#[test]
fn test_connect_fan_out() {
    let mut engine = GraphEngine::new();
    let src = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let dst1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let dst2 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // 1つの出力ポートから複数の入力ポートへ分岐接続可能
    let edge1 = engine.connect(src, 0, dst1, 0).expect("Branch 1 should succeed");
    let edge2 = engine.connect(src, 0, dst2, 0).expect("Branch 2 should succeed");

    let src_node = engine.nodes.get(&src).unwrap();
    assert_eq!(src_node.out_edges[0], vec![edge1, edge2]);
}

/// 自己ループ防止テスト:
/// - src_node と dst_node が同一の場合にエラーとして拒否されること
#[test]
fn test_connect_self_loop_rejected() {
    let mut engine = GraphEngine::new();
    let node = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // 自己ループは拒否
    let res = engine.connect(node, 0, node, 0);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Cannot connect node to itself");
}

/// 存在しないノードIDでの接続拒否テスト:
/// - 存在しない src_node または dst_node を指定したとき安全に Err になること
#[test]
fn test_connect_invalid_node_id() {
    let mut engine = GraphEngine::new();
    let valid_node = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // 存在しない src_node
    let res1 = engine.connect(999, 0, valid_node, 0);
    assert!(res1.is_err());
    assert_eq!(res1.unwrap_err(), "Cannot find src node");

    // 存在しない dst_node
    let res2 = engine.connect(valid_node, 0, 999, 0);
    assert!(res2.is_err());
    assert_eq!(res2.unwrap_err(), "Cannot find dst node");
}

/// 存在しないポート番号での接続拒否テスト:
/// - ポート数の範囲外のインデックスを指定したとき安全に Err になること（パニックしない）
#[test]
fn test_connect_invalid_port() {
    let mut engine = GraphEngine::new();
    let node1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let node2 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // 存在しない src_port（Gainは出力が1ポートのみ）
    let res1 = engine.connect(node1, 10, node2, 0);
    assert!(res1.is_err());
    assert_eq!(res1.unwrap_err(), "Cannot find src port");

    // 存在しない dst_port（Gainは入力が1ポートのみ）
    let res2 = engine.connect(node1, 0, node2, 10);
    assert!(res2.is_err());
    assert_eq!(res2.unwrap_err(), "Cannot find dst port");
}

/// 入力ポート重複接続拒否テスト:
/// - 既にエッジが接続されている入力ポートへの再接続が拒否されること（1入力1本ルール）
#[test]
fn test_connect_already_connected_input() {
    let mut engine = GraphEngine::new();
    let node1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let node2 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let node3 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // node1 -> node3:0
    assert!(engine.connect(node1, 0, node3, 0).is_ok());

    // node2 -> node3:0（既に刺さっているポートへの重複接続はエラー）
    let res = engine.connect(node2, 0, node3, 0);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "dst port 0 is already connected");
}

/// ポート型不一致の拒否テスト:
/// - 出力ポートの型と入力ポートの受け入れ可能型に互換性がない場合、接続が拒否されること
#[test]
fn test_connect_type_mismatch() {
    let mut engine = GraphEngine::new();
    // TimeDomain を出力するノード
    let time_node = engine.add_node(Box::new(GainProcessor::new(1.0)));
    // FrequencyDomain のみを受け付けるノード
    let freq_node = engine.add_node(Box::new(FrequencyDummyProcessor));

    // Time -> Frequency の接続は拒否
    let res = engine.connect(time_node, 0, freq_node, 0);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Invalid port types specified");
}

// ==========================================
// 4. disconnect テスト
// ==========================================

/// 切断の正常系と状態リセットの検証テスト:
/// - disconnect すると edges からエッジが消えること
/// - src_node.out_edges から edge_id が削除されること
/// - dst_node.in_edges が None に戻ること
#[test]
fn test_disconnect_success_and_state_verification() {
    let mut engine = GraphEngine::new();
    let node1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let node2 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    let edge_id = engine.connect(node1, 0, node2, 0).unwrap();

    // 切断実行
    assert!(engine.disconnect(edge_id).is_ok());

    // 1. edges マップから消えているか
    assert!(engine.edges.get(&edge_id).is_none());

    // 2. src_node の out_edges が空になっているか
    let src = engine.nodes.get(&node1).unwrap();
    assert!(src.out_edges[0].is_empty());

    // 3. dst_node の in_edges が None に戻っているか
    let dst = engine.nodes.get(&node2).unwrap();
    assert_eq!(dst.in_edges[0], None);
}

/// 切断後の再接続テスト:
/// - 切断した後の入力ポートに、再度同じ（または別）ノードから接続できること
#[test]
fn test_disconnect_allows_reconnect() {
    let mut engine = GraphEngine::new();
    let node1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let node2 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let node3 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    let edge1 = engine.connect(node1, 0, node2, 0).unwrap();

    // 一旦切断
    assert!(engine.disconnect(edge1).is_ok());

    // 空いた node2 の入力ポートに、node3 から再接続できること
    let edge2 = engine.connect(node3, 0, node2, 0);
    assert!(edge2.is_ok());

    let dst = engine.nodes.get(&node2).unwrap();
    assert_eq!(dst.in_edges[0], Some(edge2.unwrap()));
}

/// 分岐配線の一部分切断テスト:
/// - 1つの出力ポートから2本配線されている状態で1本だけ切断したとき、もう1本の配線は維持されること
#[test]
fn test_disconnect_partial_fan_out() {
    let mut engine = GraphEngine::new();
    let src = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let dst1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let dst2 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    let edge1 = engine.connect(src, 0, dst1, 0).unwrap();
    let edge2 = engine.connect(src, 0, dst2, 0).unwrap();

    // edge1 のみ切断
    assert!(engine.disconnect(edge1).is_ok());

    let src_node = engine.nodes.get(&src).unwrap();
    // edge2 だけが残っていること
    assert_eq!(src_node.out_edges[0], vec![edge2]);

    // dst1 は切断され、dst2 は接続維持されていること
    assert_eq!(engine.nodes.get(&dst1).unwrap().in_edges[0], None);
    assert_eq!(engine.nodes.get(&dst2).unwrap().in_edges[0], Some(edge2));
}

/// 存在しないエッジID切断拒否テスト:
/// - 存在しない edge_id の切断要求で Err("Cannot find edge") が返ること
#[test]
fn test_disconnect_invalid_edge_id() {
    let mut engine = GraphEngine::new();

    // 存在しない edge_id
    let res = engine.disconnect(999);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Cannot find edge");
}

/// 二重切断防止テスト:
/// - 一度切断した edge_id を再度切断しようとしたら Err("Cannot find edge") が返ること
#[test]
fn test_disconnect_double_disconnect() {
    let mut engine = GraphEngine::new();
    let node1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let node2 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    let edge_id = engine.connect(node1, 0, node2, 0).unwrap();

    // 1回目の切断は成功
    assert!(engine.disconnect(edge_id).is_ok());

    // 2回目の切断はエラー
    let res = engine.disconnect(edge_id);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Cannot find edge");
}

// ==========================================
// 5. build_execution_order (トポロジカルソート) テスト
// ==========================================

/// 直列接続の実行順テスト:
/// - n1 -> n2 -> n3 の依存関係がある場合、必ず [n1, n2, n3] の順序で返ること
#[test]
fn test_execution_order_linear() {
    let mut engine = GraphEngine::new();
    let n1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n2 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n3 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // n1 -> n2 -> n3
    engine.connect(n1, 0, n2, 0).unwrap();
    engine.connect(n2, 0, n3, 0).unwrap();

    let order = engine.build_execution_order().expect("Should succeed");
    // 必ず n1 -> n2 -> n3 の順序になる
    assert_eq!(order, vec![n1, n2, n3]);
}

/// 分岐合流（ダイヤモンド構造）の実行順テスト:
/// - A -> B, A -> C, B -> D の接続において、親が子よりも先頭側にあること
///   (A は B, C より前、B は D より前)
#[test]
fn test_execution_order_diamond() {
    let mut engine = GraphEngine::new();
    let n_a = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n_b = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n_c = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n_d = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // A -> B, A -> C, B -> D
    engine.connect(n_a, 0, n_b, 0).unwrap();
    engine.connect(n_a, 0, n_c, 0).unwrap();
    engine.connect(n_b, 0, n_d, 0).unwrap();

    let order = engine.build_execution_order().expect("Should succeed");
    assert_eq!(order.len(), 4);

    let pos = |id: usize| order.iter().position(|&x| x == id).unwrap();

    // A は B, C より先
    assert!(pos(n_a) < pos(n_b));
    assert!(pos(n_a) < pos(n_c));
    // B は D より先
    assert!(pos(n_b) < pos(n_d));
}

/// 孤立ノードを含むグラフの実行順テスト:
/// - 誰とも接続されていない孤立ノードがあっても、全ノードが漏れなくリストに含まれること
/// - 接続されているノード間の順序関係が正しく保たれていること
#[test]
fn test_execution_order_isolated_nodes() {
    let mut engine = GraphEngine::new();
    let n1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n2 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n_isolated = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // n1 -> n2 のみ接続。n_isolated は孤立
    engine.connect(n1, 0, n2, 0).unwrap();

    let order = engine.build_execution_order().expect("Should succeed");
    assert_eq!(order.len(), 3);

    let pos = |id: usize| order.iter().position(|&x| x == id).unwrap();
    // 依存関係のある n1 は n2 より先
    assert!(pos(n1) < pos(n2));
    // 孤立ノードも漏れなく含まれていること
    assert!(order.contains(&n_isolated));
}

/// 単純な循環参照（2ノード間ループ）の検知テスト:
/// - n1 -> n2 -> n1 のようなループがある場合、Err が返ること
#[test]
fn test_execution_order_simple_cycle_detected() {
    let mut engine = GraphEngine::new();
    let n1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n2 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // n1 -> n2 -> n1 (2ノード間のループ)
    engine.connect(n1, 0, n2, 0).unwrap();
    engine.connect(n2, 0, n1, 0).unwrap();

    // ループが検知されて Err になること
    let res = engine.build_execution_order();
    assert!(res.is_err());
}

/// 長大な循環参照（3ノード間ループ）の検知テスト:
/// - n1 -> n2 -> n3 -> n1 のような3ノード以上のループでも正しく Err が返ること
#[test]
fn test_execution_order_long_cycle_detected() {
    let mut engine = GraphEngine::new();
    let n1 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n2 = engine.add_node(Box::new(GainProcessor::new(1.0)));
    let n3 = engine.add_node(Box::new(GainProcessor::new(1.0)));

    // n1 -> n2 -> n3 -> n1 (3ノード間のループ)
    engine.connect(n1, 0, n2, 0).unwrap();
    engine.connect(n2, 0, n3, 0).unwrap();
    engine.connect(n3, 0, n1, 0).unwrap();

    let res = engine.build_execution_order();
    assert!(res.is_err());
}

