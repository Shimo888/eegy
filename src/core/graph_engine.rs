use crate::core::processor::Processor;
use std::collections::HashMap;

pub type NodeId = usize;
pub type EdgeId = usize;

pub struct GraphEngine {
    pub nodes: HashMap<NodeId, Node>,
    pub edges: HashMap<EdgeId, Edge>,
    pub execution_order: Vec<NodeId>,
    pub next_node_id: NodeId,
    pub next_edge_id: EdgeId,
}

impl GraphEngine {
    pub fn new() -> Self {
        Self{
            nodes: HashMap::new(),
            edges: HashMap::new(),
            execution_order: Vec::new(),
            next_node_id: 1,
            next_edge_id: 1,
        }
    }
    
    pub fn add_node(&mut self, processor: Box<dyn Processor>) -> NodeId{
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        
        self.nodes.insert(node_id, Node::new(node_id, processor));
        node_id
    }
    
    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(), String>{
        // 削除に必要なedgeを集める
        let mut target_edges = Vec::new();
        {
            let node = self.nodes.get(&node_id).ok_or("Cannot find node")?;
            for edge in &node.in_edges{
                if let Some(edge_id) = edge {
                    target_edges.push(*edge_id);
                }
            }

            for edge_ids in &node.out_edges{
                for edge_id in edge_ids {
                    target_edges.push(*edge_id);
                }
            }
        }
        
        // edge_idとnode_idの削除
        // edgeから削除しないとpanicしてしまう
        for edge_id in target_edges {
            self.disconnect(edge_id)?;
        }
        self.nodes.remove(&node_id);
        
        Ok(())
    }
    
    pub fn connect(&mut self, src_node_id: NodeId, src_port: usize, dst_node_id: NodeId, dst_port:usize) -> Result<EdgeId, String>{
        // todo: この辺のグラフのバリデーションは別で用意する
        if src_node_id == dst_node_id {
            return Err("Cannot connect node to itself".to_string());
        }
        
        let src_node = self.nodes.get(&src_node_id).ok_or("Cannot find src node")?;
        let dst_node = self.nodes.get(&dst_node_id).ok_or("Cannot find dst node")?;
        
        // 対応するポートが存在するかどうか？
        let src_port_info = src_node.processor.get_output_ports().get(src_port).ok_or("Cannot find src port")?;
        let dst_port_info = dst_node.processor.get_input_ports().get(dst_port).ok_or("Cannot find dst port")?;
        
        // dst_nodeのinput_portが使用できるかどうか？
        if let Some(_) = dst_node.in_edges[dst_port]{
            return Err(format!("dst port {} is already connected", dst_port));
        }
        
        // src_portのtypeが受け入れ可能かどうか？
        let validate_port_types = src_port_info
            .types
            .iter()
            .any(|src_type| dst_port_info.types.contains(src_type));
        if !validate_port_types {
            return Err(format!("Invalid port types specified"));
        }

        // 存在していればEdgeを作成
        let edge_id = self.next_edge_id;
        self.next_edge_id += 1;
        self.edges.insert(edge_id, Edge{
            id: edge_id,
            src_node: src_node_id,
            src_port,
            dst_node: dst_node_id,
            dst_port,
        });

        // src_nodeにedgeの参照を追加
        {
            let src_node = self.nodes.get_mut(&src_node_id).ok_or("Cannot find src node")?;
            src_node.out_edges[src_port].push(edge_id);
        }
        
        // dst_nodeにedgeの参照を追加
        {
            let dst_node = self.nodes.get_mut(&dst_node_id).ok_or("Cannot find dst node")?;
            dst_node.in_edges[dst_port] = Some(edge_id);
        }
        
        Ok(edge_id)
    }
    
    pub fn disconnect(&mut self, edge_id: EdgeId) -> Result<(), String>{
        // edgeを削除、なければエラー
        let edge = self.edges.remove(&edge_id).ok_or("Cannot find edge")?;

        // src_nodeから対象edgeを削除
        {
            let src_node = self.nodes.get_mut(&edge.src_node).ok_or("Cannot find src node")?;   
            src_node.out_edges[edge.src_port].retain(|edge_id| *edge_id != edge.id);
        }
        
        // dst_nodeから対象edgeを削除
        {
            let dst_node = self.nodes.get_mut(&edge.dst_node).ok_or("Cannot find dst node")?;
            dst_node.in_edges[edge.dst_port] = None;
        }
        Ok(())
    }
    
    /// グラフ内の全ノードの依存関係を解析し、実行可能な順序（トポロジカル順）を決定する。
    /// # アルゴリズム
    /// Kahn（カーン）のアルゴリズムを用いてトポロジカルソートを行う。
    /// # 戻り値
    /// - `Ok(Vec<NodeId>)`: 循環のない正しい実行順リスト。
    /// - `Err(String)`: グラフ内に循環参照（ループ）が検知された場合。
    pub fn build_execution_order(&self) -> Result<Vec<NodeId>, String>{

        let mut input_nums = HashMap::new();
        let mut execution_queue = Vec::new();
        let mut execution_order = Vec::new();
        
        // 1. 各ノードの入力数を記録 
        for (node_id, node) in &self.nodes {
            let input_count = node.in_edges.iter().filter(|edge| edge.is_some()).count();
            input_nums.insert(*node_id, input_count);
            
            // 2. 入力数が0のノードをストック
            if input_count == 0{
                execution_queue.push(*node_id);
            }
        }
        
        // 3. 入力数が0のノードを取り出す
        while let Some(node_id) = execution_queue.pop(){

            // 4. ノードの接続先の入力数を減らす 
            let node = self.nodes.get(&node_id).ok_or("Cannot find node")?;
            for out_edges in &node.out_edges{
                for edge_id in out_edges {
                    let edge = self.edges.get(edge_id).ok_or("Cannot find edge")?;
                    let dst_node_id = edge.dst_node;
                    if let Some(count) = input_nums.get_mut(&dst_node_id){
                        *count -= 1;
                        
                        // 入力数が0のノードを記録
                        if *count == 0{
                            execution_queue.push(dst_node_id);
                        }
                    }
                }
            }
            
            // 5. 実行可能リストへの追加
            execution_order.push(node_id);
        }
        
        // ループがある
        if execution_order.len() != self.nodes.len(){
            return Err("Cyclic dependency detected in graph".to_string());
        } 
        
        Ok(execution_order)
    }
}

pub struct Node {
    pub node_id: usize,
    pub in_edges: Vec<Option<EdgeId>>, 
    pub out_edges: Vec<Vec<EdgeId>>,
    pub processor: Box<dyn Processor>, 
}

impl Node{
    pub fn new(node_id: NodeId, processor: Box<dyn Processor>) -> Self{
        let num_input = processor.get_input_ports().len();
        let num_output = processor.get_output_ports().len();
        
        Self{
            node_id,
            processor,
            in_edges: vec![None; num_input],
            out_edges: vec![Vec::new(); num_output],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge{
    pub id: EdgeId,
    pub src_node: NodeId,
    pub src_port: usize,
    pub dst_node: NodeId,
    pub dst_port: usize,
}
