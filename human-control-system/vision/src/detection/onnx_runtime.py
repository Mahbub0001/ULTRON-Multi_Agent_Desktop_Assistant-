"""ONNX Runtime utilities and model management."""

import os
import numpy as np
from typing import List, Dict, Any, Optional, Tuple
import onnxruntime as ort


def get_available_providers() -> List[str]:
    """Get list of available ONNX Runtime execution providers."""
    return ort.get_available_providers()


def create_session(model_path: str, providers: Optional[List[str]] = None,
                   use_tensorrt: bool = False, tensorrt_cache_dir: str = "",
                   optimization_level: int = ort.GraphOptimizationLevel.ORT_ENABLE_ALL) -> ort.InferenceSession:
    """Create an ONNX Runtime inference session with optimal settings."""
    if providers is None:
        providers = get_available_providers()

    # Configure TensorRT provider if requested
    if use_tensorrt and "TensorrtExecutionProvider" in providers:
        trt_options = {
            "trt_engine_cache_enable": True,
            "trt_engine_cache_path": tensorrt_cache_dir or "./tensorrt_cache",
            "trt_fp16_enable": True,
            "trt_int8_enable": False,
        }
        providers = [("TensorrtExecutionProvider", trt_options)] + [p for p in providers if p != "TensorrtExecutionProvider"]

    sess_options = ort.SessionOptions()
    sess_options.graph_optimization_level = optimization_level
    sess_options.enable_cpu_mem_arena = True
    sess_options.enable_mem_pattern = True
    sess_options.execution_mode = ort.ExecutionMode.ORT_SEQUENTIAL

    return ort.InferenceSession(model_path, sess_options=sess_options, providers=providers)


def get_model_metadata(model_path: str) -> Dict[str, Any]:
    """Extract metadata from ONNX model."""
    import onnx
    model = onnx.load(model_path)
    metadata = {
        "ir_version": model.ir_version,
        "producer_name": model.producer_name,
        "producer_version": model.producer_version,
        "domain": model.domain,
        "model_version": model.model_version,
        "doc_string": model.doc_string,
        "inputs": [],
        "outputs": [],
        "metadata_props": {}
    }

    for inp in model.graph.input:
        metadata["inputs"].append({
            "name": inp.name,
            "type": str(inp.type.tensor_type.elem_type),
            "shape": [dim.dim_value if dim.dim_value > 0 else -1 for dim in inp.type.tensor_type.shape.dim]
        })

    for out in model.graph.output:
        metadata["outputs"].append({
            "name": out.name,
            "type": str(out.type.tensor_type.elem_type),
            "shape": [dim.dim_value if dim.dim_value > 0 else -1 for dim in out.type.tensor_type.shape.dim]
        })

    for prop in model.metadata_props:
        metadata["metadata_props"][prop.key] = prop.value

    return metadata


def optimize_model_for_inference(model_path: str, output_path: str,
                                  optimization_level: int = ort.GraphOptimizationLevel.ORT_ENABLE_ALL):
    """Optimize ONNX model for inference."""
    from onnxruntime.transformers import optimizer
    opt_model = optimizer.optimize_model(
        model_path,
        model_type="yolo",
        opt_level=optimization_level
    )
    opt_model.save_model_to_file(output_path)


class ModelManager:
    """Manage multiple ONNX models."""

    def __init__(self, model_dir: str = "models"):
        self.model_dir = model_dir
        self._sessions: Dict[str, ort.InferenceSession] = {}
        self._metadata: Dict[str, Dict[str, Any]] = {}

    def load_model(self, name: str, model_path: str, providers: Optional[List[str]] = None,
                   use_tensorrt: bool = False, tensorrt_cache_dir: str = "") -> ort.InferenceSession:
        """Load a model and cache the session."""
        if name in self._sessions:
            return self._sessions[name]

        full_path = os.path.join(self.model_dir, model_path) if not os.path.isabs(model_path) else model_path

        session = create_session(full_path, providers, use_tensorrt, tensorrt_cache_dir)
        self._sessions[name] = session
        self._metadata[name] = get_model_metadata(full_path)

        return session

    def get_session(self, name: str) -> Optional[ort.InferenceSession]:
        """Get a loaded session."""
        return self._sessions.get(name)

    def get_metadata(self, name: str) -> Optional[Dict[str, Any]]:
        """Get model metadata."""
        return self._metadata.get(name)

    def unload_model(self, name: str):
        """Unload a model."""
        if name in self._sessions:
            del self._sessions[name]
        if name in self._metadata:
            del self._metadata[name]

    def list_models(self) -> List[str]:
        """List loaded model names."""
        return list(self._sessions.keys())

    def close_all(self):
        """Close all sessions."""
        self._sessions.clear()
        self._metadata.clear()