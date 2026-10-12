package com.ultron.phone;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.GestureDescription;
import android.graphics.Path;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import org.json.JSONArray;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.concurrent.Callable;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.FutureTask;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

@SuppressWarnings("deprecation")
public class PhoneAccessibility extends AccessibilityService {
    static volatile PhoneAccessibility instance;
    private final Handler main = new Handler(Looper.getMainLooper());
    @Override protected void onServiceConnected() { instance = this; }
    @Override public void onAccessibilityEvent(AccessibilityEvent event) { }
    @Override public void onInterrupt() { }
    @Override public void onDestroy() { if (instance == this) instance = null; super.onDestroy(); }

    <T> T onMain(Callable<T> work) throws Exception {
        FutureTask<T> task = new FutureTask<>(work);
        main.post(task);
        try { return task.get(4, TimeUnit.SECONDS); }
        catch (Exception e) { task.cancel(false); throw e; }
    }
    private boolean eligible(AccessibilityNodeInfo n) {
        return n.isVisibleToUser() && n.isEnabled() && !n.isPassword()
            && (n.getText() != null || n.getContentDescription() != null || n.isClickable() || n.isScrollable() || n.isEditable());
    }
    private void collect(AccessibilityNodeInfo n, List<AccessibilityNodeInfo> nodes, int depth, int[] budget) {
        if (n == null || depth > 50 || --budget[0] < 0 || nodes.size() >= 200 || !n.refresh() || n.isPassword()) return;
        if (eligible(n)) nodes.add(AccessibilityNodeInfo.obtain(n));
        for (int i = 0; i < n.getChildCount() && nodes.size() < 200; i++) {
            AccessibilityNodeInfo child = n.getChild(i);
            if (child != null) { collect(child, nodes, depth + 1, budget); child.recycle(); }
        }
    }
    private List<AccessibilityNodeInfo> nodes() {
        AccessibilityNodeInfo root = getRootInActiveWindow();
        if (root == null) throw new IllegalStateException("No active screen. Wake/unlock the phone first.");
        List<AccessibilityNodeInfo> nodes = new ArrayList<>();
        try { collect(root, nodes, 0, new int[]{1500}); }
        finally { root.recycle(); }
        return nodes;
    }
    private void release(List<AccessibilityNodeInfo> nodes) { for (AccessibilityNodeInfo n : nodes) n.recycle(); }
    private String text(CharSequence value) {
        String s = value == null ? "" : value.toString();
        return s.substring(0, Math.min(s.length(), 2000));
    }
    JSONArray screen() throws Exception {
        return onMain(() -> {
            List<AccessibilityNodeInfo> nodes = nodes();
            JSONArray result = new JSONArray();
            try {
                for (int i = 0; i < nodes.size(); i++) {
                    AccessibilityNodeInfo n = nodes.get(i); Rect b = new Rect(); n.getBoundsInScreen(b);
                    result.put(new JSONObject().put("index", i + 1).put("text", text(n.getText()))
                        .put("description", text(n.getContentDescription())).put("resource_id", text(n.getViewIdResourceName()))
                        .put("class_name", text(n.getClassName())).put("clickable", n.isClickable()).put("scrollable", n.isScrollable())
                        .put("package", text(n.getPackageName()))
                        .put("bounds", "[" + b.left + "," + b.top + "][" + b.right + "," + b.bottom + "]"));
                }
            } finally { release(nodes); }
            return result;
        });
    }
    private int score(AccessibilityNodeInfo n, String query) {
        String t = text(n.getText()).toLowerCase(Locale.ROOT), d = text(n.getContentDescription()).toLowerCase(Locale.ROOT),
            r = text(n.getViewIdResourceName()).toLowerCase(Locale.ROOT);
        if (query.isEmpty() || !(t.contains(query) || d.contains(query) || r.contains(query))) return -1;
        return (t.equals(query) || d.equals(query) ? 2 : 0) + (n.isClickable() ? 1 : 0);
    }
    private void ensurePackage(String expected) {
        if (expected.isEmpty()) return;
        AccessibilityNodeInfo root = getRootInActiveWindow();
        if (root == null) throw new IllegalStateException("No active screen.");
        try {
            if (!expected.equals(text(root.getPackageName())))
                throw new IllegalStateException("Foreground app changed; no action performed. Inspect the screen again.");
        } finally { root.recycle(); }
    }
    JSONObject tap(String description, int index, boolean longPress, String expectedPackage) throws Exception {
        final Rect bounds = new Rect();
        final String[] chosenPackage = new String[]{expectedPackage};
        boolean clicked = onMain(() -> {
            ensurePackage(expectedPackage);
            List<AccessibilityNodeInfo> nodes = nodes();
            try {
                AccessibilityNodeInfo chosen = null;
                if (index > 0) {
                    if (index > nodes.size()) throw new IllegalArgumentException("Element index is no longer visible. Inspect the screen again.");
                    chosen = nodes.get(index - 1);
                } else {
                    String query = description.trim().toLowerCase(Locale.ROOT); int best = -1;
                    for (AccessibilityNodeInfo n : nodes) {
                        int score = score(n, query);
                        if (score > best) { chosen = n; best = score; }
                    }
                }
                if (chosen == null) throw new IllegalArgumentException("No visible element matches that description.");
                chosen.getBoundsInScreen(bounds);
                chosenPackage[0] = text(chosen.getPackageName());
                int action = longPress ? AccessibilityNodeInfo.ACTION_LONG_CLICK : AccessibilityNodeInfo.ACTION_CLICK;
                if (chosen.performAction(action)) return true;
                AccessibilityNodeInfo parent = chosen.getParent();
                for (int i = 0; parent != null && i < 5; i++) {
                    AccessibilityNodeInfo next = null;
                    try {
                        if (!parent.isPassword()) {
                            if (parent.performAction(action)) return true;
                            next = parent.getParent();
                        }
                    } finally { parent.recycle(); }
                    parent = next;
                }
                if (parent != null) parent.recycle();
                return false;
            } finally { release(nodes); }
        });
        if (!clicked) {
            if (bounds.isEmpty()) throw new IllegalStateException("Element has no visible bounds.");
            gesture(bounds.centerX(), bounds.centerY(), bounds.centerX(), bounds.centerY(), longPress ? 800 : 80, chosenPackage[0]);
        }
        return new JSONObject().put("action_completed", true);
    }
    JSONObject type(String value, String expectedPackage, String expectedText) throws Exception {
        if (value.length() > 16000) throw new IllegalArgumentException("Text must contain at most 16000 characters.");
        return onMain(() -> {
            ensurePackage(expectedPackage);
            AccessibilityNodeInfo root = getRootInActiveWindow();
            if (root == null) throw new IllegalStateException("No active screen.");
            AccessibilityNodeInfo input = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
            root.recycle();
            if (input == null) throw new IllegalStateException("Tap an editable text field before typing.");
            try {
                if (!input.refresh()) throw new IllegalStateException("Focused control is no longer available.");
                if (input.isPassword()) throw new IllegalStateException("Password entry is excluded from phone control.");
                if (!input.isEditable()) throw new IllegalStateException("Focused control is not editable.");
                String currentText = input.getText() == null ? "" : input.getText().toString();
                if (expectedText != null && !expectedText.equals(currentText))
                    throw new IllegalStateException("Focused text changed; no text was cleared.");
                Bundle args = new Bundle(); args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, value);
                if (!input.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)) throw new IllegalStateException("The app rejected text entry.");
            } finally { input.recycle(); }
            return new JSONObject().put("text_entered", true);
        });
    }
    JSONObject scroll(String direction, int count, String expectedPackage) throws Exception {
        if (count < 1 || count > 10) throw new IllegalArgumentException("Scroll count must be 1 to 10.");
        final int action;
        switch (direction) {
            case "down": action = AccessibilityNodeInfo.ACTION_SCROLL_FORWARD; break;
            case "up": action = AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD; break;
            case "left": action = AccessibilityNodeInfo.AccessibilityAction.ACTION_SCROLL_LEFT.getId(); break;
            case "right": action = AccessibilityNodeInfo.AccessibilityAction.ACTION_SCROLL_RIGHT.getId(); break;
            default: throw new IllegalArgumentException("Unsupported scroll direction.");
        }
        for (int i = 0; i < count; i++) {
            boolean scrolled = onMain(() -> {
                ensurePackage(expectedPackage);
                List<AccessibilityNodeInfo> nodes = nodes();
                try { for (AccessibilityNodeInfo n : nodes) if (n.isScrollable() && n.performAction(action)) return true; }
                finally { release(nodes); }
                return false;
            });
            if (!scrolled) {
                android.util.DisplayMetrics m = getResources().getDisplayMetrics();
                float w = m.widthPixels, h = m.heightPixels;
                switch (direction) {
                    case "down": gesture(w / 2, h * .75f, w / 2, h * .25f, 400, expectedPackage); break;
                    case "up": gesture(w / 2, h * .25f, w / 2, h * .75f, 400, expectedPackage); break;
                    case "left": gesture(w * .25f, h / 2, w * .75f, h / 2, 400, expectedPackage); break;
                    case "right": gesture(w * .75f, h / 2, w * .25f, h / 2, 400, expectedPackage); break;
                }
            }
            Thread.sleep(180);
        }
        return new JSONObject().put("scroll_completed", true);
    }
    private void gesture(float x1, float y1, float x2, float y2, int duration, String expectedPackage) throws Exception {
        CountDownLatch done = new CountDownLatch(1); AtomicBoolean success = new AtomicBoolean(false);
        main.post(() -> {
            try { ensurePackage(expectedPackage); }
            catch (IllegalStateException e) { done.countDown(); return; }
            Path path = new Path(); path.moveTo(x1, y1); path.lineTo(x2, y2);
            GestureDescription gesture = new GestureDescription.Builder()
                .addStroke(new GestureDescription.StrokeDescription(path, 0, duration)).build();
            boolean accepted = dispatchGesture(gesture, new GestureResultCallback() {
                @Override public void onCompleted(GestureDescription g) { success.set(true); done.countDown(); }
                @Override public void onCancelled(GestureDescription g) { done.countDown(); }
            }, main);
            if (!accepted) done.countDown();
        });
        if (!done.await(3, TimeUnit.SECONDS) || !success.get()) throw new IllegalStateException("Phone gesture was cancelled or timed out.");
    }
}
