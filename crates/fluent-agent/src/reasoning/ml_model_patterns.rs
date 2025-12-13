//! ML Model Conversion and Optimization Pattern Detection
//!
//! This module provides pattern detection and guidance for ML model conversion tasks,
//! helping agents convert models between frameworks, optimize for deployment, and
//! apply quantization/pruning techniques effectively.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ML frameworks for model conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MLFramework {
    PyTorch,
    TensorFlow,
    Keras,
    ONNX,
    TensorRT,
    CoreML,
    TFLite,
    OpenVINO,
    JAX,
    MXNet,
    Caffe,
    Caffe2,
    PaddlePaddle,
    NCNN,
    TVM,
    Triton,
    MLX,
    SafeTensors,
}

impl MLFramework {
    /// Get framework keywords for detection
    pub fn keywords(&self) -> Vec<&'static str> {
        match self {
            MLFramework::PyTorch => vec!["pytorch", "torch", ".pt", ".pth", "torchscript", ".ckpt"],
            MLFramework::TensorFlow => vec![
                "tensorflow",
                "tf",
                ".pb",
                ".h5",
                "savedmodel",
                "saved_model",
            ],
            MLFramework::Keras => vec!["keras", ".keras", ".h5", "keras model"],
            MLFramework::ONNX => vec!["onnx", ".onnx", "open neural network"],
            MLFramework::TensorRT => vec!["tensorrt", "trt", ".engine", ".plan"],
            MLFramework::CoreML => vec!["coreml", ".mlmodel", ".mlpackage", "apple ml"],
            MLFramework::TFLite => vec!["tflite", "tensorflow lite", ".tflite", "flatbuffer"],
            MLFramework::OpenVINO => vec!["openvino", ".xml", ".bin", "intel inference"],
            MLFramework::JAX => vec!["jax", "flax", "haiku", ".npz"],
            MLFramework::MXNet => vec!["mxnet", "gluon", ".params"],
            MLFramework::Caffe => vec!["caffe", ".caffemodel", ".prototxt"],
            MLFramework::Caffe2 => vec!["caffe2", "caffe 2"],
            MLFramework::PaddlePaddle => vec!["paddlepaddle", "paddle", ".pdmodel"],
            MLFramework::NCNN => vec!["ncnn", ".ncnn", "tencent ncnn"],
            MLFramework::TVM => vec!["tvm", "apache tvm", "relay"],
            MLFramework::Triton => vec!["triton", "nvidia triton", "triton server"],
            MLFramework::MLX => vec!["mlx", "apple mlx", ".mlx"],
            MLFramework::SafeTensors => vec!["safetensors", ".safetensors", "safe tensors"],
        }
    }

    /// Get file extensions typically associated with this framework
    pub fn file_extensions(&self) -> Vec<&'static str> {
        match self {
            MLFramework::PyTorch => vec![".pt", ".pth", ".ckpt", ".bin"],
            MLFramework::TensorFlow => vec![".pb", ".h5", ".tf"],
            MLFramework::Keras => vec![".keras", ".h5"],
            MLFramework::ONNX => vec![".onnx"],
            MLFramework::TensorRT => vec![".engine", ".plan", ".trt"],
            MLFramework::CoreML => vec![".mlmodel", ".mlpackage"],
            MLFramework::TFLite => vec![".tflite"],
            MLFramework::OpenVINO => vec![".xml", ".bin"],
            MLFramework::JAX => vec![".npz", ".msgpack"],
            MLFramework::MXNet => vec![".params", ".json"],
            MLFramework::Caffe => vec![".caffemodel", ".prototxt"],
            MLFramework::Caffe2 => vec![".pb"],
            MLFramework::PaddlePaddle => vec![".pdmodel", ".pdiparams"],
            MLFramework::NCNN => vec![".ncnn.param", ".ncnn.bin"],
            MLFramework::TVM => vec![".so", ".tar"],
            MLFramework::Triton => vec![".savedmodel", ".plan", ".onnx"],
            MLFramework::MLX => vec![".mlx", ".npz"],
            MLFramework::SafeTensors => vec![".safetensors"],
        }
    }

    /// Get primary language for this framework
    pub fn primary_language(&self) -> &'static str {
        match self {
            MLFramework::PyTorch
            | MLFramework::TensorFlow
            | MLFramework::Keras
            | MLFramework::ONNX
            | MLFramework::JAX
            | MLFramework::MXNet
            | MLFramework::PaddlePaddle
            | MLFramework::TVM
            | MLFramework::SafeTensors => "Python",
            MLFramework::TensorRT
            | MLFramework::OpenVINO
            | MLFramework::NCNN
            | MLFramework::Caffe
            | MLFramework::Caffe2
            | MLFramework::Triton => "C++/Python",
            MLFramework::CoreML | MLFramework::MLX => "Swift/Python",
            MLFramework::TFLite => "Java/Python/C++",
        }
    }
}

/// Categories of ML model conversion challenges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversionCategory {
    /// Framework-to-framework conversion
    FrameworkConversion,
    /// Quantization (FP32 → INT8, etc.)
    Quantization,
    /// Model pruning and sparsification
    Pruning,
    /// Knowledge distillation
    Distillation,
    /// Graph optimization
    GraphOptimization,
    /// Operator fusion
    OperatorFusion,
    /// Dynamic shape handling
    DynamicShapes,
    /// Custom operators
    CustomOperators,
    /// Batch size optimization
    BatchOptimization,
    /// Memory optimization
    MemoryOptimization,
    /// Platform-specific deployment
    PlatformDeployment,
    /// Model serialization
    Serialization,
}

/// Quantization precision levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuantizationLevel {
    FP32,
    FP16,
    BF16,
    INT8,
    INT4,
    Mixed,
}

impl QuantizationLevel {
    /// Get relative model size compared to FP32
    pub fn size_ratio(&self) -> f64 {
        match self {
            QuantizationLevel::FP32 => 1.0,
            QuantizationLevel::FP16 | QuantizationLevel::BF16 => 0.5,
            QuantizationLevel::INT8 => 0.25,
            QuantizationLevel::INT4 => 0.125,
            QuantizationLevel::Mixed => 0.3, // Approximate
        }
    }

    /// Get potential accuracy impact description
    pub fn accuracy_impact(&self) -> &'static str {
        match self {
            QuantizationLevel::FP32 => "No impact (baseline)",
            QuantizationLevel::FP16 => "Minimal (<0.1% typical)",
            QuantizationLevel::BF16 => "Minimal, better for training",
            QuantizationLevel::INT8 => "Low (0.5-2% typical, may need calibration)",
            QuantizationLevel::INT4 => "Moderate (2-5%, requires careful calibration)",
            QuantizationLevel::Mixed => "Varies by layer configuration",
        }
    }
}

/// Guidance for ML model conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionGuidance {
    /// General approach description
    pub approach: String,
    /// Steps for the conversion process
    pub steps: Vec<String>,
    /// Required dependencies and tools
    pub dependencies: Vec<String>,
    /// Code snippet for conversion
    pub code_example: Option<String>,
    /// Common pitfalls to avoid
    pub pitfalls: Vec<String>,
    /// Validation steps after conversion
    pub validation_steps: Vec<String>,
    /// Performance expectations
    pub performance_notes: Vec<String>,
}

/// A specific framework pair conversion pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConversionPattern {
    /// Source framework
    pub source: MLFramework,
    /// Target framework
    pub target: MLFramework,
    /// Name of this conversion pattern
    pub name: String,
    /// Keywords that identify this pattern
    pub keywords: Vec<String>,
    /// Detection confidence
    pub confidence: f64,
    /// Detailed conversion guidance
    pub guidance: ConversionGuidance,
    /// Specific challenges for this pair
    pub challenges: Vec<ConversionCategory>,
    /// Whether this conversion path is well-supported
    pub is_well_supported: bool,
    /// Alternative paths if direct conversion is not supported
    pub alternative_paths: Vec<Vec<MLFramework>>,
}

/// Result of ML conversion pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLConversionDetectionResult {
    /// Whether this task involves ML model conversion
    pub is_conversion_task: bool,
    /// Detected source framework
    pub source_framework: Option<MLFramework>,
    /// Detected target framework
    pub target_framework: Option<MLFramework>,
    /// Detected quantization requirements
    pub quantization: Option<QuantizationLevel>,
    /// Detected optimization categories
    pub optimization_categories: Vec<ConversionCategory>,
    /// Matching conversion patterns
    pub matching_patterns: Vec<FrameworkConversionPattern>,
    /// Overall detection confidence
    pub confidence: f64,
    /// Augmented prompt with ML conversion context
    pub augmented_prompt: Option<String>,
}

/// Pattern detector for ML model conversion tasks
pub struct MLConversionPatternDetector {
    patterns: Vec<FrameworkConversionPattern>,
}

impl Default for MLConversionPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl MLConversionPatternDetector {
    /// Create a new ML conversion pattern detector with built-in patterns
    pub fn new() -> Self {
        Self {
            patterns: Self::built_in_patterns(),
        }
    }

    /// Get built-in conversion patterns
    fn built_in_patterns() -> Vec<FrameworkConversionPattern> {
        vec![
            // PyTorch → ONNX (most common conversion)
            FrameworkConversionPattern {
                source: MLFramework::PyTorch,
                target: MLFramework::ONNX,
                name: "PyTorch to ONNX Export".to_string(),
                keywords: vec![
                    "pytorch to onnx".to_string(),
                    "torch.onnx.export".to_string(),
                    "export onnx".to_string(),
                    "convert pytorch onnx".to_string(),
                ],
                confidence: 0.9,
                guidance: ConversionGuidance {
                    approach: "Use torch.onnx.export() with proper input shapes and dynamic axes configuration".to_string(),
                    steps: vec![
                        "Load the PyTorch model and set to eval mode".to_string(),
                        "Create dummy input with correct shape and dtype".to_string(),
                        "Define dynamic_axes for variable dimensions (batch size, sequence length)".to_string(),
                        "Export using torch.onnx.export() with opset_version >= 11".to_string(),
                        "Validate with onnx.checker.check_model()".to_string(),
                        "Compare outputs between PyTorch and ONNX Runtime".to_string(),
                    ],
                    dependencies: vec![
                        "torch".to_string(),
                        "onnx".to_string(),
                        "onnxruntime".to_string(),
                    ],
                    code_example: Some(r#"import torch
import onnx
import onnxruntime as ort

# Load model
model = MyModel()
model.load_state_dict(torch.load("model.pt"))
model.eval()

# Create dummy input
dummy_input = torch.randn(1, 3, 224, 224)

# Export to ONNX
torch.onnx.export(
    model,
    dummy_input,
    "model.onnx",
    input_names=["input"],
    output_names=["output"],
    dynamic_axes={"input": {0: "batch_size"}, "output": {0: "batch_size"}},
    opset_version=13
)

# Validate
onnx_model = onnx.load("model.onnx")
onnx.checker.check_model(onnx_model)

# Compare outputs
session = ort.InferenceSession("model.onnx")
onnx_output = session.run(None, {"input": dummy_input.numpy()})[0]
torch_output = model(dummy_input).detach().numpy()
assert np.allclose(torch_output, onnx_output, rtol=1e-3, atol=1e-5)"#.to_string()),
                    pitfalls: vec![
                        "Not setting model to eval mode before export".to_string(),
                        "Missing dynamic_axes for variable-length dimensions".to_string(),
                        "Using unsupported operations (check ONNX opset version)".to_string(),
                        "Not handling custom layers with symbolic functions".to_string(),
                        "Forgetting to freeze batch normalization statistics".to_string(),
                    ],
                    validation_steps: vec![
                        "Run onnx.checker.check_model() for structural validity".to_string(),
                        "Compare numerical outputs with torch model".to_string(),
                        "Test with different batch sizes if dynamic".to_string(),
                        "Profile inference time with onnxruntime".to_string(),
                    ],
                    performance_notes: vec![
                        "ONNX Runtime is typically faster than PyTorch for inference".to_string(),
                        "Use onnxruntime-gpu for GPU acceleration".to_string(),
                        "Consider ONNX graph optimization for further speedup".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::FrameworkConversion,
                    ConversionCategory::DynamicShapes,
                    ConversionCategory::CustomOperators,
                ],
                is_well_supported: true,
                alternative_paths: vec![],
            },

            // ONNX → TensorRT (high-performance inference)
            FrameworkConversionPattern {
                source: MLFramework::ONNX,
                target: MLFramework::TensorRT,
                name: "ONNX to TensorRT Engine".to_string(),
                keywords: vec![
                    "onnx to tensorrt".to_string(),
                    "trtexec".to_string(),
                    "tensorrt engine".to_string(),
                    "nvidia optimization".to_string(),
                ],
                confidence: 0.9,
                guidance: ConversionGuidance {
                    approach: "Use trtexec CLI or TensorRT Python API to build optimized engine".to_string(),
                    steps: vec![
                        "Validate ONNX model with onnx.checker".to_string(),
                        "Simplify ONNX graph with onnx-simplifier".to_string(),
                        "Create TensorRT builder and network".to_string(),
                        "Parse ONNX model into TensorRT network".to_string(),
                        "Configure builder settings (FP16, INT8, workspace)".to_string(),
                        "Build and serialize engine".to_string(),
                        "Validate inference outputs".to_string(),
                    ],
                    dependencies: vec![
                        "tensorrt".to_string(),
                        "pycuda".to_string(),
                        "onnx".to_string(),
                        "onnx-simplifier".to_string(),
                    ],
                    code_example: Some(r#"import tensorrt as trt
import pycuda.driver as cuda
import pycuda.autoinit

TRT_LOGGER = trt.Logger(trt.Logger.WARNING)

# Build engine from ONNX
def build_engine(onnx_path, fp16=True):
    builder = trt.Builder(TRT_LOGGER)
    network = builder.create_network(
        1 << int(trt.NetworkDefinitionCreationFlag.EXPLICIT_BATCH)
    )
    parser = trt.OnnxParser(network, TRT_LOGGER)

    with open(onnx_path, 'rb') as f:
        if not parser.parse(f.read()):
            for error in range(parser.num_errors):
                print(parser.get_error(error))
            return None

    config = builder.create_builder_config()
    config.set_memory_pool_limit(trt.MemoryPoolType.WORKSPACE, 1 << 30)

    if fp16:
        config.set_flag(trt.BuilderFlag.FP16)

    engine = builder.build_serialized_network(network, config)
    return engine

# Save engine
engine = build_engine("model.onnx")
with open("model.engine", "wb") as f:
    f.write(engine)"#.to_string()),
                    pitfalls: vec![
                        "Not matching CUDA and TensorRT versions".to_string(),
                        "Unsupported ONNX operators for TensorRT".to_string(),
                        "Not setting sufficient workspace memory".to_string(),
                        "Dynamic shapes require optimization profiles".to_string(),
                        "INT8 calibration needs representative dataset".to_string(),
                    ],
                    validation_steps: vec![
                        "Compare outputs with original ONNX model".to_string(),
                        "Benchmark latency and throughput".to_string(),
                        "Test with different batch sizes".to_string(),
                        "Verify memory consumption".to_string(),
                    ],
                    performance_notes: vec![
                        "TensorRT provides 2-10x speedup over ONNX Runtime on NVIDIA GPUs".to_string(),
                        "FP16 provides ~2x speedup with minimal accuracy loss".to_string(),
                        "INT8 can provide ~4x speedup but requires calibration".to_string(),
                        "Engine is hardware-specific (rebuild for different GPUs)".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::FrameworkConversion,
                    ConversionCategory::OperatorFusion,
                    ConversionCategory::Quantization,
                    ConversionCategory::MemoryOptimization,
                ],
                is_well_supported: true,
                alternative_paths: vec![],
            },

            // PyTorch → TFLite (mobile deployment)
            FrameworkConversionPattern {
                source: MLFramework::PyTorch,
                target: MLFramework::TFLite,
                name: "PyTorch to TFLite Conversion".to_string(),
                keywords: vec![
                    "pytorch to tflite".to_string(),
                    "torch to tensorflow lite".to_string(),
                    "mobile deployment".to_string(),
                    "android pytorch".to_string(),
                ],
                confidence: 0.85,
                guidance: ConversionGuidance {
                    approach: "Convert PyTorch → ONNX → TensorFlow → TFLite using tf2onnx and TFLite converter".to_string(),
                    steps: vec![
                        "Export PyTorch model to ONNX".to_string(),
                        "Convert ONNX to TensorFlow SavedModel using tf2onnx".to_string(),
                        "Use TFLite converter to create .tflite file".to_string(),
                        "Apply post-training quantization if needed".to_string(),
                        "Test with TFLite interpreter".to_string(),
                    ],
                    dependencies: vec![
                        "torch".to_string(),
                        "onnx".to_string(),
                        "onnx-tf".to_string(),
                        "tensorflow".to_string(),
                    ],
                    code_example: Some(r#"import torch
import onnx
from onnx_tf.backend import prepare
import tensorflow as tf

# Step 1: PyTorch → ONNX
model.eval()
dummy_input = torch.randn(1, 3, 224, 224)
torch.onnx.export(model, dummy_input, "model.onnx", opset_version=13)

# Step 2: ONNX → TensorFlow
onnx_model = onnx.load("model.onnx")
tf_rep = prepare(onnx_model)
tf_rep.export_graph("saved_model")

# Step 3: TensorFlow → TFLite
converter = tf.lite.TFLiteConverter.from_saved_model("saved_model")
converter.optimizations = [tf.lite.Optimize.DEFAULT]
converter.target_spec.supported_types = [tf.float16]  # FP16 quantization
tflite_model = converter.convert()

with open("model.tflite", "wb") as f:
    f.write(tflite_model)

# Validate
interpreter = tf.lite.Interpreter(model_path="model.tflite")
interpreter.allocate_tensors()"#.to_string()),
                    pitfalls: vec![
                        "Not all PyTorch ops have TFLite equivalents".to_string(),
                        "Shape inference issues during ONNX-TF conversion".to_string(),
                        "Dynamic shapes not well supported in TFLite".to_string(),
                        "Quantization may significantly impact accuracy".to_string(),
                    ],
                    validation_steps: vec![
                        "Compare outputs at each conversion stage".to_string(),
                        "Test on target mobile device".to_string(),
                        "Measure latency and memory on device".to_string(),
                        "Validate with edge cases and different inputs".to_string(),
                    ],
                    performance_notes: vec![
                        "Consider using ONNX Runtime Mobile as an alternative".to_string(),
                        "TFLite delegates (GPU, NNAPI) provide hardware acceleration".to_string(),
                        "INT8 quantization reduces size by 4x".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::FrameworkConversion,
                    ConversionCategory::Quantization,
                    ConversionCategory::PlatformDeployment,
                ],
                is_well_supported: false,
                alternative_paths: vec![
                    vec![MLFramework::PyTorch, MLFramework::ONNX, MLFramework::TFLite],
                ],
            },

            // PyTorch → CoreML (Apple deployment)
            FrameworkConversionPattern {
                source: MLFramework::PyTorch,
                target: MLFramework::CoreML,
                name: "PyTorch to CoreML Conversion".to_string(),
                keywords: vec![
                    "pytorch to coreml".to_string(),
                    "ios deployment".to_string(),
                    "apple neural engine".to_string(),
                    "coremltools".to_string(),
                ],
                confidence: 0.9,
                guidance: ConversionGuidance {
                    approach: "Use coremltools to convert PyTorch models directly or via TorchScript".to_string(),
                    steps: vec![
                        "Install coremltools with PyTorch support".to_string(),
                        "Trace or script the PyTorch model".to_string(),
                        "Convert using coremltools.convert()".to_string(),
                        "Specify compute_units for ANE optimization".to_string(),
                        "Set input/output descriptions and metadata".to_string(),
                        "Save as .mlpackage or .mlmodel".to_string(),
                    ],
                    dependencies: vec![
                        "torch".to_string(),
                        "coremltools".to_string(),
                    ],
                    code_example: Some(r#"import torch
import coremltools as ct

# Load and trace model
model = MyModel()
model.load_state_dict(torch.load("model.pt"))
model.eval()

example_input = torch.randn(1, 3, 224, 224)
traced_model = torch.jit.trace(model, example_input)

# Convert to CoreML
mlmodel = ct.convert(
    traced_model,
    inputs=[ct.TensorType(name="input", shape=example_input.shape)],
    compute_units=ct.ComputeUnit.ALL,  # Use ANE when available
    minimum_deployment_target=ct.target.iOS15,
)

# Add metadata
mlmodel.author = "Your Name"
mlmodel.short_description = "Image classification model"
mlmodel.input_description["input"] = "Input image"
mlmodel.output_description["output"] = "Classification probabilities"

# Save
mlmodel.save("model.mlpackage")"#.to_string()),
                    pitfalls: vec![
                        "Some PyTorch ops not supported by CoreML".to_string(),
                        "Dynamic shapes require enumerated shapes in CoreML".to_string(),
                        "Control flow (if/loops) may not convert correctly".to_string(),
                        "ANE has operator restrictions compared to GPU".to_string(),
                    ],
                    validation_steps: vec![
                        "Test with coremltools.models.MLModel predictions".to_string(),
                        "Compare numerical outputs with PyTorch".to_string(),
                        "Test on actual iOS/macOS device".to_string(),
                        "Profile with Instruments for ANE usage".to_string(),
                    ],
                    performance_notes: vec![
                        "ANE provides best battery efficiency on Apple devices".to_string(),
                        "FP16 is default and recommended for Apple Silicon".to_string(),
                        "Use mlpackage format for iOS 15+ for best performance".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::FrameworkConversion,
                    ConversionCategory::PlatformDeployment,
                    ConversionCategory::CustomOperators,
                ],
                is_well_supported: true,
                alternative_paths: vec![],
            },

            // TensorFlow → TFLite
            FrameworkConversionPattern {
                source: MLFramework::TensorFlow,
                target: MLFramework::TFLite,
                name: "TensorFlow to TFLite Conversion".to_string(),
                keywords: vec![
                    "tensorflow to tflite".to_string(),
                    "tf lite convert".to_string(),
                    "savedmodel to tflite".to_string(),
                    "keras to tflite".to_string(),
                ],
                confidence: 0.95,
                guidance: ConversionGuidance {
                    approach: "Use TFLiteConverter from SavedModel or Keras model with optional quantization".to_string(),
                    steps: vec![
                        "Save model as SavedModel format".to_string(),
                        "Create TFLiteConverter from saved model".to_string(),
                        "Configure optimizations and quantization".to_string(),
                        "Convert and save .tflite file".to_string(),
                        "Validate with TFLite interpreter".to_string(),
                    ],
                    dependencies: vec![
                        "tensorflow".to_string(),
                    ],
                    code_example: Some(r#"import tensorflow as tf

# From SavedModel
converter = tf.lite.TFLiteConverter.from_saved_model("saved_model_dir")

# Or from Keras model
# converter = tf.lite.TFLiteConverter.from_keras_model(model)

# Enable optimizations
converter.optimizations = [tf.lite.Optimize.DEFAULT]

# For full integer quantization (INT8)
def representative_dataset():
    for _ in range(100):
        yield [np.random.randn(1, 224, 224, 3).astype(np.float32)]

converter.representative_dataset = representative_dataset
converter.target_spec.supported_ops = [tf.lite.OpsSet.TFLITE_BUILTINS_INT8]
converter.inference_input_type = tf.uint8
converter.inference_output_type = tf.uint8

# Convert
tflite_model = converter.convert()

# Save
with open("model.tflite", "wb") as f:
    f.write(tflite_model)"#.to_string()),
                    pitfalls: vec![
                        "Custom ops need TFLite Select ops or custom implementation".to_string(),
                        "Dynamic tensor shapes limited support".to_string(),
                        "SparseTensor not fully supported".to_string(),
                        "Some TF ops have no TFLite equivalent".to_string(),
                    ],
                    validation_steps: vec![
                        "Run inference with TFLite interpreter".to_string(),
                        "Compare outputs with original TF model".to_string(),
                        "Test quantized model accuracy on validation set".to_string(),
                        "Benchmark on target device".to_string(),
                    ],
                    performance_notes: vec![
                        "TFLite is optimized for ARM CPUs and mobile GPUs".to_string(),
                        "Use GPU delegate for significant speedup".to_string(),
                        "NNAPI delegate enables Android neural engine".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::FrameworkConversion,
                    ConversionCategory::Quantization,
                    ConversionCategory::PlatformDeployment,
                ],
                is_well_supported: true,
                alternative_paths: vec![],
            },

            // Quantization pattern (generic)
            FrameworkConversionPattern {
                source: MLFramework::PyTorch,
                target: MLFramework::PyTorch,
                name: "PyTorch Quantization".to_string(),
                keywords: vec![
                    "quantize".to_string(),
                    "int8".to_string(),
                    "quantization".to_string(),
                    "reduce model size".to_string(),
                    "fp16".to_string(),
                    "mixed precision".to_string(),
                ],
                confidence: 0.85,
                guidance: ConversionGuidance {
                    approach: "Apply post-training quantization or quantization-aware training".to_string(),
                    steps: vec![
                        "Choose quantization approach (dynamic, static, QAT)".to_string(),
                        "Prepare model with torch.quantization.prepare".to_string(),
                        "Calibrate with representative data (for static)".to_string(),
                        "Convert using torch.quantization.convert".to_string(),
                        "Evaluate accuracy on validation set".to_string(),
                        "Fine-tune with QAT if accuracy drops".to_string(),
                    ],
                    dependencies: vec![
                        "torch".to_string(),
                    ],
                    code_example: Some(r#"import torch
from torch.quantization import quantize_dynamic, quantize_static, get_default_qconfig

# Option 1: Dynamic quantization (easiest, for RNNs/Transformers)
quantized_model = quantize_dynamic(
    model, {torch.nn.Linear, torch.nn.LSTM}, dtype=torch.qint8
)

# Option 2: Static quantization (best for CNNs)
model.qconfig = get_default_qconfig('fbgemm')  # or 'qnnpack' for ARM
model_prepared = torch.quantization.prepare(model)

# Calibrate with representative data
for data, _ in calibration_loader:
    model_prepared(data)

model_quantized = torch.quantization.convert(model_prepared)

# Option 3: Quantization-aware training (best accuracy)
model.qconfig = get_default_qconfig('fbgemm')
model_prepared = torch.quantization.prepare_qat(model.train())

# Train with fake quantization
for epoch in range(num_epochs):
    train(model_prepared, train_loader)

model_quantized = torch.quantization.convert(model_prepared.eval())"#.to_string()),
                    pitfalls: vec![
                        "Not all operations support quantization".to_string(),
                        "Batch normalization must be fused before quantization".to_string(),
                        "Calibration data must be representative".to_string(),
                        "Per-channel quantization often better than per-tensor".to_string(),
                    ],
                    validation_steps: vec![
                        "Compare model size before/after".to_string(),
                        "Measure inference speedup".to_string(),
                        "Evaluate accuracy on test set".to_string(),
                        "Profile operator coverage".to_string(),
                    ],
                    performance_notes: vec![
                        "INT8 typically gives 2-4x speedup on CPU".to_string(),
                        "Use 'fbgemm' backend for x86, 'qnnpack' for ARM".to_string(),
                        "Dynamic quantization is fastest to implement".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::Quantization,
                    ConversionCategory::MemoryOptimization,
                ],
                is_well_supported: true,
                alternative_paths: vec![],
            },

            // ONNX → OpenVINO
            FrameworkConversionPattern {
                source: MLFramework::ONNX,
                target: MLFramework::OpenVINO,
                name: "ONNX to OpenVINO Conversion".to_string(),
                keywords: vec![
                    "onnx to openvino".to_string(),
                    "intel inference".to_string(),
                    "model optimizer".to_string(),
                    "openvino convert".to_string(),
                ],
                confidence: 0.9,
                guidance: ConversionGuidance {
                    approach: "Use OpenVINO Model Optimizer or direct Python API conversion".to_string(),
                    steps: vec![
                        "Install OpenVINO toolkit".to_string(),
                        "Simplify ONNX model with onnx-simplifier".to_string(),
                        "Run model optimizer (mo) or use Python API".to_string(),
                        "Specify input shape and data type".to_string(),
                        "Apply FP16 or INT8 optimization".to_string(),
                        "Test with OpenVINO inference engine".to_string(),
                    ],
                    dependencies: vec![
                        "openvino".to_string(),
                        "onnx".to_string(),
                        "onnx-simplifier".to_string(),
                    ],
                    code_example: Some(r#"from openvino.tools import mo
from openvino.runtime import Core

# Method 1: Using Model Optimizer
# Command line: mo --input_model model.onnx --output_dir ./ir

# Method 2: Python API
from openvino.tools.mo import convert_model

ov_model = convert_model(
    "model.onnx",
    input_shape=[1, 3, 224, 224],
    compress_to_fp16=True
)

# Save IR format
from openvino.runtime import serialize
serialize(ov_model, "model.xml")

# Load and run inference
core = Core()
compiled_model = core.compile_model(ov_model, "CPU")
infer_request = compiled_model.create_infer_request()
result = infer_request.infer(input_tensor)"#.to_string()),
                    pitfalls: vec![
                        "Dynamic shapes require explicit range specification".to_string(),
                        "Some ONNX operators need custom extensions".to_string(),
                        "INT8 requires POT (Post-training Optimization Tool)".to_string(),
                        "IR format is OpenVINO version specific".to_string(),
                    ],
                    validation_steps: vec![
                        "Compare outputs with original ONNX model".to_string(),
                        "Benchmark on Intel CPU/GPU/VPU".to_string(),
                        "Test with OpenVINO Benchmark app".to_string(),
                        "Validate accuracy after compression".to_string(),
                    ],
                    performance_notes: vec![
                        "OpenVINO optimizes for Intel CPUs, GPUs, and VPUs".to_string(),
                        "FP16 provides ~2x throughput on Intel GPUs".to_string(),
                        "Use async inference for maximum throughput".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::FrameworkConversion,
                    ConversionCategory::Quantization,
                    ConversionCategory::PlatformDeployment,
                ],
                is_well_supported: true,
                alternative_paths: vec![],
            },

            // Hugging Face → ONNX (transformers)
            FrameworkConversionPattern {
                source: MLFramework::PyTorch,
                target: MLFramework::ONNX,
                name: "Hugging Face Transformers to ONNX".to_string(),
                keywords: vec![
                    "huggingface to onnx".to_string(),
                    "transformers onnx".to_string(),
                    "bert onnx".to_string(),
                    "optimum".to_string(),
                    "export transformer".to_string(),
                ],
                confidence: 0.9,
                guidance: ConversionGuidance {
                    approach: "Use Optimum library for standardized HuggingFace → ONNX conversion".to_string(),
                    steps: vec![
                        "Install optimum with onnxruntime backend".to_string(),
                        "Load model using ORTModelForXxx or export directly".to_string(),
                        "Specify task and opset version".to_string(),
                        "Handle tokenizer export if needed".to_string(),
                        "Optimize with ONNX Runtime graph optimizations".to_string(),
                        "Validate with sample inference".to_string(),
                    ],
                    dependencies: vec![
                        "optimum[onnxruntime]".to_string(),
                        "transformers".to_string(),
                        "onnx".to_string(),
                    ],
                    code_example: Some(r#"from optimum.onnxruntime import ORTModelForSequenceClassification
from transformers import AutoTokenizer

# Method 1: Direct loading with conversion
model = ORTModelForSequenceClassification.from_pretrained(
    "bert-base-uncased",
    export=True
)
model.save_pretrained("onnx_model")

# Method 2: Using optimum CLI
# optimum-cli export onnx --model bert-base-uncased --task text-classification onnx_model/

# Method 3: Manual export with better control
from optimum.exporters.onnx import main_export

main_export(
    "bert-base-uncased",
    output="onnx_model/",
    task="text-classification",
    opset=13,
    fp16=False,
)

# Run inference
tokenizer = AutoTokenizer.from_pretrained("bert-base-uncased")
inputs = tokenizer("Hello world", return_tensors="np")
outputs = model(**inputs)"#.to_string()),
                    pitfalls: vec![
                        "Past key values for decoder models need special handling".to_string(),
                        "Dynamic sequence lengths require careful axis specification".to_string(),
                        "Some custom model architectures may not export cleanly".to_string(),
                        "Token type IDs may be optional depending on model".to_string(),
                    ],
                    validation_steps: vec![
                        "Compare logits with original HuggingFace model".to_string(),
                        "Test with various input lengths".to_string(),
                        "Benchmark latency improvement".to_string(),
                        "Validate tokenizer compatibility".to_string(),
                    ],
                    performance_notes: vec![
                        "ONNX Runtime typically 2-3x faster than PyTorch for transformers".to_string(),
                        "Use ORTOptimizer for transformer-specific optimizations".to_string(),
                        "Quantization can provide additional 2-4x speedup".to_string(),
                    ],
                },
                challenges: vec![
                    ConversionCategory::FrameworkConversion,
                    ConversionCategory::DynamicShapes,
                    ConversionCategory::GraphOptimization,
                ],
                is_well_supported: true,
                alternative_paths: vec![],
            },
        ]
    }

    /// Detect ML conversion patterns in the given description
    pub fn detect(&self, description: &str) -> MLConversionDetectionResult {
        let lower_desc = description.to_lowercase();

        // Check if this is an ML conversion task
        let is_conversion_task = self.is_conversion_task(&lower_desc);

        if !is_conversion_task {
            return MLConversionDetectionResult {
                is_conversion_task: false,
                source_framework: None,
                target_framework: None,
                quantization: None,
                optimization_categories: vec![],
                matching_patterns: vec![],
                confidence: 0.0,
                augmented_prompt: None,
            };
        }

        // Detect source and target frameworks
        let source_framework = self.detect_framework(&lower_desc, true);
        let target_framework = self.detect_framework(&lower_desc, false);

        // Detect quantization requirements
        let quantization = self.detect_quantization(&lower_desc);

        // Detect optimization categories
        let optimization_categories = self.detect_categories(&lower_desc);

        // Find matching patterns
        let matching_patterns =
            self.find_matching_patterns(&lower_desc, source_framework, target_framework);

        // Calculate overall confidence
        let confidence = self.calculate_confidence(
            &matching_patterns,
            source_framework.is_some(),
            target_framework.is_some(),
            &optimization_categories,
        );

        // Generate augmented prompt
        let augmented_prompt = if confidence > 0.5 {
            Some(self.generate_augmented_prompt(
                description,
                &matching_patterns,
                source_framework,
                target_framework,
                quantization,
                &optimization_categories,
            ))
        } else {
            None
        };

        MLConversionDetectionResult {
            is_conversion_task,
            source_framework,
            target_framework,
            quantization,
            optimization_categories,
            matching_patterns,
            confidence,
            augmented_prompt,
        }
    }

    /// Check if the description is about ML model conversion
    fn is_conversion_task(&self, lower_desc: &str) -> bool {
        let strong_keywords = [
            "convert model",
            "export model",
            "model conversion",
            "onnx export",
            "to onnx",
            "to tflite",
            "to tensorrt",
            "to coreml",
            "to openvino",
            "quantize model",
            "quantization",
            "deploy model",
            "model optimization",
            "inference optimization",
            "model export",
            "framework conversion",
            "mixed precision",
            "fp16 training",
            "bf16 training",
        ];

        let context_keywords = [
            "convert", "export", "deploy", "optimize", "quantize", "compress",
        ];

        let ml_keywords = [
            "model",
            "neural network",
            "deep learning",
            "machine learning",
            "inference",
            "pytorch",
            "tensorflow",
            "onnx",
            "tflite",
            "coreml",
            "tensorrt",
        ];

        // Check for strong keywords
        let has_strong_keyword = strong_keywords.iter().any(|kw| lower_desc.contains(kw));

        // Check for context + ML keywords
        let has_context_keyword = context_keywords.iter().any(|kw| lower_desc.contains(kw));
        let has_ml_keyword = ml_keywords.iter().any(|kw| lower_desc.contains(kw));

        has_strong_keyword || (has_context_keyword && has_ml_keyword)
    }

    /// Detect framework from description
    fn detect_framework(&self, lower_desc: &str, is_source: bool) -> Option<MLFramework> {
        // Order matters: more specific frameworks (TFLite) must come before
        // more general ones (TensorFlow) to avoid incorrect matches
        let frameworks = [
            MLFramework::TFLite, // Must be before TensorFlow
            MLFramework::TensorRT,
            MLFramework::CoreML,
            MLFramework::OpenVINO,
            MLFramework::PyTorch,
            MLFramework::TensorFlow,
            MLFramework::Keras,
            MLFramework::ONNX,
            MLFramework::JAX,
            MLFramework::SafeTensors,
            MLFramework::MLX,
        ];

        let source_patterns = ["from ", "convert ", "export "];
        let target_patterns = [" to ", " into ", " for "];

        for framework in frameworks {
            for keyword in framework.keywords() {
                // Check with context patterns
                if is_source {
                    for pattern in source_patterns {
                        if lower_desc.contains(&format!("{}{}", pattern, keyword)) {
                            return Some(framework);
                        }
                    }
                } else {
                    for pattern in target_patterns {
                        if lower_desc.contains(&format!("{}{}", pattern, keyword)) {
                            return Some(framework);
                        }
                    }
                }

                // Check standalone keyword as fallback
                if contains_word(lower_desc, keyword) {
                    // Prioritize based on position for source vs target
                    let keyword_pos = lower_desc.find(keyword);
                    let conversion_words: Vec<_> = ["to", "into", "from", "convert", "export"]
                        .iter()
                        .filter_map(|w| lower_desc.find(w))
                        .collect();

                    if let (Some(kw_pos), Some(&conv_pos)) = (keyword_pos, conversion_words.first())
                    {
                        if (is_source && kw_pos < conv_pos) || (!is_source && kw_pos > conv_pos) {
                            return Some(framework);
                        }
                    }
                }
            }
        }

        None
    }

    /// Detect quantization requirements
    fn detect_quantization(&self, lower_desc: &str) -> Option<QuantizationLevel> {
        if lower_desc.contains("int4") || lower_desc.contains("4-bit") {
            Some(QuantizationLevel::INT4)
        } else if lower_desc.contains("int8") || lower_desc.contains("8-bit") {
            Some(QuantizationLevel::INT8)
        } else if lower_desc.contains("bf16") || lower_desc.contains("bfloat16") {
            Some(QuantizationLevel::BF16)
        } else if lower_desc.contains("fp16") || lower_desc.contains("half precision") {
            Some(QuantizationLevel::FP16)
        } else if lower_desc.contains("mixed precision") {
            Some(QuantizationLevel::Mixed)
        } else if lower_desc.contains("quantiz") {
            // Generic quantization mention defaults to INT8
            Some(QuantizationLevel::INT8)
        } else {
            None
        }
    }

    /// Detect conversion categories
    fn detect_categories(&self, lower_desc: &str) -> Vec<ConversionCategory> {
        let mut categories = Vec::new();

        let category_keywords: Vec<(ConversionCategory, &[&str])> = vec![
            (
                ConversionCategory::FrameworkConversion,
                &["convert", "export", "to onnx", "to tflite"],
            ),
            (
                ConversionCategory::Quantization,
                &["quantiz", "int8", "fp16", "reduce precision"],
            ),
            (
                ConversionCategory::Pruning,
                &["prun", "spars", "remove weights"],
            ),
            (
                ConversionCategory::Distillation,
                &["distill", "student", "teacher", "knowledge transfer"],
            ),
            (
                ConversionCategory::GraphOptimization,
                &["graph optim", "fusion", "optimize graph"],
            ),
            (
                ConversionCategory::OperatorFusion,
                &["fuse", "fusion", "operator fusion"],
            ),
            (
                ConversionCategory::DynamicShapes,
                &["dynamic shape", "variable batch", "variable length"],
            ),
            (
                ConversionCategory::CustomOperators,
                &["custom op", "custom layer", "plugin"],
            ),
            (
                ConversionCategory::BatchOptimization,
                &["batch size", "batching", "throughput"],
            ),
            (
                ConversionCategory::MemoryOptimization,
                &["memory", "reduce size", "smaller model"],
            ),
            (
                ConversionCategory::PlatformDeployment,
                &["deploy", "mobile", "edge", "embedded", "ios", "android"],
            ),
            (
                ConversionCategory::Serialization,
                &["save", "serialize", "checkpoint"],
            ),
        ];

        for (category, keywords) in category_keywords {
            if keywords.iter().any(|kw| lower_desc.contains(kw)) {
                categories.push(category);
            }
        }

        categories
    }

    /// Find matching conversion patterns
    fn find_matching_patterns(
        &self,
        lower_desc: &str,
        source: Option<MLFramework>,
        target: Option<MLFramework>,
    ) -> Vec<FrameworkConversionPattern> {
        self.patterns
            .iter()
            .filter(|pattern| {
                // Check keyword matches
                let keyword_match = pattern.keywords.iter().any(|kw| lower_desc.contains(kw));

                // Check framework matches
                let framework_match = match (source, target) {
                    (Some(s), Some(t)) => pattern.source == s && pattern.target == t,
                    (Some(s), None) => pattern.source == s,
                    (None, Some(t)) => pattern.target == t,
                    (None, None) => false,
                };

                keyword_match || framework_match
            })
            .cloned()
            .collect()
    }

    /// Calculate overall detection confidence
    fn calculate_confidence(
        &self,
        patterns: &[FrameworkConversionPattern],
        has_source: bool,
        has_target: bool,
        categories: &[ConversionCategory],
    ) -> f64 {
        let mut confidence = 0.0;

        // Pattern matches contribute most
        if !patterns.is_empty() {
            confidence += patterns
                .iter()
                .map(|p| p.confidence)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0)
                * 0.5;
        }

        // Framework detection contributes
        if has_source {
            confidence += 0.2;
        }
        if has_target {
            confidence += 0.2;
        }

        // Category detection contributes
        if !categories.is_empty() {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }

    /// Generate augmented prompt with ML conversion context
    fn generate_augmented_prompt(
        &self,
        original: &str,
        patterns: &[FrameworkConversionPattern],
        source: Option<MLFramework>,
        target: Option<MLFramework>,
        quantization: Option<QuantizationLevel>,
        categories: &[ConversionCategory],
    ) -> String {
        let mut augmented = String::new();

        augmented.push_str("## ML Model Conversion Context\n\n");

        // Framework information
        if let Some(src) = source {
            augmented.push_str(&format!("**Source Framework**: {:?}\n", src));
            augmented.push_str(&format!("- Primary language: {}\n", src.primary_language()));
            augmented.push_str(&format!(
                "- File extensions: {}\n\n",
                src.file_extensions().join(", ")
            ));
        }

        if let Some(tgt) = target {
            augmented.push_str(&format!("**Target Framework**: {:?}\n", tgt));
            augmented.push_str(&format!("- Primary language: {}\n", tgt.primary_language()));
            augmented.push_str(&format!(
                "- File extensions: {}\n\n",
                tgt.file_extensions().join(", ")
            ));
        }

        // Quantization info
        if let Some(quant) = quantization {
            augmented.push_str(&format!("**Quantization**: {:?}\n", quant));
            augmented.push_str(&format!("- Size ratio vs FP32: {}x\n", quant.size_ratio()));
            augmented.push_str(&format!(
                "- Accuracy impact: {}\n\n",
                quant.accuracy_impact()
            ));
        }

        // Categories
        if !categories.is_empty() {
            augmented.push_str("**Optimization Categories**:\n");
            for cat in categories {
                augmented.push_str(&format!("- {:?}\n", cat));
            }
            augmented.push('\n');
        }

        // Pattern-specific guidance
        if !patterns.is_empty() {
            augmented.push_str("## Conversion Guidance\n\n");
            for pattern in patterns.iter().take(2) {
                augmented.push_str(&format!("### {}\n\n", pattern.name));
                augmented.push_str(&format!("**Approach**: {}\n\n", pattern.guidance.approach));

                augmented.push_str("**Steps**:\n");
                for (i, step) in pattern.guidance.steps.iter().enumerate() {
                    augmented.push_str(&format!("{}. {}\n", i + 1, step));
                }
                augmented.push('\n');

                augmented.push_str("**Dependencies**:\n");
                for dep in &pattern.guidance.dependencies {
                    augmented.push_str(&format!("- {}\n", dep));
                }
                augmented.push('\n');

                if let Some(code) = &pattern.guidance.code_example {
                    augmented.push_str("**Code Example**:\n```python\n");
                    augmented.push_str(code);
                    augmented.push_str("\n```\n\n");
                }

                augmented.push_str("**Common Pitfalls**:\n");
                for pitfall in &pattern.guidance.pitfalls {
                    augmented.push_str(&format!("- {}\n", pitfall));
                }
                augmented.push('\n');

                if !pattern.guidance.validation_steps.is_empty() {
                    augmented.push_str("**Validation Steps**:\n");
                    for step in &pattern.guidance.validation_steps {
                        augmented.push_str(&format!("- {}\n", step));
                    }
                    augmented.push('\n');
                }
            }
        }

        augmented.push_str("---\n\n");
        augmented.push_str("## Original Task\n\n");
        augmented.push_str(original);

        augmented
    }
}

/// Check if text contains keyword as a complete word (with word boundaries)
fn contains_word(text: &str, word: &str) -> bool {
    let text_bytes = text.as_bytes();
    let word_bytes = word.as_bytes();

    if word_bytes.is_empty() {
        return false;
    }

    let mut i = 0;
    while i <= text_bytes.len().saturating_sub(word_bytes.len()) {
        if let Some(pos) = text[i..].find(word) {
            let abs_pos = i + pos;

            // Check word boundary before
            let before_ok = abs_pos == 0 || !text_bytes[abs_pos - 1].is_ascii_alphanumeric();

            // Check word boundary after
            let after_pos = abs_pos + word.len();
            let after_ok =
                after_pos >= text_bytes.len() || !text_bytes[after_pos].is_ascii_alphanumeric();

            if before_ok && after_ok {
                return true;
            }
            i = abs_pos + 1;
        } else {
            break;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pytorch_to_onnx_detection() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Convert my PyTorch model to ONNX format");

        assert!(result.is_conversion_task);
        assert_eq!(result.source_framework, Some(MLFramework::PyTorch));
        assert_eq!(result.target_framework, Some(MLFramework::ONNX));
        assert!(result.confidence > 0.7);
        assert!(result.augmented_prompt.is_some());
    }

    #[test]
    fn test_quantization_detection() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Quantize my model to INT8 for faster inference");

        assert!(result.is_conversion_task);
        assert_eq!(result.quantization, Some(QuantizationLevel::INT8));
        assert!(result
            .optimization_categories
            .contains(&ConversionCategory::Quantization));
    }

    #[test]
    fn test_tflite_detection() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Deploy TensorFlow model to TFLite for mobile");

        assert!(result.is_conversion_task);
        assert_eq!(result.source_framework, Some(MLFramework::TensorFlow));
        assert_eq!(result.target_framework, Some(MLFramework::TFLite));
        assert!(result
            .optimization_categories
            .contains(&ConversionCategory::PlatformDeployment));
    }

    #[test]
    fn test_coreml_detection() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Export PyTorch model to CoreML for iOS app");

        assert!(result.is_conversion_task);
        assert_eq!(result.source_framework, Some(MLFramework::PyTorch));
        assert_eq!(result.target_framework, Some(MLFramework::CoreML));
    }

    #[test]
    fn test_tensorrt_detection() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Convert ONNX to TensorRT engine for NVIDIA GPU");

        assert!(result.is_conversion_task);
        assert_eq!(result.source_framework, Some(MLFramework::ONNX));
        assert_eq!(result.target_framework, Some(MLFramework::TensorRT));
    }

    #[test]
    fn test_not_ml_task() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Write a function to calculate fibonacci numbers");

        assert!(!result.is_conversion_task);
        assert!(result.confidence < 0.3);
        assert!(result.augmented_prompt.is_none());
    }

    #[test]
    fn test_fp16_quantization() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Convert model to FP16 for faster inference");

        assert!(result.is_conversion_task);
        assert_eq!(result.quantization, Some(QuantizationLevel::FP16));
    }

    #[test]
    fn test_huggingface_onnx() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Export BERT model from HuggingFace to ONNX using optimum");

        assert!(result.is_conversion_task);
        assert!(!result.matching_patterns.is_empty());
        // Should match the HuggingFace to ONNX pattern
        let has_hf_pattern = result
            .matching_patterns
            .iter()
            .any(|p| p.name.contains("Hugging Face"));
        assert!(has_hf_pattern);
    }

    #[test]
    fn test_framework_keywords() {
        assert!(MLFramework::PyTorch.keywords().contains(&"pytorch"));
        assert!(MLFramework::TensorFlow.keywords().contains(&"tensorflow"));
        assert!(MLFramework::ONNX.keywords().contains(&"onnx"));
        assert!(MLFramework::CoreML.keywords().contains(&"coreml"));
    }

    #[test]
    fn test_quantization_size_ratio() {
        assert_eq!(QuantizationLevel::FP32.size_ratio(), 1.0);
        assert_eq!(QuantizationLevel::FP16.size_ratio(), 0.5);
        assert_eq!(QuantizationLevel::INT8.size_ratio(), 0.25);
        assert_eq!(QuantizationLevel::INT4.size_ratio(), 0.125);
    }

    #[test]
    fn test_openvino_detection() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Convert ONNX model to OpenVINO for Intel deployment");

        assert!(result.is_conversion_task);
        assert_eq!(result.source_framework, Some(MLFramework::ONNX));
        assert_eq!(result.target_framework, Some(MLFramework::OpenVINO));
    }

    #[test]
    fn test_mixed_precision() {
        let detector = MLConversionPatternDetector::new();
        let result = detector.detect("Train model with mixed precision for better performance");

        assert!(result.is_conversion_task);
        assert_eq!(result.quantization, Some(QuantizationLevel::Mixed));
    }
}
