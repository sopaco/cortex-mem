use super::QueryIntentType;

/// 三层检索权重（L0 / L1 / L2）
#[derive(Debug, Clone, Copy)]
pub struct LayerWeights {
    pub l0: f32,
    pub l1: f32,
    pub l2: f32,
}

impl Default for LayerWeights {
    fn default() -> Self {
        Self {
            l0: 0.2,
            l1: 0.3,
            l2: 0.5,
        }
    }
}

impl LayerWeights {
    /// 归一化权重（确保三者之和为 1.0）
    pub fn normalize(self) -> Self {
        let total = self.l0 + self.l1 + self.l2;
        if total <= 0.0 {
            return Self::default();
        }
        Self {
            l0: self.l0 / total,
            l1: self.l1 / total,
            l2: self.l2 / total,
        }
    }
}

/// 根据查询意图类型返回对应的动态权重
///
/// 权重策略：
/// - EntityLookup：极度倾向 L2 Detail（实体信息最可能在原始内容中）
/// - Factual：偏向 L2，但 L1 也有价值
/// - Temporal：L1 Overview 时间线归纳有优势，适当平衡
/// - Relational：L1 Overview 全局结构对比能力最强
/// - Search：L0 Abstract 宽泛定位，权重均衡
/// - General：默认均衡权重
pub fn weights_for_intent(intent_type: &QueryIntentType) -> LayerWeights {
    match intent_type {
        QueryIntentType::EntityLookup => LayerWeights {
            l0: 0.1,
            l1: 0.2,
            l2: 0.7,
        },
        QueryIntentType::Factual => LayerWeights {
            l0: 0.15,
            l1: 0.25,
            l2: 0.6,
        },
        QueryIntentType::Temporal => LayerWeights {
            l0: 0.2,
            l1: 0.35,
            l2: 0.45,
        },
        QueryIntentType::Relational => LayerWeights {
            l0: 0.2,
            l1: 0.5,
            l2: 0.3,
        },
        QueryIntentType::Search => LayerWeights {
            l0: 0.35,
            l1: 0.35,
            l2: 0.3,
        },
        QueryIntentType::General => LayerWeights::default(),
    }
}
