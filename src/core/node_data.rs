use std::collections::HashMap;
use crate::core::matrix::Matrix;

/// スカラー用のデータ
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarData {
    Float(f64),
    Int(i64),
    String(String),
}

/// ベクトル用のデータ
#[derive(Clone, Debug, PartialEq)]
pub enum VectorData {
    FloatVec(Vec<f64>),
    IntVec(Vec<i64>),
    StringVec(Vec<String>),
}

impl VectorData {
    /// ベクトルの長さを取得
    pub fn len(&self) -> usize {
        match self {
            VectorData::FloatVec(v) => v.len(),
            VectorData::IntVec(v) => v.len(),
            VectorData::StringVec(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 行列用のデータ 
#[derive(Clone, Debug, PartialEq)]
pub enum MatrixData {
    FloatMat(Matrix<f64>),
    IntMat(Matrix<i64>),
    StringMat(Matrix<String>),
}

impl MatrixData {
    /// 行列の形状 (rows, cols) を取得
    pub fn shape(&self) -> (usize, usize) {
        match self {
            MatrixData::FloatMat(m) => (m.rows, m.cols),
            MatrixData::IntMat(m) => (m.rows, m.cols),
            MatrixData::StringMat(m) => (m.rows, m.cols),
        }
    }

    /// 行列内部のデータ長が整合しているかをチェック
    pub fn is_valid(&self) -> bool {
        match self {
            MatrixData::FloatMat(m) => m.is_valid(),
            MatrixData::IntMat(m) => m.is_valid(),
            MatrixData::StringMat(m) => m.is_valid(),
        }
    }
}

/// スカラー、ベクトル、行列をまとめたデータ
#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
    Scalar(ScalarData),
    Vector(VectorData),
    Matrix(MatrixData),
}

/// シグナルドメイン
/// 時間領域、周波数領域、時間周波数領域
#[derive(Clone, Debug, PartialEq)]
pub enum SignalDomain {
    Time(usize),
    Frequency(usize),
    TimeFrequency(usize, usize),
    None,
}

/// ノードが管理するデータ 
#[derive(Clone, Debug, PartialEq)] 
pub struct NodeData {
    pub domain: SignalDomain,
    pub meta_table: HashMap<String, NodeValue>,
    pub signal_table: HashMap<String, NodeValue>,
}

impl NodeData {
    /// テーブル全体の整合性を検証する
    /// 不正なデータがある場合は Err(エラーメッセージ) を返す
    pub fn validate(&self) -> Result<(), String> {
        match self.domain {
            // 時間領域・周波数領域
            // すべてのシグナルが Vector であり、長さが expected_len と一致すること
            SignalDomain::Time(expected_len) | SignalDomain::Frequency(expected_len) => {
                for (name, val) in &self.signal_table {
                    match val {
                        NodeValue::Vector(vec) => {
                            if vec.len() != expected_len {
                                return Err(format!(
                                    "Signal '{}' length mismatch: expected {}, got {}",
                                    name, expected_len, vec.len()
                                ));
                            }
                        }
                        _ => {
                            return Err(format!(
                                "Signal '{}' must be a Vector in {:?} domain",
                                name, self.domain
                            ));
                        }
                    }
                }
            }

            // 時間周波数領域
            // すべてのシグナルが Matrix であり、shape が一致し、データ長が壊れていないこと
            SignalDomain::TimeFrequency(exp_rows, exp_cols) => {
                for (name, val) in &self.signal_table {
                    match val {
                        NodeValue::Matrix(mat) => {
                            if !mat.is_valid() {
                                return Err(format!("Matrix '{}' data length is corrupted", name));
                            }
                            if mat.shape() != (exp_rows, exp_cols) {
                                return Err(format!(
                                    "Matrix '{}' shape mismatch: expected ({}, {}), got {:?}",
                                    name, exp_rows, exp_cols, mat.shape()
                                ));
                            }
                        }
                        _ => {
                            return Err(format!(
                                "Signal '{}' must be a Matrix in TimeFrequency domain",
                                name
                            ));
                        }
                    }
                }
            }

            // Noneのとき
            // 信号データは存在してはならない
            SignalDomain::None => {
                if !self.signal_table.is_empty() {
                    return Err("Signal table must be empty when domain is None".to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_time_domain() {
        let mut node = NodeData {
            domain: SignalDomain::Time(3),
            meta_table: HashMap::new(),
            signal_table: HashMap::new(),
        };

        node.signal_table.insert(
            "Fp1".to_string(),
            NodeValue::Vector(VectorData::FloatVec(vec![1.0, 2.0, 3.0])),
        );
        node.signal_table.insert(
            "Fp2".to_string(),
            NodeValue::Vector(VectorData::FloatVec(vec![4.0, 5.0, 6.0])),
        );

        assert!(node.validate().is_ok());
    }

    #[test]
    fn test_invalid_length_in_time_domain() {
        let mut node = NodeData {
            domain: SignalDomain::Time(3),
            meta_table: HashMap::new(),
            signal_table: HashMap::new(),
        };

        node.signal_table.insert(
            "Fp1".to_string(),
            NodeValue::Vector(VectorData::FloatVec(vec![1.0, 2.0])),
        );

        assert!(node.validate().is_err());
    }

    #[test]
    fn test_wrong_type_in_time_domain() {
        let mut node = NodeData {
            domain: SignalDomain::Time(3),
            meta_table: HashMap::new(),
            signal_table: HashMap::new(),
        };

        node.signal_table.insert(
            "Fp1".to_string(),
            NodeValue::Matrix(MatrixData::FloatMat(Matrix::new(2, 2))),
        );

        assert!(node.validate().is_err());
    }

    #[test]
    fn test_valid_matrix_domain() {
        let mut node = NodeData {
            domain: SignalDomain::TimeFrequency(2, 3),
            meta_table: HashMap::new(),
            signal_table: HashMap::new(),
        };

        node.signal_table.insert(
            "spec_Fp1".to_string(),
            NodeValue::Matrix(MatrixData::FloatMat(Matrix::new(2, 3))),
        );

        assert!(node.validate().is_ok());
    }
}