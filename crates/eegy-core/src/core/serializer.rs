use crate::core::graph_engine::{EdgeId, GraphEngine, NodeId};
use serde::{Deserialize, Serialize};
use crate::core::registry::Registry;

pub fn serialize(engine: &GraphEngine) -> Result<String, String>{
    let mut nodes = Vec::new();
    for node in engine.nodes.values(){
        let params = serde_json::to_value(&*node.processor)
            .map_err(|_|"cannot serialize processor".to_string())?;
        
        let node_description = NodeDescription {
            id: node.node_id,
            processor_type: node.processor.get_type().to_string(),
            params,
        };
        nodes.push(node_description);
    }
    
    let mut edges = Vec::new();
    for edge in engine.edges.values(){
        let edge_description = EdgeDescription{
            id: edge.id,
            src_node: edge.src_node,
            src_port: edge.src_port,
            dst_node: edge.dst_node,
            dst_port: edge.dst_port,
        };
        edges.push(edge_description);
    }
    
    let graph = GraphDescription{
        nodes,
        edges,
    };
    
    let json = serde_json::to_string_pretty(&graph).map_err(|_| "cannot serialize graph".to_string())?;
    Ok(json)
}

pub fn deserialize(data: &str, registry: &Registry) -> Result<GraphEngine, String>{
    let graph_description = serde_json::from_str::<GraphDescription>(data)
        .map_err(|_| "cannot deserialize".to_string())?;
    
    let mut graph = GraphEngine::new();
    
    for node_description in graph_description.nodes{
        let processor = registry.create(&node_description.processor_type, node_description.params)?;
        graph.add_node_with_id(node_description.id, processor)?;
    }
   
    for edge_description in graph_description.edges{
        graph.connect_with_id(
            edge_description.id,
            edge_description.src_node,
            edge_description.src_port,
            edge_description.dst_node,
            edge_description.dst_port,
        )?;
    }
    
    Ok(graph)
}
    

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct GraphDescription{
    nodes: Vec<NodeDescription>,
    edges: Vec<EdgeDescription>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct NodeDescription {
    id: NodeId,
    processor_type: String,
    params: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct EdgeDescription {
    id: EdgeId,
    src_node: NodeId,
    src_port: usize,
    dst_node: NodeId,
    dst_port: usize,
}