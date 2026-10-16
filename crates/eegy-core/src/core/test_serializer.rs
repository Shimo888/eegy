use serde::{Deserialize, Serialize};
use crate::core::buffer::{Buffer, TimeDomainBuffer};
use crate::core::graph_engine::GraphEngine;
use crate::core::processor::{IOPort, IOType, Processor, ProcessorMeta};
use crate::core::registry::Registry;
use crate::core::serializer::{deserialize, serialize};
use crate::processors::gain::GainProcessor;

// ==========================================
// テスト用ダミープロセッサ定義
// ==========================================

/// 信号源（ソース）用のテストプロセッサ
/// - 入力ポートなし、Time領域の出力ポート1つ
/// - 固定の初期信号（[1.0, 2.0, 3.0]）を生成する
#[derive(Default, Serialize, Deserialize)]
struct ConstantSourceProcessor {
    value_scale: f64,
}
impl ConstantSourceProcessor {
    fn new(value_scale: f64) -> Self {
        Self { value_scale }
    }
}
impl ProcessorMeta for ConstantSourceProcessor {
    fn get_type(&self) -> &'static str {
        "ConstantSource"
    }
    fn get_input_ports(&self) -> &'static [IOPort] {
        &[]
    }
    fn get_output_ports(&self) -> &'static [IOPort] {
        &[IOPort {
            name: "out",
            optional: false,
            types: &[IOType::Time],
        }]
    }
}
impl Processor for ConstantSourceProcessor {
    fn setup(&mut self, _inputs: &[&Buffer], outputs: &mut [&mut Buffer]) -> Result<(), String> {
        *outputs[0] = Buffer::Time(TimeDomainBuffer {
            first_packet_num: 1,
            channels: vec!["Ch1".to_string()],
            sampling_rate: 250.0,
            data: vec![1.0 * self.value_scale, 2.0 * self.value_scale, 3.0 * self.value_scale],
        });
        Ok(())
    }

    fn process(&mut self, _inputs: &[&Buffer], outputs: &mut [&mut Buffer]) -> Result<(), String> {
        if let Some(buf) = outputs[0].as_time_mut() {
            buf.data = vec![1.0 * self.value_scale, 2.0 * self.value_scale, 3.0 * self.value_scale];
        }
        Ok(())
    }
}

/// 2入力1出力の加算テストプロセッサ（複数ポート・合流トポロジー検証用）
#[derive(Default, Serialize, Deserialize)]
struct AddProcessor;
impl ProcessorMeta for AddProcessor {
    fn get_type(&self) -> &'static str {
        "Add"
    }
    fn get_input_ports(&self) -> &'static [IOPort] {
        &[
            IOPort {
                name: "in_a",
                optional: false,
                types: &[IOType::Time],
            },
            IOPort {
                name: "in_b",
                optional: false,
                types: &[IOType::Time],
            },
        ]
    }
    fn get_output_ports(&self) -> &'static [IOPort] {
        &[IOPort {
            name: "out",
            optional: false,
            types: &[IOType::Time],
        }]
    }
}
impl Processor for AddProcessor {
    fn setup(&mut self, inputs: &[&Buffer], outputs: &mut [&mut Buffer]) -> Result<(), String> {
        *outputs[0] = (*inputs[0]).clone();
        Ok(())
    }

    fn process(&mut self, inputs: &[&Buffer], outputs: &mut [&mut Buffer]) -> Result<(), String> {
        let a = inputs[0].as_time().ok_or("in_a must be Time")?;
        let b = inputs[1].as_time().ok_or("in_b must be Time")?;
        let out = outputs[0].as_time_mut().ok_or("out must be Time")?;
        for ((va, vb), vo) in a.data.iter().zip(b.data.iter()).zip(out.data.iter_mut()) {
            *vo = va + vb;
        }
        Ok(())
    }
}

/// 周波数領域専用のダミープロセッサ（ポート型不一致テスト用）
#[derive(Default, Serialize, Deserialize)]
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
    fn setup(&mut self, _inputs: &[&Buffer], _outputs: &mut [&mut Buffer]) -> Result<(), String> {
        Ok(())
    }
    fn process(&mut self, _inputs: &[&Buffer], _outputs: &mut [&mut Buffer]) -> Result<(), String> {
        Ok(())
    }
}

fn create_test_registry() -> Registry {
    let mut registry = Registry::new();
    registry.register::<GainProcessor>("Gain");
    registry.register::<ConstantSourceProcessor>("ConstantSource");
    registry.register::<AddProcessor>("Add");
    registry.register::<FrequencyDummyProcessor>("FrequencyDummy");
    registry
}

// ==========================================
// 1. 正常系・ラウンドトリップ（往復）テスト
// ==========================================

/// 空グラフのシリアライズ・デシリアライズテスト:
/// - ノードもエッジもない空のグラフを正常にシリアライズ・デシリアライズできること
/// - 復元後の nodes, edges が空であり、next_node_id と next_edge_id が初期値 1 であること
#[test]
fn test_serialize_and_deserialize_empty_graph() {
    let engine = GraphEngine::new();
    let registry = create_test_registry();

    let json = serialize(&engine).expect("serialize empty graph should succeed");
    let restored = deserialize(&json, &registry).expect("deserialize empty graph should succeed");

    assert!(restored.nodes.is_empty());
    assert!(restored.edges.is_empty());
    assert_eq!(restored.next_node_id, 1);
    assert_eq!(restored.next_edge_id, 1);
}

/// 直列パイプラインの往復（ラウンドトリップ）テスト:
/// - ノードID、エッジID、プロセッサの型とパラメータ（gain値）が完全に保持されること
/// - Node 内の in_edges / out_edges のリンク状態も元通りに復元されること
/// - 復元したグラフを再シリアライズした JSON が元の JSON と完全一致すること
#[test]
fn test_serialize_and_deserialize_linear_pipeline() {
    let mut engine = GraphEngine::new();
    let n1 = engine.add_node(Box::new(GainProcessor::new(2.5)));
    let n2 = engine.add_node(Box::new(GainProcessor::new(0.25)));
    let e1 = engine.connect(n1, 0, n2, 0).unwrap();

    let registry = create_test_registry();

    let json1 = serialize(&engine).expect("serialize should succeed");
    let restored = deserialize(&json1, &registry).expect("deserialize should succeed");

    // 1. ノードとエッジのID・数が一致していること
    assert_eq!(restored.nodes.len(), 2);
    assert_eq!(restored.edges.len(), 1);
    assert!(restored.nodes.contains_key(&n1));
    assert!(restored.nodes.contains_key(&n2));
    assert_eq!(restored.edges.get(&e1), engine.edges.get(&e1));

    // 2. Node 内部の in_edges / out_edges が正しく張られていること
    assert_eq!(restored.nodes.get(&n1).unwrap().out_edges[0], vec![e1]);
    assert_eq!(restored.nodes.get(&n2).unwrap().in_edges[0], Some(e1));

    // 3. 再シリアライズ結果が完全一致すること（パラメータ値まで一致している証明）
    let json2 = serialize(&restored).expect("re-serialize should succeed");
    assert_eq!(json1, json2);
}

/// 分岐（Fan-out）および合流（Diamond）トポロジーの往復テスト:
/// - 1つの出力ポートから複数ノードへ分岐する接続が正しく復元されること
/// - 複数入力ポート（port 0, port 1）への接続がポート番号も含めて正確に復元されること
#[test]
fn test_serialize_and_deserialize_diamond_graph() {
    let mut engine = GraphEngine::new();
    let src = engine.add_node(Box::new(ConstantSourceProcessor::new(1.0)));
    let gain_a = engine.add_node(Box::new(GainProcessor::new(2.0)));
    let gain_b = engine.add_node(Box::new(GainProcessor::new(3.0)));
    let adder = engine.add_node(Box::new(AddProcessor));

    // src -> gain_a (port 0), src -> gain_b (port 0)
    let e1 = engine.connect(src, 0, gain_a, 0).unwrap();
    let e2 = engine.connect(src, 0, gain_b, 0).unwrap();
    // gain_a -> adder (port 0), gain_b -> adder (port 1)
    let e3 = engine.connect(gain_a, 0, adder, 0).unwrap();
    let e4 = engine.connect(gain_b, 0, adder, 1).unwrap();

    let registry = create_test_registry();
    let json = serialize(&engine).unwrap();
    let restored = deserialize(&json, &registry).unwrap();

    assert_eq!(restored.nodes.len(), 4);
    assert_eq!(restored.edges.len(), 4);
    assert_eq!(restored.nodes.get(&src).unwrap().out_edges[0], vec![e1, e2]);
    assert_eq!(restored.nodes.get(&adder).unwrap().in_edges[0], Some(e3));
    assert_eq!(restored.nodes.get(&adder).unwrap().in_edges[1], Some(e4));
}

// ==========================================
// 2. ID 維持・採番（next_node_id / next_edge_id）テスト
// ==========================================

/// 歯抜けID（削除後のグラフ）の維持と、復元後の新規ノード・エッジ追加テスト:
/// - 途中のノードやエッジが削除されてIDが飛んでいる（例: Node 1, 3 / Edge 2）場合でも、元のIDのまま復元されること
/// - 復元後に add_node / connect を呼んだ際、最大ID + 1（Node 4 / Edge 3）が採番され、ID衝突が起きないこと
#[test]
fn test_deserialize_preserves_sparse_ids_and_updates_next_ids() {
    let mut engine = GraphEngine::new();
    let n1 = engine.add_node(Box::new(GainProcessor::new(1.0))); // ID: 1
    let n2 = engine.add_node(Box::new(GainProcessor::new(2.0))); // ID: 2
    let n3 = engine.add_node(Box::new(GainProcessor::new(3.0))); // ID: 3

    let _e1 = engine.connect(n1, 0, n2, 0).unwrap(); // Edge ID: 1
    let e2 = engine.connect(n1, 0, n3, 0).unwrap();  // Edge ID: 2

    // n2 を削除（これにより Node 2 と Edge 1 が消え、ID が歯抜けになる）
    engine.remove_node(n2).unwrap();

    let registry = create_test_registry();
    let json = serialize(&engine).unwrap();
    let mut restored = deserialize(&json, &registry).unwrap();

    // 1. 元の歯抜けID（Node 1, 3 および Edge 2）がそのまま存在すること
    assert!(restored.nodes.contains_key(&1));
    assert!(!restored.nodes.contains_key(&2));
    assert!(restored.nodes.contains_key(&3));
    assert!(!restored.edges.contains_key(&1));
    assert!(restored.edges.contains_key(&e2));

    // 2. next_node_id は 4 (max=3 + 1)、next_edge_id は 3 (max=2 + 1) になっていること
    assert_eq!(restored.next_node_id, 4);
    assert_eq!(restored.next_edge_id, 3);

    // 3. 復元後に新しくノードとエッジを追加しても衝突せず正しく採番されること
    let n4 = restored.add_node(Box::new(GainProcessor::new(4.0)));
    assert_eq!(n4, 4);
    let e3 = restored.connect(n3, 0, n4, 0).unwrap();
    assert_eq!(e3, 3);
}

// ==========================================
// 3. 復元後の信号処理（setup & process）統合テスト
// ==========================================

/// デシリアライズ後のグラフで setup() と process() が正しく動作するかの検証テスト:
/// - Source([2.0, 4.0, 6.0]) -> Gain(3.0) -> Gain(0.5) のグラフをシリアライズ・デシリアライズ
/// - 復元したグラフで setup() と process() が成功し、出力信号が 1.5倍 ([3.0, 6.0, 9.0]) になること
#[test]
fn test_deserialized_graph_execution_setup_and_process() {
    let mut engine = GraphEngine::new();
    let src = engine.add_node(Box::new(ConstantSourceProcessor::new(2.0))); // [2.0, 4.0, 6.0]
    let g1 = engine.add_node(Box::new(GainProcessor::new(3.0)));            // * 3.0 = [6.0, 12.0, 18.0]
    let g2 = engine.add_node(Box::new(GainProcessor::new(0.5)));            // * 0.5 = [3.0, 6.0, 9.0]

    engine.connect(src, 0, g1, 0).unwrap();
    engine.connect(g1, 0, g2, 0).unwrap();

    let registry = create_test_registry();
    let json = serialize(&engine).unwrap();
    let mut restored = deserialize(&json, &registry).unwrap();

    // 復元したグラフをセットアップして実行
    assert!(restored.setup().is_ok());
    assert!(restored.process().is_ok());

    // 最終ノード (g2) の出力バッファの計算結果を検証
    let g2_out_idx = restored.nodes.get(&g2).unwrap().out_buffer_indices[0];
    let out_time = restored.buffers[g2_out_idx].as_time().unwrap();
    assert_eq!(out_time.data, vec![3.0, 6.0, 9.0]);
}

// ==========================================
// 4. 異常系・バリデーションエラーテスト
// ==========================================

/// 不正な JSON 構文のデシリアライズ失敗テスト:
/// - 壊れた JSON 文字列を渡したときに Err が返ること
#[test]
fn test_deserialize_invalid_json_syntax_fails() {
    let registry = create_test_registry();
    let invalid_json = "{ invalid json syntax ";

    let res = deserialize(invalid_json, &registry);
    assert!(res.is_err());
}

/// 未登録のプロセッサ型のデシリアライズ失敗テスト:
/// - Registry に登録されていない processor_type が含まれる場合、Unknown processor エラーになること
#[test]
fn test_deserialize_unregistered_processor_fails() {
    let registry = create_test_registry();
    let json = r#"{
        "nodes": [
            {
                "id": 1,
                "processor_type": "NonExistentFilter",
                "params": {}
            }
        ],
        "edges": []
    }"#;

    let res = deserialize(json, &registry);
    assert!(res.is_err());
    assert!(res.err().unwrap().contains("Unknown processor NonExistentFilter"));
}

/// プロセッサのパラメータ型不正によるデシリアライズ失敗テスト:
/// - GainProcessor の gain フィールドに数値ではなく文字列が指定されている場合、エラーになること
#[test]
fn test_deserialize_invalid_processor_params_fails() {
    let registry = create_test_registry();
    let json = r#"{
        "nodes": [
            {
                "id": 1,
                "processor_type": "Gain",
                "params": {
                    "gain": "not_a_number"
                }
            }
        ],
        "edges": []
    }"#;

    let res = deserialize(json, &registry);
    assert!(res.is_err());
}

/// 重複するノードIDが含まれる JSON のデシリアライズ失敗テスト:
/// - 同じ id を持つノードが複数定義されている場合、duplicate node エラーになること
#[test]
fn test_deserialize_duplicate_node_id_fails() {
    let registry = create_test_registry();
    let json = r#"{
        "nodes": [
            {
                "id": 1,
                "processor_type": "Gain",
                "params": { "gain": 1.0 }
            },
            {
                "id": 1,
                "processor_type": "Gain",
                "params": { "gain": 2.0 }
            }
        ],
        "edges": []
    }"#;

    let res = deserialize(json, &registry);
    assert!(res.is_err());
    assert_eq!(res.err().unwrap(), "duplicate node with id 1");
}

/// 重複するエッジIDが含まれる JSON のデシリアライズ失敗テスト:
/// - 同じ id を持つエッジが複数定義されている場合、duplicate edge エラーになること
#[test]
fn test_deserialize_duplicate_edge_id_fails() {
    let registry = create_test_registry();
    let json = r#"{
        "nodes": [
            {
                "id": 1,
                "processor_type": "Gain",
                "params": { "gain": 1.0 }
            },
            {
                "id": 2,
                "processor_type": "Gain",
                "params": { "gain": 2.0 }
            },
            {
                "id": 3,
                "processor_type": "Gain",
                "params": { "gain": 3.0 }
            }
        ],
        "edges": [
            {
                "id": 10,
                "src_node": 1,
                "src_port": 0,
                "dst_node": 2,
                "dst_port": 0
            },
            {
                "id": 10,
                "src_node": 2,
                "src_port": 0,
                "dst_node": 3,
                "dst_port": 0
            }
        ]
    }"#;

    let res = deserialize(json, &registry);
    assert!(res.is_err());
    assert_eq!(res.err().unwrap(), "duplicate edge id: 10");
}

/// 不正なエッジ接続（ポート型不一致・存在しないノード）のデシリアライズ失敗テスト:
/// - Time 出力を Frequency 入力に繋ごうとする不正なエッジが JSON に含まれる場合、
///   connect_with_id のバリデーションによりエラーになること
#[test]
fn test_deserialize_invalid_edge_connection_fails() {
    let registry = create_test_registry();
    let json = r#"{
        "nodes": [
            {
                "id": 1,
                "processor_type": "Gain",
                "params": { "gain": 1.0 }
            },
            {
                "id": 2,
                "processor_type": "FrequencyDummy",
                "params": null
            }
        ],
        "edges": [
            {
                "id": 1,
                "src_node": 1,
                "src_port": 0,
                "dst_node": 2,
                "dst_port": 0
            }
        ]
    }"#;

    let res = deserialize(json, &registry);
    assert!(res.is_err());
    assert_eq!(res.err().unwrap(), "Invalid port types specified");
}
