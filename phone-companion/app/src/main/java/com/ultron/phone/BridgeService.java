package com.ultron.phone;

import android.Manifest;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothManager;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageManager;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraManager;
import android.media.AudioManager;
import android.net.wifi.WifiManager;
import android.os.BatteryManager;
import android.os.Build;
import android.os.IBinder;
import android.os.PowerManager;
import android.provider.Settings;
import org.json.JSONObject;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Locale;

public class BridgeService extends Service {
    static volatile boolean running;
    static volatile String lastError = "";
    private volatile ServerSocket server;
    private volatile Socket client;
    private volatile boolean stopped;
    private Thread worker;
    private String torchCamera;

    @Override public IBinder onBind(Intent intent) { return null; }
    @Override public void onCreate() {
        super.onCreate();
        NotificationManager nm = getSystemService(NotificationManager.class);
        nm.createNotificationChannel(new NotificationChannel("connection", "Phone connection", NotificationManager.IMPORTANCE_LOW));
        PendingIntent open = PendingIntent.getActivity(this, 0, new Intent(this, MainActivity.class), PendingIntent.FLAG_IMMUTABLE);
        Intent stop = new Intent(this, BridgeService.class).setAction("stop");
        PendingIntent stopAction = PendingIntent.getService(this, 1, stop, PendingIntent.FLAG_IMMUTABLE);
        Notification notification = new Notification.Builder(this, "connection").setContentTitle("ULTRON phone connection active")
            .setContentText("Commands require your token and authorized ADB. Tap to manage.")
            .setSmallIcon(android.R.drawable.stat_notify_sync).setContentIntent(open).setOngoing(true)
            .addAction(new Notification.Action.Builder(null, "Stop connection", stopAction).build()).build();
        startForeground(1, notification);
        lastError = "";
        worker = new Thread(this::listen, "ultron-phone-bridge"); worker.start();
    }
    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && "stop".equals(intent.getAction())) stopSelf();
        // Explicit activation after reboot/process death; never start silently.
        return START_NOT_STICKY;
    }
    private void listen() {
        try (ServerSocket socket = new ServerSocket(8765, 4, InetAddress.getByName("127.0.0.1"))) {
            server = socket; running = true;
            while (!stopped) {
                try (Socket connection = socket.accept()) {
                    client = connection;
                    connection.setSoTimeout(5000);
                    serve(connection);
                } catch (Exception ignored) { /* No request bodies or tokens in logs. */ }
                finally { client = null; }
            }
        } catch (Exception e) {
            if (!stopped) lastError = "Connection could not start; stop and start again.";
        } finally { running = false; stopSelf(); }
    }
    private String line(InputStream in) throws Exception {
        ByteArrayOutputStream data = new ByteArrayOutputStream(); int c;
        while ((c = in.read()) != -1) {
            if (c == '\n') break;
            if (c != '\r') data.write(c);
            if (data.size() > 8192) throw new IllegalArgumentException("HTTP line too long.");
        }
        if (c == -1) throw new IllegalArgumentException("Incomplete HTTP request.");
        return data.toString(StandardCharsets.US_ASCII.name());
    }
    private void serve(Socket connection) throws Exception {
        InputStream in = connection.getInputStream(); OutputStream out = connection.getOutputStream();
        if (!"POST /command HTTP/1.1".equals(line(in))) { respond(out, 404, new JSONObject().put("ok", false)); return; }
        String auth = ""; int length = -1, headers = 0;
        String header;
        while (!(header = line(in)).isEmpty()) {
            if (++headers > 32) throw new IllegalArgumentException("Too many HTTP headers.");
            int split = header.indexOf(':'); if (split <= 0) throw new IllegalArgumentException("Invalid HTTP header.");
            String name = header.substring(0, split).trim().toLowerCase(Locale.ROOT), value = header.substring(split + 1).trim();
            if ("authorization".equals(name)) auth = value;
            if ("content-length".equals(name)) { if (length != -1) throw new IllegalArgumentException("Duplicate length."); length = Integer.parseInt(value); }
            if ("transfer-encoding".equals(name)) throw new IllegalArgumentException("Chunked requests are unsupported.");
        }
        String expected = "Bearer " + MainActivity.token(this);
        if (!MessageDigest.isEqual(auth.getBytes(StandardCharsets.UTF_8), expected.getBytes(StandardCharsets.UTF_8))) {
            respond(out, 401, new JSONObject().put("ok", false)); return;
        }
        if (length < 0 || length > 262144) { respond(out, 413, new JSONObject().put("ok", false)); return; }
        byte[] body = new byte[length]; int offset = 0;
        while (offset < length) {
            int read = in.read(body, offset, length - offset);
            if (read < 0) throw new IllegalArgumentException("Incomplete HTTP body."); offset += read;
        }
        JSONObject response;
        try {
            JSONObject request = new JSONObject(new String(body, StandardCharsets.UTF_8));
            Object data = dispatch(request);
            response = new JSONObject().put("ok", true).put("data", data);
        } catch (Exception e) {
            Throwable cause = e;
            while (cause.getCause() != null) cause = cause.getCause();
            String error = cause instanceof SecurityException ? "Android permission denied. Enable the required access in ULTRON Phone settings." : cause.getMessage();
            response = new JSONObject().put("ok", false).put("error", error == null ? "Phone command failed." : error);
        }
        respond(out, 200, response);
    }
    private void respond(OutputStream out, int status, JSONObject data) throws Exception {
        byte[] body = data.toString().getBytes(StandardCharsets.UTF_8);
        out.write(("HTTP/1.1 " + status + " Response\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: "
            + body.length + "\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n").getBytes(StandardCharsets.US_ASCII));
        out.write(body); out.flush();
    }
    private PhoneAccessibility accessibility() {
        PhoneAccessibility service = PhoneAccessibility.instance;
        if (service == null) throw new IllegalStateException("Enable ULTRON Phone control in Android Accessibility settings.");
        return service;
    }
    private PhoneNotifications notifications() {
        PhoneNotifications service = PhoneNotifications.instance;
        if (service == null) throw new IllegalStateException("Enable ULTRON Phone notification access in Android settings.");
        return service;
    }
    private Object dispatch(JSONObject p) throws Exception {
        if (stopped) throw new IllegalStateException("Phone connection is stopped.");
        switch (p.getString("command")) {
            case "status": return status();
            case "test_notification": {
                NotificationTestReceiver.post(this);
                return new JSONObject().put("posted", true);
            }
            case "test_reply_status": return new JSONObject().put("reply", NotificationTestReceiver.lastReply);
            case "apps": {
                org.json.JSONArray apps = new org.json.JSONArray();
                Intent launch = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER);
                java.util.Set<String> seen = new java.util.HashSet<>();
                for (android.content.pm.ResolveInfo info : getPackageManager().queryIntentActivities(launch, 0)) {
                    if (seen.add(info.activityInfo.packageName)) apps.put(new JSONObject()
                        .put("package", info.activityInfo.packageName).put("label", info.loadLabel(getPackageManager()).toString()));
                }
                return apps;
            }
            case "screen": return accessibility().screen();
            case "global": {
                final int action;
                switch (p.getString("action")) {
                    case "back": action = android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_BACK; break;
                    case "home": action = android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME; break;
                    case "recent": action = android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_RECENTS; break;
                    default: throw new IllegalArgumentException("Unsupported global navigation action.");
                }
                PhoneAccessibility control = accessibility();
                if (!control.onMain(() -> control.performGlobalAction(action))) throw new IllegalStateException("Navigation action is unavailable on this screen.");
                return new JSONObject().put("navigation_completed", true);
            }
            case "text": return accessibility().type(p.getString("text"), p.optString("expected_package", ""), p.has("expected_text") ? p.getString("expected_text") : null);
            case "tap": return accessibility().tap(p.optString("description", ""), p.optInt("index", 0), p.optBoolean("long_press", false), p.optString("expected_package", ""));
            case "scroll": return accessibility().scroll(p.getString("direction"), p.optInt("count", 1), p.optString("expected_package", ""));
            case "notifications": return notifications().list();
            case "reply": return notifications().reply(p.getString("key"), p.getString("message"));
            case "volume": {
                int percent = p.getInt("percent");
                if (percent < 0 || percent > 100) throw new IllegalArgumentException("Volume percent must be 0 to 100.");
                AudioManager audio = getSystemService(AudioManager.class);
                int value = Math.round(percent * audio.getStreamMaxVolume(AudioManager.STREAM_MUSIC) / 100f);
                audio.setStreamVolume(AudioManager.STREAM_MUSIC, value, 0);
                if (audio.getStreamVolume(AudioManager.STREAM_MUSIC) != value) throw new IllegalStateException("Could not verify media volume.");
                return new JSONObject().put("volume", value);
            }
            case "flashlight": {
                CameraManager cameras = getSystemService(CameraManager.class);
                if (torchCamera == null) for (String id : cameras.getCameraIdList()) {
                    CameraCharacteristics c = cameras.getCameraCharacteristics(id);
                    if (Boolean.TRUE.equals(c.get(CameraCharacteristics.FLASH_INFO_AVAILABLE))) { torchCamera = id; break; }
                }
                if (torchCamera == null) throw new IllegalStateException("This phone has no available flashlight.");
                cameras.setTorchMode(torchCamera, p.getBoolean("enabled"));
                return new JSONObject().put("flashlight_requested", p.getBoolean("enabled"));
            }
            default: throw new IllegalArgumentException("Unsupported phone command.");
        }
    }
    @SuppressWarnings("deprecation")
    private JSONObject status() throws Exception {
        Intent battery = registerReceiver(null, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
        JSONObject result = new JSONObject().put("source", "companion").put("accessibility_enabled", PhoneAccessibility.instance != null)
            .put("notification_access", PhoneNotifications.instance != null);
        if (battery != null) {
            int level = battery.getIntExtra(BatteryManager.EXTRA_LEVEL, -1), scale = battery.getIntExtra(BatteryManager.EXTRA_SCALE, 100);
            int state = battery.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
            result.put("battery_percent", level >= 0 && scale > 0 ? Math.round(level * 100f / scale) : JSONObject.NULL)
                .put("charging", state == BatteryManager.BATTERY_STATUS_CHARGING || state == BatteryManager.BATTERY_STATUS_FULL);
        }
        WifiManager wifi = (WifiManager) getApplicationContext().getSystemService(WIFI_SERVICE);
        result.put("wifi_enabled", wifi != null && wifi.isWifiEnabled());
        BluetoothManager bluetooth = getSystemService(BluetoothManager.class);
        BluetoothAdapter adapter = bluetooth == null ? null : bluetooth.getAdapter();
        boolean canReadBluetooth = Build.VERSION.SDK_INT < 31 || checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED;
        result.put("bluetooth_enabled", canReadBluetooth && adapter != null ? adapter.isEnabled() : JSONObject.NULL);
        result.put("brightness_raw", Settings.System.getInt(getContentResolver(), Settings.System.SCREEN_BRIGHTNESS, -1));
        result.put("screen_awake", getSystemService(PowerManager.class).isInteractive());
        return result;
    }
    @Override public void onDestroy() {
        stopped = true; running = false;
        try { if (client != null) client.close(); } catch (Exception ignored) { }
        try { if (server != null) server.close(); } catch (Exception ignored) { }
        if (worker != null) worker.interrupt();
        if (torchCamera != null) try { getSystemService(CameraManager.class).setTorchMode(torchCamera, false); } catch (Exception ignored) { }
        stopForeground(STOP_FOREGROUND_REMOVE);
        super.onDestroy();
    }
}
