# Adapters Service API Reference

## Overview

The Adapters Service (`hcs-adapters`) provides application-specific integrations for Photoshop, Chrome, Games, and Window Management. Each adapter runs as a separate gRPC service on port 50054 with service-specific methods.

## Services

### PhotoshopService
Photoshop automation via UXP/CEP.

#### Document Operations

**CreateDocument**
```protobuf
rpc CreateDocument(CreateDocumentRequest) returns (DocumentResponse);
```
```json
{
  "width": 1920,
  "height": 1080,
  "resolution": 72.0,
  "color_mode": "COLOR_MODE_RGB",
  "background": "BACKGROUND_CONTENTS_WHITE",
  "name": "New Document"
}
```

**OpenDocument**
```protobuf
rpc OpenDocument(OpenDocumentRequest) returns (DocumentResponse);
```
```json
{ "path": "/path/to/document.psd" }
```

**SaveDocument**
```protobuf
rpc SaveDocument(SaveDocumentRequest) returns (DocumentResponse);
```
```json
{
  "document_id": "doc-123",
  "path": "/path/to/save.psd",
  "options": { "save_as_copy": false, "jpeg_quality": 90 }
}
```

**CloseDocument**
```protobuf
rpc CloseDocument(CloseDocumentRequest) returns (DocumentResponse);
```
```json
{ "document_id": "doc-123", "force": false }
```

**GetDocumentInfo**
```protobuf
rpc GetDocumentInfo(GetDocumentInfoRequest) returns (DocumentResponse);
```
```json
{ "document_id": "doc-123" }
```

**ListDocuments**
```protobuf
rpc ListDocuments(Empty) returns (DocumentList);
```

#### Layer Operations

**GetLayers**
```protobuf
rpc GetLayers(GetLayersRequest) returns (LayerList);
```
```json
{ "document_id": "doc-123" }
```

**CreateLayer**
```protobuf
rpc CreateLayer(CreateLayerRequest) returns (LayerResponse);
```
```json
{
  "document_id": "doc-123",
  "name": "New Layer",
  "kind": "LAYER_KIND_NORMAL",
  "insert_at": -1
}
```

**DeleteLayer**
```protobuf
rpc DeleteLayer(DeleteLayerRequest) returns (LayerResponse);
```
```json
{ "document_id": "doc-123", "layer_id": "layer-456" }
```

**DuplicateLayer**
```protobuf
rpc DuplicateLayer(DuplicateLayerRequest) returns (LayerResponse);
```
```json
{
  "document_id": "doc-123",
  "layer_id": "layer-456",
  "new_name": "Copy of Layer",
  "insert_at": -1
}
```

**MoveLayer**
```protobuf
rpc MoveLayer(MoveLayerRequest) returns (LayerResponse);
```
```json
{ "document_id": "doc-123", "layer_id": "layer-456", "new_index": 0 }
```

**SetLayerVisibility**
```protobuf
rpc SetLayerVisibility(SetLayerVisibilityRequest) returns (LayerResponse);
```
```json
{ "document_id": "doc-123", "layer_id": "layer-456", "visible": true }
```

**SetLayerOpacity**
```protobuf
rpc SetLayerOpacity(SetLayerOpacityRequest) returns (LayerResponse);
```
```json
{ "document_id": "doc-123", "layer_id": "layer-456", "opacity": 0.5 }
```

**SetLayerBlendMode**
```protobuf
rpc SetLayerBlendMode(SetLayerBlendModeRequest) returns (LayerResponse);
```
```json
{ "document_id": "doc-123", "layer_id": "layer-456", "mode": "BLEND_MODE_MULTIPLY" }
```

**GetLayerBounds**
```protobuf
rpc GetLayerBounds(GetLayerBoundsRequest) returns (LayerBoundsResponse);
```
```json
{ "document_id": "doc-123", "layer_id": "layer-456" }
```

**ApplyLayerStyle**
```protobuf
rpc ApplyLayerStyle(ApplyLayerStyleRequest) returns (LayerResponse);
```
```json
{
  "document_id": "doc-123",
  "layer_id": "layer-456",
  "style": {
    "drop_shadow": true,
    "drop_shadow_params": { "opacity": 0.5, "angle": 120, "distance": 5, "size": 10 }
  }
}
```

#### Adjustments & Filters

**ApplyAdjustment**
```protobuf
rpc ApplyAdjustment(ApplyAdjustmentRequest) returns (AdjustmentResponse);
```
```json
{
  "document_id": "doc-123",
  "layer_id": "layer-456",
  "type": "ADJUSTMENT_TYPE_BRIGHTNESS_CONTRAST",
  "parameters": { "brightness": "20", "contrast": "15" }
}
```

**ApplyFilter**
```protobuf
rpc ApplyFilter(ApplyFilterRequest) returns (FilterResponse);
```
```json
{
  "document_id": "doc-123",
  "layer_id": "layer-456",
  "type": "FILTER_TYPE_GAUSSIAN_BLUR",
  "parameters": { "radius": "5.0" },
  "smart_filter": true
}
```

#### Actions

**PlayAction**
```protobuf
rpc PlayAction(PlayActionRequest) returns (ActionResponse);
```
```json
{
  "action_name": "Vintage Effect",
  "action_set": "Default Actions",
  "document_id": "doc-123",
  "parameters": {}
}
```

**ListActions**
```protobuf
rpc ListActions(ListActionsRequest) returns (ActionList);
```
```json
{ "action_set": "Default Actions" }
```

#### Batch Processing

**BatchProcess**
```protobuf
rpc BatchProcess(BatchProcessRequest) returns (BatchProcessResponse);
```
```json
{
  "action_name": "Resize for Web",
  "action_set": "My Actions",
  "source_files": ["/input/img1.jpg", "/input/img2.jpg"],
  "destination_folder": "/output",
  "options": {
    "override_open": true,
    "include_subfolders": false,
    "file_naming": "Document Name + 2 Digit Serial"
  }
}
```

#### Export/Import

**ExportDocument**
```protobuf
rpc ExportDocument(ExportDocumentRequest) returns (ExportResponse);
```
```json
{
  "document_id": "doc-123",
  "path": "/output/image.png",
  "format": "EXPORT_FORMAT_PNG",
  "options": { "quality": 90, "transparency": true }
}
```

**ImportFile**
```protobuf
rpc ImportFile(ImportFileRequest) returns (ImportResponse);
```
```json
{
  "path": "/input/image.jpg",
  "document_id": "doc-123",
  "options": { "place_as_smart_object": true }
}
```

---

### ChromeService
Chrome automation via Chrome DevTools Protocol (CDP).

#### Browser Lifecycle

**Connect**
```protobuf
rpc Connect(ConnectRequest) returns (ConnectResponse);
```
```json
{ "endpoint": "http://localhost:9222", "timeout_ms": 10000 }
```

**Disconnect**
```protobuf
rpc Disconnect(DisconnectRequest) returns (Empty);
```
```json
{ "session_id": "session-123" }
```

**GetVersion**
```protobuf
rpc GetVersion(Empty) returns (VersionResponse);
```

**GetTargets**
```protobuf
rpc GetTargets(GetTargetsRequest) returns (TargetList);
```
```json
{ "session_id": "session-123" }
```

#### Tab Management

**CreateTab**
```protobuf
rpc CreateTab(CreateTabRequest) returns (TabResponse);
```
```json
{ "session_id": "session-123", "url": "https://example.com", "background": false }
```

**CloseTab**
```protobuf
rpc CloseTab(CloseTabRequest) returns (TabResponse);
```
```json
{ "session_id": "session-123", "target_id": "target-456" }
```

**ActivateTab**
```protobuf
rpc ActivateTab(ActivateTabRequest) returns (TabResponse);
```
```json
{ "session_id": "session-123", "target_id": "target-456" }
```

**Navigate**
```protobuf
rpc Navigate(NavigateRequest) returns (NavigationResponse);
```
```json
{
  "session_id": "session-123",
  "target_id": "target-456",
  "url": "https://google.com",
  "options": { "timeout_ms": 30000, "wait_until_networkidle": true }
}
```

**Reload**
```protobuf
rpc Reload(ReloadRequest) returns (NavigationResponse);
```
```json
{ "session_id": "session-123", "target_id": "target-456", "ignore_cache": false }
```

**GoBack/GoForward**
```protobuf
rpc GoBack(GoBackRequest) returns (NavigationResponse);
rpc GoForward(GoForwardRequest) returns (NavigationResponse);
```

**GetTabs**
```protobuf
rpc GetTabs(Empty) returns (TabList);
```

#### DOM Interaction

**Evaluate**
```protobuf
rpc Evaluate(EvaluateRequest) returns (EvaluateResponse);
```
```json
{
  "session_id": "session-123",
  "target_id": "target-456",
  "expression": "document.title",
  "options": { "return_by_value": true, "await_promise": false }
}
```

**CallFunction**
```protobuf
rpc CallFunction(CallFunctionRequest) returns (EvaluateResponse);
```
```json
{
  "session_id": "session-123",
  "target_id": "target-456",
  "function_declaration": "function click(selector) { document.querySelector(selector).click(); }",
  "arguments": [],
  "options": {}
}
```

**GetDocument**
```protobuf
rpc GetDocument(GetDocumentRequest) returns (NodeResponse);
```
```json
{ "session_id": "session-123", "target_id": "target-456", "depth": 2 }
```

**QuerySelector**
```protobuf
rpc QuerySelector(QuerySelectorRequest) returns (NodeResponse);
```
```json
{ "session_id": "session-123", "target_id": "target-456", "selector": "#search-input", "node_id": 0 }
```

**QuerySelectorAll**
```protobuf
rpc QuerySelectorAll(QuerySelectorAllRequest) returns (NodeList);
```
```json
{ "session_id": "session-123", "target_id": "target-456", "selector": ".result-item" }
```

**ClickElement**
```protobuf
rpc ClickElement(ClickElementRequest) returns (ClickResponse);
```
```json
{
  "session_id": "session-123",
  "target_id": "target-456",
  "node_id": 12345,
  "options": { "click_type": "CLICK_TYPE_CLICK", "button": 0 }
}
```

**TypeText**
```protobuf
rpc TypeText(TypeTextRequest) returns (TypeResponse);
```
```json
{
  "session_id": "session-123",
  "target_id": "target-456",
  "node_id": 12345,
  "text": "Hello World",
  "options": { "delay_ms": 50, "clear_first": true }
}
```

**GetElementBounds**
```protobuf
rpc GetElementBounds(GetElementBoundsRequest) returns (ElementBoundsResponse);
```

**ScreenshotElement**
```protobuf
rpc ScreenshotElement(ScreenshotElementRequest) returns (ScreenshotResponse);
```

#### Network Interception

**EnableNetwork**
```protobuf
rpc EnableNetwork(EnableNetworkRequest) returns (Empty);
```
```json
{ "session_id": "session-123", "patterns": ["*.api.*"] }
```

**SetRequestInterception**
```protobuf
rpc SetRequestInterception(SetRequestInterceptionRequest) returns (Empty);
```

**ContinueRequest**
```protobuf
rpc ContinueRequest(ContinueRequestRequest) returns (Empty);
```

**ModifyRequest**
```protobuf
rpc ModifyRequest(ModifyRequestRequest) returns (Empty);
```

**BlockUrls**
```protobuf
rpc BlockUrls(BlockUrlsRequest) returns (Empty);
```
```json
{ "session_id": "session-123", "url_patterns": ["*ads*", "*tracking*"] }
```

**GetNetworkLogs**
```protobuf
rpc GetNetworkLogs(Empty) returns (NetworkLogList);
```

#### Console

**EnableConsole/DisableConsole/GetConsoleLogs/ClearConsole**
```protobuf
rpc EnableConsole(Empty) returns (Empty);
rpc GetConsoleLogs(Empty) returns (ConsoleLogList);
```

#### Screenshots

**CaptureScreenshot**
```protobuf
rpc CaptureScreenshot(CaptureScreenshotRequest) returns (ScreenshotResponse);
```

**CaptureFullPageScreenshot**
```protobuf
rpc CaptureFullPageScreenshot(CaptureFullPageScreenshotRequest) returns (ScreenshotResponse);
```

#### Cookies/Storage

**GetCookies/SetCookie/DeleteCookie**
```protobuf
rpc GetCookies(GetCookiesRequest) returns (CookieList);
rpc SetCookie(SetCookieRequest) returns (CookieResponse);
rpc DeleteCookie(DeleteCookieRequest) returns (CookieResponse);
```

**GetLocalStorage/SetLocalStorage**
```protobuf
rpc GetLocalStorage(GetLocalStorageRequest) returns (StorageResponse);
rpc SetLocalStorage(SetLocalStorageRequest) returns (StorageResponse);
```

---

### GameService
Game memory manipulation and code injection.

#### Process Attachment

**AttachProcess**
```protobuf
rpc AttachProcess(AttachProcessRequest) returns (AttachResponse);
```
```json
{ "pid": 1234, "access": "ACCESS_RIGHTS_READ_WRITE" }
```

**DetachProcess**
```protobuf
rpc DetachProcess(DetachProcessRequest) returns (Empty);
```
```json
{ "pid": 1234 }
```

**GetProcessInfo**
```protobuf
rpc GetProcessInfo(GetProcessInfoRequest) returns (ProcessInfoResponse);
```
```json
{ "pid": 1234 }
```

**ListProcesses**
```protobuf
rpc ListProcesses(ListProcessesRequest) returns (ProcessList);
```
```json
{ "name_filter": "game" }
```

**FindProcessByName**
```protobuf
rpc FindProcessByName(FindProcessByNameRequest) returns (ProcessInfoResponse);
```
```json
{ "name": "game.exe" }
```

**FindProcessByWindow**
```protobuf
rpc FindProcessByWindow(FindProcessByWindowRequest) returns (ProcessInfoResponse);
```
```json
{ "window_title": "Game Window", "class_name": "GameClass" }
```

#### Memory Reading

**ReadMemory**
```protobuf
rpc ReadMemory(ReadMemoryRequest) returns (ReadMemoryResponse);
```
```json
{ "pid": 1234, "address": "0x140000000", "size": 4 }
```

**ReadPointerChain**
```protobuf
rpc ReadPointerChain(ReadPointerChainRequest) returns (ReadMemoryResponse);
```
```json
{
  "pid": 1234,
  "base_address": "0x140000000",
  "offsets": ["0x10", "0x20", "0x8"],
  "size": 8
}
```

**ReadString**
```protobuf
rpc ReadString(ReadStringRequest) returns (ReadStringResponse);
```
```json
{ "pid": 1234, "address": "0x140000000", "max_length": 256, "encoding": "STRING_ENCODING_UTF8" }
```

**ReadStruct**
```protobuf
rpc ReadStruct(ReadStructRequest) returns (ReadStructResponse);
```
```json
{
  "pid": 1234,
  "address": "0x140000000",
  "struct_definition": "{\"fields\": [{\"name\": \"health\", \"type\": \"int32\", \"offset\": 0}, {\"name\": \"mana\", \"type\": \"int32\", \"offset\": 4}]}"
}
```

#### Memory Writing

**WriteMemory**
```protobuf
rpc WriteMemory(WriteMemoryRequest) returns (WriteMemoryResponse);
```
```json
{ "pid": 1234, "address": "0x140000000", "data": "AQIDBA==" }
```

**WritePointerChain**
```protobuf
rpc WritePointerChain(WritePointerChainRequest) returns (WriteMemoryResponse);
```

**WriteString/WriteStruct**
```protobuf
rpc WriteString(WriteStringRequest) returns (WriteMemoryResponse);
rpc WriteStruct(WriteStructRequest) returns (WriteMemoryResponse);
```

#### Pattern Scanning

**ScanPattern**
```protobuf
rpc ScanPattern(ScanPatternRequest) returns (ScanPatternResponse);
```
```json
{
  "pid": 1234,
  "pattern": "48 8B 05 ?? ?? ?? ?? 48 85 C0",
  "start_address": "0x140000000",
  "end_address": "0x150000000",
  "options": { "executable": true, "max_results": 10 }
}
```

**ScanPatternModule**
```protobuf
rpc ScanPatternModule(ScanPatternModuleRequest) returns (ScanPatternResponse);
```

**FindPattern** (raw bytes with mask)
```protobuf
rpc FindPattern(FindPatternRequest) returns (FindPatternResponse);
```

#### Code Injection

**AllocateMemory**
```protobuf
rpc AllocateMemory(AllocateMemoryRequest) returns (AllocateMemoryResponse);
```
```json
{
  "pid": 1234,
  "size": 4096,
  "protection": "MEMORY_PROTECTION_READ_WRITE_EXECUTE",
  "type": "ALLOCATION_TYPE_COMMIT_RESERVE"
}
```

**FreeMemory**
```protobuf
rpc FreeMemory(FreeMemoryRequest) returns (Empty);
```

**InjectCode**
```protobuf
rpc InjectCode(InjectCodeRequest) returns (InjectCodeResponse);
```
```json
{
  "pid": 1234,
  "shellcode": "base64_encoded_shellcode",
  "options": { "create_thread": true, "restore_context": true }
}
```

**CreateThread**
```protobuf
rpc CreateThread(CreateThreadRequest) returns (CreateThreadResponse);
```

**InjectDll**
```protobuf
rpc InjectDll(InjectDllRequest) returns (InjectDllResponse);
```
```json
{
  "pid": 1234,
  "dll_path": "C:\\Mods\\my_mod.dll",
  "method": "INJECTION_METHOD_MANUAL_MAPPING"
}
```

**SetHook/RemoveHook**
```protobuf
rpc SetHook(SetHookRequest) returns (HookResponse);
rpc RemoveHook(RemoveHookRequest) returns (HookResponse);
```

#### Modules

**GetModules/GetModuleBase/GetModuleExport**
```protobuf
rpc GetModules(Empty) returns (ModuleList);
rpc GetModuleBase(GetModuleBaseRequest) returns (ModuleBaseResponse);
rpc GetModuleExport(GetModuleExportRequest) returns (ModuleExportResponse);
```

---

### WindowService
Cross-platform window management.

#### Window Enumeration

**ListWindows**
```protobuf
rpc ListWindows(ListWindowsRequest) returns (WindowList);
```
```json
{
  "visible_only": true,
  "enabled_only": false,
  "class_filter": "Chrome_WidgetWin_1",
  "title_filter": "Google Chrome"
}
```

**GetWindow**
```protobuf
rpc GetWindow(GetWindowRequest) returns (WindowResponse);
```
```json
{ "handle": 123456 }
```

**FindWindow**
```protobuf
rpc FindWindow(FindWindowRequest) returns (WindowResponse);
```
```json
{ "class_name": "Notepad", "window_title": "Untitled - Notepad" }
```

**GetForegroundWindow/GetDesktopWindow/GetShellWindow**
```protobuf
rpc GetForegroundWindow(Empty) returns (WindowResponse);
```

#### Window State

**FocusWindow/ActivateWindow**
```protobuf
rpc FocusWindow(FocusWindowRequest) returns (WindowResponse);
rpc ActivateWindow(ActivateWindowRequest) returns (WindowResponse);
```

**ShowWindow**
```protobuf
rpc ShowWindow(ShowWindowRequest) returns (WindowResponse);
```
```json
{ "handle": 123456, "command": "SHOW_COMMAND_NORMAL" }
```

**MinimizeWindow/MaximizeWindow/RestoreWindow/CloseWindow**
```protobuf
rpc MinimizeWindow(MinimizeWindowRequest) returns (WindowResponse);
```

**IsWindowVisible/IsWindowEnabled/GetWindowState**
```protobuf
rpc IsWindowVisible(IsWindowVisibleRequest) returns (WindowVisibilityResponse);
```

#### Position/Size

**MoveWindow/ResizeWindow/MoveResizeWindow**
```protobuf
rpc MoveWindow(MoveWindowRequest) returns (WindowResponse);
rpc ResizeWindow(ResizeWindowRequest) returns (WindowResponse);
rpc MoveResizeWindow(MoveResizeWindowRequest) returns (WindowResponse);
```

**GetWindowRect/GetClientRect**
```protobuf
rpc GetWindowRect(GetWindowRectRequest) returns (WindowRectResponse);
```

**SetWindowPos**
```protobuf
rpc SetWindowPos(SetWindowPosRequest) returns (WindowResponse);
```

#### Screenshots

**CaptureWindow/CaptureClientArea/CaptureRegion**
```protobuf
rpc CaptureWindow(CaptureWindowRequest) returns (ScreenshotResponse);
rpc CaptureClientArea(CaptureClientAreaRequest) returns (ScreenshotResponse);
rpc CaptureRegion(CaptureRegionRequest) returns (ScreenshotResponse);
```

#### Window Properties

**GetWindowTitle/SetWindowTitle**
```protobuf
rpc GetWindowTitle(GetWindowTitleRequest) returns (WindowTitleResponse);
rpc SetWindowTitle(SetWindowTitleRequest) returns (WindowResponse);
```

**GetWindowClass/GetWindowProcessId**
```protobuf
rpc GetWindowClass(GetWindowClassRequest) returns (WindowClassResponse);
rpc GetWindowProcessId(GetWindowProcessIdRequest) returns (WindowProcessIdResponse);
```

**GetWindowStyles/SetWindowStyles/GetWindowExStyles/SetWindowExStyles**
```protobuf
rpc GetWindowStyles(GetWindowStylesRequest) returns (WindowStylesResponse);
```

#### Z-Order

**BringToTop/SendToBottom/SetWindowZOrder**
```protobuf
rpc BringToTop(BringToTopRequest) returns (WindowResponse);
```

#### Input

**SetFocus/GetFocus**
```protobuf
rpc SetFocus(SetFocusRequest) returns (WindowResponse);
rpc GetFocus(Empty) returns (WindowResponse);
```

**PostMessage/SendMessage**
```protobuf
rpc PostMessage(PostMessageRequest) returns (MessageResponse);
rpc SendMessage(SendMessageRequest) returns (MessageResponse);
```

#### Multi-Monitor

**GetMonitors/GetWindowMonitor**
```protobuf
rpc GetMonitors(Empty) returns (MonitorList);
rpc GetWindowMonitor(GetWindowMonitorRequest) returns (MonitorResponse);
```

---

## Common Types

### HealthCheckResponse
All services implement:
```protobuf
rpc HealthCheck(Empty) returns (HealthCheckResponse);
```

```json
{
  "status": "SERVING_STATUS_SERVING",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "components": {
    "photoshop": { "status": "COMPONENT_STATUS_HEALTHY", "message": "UXP connected" },
    "chrome": { "status": "COMPONENT_STATUS_HEALTHY", "message": "CDP connected to localhost:9222" }
  }
}
```

---

## Client Examples

### Python - Photoshop
```python
import grpc
from hcs_adapters_proto import adapters_pb2, adapters_pb2_grpc

class PhotoshopClient:
    def __init__(self, host="localhost", port=50054):
        self.channel = grpc.insecure_channel(f"{host}:{port}")
        self.stub = adapters_pb2_grpc.PhotoshopServiceStub(self.channel)
    
    def create_doc(self, width, height, name="Doc"):
        req = adapters_pb2.CreateDocumentRequest(
            width=width, height=height, name=name,
            color_mode=adapters_pb2.COLOR_MODE_RGB
        )
        return self.stub.CreateDocument(req)
    
    def add_layer(self, doc_id, name):
        req = adapters_pb2.CreateLayerRequest(
            document_id=doc_id, name=name,
            kind=adapters_pb2.LAYER_KIND_NORMAL
        )
        return self.stub.CreateLayer(req)
    
    def run_action(self, doc_id, action_name, action_set="Default Actions"):
        req = adapters_pb2.PlayActionRequest(
            action_name=action_name, action_set=action_set, document_id=doc_id
        )
        return self.stub.PlayAction(req)

# Usage
ps = PhotoshopClient()
doc = ps.create_doc(1920, 1080, "My Art")
ps.add_layer(doc.document.id, "Background")
ps.run_action(doc.document.id, "Gaussian Blur")
```

### Python - Chrome
```python
class ChromeClient:
    def __init__(self, host="localhost", port=50054):
        self.channel = grpc.insecure_channel(f"{host}:{port}")
        self.stub = adapters_pb2_grpc.ChromeServiceStub(self.channel)
        self.session_id = None
    
    def connect(self, endpoint="http://localhost:9222"):
        req = adapters_pb2.ConnectRequest(endpoint=endpoint)
        resp = self.stub.Connect(req)
        self.session_id = resp.session_id
        return resp
    
    def navigate(self, url):
        req = adapters_pb2.NavigateRequest(
            session_id=self.session_id, url=url,
            options=adapters_pb2.NavigationOptions(wait_until_networkidle=True)
        )
        return self.stub.Navigate(req)
    
    def click(self, selector):
        # Get document, query selector, click
        doc = self.stub.GetDocument(adapters_pb2.GetDocumentRequest(
            session_id=self.session_id, target_id=self.target_id
        ))
        node = self.stub.QuerySelector(adapters_pb2.QuerySelectorRequest(
            session_id=self.session_id, target_id=self.target_id, selector=selector
        ))
        return self.stub.ClickElement(adapters_pb2.ClickElementRequest(
            session_id=self.session_id, target_id=self.target_id, node_id=node.node.node_id
        ))

# Usage
chrome = ChromeClient()
chrome.connect()
chrome.navigate("https://github.com")
chrome.click(".repo-link")
```

### Python - Window
```python
class WindowClient:
    def __init__(self, host="localhost", port=50054):
        self.channel = grpc.insecure_channel(f"{host}:{port}")
        self.stub = adapters_pb2_grpc.WindowServiceStub(self.channel)
    
    def find_window(self, title, class_name=""):
        req = adapters_pb2.FindWindowRequest(window_title=title, class_name=class_name)
        return self.stub.FindWindow(req)
    
    def focus(self, handle):
        return self.stub.FocusWindow(adapters_pb2.FocusWindowRequest(handle=handle))
    
    def move_resize(self, handle, x, y, w, h):
        return self.stub.MoveResizeWindow(adapters_pb2.MoveResizeWindowRequest(
            handle=handle, x=x, y=y, width=w, height=h
        ))
    
    def capture(self, handle):
        return self.stub.CaptureWindow(adapters_pb2.CaptureWindowRequest(handle=handle))

# Usage
win = WindowClient()
notepad = win.find_window("Untitled - Notepad")
win.focus(notepad.window.handle)
win.move_resize(notepad.window.handle, 100, 100, 800, 600)
img = win.capture(notepad.window.handle)
```

---

## Prerequisites

### Photoshop
- Photoshop 2021+ (UXP) or 2015+ (CEP)
- Enable "Allow Extensions" in Preferences
- Install HCS UXP plugin (provided separately)

### Chrome
- Chrome with `--remote-debugging-port=9222`
- Start: `chrome.exe --remote-debugging-port=9222 --user-data-dir=/tmp/chrome-debug`

### Game
- Target game running
- Appropriate permissions (admin for some operations)
- Anti-cheat may block memory access

### Window
- Windows: Native API
- Linux: X11 (Xlib) or Wayland (requires appropriate permissions)

---

## Error Codes

| Code | Description |
|------|-------------|
| `NOT_FOUND` | Document/window/process not found |
| `FAILED_PRECONDITION` | App not connected, invalid state |
| `PERMISSION_DENIED` | Insufficient privileges (game memory) |
| `INVALID_ARGUMENT` | Invalid parameters |
| `UNAVAILABLE` | Target application not running |
| `INTERNAL` | Adapter internal error |

---

## Changelog

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial release with all 4 adapters |
| 1.1.0 | Added batch processing, network interception |
| 1.2.0 | Added Wayland support, smart filter support |