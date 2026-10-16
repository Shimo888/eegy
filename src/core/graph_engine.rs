use smallvec::SmallVec;
use crate::core::processor::Processor;
use std::collections::BTreeMap;
use crate::core::buffer::Buffer;

pub type NodeId = usize;
pub type EdgeId = usize;

pub struct GraphEngine {
    pub(crate) nodes: BTreeMap<NodeId, Node>,
    pub(crate) edges: BTreeMap<EdgeId, Edge>,
    pub(crate) execution_order: Vec<NodeId>,
    pub(crate) next_node_id: NodeId,
    pub(crate) next_edge_id: EdgeId,
    pub(crate) buffers: Vec<Buffer>,
    pub(crate) is_ready: bool,
}

impl GraphEngine {
    pub fn new() -> Self {
        Self{
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            execution_order: Vec::new(),
            next_node_id: 1,
            next_edge_id: 1,
            buffers: Vec::new(),
            is_ready: false,
        }
    }
    
    pub fn add_node(&mut self, processor: Box<dyn Processor>) -> NodeId{
        let node_id = self.next_node_id;
        self.add_node_with_id(node_id, processor).unwrap()
    }

    pub(crate) fn add_node_with_id(&mut self, node_id: NodeId, processor: Box<dyn Processor>) -> Result<NodeId, String>{
        if self.nodes.contains_key(&node_id) {
            return Err(format!("duplicate node with id {}", node_id));
        }
        self.is_ready = false;
        if node_id >= self.next_node_id {
            self.next_node_id = node_id + 1;
        }
        
        self.nodes.insert(node_id, Node::new(node_id, processor));
        Ok(node_id)
    }
    
    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(), String>{
        self.is_ready = false;
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
        let edge_id = self.next_edge_id;
        self.connect_with_id(edge_id, src_node_id, src_port, dst_node_id, dst_port)
    }

    pub(crate) fn connect_with_id(&mut self, edge_id: EdgeId, src_node_id: NodeId, src_port: usize, dst_node_id: NodeId, dst_port:usize) -> Result<EdgeId, String>{
        self.is_ready = false;
        if self.edges.contains_key(&edge_id) {
            return Err(format!("duplicate edge id: {}", edge_id));
        }
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
            return Err("Invalid port types specified".to_string());
        }

        // 存在していればEdgeを作成
        if edge_id >= self.next_edge_id {
            self.next_edge_id = edge_id + 1;
        }
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
        self.is_ready = false;
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
    
    /// Graph全体のセットアップ
    pub fn setup(&mut self) -> Result<(), String>{
        self.is_ready = false;
        let execution_order = self.build_execution_order()?;
        let mut buffers  = self.set_up_buffers(&execution_order)?;
        
        for node_id in &execution_order{
            let node = self.nodes.get_mut(node_id).ok_or("Cannot find node")?;

            node.with_buffers(&mut buffers,
                              |processor, inputs, outputs|{
                                  processor.setup(inputs, outputs) })?;
        }

        self.execution_order = execution_order;
        self.buffers = buffers;
        self.is_ready = true;
        Ok(())
    }
    
    // BufferとBufferのIndexの初期化
    fn set_up_buffers(&mut self, execution_order: &Vec<NodeId>) -> Result<Vec<Buffer>, String>{
        let mut buffers = Vec::new();
        
        for node_id in execution_order{
            let mut in_buffer_indices = Vec::new();
            
            // 1: 入力Bufferのidxを解決
            let node = self.nodes.get(node_id).ok_or("Cannot find node")?;
            for (port_num, port) in node.processor.get_input_ports().iter().enumerate() {
                let edge = node.in_edges[port_num]
                    .and_then(|edge_id| self.edges.get(&edge_id));
                
                // 1.1: 必須ポートがconnectされていなければエラー
                if !port.optional && edge.is_none() {
                    return Err(format!("node {} port {} requires inputs", node_id, port_num));
                }
                
                let input_buffer_idx = edge.and_then(|edge| { 
                    self.nodes.get(&edge.src_node)?
                        .out_buffer_indices.get(edge.src_port).copied()                
                });
                
                in_buffer_indices.push(input_buffer_idx);
            }

            // 2: 入力BufferのIdの代入
            let node = self.nodes.get_mut(node_id).ok_or("Cannot find node")?;
            node.in_buffer_indices = in_buffer_indices;

            // 3: 出力BufferのIdの代入とBufferの確保
            for out_buffer_index in &mut node.out_buffer_indices{
                *out_buffer_index = buffers.len();
                buffers.push(Buffer::None);
            }
        }
        
        Ok(buffers)
    }
    
    pub fn process(&mut self) -> Result<(), String>{
        if !self.is_ready{
            return Err("not ready".to_string());
        }

        for node_id in &self.execution_order{
            let node = self.nodes.get_mut(node_id).ok_or("Cannot find node")?;
            node.with_buffers(&mut self.buffers, 
                              |processor, inputs, outputs|{ 
                                  processor.process(inputs, outputs)})?;
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

        let mut input_nums = BTreeMap::new();
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
    pub(crate) node_id: NodeId,
    pub(crate) in_edges: Vec<Option<EdgeId>>, 
    pub(crate) out_edges: Vec<Vec<EdgeId>>,
    pub(crate) processor: Box<dyn Processor>, 
    pub(crate) in_buffer_indices: Vec<Option<usize>>,
    pub(crate) out_buffer_indices: Vec<usize>,
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
            in_buffer_indices: vec![None; num_input],
            out_buffer_indices: vec![0; num_output],
        }
    }

    fn with_buffers<F>(
        &mut self,
        buffers: &mut [Buffer],
        mut f: F,
    ) -> Result<(), String>
    where F: FnMut(&mut dyn Processor, &[&Buffer], &mut [&mut Buffer]) -> Result<(), String>,
    {
        // 1: bufferをinputとoutput側で分離する (input_idx<output_idxなので可能) 
        let n = self.out_buffer_indices.first().copied().unwrap_or(buffers.len());
        let (prev_buffers, rest_buffers) = buffers.split_at_mut(n);

        // 2: 前半から入力を集める (最大8ポートまでスタック)
        let mut inputs: SmallVec<[&Buffer; 8]> = SmallVec::new();
        for idx_opt in &self.in_buffer_indices {
            if let Some(idx) = idx_opt {
                inputs.push(&prev_buffers[*idx]);
            } else {
                inputs.push(&Buffer::None);
            }
        }

        // 3: 後半から出力を集める (最大8ポートまでスタック)
        let num_outputs = self.out_buffer_indices.len();
        let mut outputs: SmallVec<[&mut Buffer; 8]> = SmallVec::new();
        for buf in rest_buffers[0..num_outputs].iter_mut() {
            outputs.push(buf);
        }
        
        // 4: 実行する
        f(&mut *self.processor, &inputs, &mut outputs)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge{
    pub(crate) id: EdgeId,
    pub(crate) src_node: NodeId,
    pub(crate) src_port: usize,
    pub(crate) dst_node: NodeId,
    pub(crate) dst_port: usize,
}
