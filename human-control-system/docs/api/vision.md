# Vision Service API Reference

## Overview

The Vision Service (`hcs-vision`) provides computer vision capabilities including screen capture, object detection (YOLOv8), and OCR (PaddleOCR/Tesseract). It exposes gRPC services on port 50052 by default.

## Services

### VisionService

#### CaptureScreen
Capture a single screen frame.

```protobuf
rpc CaptureScreen(CaptureRequest) returns (CaptureResponse);
```

**Request:**
```json
{
  "monitor_index": 0,           // Monitor to capture (0 = primary)
  "region": {                   // Optional region (null = full screen)
    "x": 100,
    "y": 100,
    "width": 800,
    "height": 600
  },
  "window_title": "",           // Capture specific window by title
  "include_cursor": true,       // Include mouse cursor
  "format": "IMAGE_FORMAT_BGR"  // Output format
}
```

**Response:**
```json
{
  "image_data": "<bytes>",      // Raw image bytes
  "width": 1920,
  "height": 1080,
  "stride": 5760,               // Bytes per row (width * channels)
  "format": "IMAGE_FORMAT_BGR",
  "timestamp_us": 1700000000000,
  "monitor_index": 0
}
```

**Image Formats:**
- `IMAGE_FORMAT_BGR` (1): OpenCV default, 3 channels
- `IMAGE_FORMAT_RGB` (2): Standard RGB, 3 channels
- `IMAGE_FORMAT_RGBA` (3): With alpha, 4 channels
- `IMAGE_FORMAT_GRAY` (4): Grayscale, 1 channel
- `IMAGE_FORMAT_JPEG` (5): JPEG compressed
- `IMAGE_FORMAT_PNG` (6): PNG compressed

**Example:**
```bash
# Capture primary monitor
grpcurl -plaintext -d '{"monitor_index": 0, "format": "IMAGE_FORMAT_JPEG"}' \
  localhost:50052 hcs.vision.v1.VisionService/CaptureScreen | \
  jq -r '.image_data' | base64 -d > screenshot.jpg

# Capture region
grpcurl -plaintext -d '{"region": {"x": 0, "y": 0, "width": 1920, "height": 1080}}' \
  localhost:50052 hcs.vision.v1.VisionService/CaptureScreen
```

#### StreamFrames
Continuous screen capture stream (server-streaming RPC).

```protobuf
rpc StreamFrames(StreamFramesRequest) returns (stream FrameChunk);
```

**Request:**
```json
{
  "monitor_index": 0,
  "region": null,
  "target_fps": 30,
  "format": "IMAGE_FORMAT_BGR",
  "include_cursor": false
}
```

**Response Stream (FrameChunk):**
```json
{
  "image_data": "<bytes>",
  "width": 1920,
  "height": 1080,
  "stride": 5760,
  "format": "IMAGE_FORMAT_BGR",
  "timestamp_us": 1700000000000,
  "frame_number": 1
}
```

**Example:**
```bash
# Stream at 10 FPS
grpcurl -plaintext -d '{"target_fps": 10, "format": "IMAGE_FORMAT_JPEG"}' \
  localhost:50052 hcs.vision.v1.VisionService/StreamFrames
```

#### ListMonitors
List available monitors.

```protobuf
rpc ListMonitors(google.protobuf.Empty) returns (MonitorList);
```

**Response:**
```json
{
  "monitors": [
    {
      "index": 0,
      "name": "Dell U2719D",
      "x": 0,
      "y": 0,
      "width": 2560,
      "height": 1440,
      "is_primary": true,
      "scale_factor": 1.0
    },
    {
      "index": 1,
      "name": "LG 27GN950",
      "x": 2560,
      "y": 0,
      "width": 3840,
      "height": 2160,
      "is_primary": false,
      "scale_factor": 1.5
    }
  ]
}
```

---

#### DetectObjects
Run YOLOv8 object detection on an image.

```protobuf
rpc DetectObjects(DetectionRequest) returns (DetectionResponse);
```

**Request:**
```json
{
  "image_data": "<bytes>",      // Image from CaptureScreen or raw bytes
  "width": 1920,
  "height": 1080,
  "format": "IMAGE_FORMAT_BGR",
  "class_filter": ["person", "laptop", "cell phone"],  // Optional filter
  "confidence_threshold": 0.5,
  "iou_threshold": 0.45,
  "max_detections": 100
}
```

**Response:**
```json
{
  "detections": [
    {
      "class_name": "person",
      "class_id": 0,
      "confidence": 0.92,
      "bbox": { "x": 100.5, "y": 200.3, "width": 150.2, "height": 300.7 },
      "attributes": { "pose": "standing" }
    },
    {
      "class_name": "laptop",
      "class_id": 63,
      "confidence": 0.87,
      "bbox": { "x": 400.0, "y": 300.0, "width": 400.0, "height": 300.0 },
      "attributes": {}
    }
  ],
  "inference_time_us": 15000,
  "image_width": 1920,
  "image_height": 1080
}
```

**BoundingBox:** Normalized coordinates (0.0-1.0) relative to image dimensions.

**Example:**
```bash
# Detect objects in captured frame
grpcurl -plaintext -d '{
  "image_data": "'$(base64 -w0 screenshot.jpg)'",
  "width": 1920,
  "height": 1080,
  "format": "IMAGE_FORMAT_JPEG",
  "confidence_threshold": 0.5
}' localhost:50052 hcs.vision.v1.VisionService/DetectObjects
```

#### BatchDetectObjects
Run detection on multiple images.

```protobuf
rpc BatchDetectObjects(BatchDetectionRequest) returns (BatchDetectionResponse);
```

**Request:**
```json
{
  "requests": [
    { "image_data": "...", "width": 1920, "height": 1080, ... },
    { "image_data": "...", "width": 1920, "height": 1080, ... }
  ]
}
```

**Response:**
```json
{
  "responses": [
    { "detections": [...], "inference_time_us": 15000, ... },
    { "detections": [...], "inference_time_us": 14500, ... }
  ],
  "total_inference_time_us": 29500
}
```

---

#### RecognizeText
Run OCR on an image region.

```protobuf
rpc RecognizeText(OcrRequest) returns (OcrResponse);
```

**Request:**
```json
{
  "image_data": "<bytes>",
  "width": 1920,
  "height": 1080,
  "format": "IMAGE_FORMAT_BGR",
  "region": { "x": 100, "y": 100, "width": 500, "height": 200 },
  "languages": ["en", "zh"],
  "detect_orientation": true,
  "confidence_threshold": 0.5
}
```

**Response:**
```json
{
  "text_blocks": [
    {
      "text": "Hello World",
      "confidence": 0.98,
      "bbox": { "x": 110.0, "y": 115.0, "width": 200.0, "height": 30.0 },
      "polygon": [
        { "x": 110.0, "y": 115.0 },
        { "x": 310.0, "y": 115.0 },
        { "x": 310.0, "y": 145.0 },
        { "x": 110.0, "y": 145.0 }
      ],
      "language": "en"
    }
  ],
  "inference_time_us": 45000,
  "image_width": 1920,
  "image_height": 1080
}
```

**Example:**
```bash
# OCR on captured region
grpcurl -plaintext -d '{
  "image_data": "'$(base64 -w0 screenshot.jpg)'",
  "width": 1920,
  "height": 1080,
  "format": "IMAGE_FORMAT_JPEG",
  "region": {"x": 0, "y": 0, "width": 500, "height": 100},
  "languages": ["en"]
}' localhost:50052 hcs.vision.v1.VisionService/RecognizeText
```

---

#### GetModelInfo
Get loaded model information.

```protobuf
rpc GetModelInfo(google.protobuf.Empty) returns (ModelList);
```

**Response:**
```json
{
  "models": [
    {
      "name": "yolov8n",
      "version": "8.0.0",
      "path": "models/yolov8n.onnx",
      "device": "DEVICE_TYPE_TENSORRT",
      "loaded": true,
      "metadata": { "classes": "80", "input_shape": "1x3x640x640" }
    },
    {
      "name": "paddleocr_det",
      "version": "2.7.0",
      "path": "models/paddleocr/det",
      "device": "DEVICE_TYPE_CUDA",
      "loaded": true,
      "metadata": {}
    }
  ]
}
```

#### ReloadModels
Reload models (useful after updating model files).

```protobuf
rpc ReloadModels(ReloadModelsRequest) returns (google.protobuf.Empty);
```

**Request:**
```json
{
  "model_names": ["yolov8n", "paddleocr_rec"]  // Empty = reload all
}
```

---

#### HealthCheck
```protobuf
rpc HealthCheck(google.protobuf.Empty) returns (HealthCheckResponse);
```

**Response:**
```json
{
  "status": "SERVING_STATUS_SERVING",
  "version": "1.0.0",
  "uptime_seconds": 7200,
  "components": {
    "capture": { "status": "COMPONENT_STATUS_HEALTHY", "message": "DXGI capture ready" },
    "detection": { "status": "COMPONENT_STATUS_HEALTHY", "message": "YOLOv8n loaded on TensorRT" },
    "ocr": { "status": "COMPONENT_STATUS_HEALTHY", "message": "PaddleOCR loaded" },
    "models": { "status": "COMPONENT_STATUS_HEALTHY", "message": "3 models loaded" }
  }
}
```

#### GetConfig
Get current configuration.

```protobuf
rpc GetConfig(google.protobuf.Empty) returns (VisionConfig);
```

#### UpdateConfig
Update configuration at runtime.

```protobuf
rpc UpdateConfig(VisionConfig) returns (google.protobuf.Empty);
```

---

## Configuration Messages

### VisionConfig
```protobuf
message VisionConfig {
  CaptureConfig capture = 1;
  DetectionConfig detection = 2;
  OcrConfig ocr = 3;
}
```

### CaptureConfig
```protobuf
message CaptureConfig {
  int32 default_fps = 1;
  int32 default_monitor = 2;
  int32 jpeg_quality = 3;
  bool use_hardware_acceleration = 4;
}
```

### DetectionConfig
```protobuf
message DetectionConfig {
  string model_path = 1;
  DeviceType device = 2;
  int32 batch_size = 3;
  float default_confidence = 4;
  float default_iou = 5;
  int32 input_width = 6;
  int32 input_height = 7;
  bool use_tensorrt = 8;
  string tensorrt_cache_dir = 9;
}
```

### OcrConfig
```protobuf
message OcrConfig {
  string engine = 1;  // "paddle" or "tesseract"
  repeated string languages = 2;
  string paddle_det_model_dir = 3;
  string paddle_rec_model_dir = 4;
  string paddle_cls_model_dir = 5;
  bool use_gpu = 6;
  int32 gpu_id = 7;
  int32 cpu_threads = 8;
  bool enable_mkldnn = 9;
}
```

### DeviceType
- `DEVICE_TYPE_CPU` (1)
- `DEVICE_TYPE_CUDA` (2)
- `DEVICE_TYPE_TENSORRT` (3)
- `DEVICE_TYPE_COREML` (4)
- `DEVICE_TYPE_DIRECTML` (5)

---

## Client Examples

### Python Client
```python
import grpc
import base64
import cv2
import numpy as np
from hcs_vision_proto import vision_pb2, vision_pb2_grpc

class VisionClient:
    def __init__(self, host="localhost", port=50052):
        self.channel = grpc.insecure_channel(f"{host}:{port}")
        self.stub = vision_pb2_grpc.VisionServiceStub(self.channel)
    
    def capture(self, monitor=0, format=vision_pb2.IMAGE_FORMAT_BGR):
        req = vision_pb2.CaptureRequest(monitor_index=monitor, format=format)
        resp = self.stub.CaptureScreen(req)
        return self._bytes_to_image(resp)
    
    def _bytes_to_image(self, resp):
        if resp.format == vision_pb2.IMAGE_FORMAT_JPEG:
            nparr = np.frombuffer(resp.image_data, np.uint8)
            return cv2.imdecode(nparr, cv2.IMREAD_COLOR)
        elif resp.format == vision_pb2.IMAGE_FORMAT_BGR:
            img = np.frombuffer(resp.image_data, dtype=np.uint8)
            return img.reshape((resp.height, resp.width, 3))
        # Add other formats...
        return None
    
    def detect(self, image, classes=None, conf=0.5):
        # Encode image
        _, buf = cv2.imencode('.jpg', image)
        req = vision_pb2.DetectionRequest(
            image_data=buf.tobytes(),
            width=image.shape[1],
            height=image.shape[0],
            format=vision_pb2.IMAGE_FORMAT_JPEG,
            class_filter=classes or [],
            confidence_threshold=conf
        )
        return self.stub.DetectObjects(req)
    
    def ocr(self, image, region=None, langs=["en"]):
        _, buf = cv2.imencode('.jpg', image)
        req = vision_pb2.OcrRequest(
            image_data=buf.tobytes(),
            width=image.shape[1],
            height=image.shape[0],
            format=vision_pb2.IMAGE_FORMAT_JPEG,
            region=region or vision_pb2.Region(),
            languages=langs
        )
        return self.stub.RecognizeText(req)
    
    def stream_frames(self, fps=10, callback=None):
        req = vision_pb2.StreamFramesRequest(target_fps=fps)
        for frame in self.stub.StreamFrames(req):
            img = self._bytes_to_image(frame)
            if callback:
                callback(img, frame.timestamp_us)

# Usage
client = VisionClient()

# Capture and detect
frame = client.capture()
detections = client.detect(frame, classes=["person", "laptop"])
for det in detections.detections:
    print(f"{det.class_name}: {det.confidence:.2f} at {det.bbox}")

# OCR
text_result = client.ocr(frame)
for block in text_result.text_blocks:
    print(f"Text: {block.text} (conf: {block.confidence:.2f})")
```

### Rust Client
```rust
use hcs_vision_proto::{
    vision_service_client::VisionServiceClient,
    CaptureRequest, DetectionRequest, OcrRequest, StreamFramesRequest,
    ImageFormat, Region,
};
use tonic::transport::Channel;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = VisionServiceClient::connect("http://localhost:50052").await?;
    
    // Capture screen
    let capture = client.capture_screen(CaptureRequest {
        monitor_index: 0,
        format: ImageFormat::ImageFormatBgr as i32,
        ..Default::default()
    }).await?;
    
    println!("Captured: {}x{}", capture.get_ref().width, capture.get_ref().height);
    
    // Detect objects
    let detection = client.detect_objects(DetectionRequest {
        image_data: capture.get_ref().image_data.clone(),
        width: capture.get_ref().width,
        height: capture.get_ref().height,
        format: ImageFormat::ImageFormatBgr as i32,
        confidence_threshold: 0.5,
        ..Default::default()
    }).await?;
    
    for det in detection.get_ref().detections {
        println!("{}: {:.2}% at ({:.0}, {:.0}) {:.0}x{:.0}",
            det.class_name, det.confidence * 100.0,
            det.bbox.x, det.bbox.y, det.bbox.width, det.bbox.height);
    }
    
    // OCR
    let ocr = client.recognize_text(OcrRequest {
        image_data: capture.get_ref().image_data,
        width: capture.get_ref().width,
        height: capture.get_ref().height,
        format: ImageFormat::ImageFormatBgr as i32,
        languages: vec!["en".into()],
        ..Default::default()
    }).await?;
    
    for block in ocr.get_ref().text_blocks {
        println!("OCR: {} ({:.1}%)", block.text, block.confidence * 100.0);
    }
    
    Ok(())
}
```

### Go Client
```go
package main

import (
	"context"
	"log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	
	visionpb "github.com/your-org/hcs/vision/proto"
)

func main() {
	conn, err := grpc.Dial("localhost:50052", grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		log.Fatal(err)
	}
	defer conn.Close()
	
	client := visionpb.NewVisionServiceClient(conn)
	
	// Capture
	capture, err := client.CaptureScreen(context.Background(), &visionpb.CaptureRequest{
		MonitorIndex: 0,
		Format:       visionpb.ImageFormat_IMAGE_FORMAT_JPEG,
	})
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("Captured: %dx%d, %d bytes", capture.Width, capture.Height, len(capture.ImageData))
	
	// Detect
	detection, err := client.DetectObjects(context.Background(), &visionpb.DetectionRequest{
		ImageData:          capture.ImageData,
		Width:              capture.Width,
		Height:             capture.Height,
		Format:             visionpb.ImageFormat_IMAGE_FORMAT_JPEG,
		ConfidenceThreshold: 0.5,
	})
	if err != nil {
		log.Fatal(err)
	}
	for _, det := range detection.Detections {
		log.Printf("Detected: %s (%.2f%%)", det.ClassName, det.Confidence*100)
	}
	
	// OCR
	ocr, err := client.RecognizeText(context.Background(), &visionpb.OcrRequest{
		ImageData:     capture.ImageData,
		Width:         capture.Width,
		Height:        capture.Height,
		Format:        visionpb.ImageFormat_IMAGE_FORMAT_JPEG,
		Languages:     []string{"en"},
		Region:        &visionpb.Region{X: 0, Y: 0, Width: 500, Height: 100},
	})
	if err != nil {
		log.Fatal(err)
	}
	for _, block := range ocr.TextBlocks {
		log.Printf("OCR: %s", block.Text)
	}
}
```

---

## Performance Tips

1. **Use JPEG format** for network transfer (smaller payload)
2. **Batch detections** when processing multiple frames
3. **Reuse connections** - keep gRPC channel open
4. **Set appropriate confidence thresholds** to reduce false positives
5. **Use TensorRT** on NVIDIA GPUs for 3-5x speedup
6. **Limit max_detections** to reduce response size

---

## Error Codes

| Code | Description |
|------|-------------|
| `NOT_FOUND` | Model not loaded |
| `INVALID_ARGUMENT` | Invalid image format/dimensions |
| `RESOURCE_EXHAUSTED` | GPU memory full |
| `UNAVAILABLE` | Service initializing |
| `INTERNAL` | Inference engine error |

---

## Model Management

### Supported Models
- **YOLOv8:** n, s, m, l, x variants (ONNX format)
- **PaddleOCR:** Detection, Recognition, Classification models
- **Tesseract:** Standard traineddata files

### Adding Custom Models
1. Place ONNX model in `models/` directory
2. Update `vision.toml` with new model path
3. Call `ReloadModels` or restart service

### TensorRT Optimization
- First run builds TensorRT engine (takes 30-60s)
- Engines cached in `tensorrt_cache_dir`
- Subsequent runs use cached engine instantly
- Invalidate cache when model or TensorRT version changes

---

## Changelog

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial release |
| 1.1.0 | Added BatchDetectObjects, TensorRT support |
| 1.2.0 | Added CoreML/DirectML device types, orientation detection |