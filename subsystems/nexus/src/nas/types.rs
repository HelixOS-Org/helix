//! Basic types and enums for Neural Architecture Search.

#![allow(dead_code)]

/// Operation types in the search space
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    /// No operation (zero)
    Zero,
    /// Skip connection (identity)
    Skip,
    /// 3x3 separable convolution
    SepConv3x3,
    /// 5x5 separable convolution
    SepConv5x5,
    /// 3x3 dilated convolution
    DilConv3x3,
    /// 5x5 dilated convolution
    DilConv5x5,
    /// 3x3 max pooling
    MaxPool3x3,
    /// 3x3 average pooling
    AvgPool3x3,
    /// Fully connected (linear)
    Linear,
    /// Linear with ReLU
    LinearReLU,
    /// Linear with GELU
    LinearGELU,
    /// Multi-head attention
    Attention,
    /// Layer normalization
    LayerNorm,
    /// Batch normalization
    BatchNorm,
    /// Squeeze and excitation
    SEBlock,
    /// Depthwise separable
    DepthwiseSep,
}

impl OperationType {
    /// Get FLOPs estimate for this operation
    pub fn estimated_flops(&self, input_size: usize, output_size: usize) -> u64 {
        match self {
            Self::Zero => 0,
            Self::Skip => 0,
            Self::Linear | Self::LinearReLU | Self::LinearGELU => {
                (input_size * output_size * 2) as u64
            },
            Self::Attention => (input_size * input_size * 3) as u64, // Q, K, V
            Self::LayerNorm | Self::BatchNorm => (input_size * 4) as u64,
            Self::SepConv3x3 | Self::DilConv3x3 => (input_size * 9 * 2) as u64,
            Self::SepConv5x5 | Self::DilConv5x5 => (input_size * 25 * 2) as u64,
            Self::MaxPool3x3 | Self::AvgPool3x3 => (input_size * 9) as u64,
            Self::SEBlock => (input_size * 4) as u64,
            Self::DepthwiseSep => (input_size * 9 + input_size * output_size) as u64,
        }
    }

    /// Get memory footprint for this operation
    #[inline]
    pub fn memory_bytes(&self, input_size: usize, output_size: usize) -> usize {
        match self {
            Self::Zero | Self::Skip => 0,
            Self::Linear | Self::LinearReLU | Self::LinearGELU => {
                input_size * output_size * 4 + output_size * 4 // weights + bias
            },
            Self::Attention => input_size * 4 * 4, // 4 projection matrices
            Self::LayerNorm | Self::BatchNorm => output_size * 8, // gamma + beta
            _ => output_size * 4,
        }
    }
}
