use std::collections::BTreeMap;
use std::path::Path;
use eframe::egui::Pos2;
use eegy::core::graph_engine::{GraphEngine, NodeId};
use eegy::core::registry::Registry;
use eegy::core::serializer;
use serde::{Deserialize, Serialize};
use crate::node_editor::NodeEditor;

/// GUI 側で保持する各ノードの表示・エディタ状態
/// （将来的に折りたたみ状態やコメント、カスタムカラーなどをここに追加可能）
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct NodeState {
    pub position: [f32; 2],
}

/// JSON 読み込み時に GUI 側の拡張情報（node_states）を抽出するための構造体
#[derive(Deserialize, Default)]
struct ProjectGuiExt {
    #[serde(default)]
    nodes: Vec<NodeIdEntry>,
    #[serde(default)]
    node_states: BTreeMap<NodeId, NodeState>,
}

#[derive(Deserialize)]
struct NodeIdEntry {
    id: NodeId,
}

/// GraphEngine のシリアライズ結果に GUI 側の `node_states` を統合して JSON 文字列を生成する。
/// 生成された JSON は `eegy::core::serializer::deserialize` でもそのまま読み込み可能。
pub fn serialize_project(
    engine: &GraphEngine,
    node_editor: &NodeEditor,
) -> Result<String, String> {
    let base_json = serializer::serialize(engine)?;
    let mut root: serde_json::Value =
        serde_json::from_str(&base_json).map_err(|e| e.to_string())?;

    let node_states: BTreeMap<NodeId, NodeState> = node_editor
        .node_positions()
        .iter()
        .map(|(&id, pos)| {
            (
                id,
                NodeState {
                    position: [pos.x, pos.y],
                },
            )
        })
        .collect();

    if let Some(obj) = root.as_object_mut() {
        let states_val = serde_json::to_value(node_states).map_err(|e| e.to_string())?;
        obj.insert("node_states".to_string(), states_val);
    }

    serde_json::to_string_pretty(&root).map_err(|e| e.to_string())
}

/// JSON 文字列から GraphEngine と NodeEditor の状態（`node_states`）を復元する。
/// もし `node_states` を含まない純粋なコア JSON が渡された場合でも、デフォルト位置に配置して復元する。
pub fn deserialize_project(
    data: &str,
    registry: &Registry,
    node_editor: &mut NodeEditor,
) -> Result<GraphEngine, String> {
    let engine = serializer::deserialize(data, registry)?;
    let gui_ext: ProjectGuiExt = serde_json::from_str(data).map_err(|e| e.to_string())?;

    node_editor.clear();

    for (idx, node_entry) in gui_ext.nodes.iter().enumerate() {
        let pos = if let Some(state) = gui_ext.node_states.get(&node_entry.id) {
            Pos2::new(state.position[0], state.position[1])
        } else {
            // node_states がない場合は重ならないように横並びで初期配置する
            Pos2::new(80.0 + (idx as f32) * 220.0, 120.0)
        };
        node_editor.register_node(node_entry.id, pos);
    }

    Ok(engine)
}

/// 指定パスへプロジェクトを JSON ファイルとして保存する
pub fn save_project_to_path(
    path: &Path,
    engine: &GraphEngine,
    node_editor: &NodeEditor,
) -> Result<(), String> {
    let json = serialize_project(engine, node_editor)?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write file: {e}"))
}

/// 指定パスの JSON ファイルからプロジェクトを読み込む
pub fn load_project_from_path(
    path: &Path,
    registry: &Registry,
    node_editor: &mut NodeEditor,
) -> Result<GraphEngine, String> {
    let data = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    deserialize_project(&data, registry, node_editor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_and_deserialize_project_with_node_states() {
        let registry = Registry::new();
        let mut engine = GraphEngine::new();
        let mut editor = NodeEditor::new();

        let p_source = registry.create_default("Source").unwrap();
        let p_gain = registry.create_default("Gain").unwrap();

        let n1 = engine.add_node(p_source);
        let n2 = engine.add_node(p_gain);
        engine.connect(n1, 0, n2, 0).unwrap();

        editor.register_node(n1, Pos2::new(120.0, 85.0));
        editor.register_node(n2, Pos2::new(380.0, 140.0));

        // 1. GUI プロジェクトとしてシリアライズ
        let json = serialize_project(&engine, &editor).unwrap();
        assert!(json.contains("\"node_states\""));

        // 2. eegy-core 単体の serializer::deserialize でもエラーなく読み込めること（Unity互換性）
        let core_only_engine = serializer::deserialize(&json, &registry).unwrap();
        assert!(core_only_engine.get_processor(n1).is_some());
        assert!(core_only_engine.get_processor(n2).is_some());

        // 3. GUI の deserialize_project でノード座標まで含めて復元されること
        let mut restored_editor = NodeEditor::new();
        let restored_engine = deserialize_project(&json, &registry, &mut restored_editor).unwrap();

        assert!(restored_engine.get_processor(n1).is_some());
        assert!(restored_engine.get_processor(n2).is_some());
        assert_eq!(
            restored_editor.node_positions().get(&n1),
            Some(&Pos2::new(120.0, 85.0))
        );
        assert_eq!(
            restored_editor.node_positions().get(&n2),
            Some(&Pos2::new(380.0, 140.0))
        );
    }
}
