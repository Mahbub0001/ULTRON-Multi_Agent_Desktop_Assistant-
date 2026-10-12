package com.ultron.phone;

import android.Manifest;
import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.EditText;
import java.security.SecureRandom;

public class MainActivity extends Activity {
    private TextView status;
    static String token(android.content.Context context) {
        SharedPreferences prefs = context.getSharedPreferences("bridge", MODE_PRIVATE);
        String token = prefs.getString("token", "");
        if (token.isEmpty()) {
            byte[] bytes = new byte[32];
            new SecureRandom().nextBytes(bytes);
            StringBuilder text = new StringBuilder();
            for (byte value : bytes) text.append(String.format(java.util.Locale.ROOT, "%02x", value & 255));
            token = text.toString();
            prefs.edit().putString("token", token).apply();
        }
        return token;
    }

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        // The token belongs only to the owner; exclude it from screenshots.
        getWindow().addFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE);
        ScrollView scroll = new ScrollView(this);
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        int padding = (int) (24 * getResources().getDisplayMetrics().density);
        layout.setPadding(padding, padding, padding, padding);
        scroll.addView(layout);
        setContentView(scroll);
        TextView title = new TextView(this);
        title.setText("ULTRON Phone"); title.setTextSize(28); layout.addView(title);
        TextView help = new TextView(this);
        help.setText("Connect your phone through USB or paired Wireless Debugging. Copy the private token into the desktop's Android Phone Control settings.\n\nStart connection enables authenticated phone commands. Screen control and notifications are optional permissions. Notification replies are reviewed in the desktop confirmation panel. No notification history is stored by this app.");
        layout.addView(help);
        status = new TextView(this); status.setTextSize(16); layout.addView(status);
        TextView tokenText = new TextView(this);
        token(this);
        tokenText.setText("Private token ready. Use Copy connection token."); layout.addView(tokenText);
        EditText testInput = new EditText(this);
        testInput.setHint("Test phone text entry (optional)");
        testInput.setContentDescription("Phone control test field"); layout.addView(testInput);
        button(layout, "Copy connection token", v -> {
            ((ClipboardManager) getSystemService(CLIPBOARD_SERVICE)).setPrimaryClip(ClipData.newPlainText("ULTRON token", token(this)));
            status.setText("Token copied. Keep it private.");
        });
        button(layout, "Start connection", v -> {
            startForegroundService(new Intent(this, BridgeService.class));
            status.postDelayed(this::refresh, 400);
        });
        button(layout, "Stop connection", v -> { stopService(new Intent(this, BridgeService.class)); status.postDelayed(this::refresh, 400); });
        button(layout, "Enable screen control", v -> startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)));
        button(layout, "Enable notification access", v -> startActivity(new Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)));
        button(layout, "Allow flashlight / Bluetooth / alerts", v -> {
            if (Build.VERSION.SDK_INT >= 33)
                requestPermissions(new String[]{Manifest.permission.CAMERA, Manifest.permission.BLUETOOTH_CONNECT, Manifest.permission.POST_NOTIFICATIONS}, 1);
            else if (Build.VERSION.SDK_INT >= 31)
                requestPermissions(new String[]{Manifest.permission.CAMERA, Manifest.permission.BLUETOOTH_CONNECT}, 1);
            else requestPermissions(new String[]{Manifest.permission.CAMERA}, 1);
        });
        button(layout, "Reset connection token", v -> {
            stopService(new Intent(this, BridgeService.class));
            getSharedPreferences("bridge", MODE_PRIVATE).edit().remove("token").apply();
            token(this);
            tokenText.setText("Private token reset. Use Copy connection token.");
            status.setText("Token reset; save the new token on the desktop and start connection again.");
        });
        button(layout, "Post local test notification", v -> {
            try { NotificationTestReceiver.post(this); status.setText("Local test notification posted. No message is sent to another person."); }
            catch (Exception e) { status.setText("Grant notification permission to post the test notification."); }
        });
    }
    private void button(LinearLayout layout, String label, View.OnClickListener click) {
        Button button = new Button(this); button.setText(label); button.setOnClickListener(click); layout.addView(button);
    }
    private void refresh() {
        status.setText("Connection: " + (BridgeService.running ? "active" : "stopped") + "\nScreen control: "
            + (PhoneAccessibility.instance != null ? "enabled" : "disabled") + "\nNotifications: "
            + (PhoneNotifications.instance != null ? "enabled" : "disabled")
            + (BridgeService.lastError.isEmpty() ? "" : "\n" + BridgeService.lastError));
    }
    @Override public void onResume() { super.onResume(); refresh(); }
}
